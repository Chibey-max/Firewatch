//! `firewatch`: check nodes against the advisory database.
//!
//! Nodes come from `firewatch.toml` (live, read-only connectors) or from a
//! JSON snapshot (a list of `NodeInfo`, e.g. `examples/nodes.json`). The
//! advisories, release table and dashboard are built in; flags override them.

mod config;
mod embedded;
mod init;
mod outbox;
mod state;
mod watch;

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use firewatch_alerts::NostrAlerter;
use firewatch_connectors::NodeConfig;
use firewatch_core::{
    exposure, load_dir, short_commit, Advisory, BuildCheck, ExposureStatus, NodeInfo, NodeReport,
    ReleaseTable, Report, Status,
};

use crate::config::Config;

#[derive(Parser)]
#[command(
    name = "firewatch",
    version,
    about = "Security alerts for Lightning nodes"
)]
struct Cli {
    /// Directory of advisory TOML files, instead of the built-in advisories.
    #[arg(long, global = true)]
    advisories: Option<PathBuf>,

    /// Official LND release table for the build check, instead of the
    /// built-in one. Regenerate it with scripts/lnd-releases.sh.
    #[arg(long, global = true)]
    releases: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check nodes against the advisories.
    ///
    /// Exit code: 0 nothing found, 1 error, 2 something can't be told, 3 affected.
    Check {
        /// Node config with read-only credentials. Default: ./firewatch.toml if
        /// it exists, else ~/.config/firewatch/firewatch.toml.
        #[arg(long, conflicts_with = "nodes")]
        config: Option<PathBuf>,
        /// Read nodes from a JSON snapshot instead of connecting to them.
        #[arg(long)]
        nodes: Option<PathBuf>,
        /// Print the report as JSON (the dashboard format).
        #[arg(long)]
        json: bool,
        /// Also run advisories not yet verified against an official source.
        #[arg(long)]
        include_unverified: bool,
    },
    /// Re-check every N seconds, alert on changes, and serve the dashboard API.
    Watch {
        /// Default: ./firewatch.toml if it exists, else ~/.config/firewatch/firewatch.toml.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Also run advisories not yet verified against an official source.
        #[arg(long)]
        include_unverified: bool,
        /// Allow `listen` on a non-loopback address. The report tells anyone
        /// who can reach it which nodes are vulnerable.
        #[arg(long)]
        allow_remote: bool,
        /// Serve a dashboard from this folder instead of the built-in one.
        #[arg(long)]
        dashboard: Option<PathBuf>,
    },
    /// Send a test alert to check that it reaches your phone.
    TestAlert {
        /// Default: ./firewatch.toml if it exists, else ~/.config/firewatch/firewatch.toml.
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Find your node, set up a read-only connection and write the config.
    Init {
        /// Where to write it. Default: ~/.config/firewatch/firewatch.toml.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Replace an existing config.
        #[arg(long)]
        force: bool,
    },
    /// Load and validate every advisory file, then list them.
    Validate,
}

#[tokio::main]
async fn main() -> ExitCode {
    firewatch_connectors::install_crypto_provider();
    match run().await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(1)
        }
    }
}

async fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    if let Command::Init { config, force } = cli.command {
        init::run(config, force).await?;
        return Ok(ExitCode::SUCCESS);
    }
    let advisories = match &cli.advisories {
        Some(dir) => {
            load_dir(dir).with_context(|| format!("loading advisories from {}", dir.display()))?
        }
        None => embedded::advisories().context("loading the built-in advisories")?,
    };
    let releases = Some(match &cli.releases {
        Some(path) => load_releases(path)?,
        None => embedded::releases().context("loading the built-in release table")?,
    });

    match cli.command {
        Command::Validate => {
            let width = advisories
                .iter()
                .map(|a| a.affected.to_string().len())
                .max()
                .unwrap_or(0);
            for a in &advisories {
                let mark = if a.verified {
                    "verified  "
                } else {
                    "UNVERIFIED"
                };
                println!(
                    "{} {mark} {:<4} {:<width$} {}",
                    a.id,
                    a.implementation.as_str(),
                    a.affected.to_string(),
                    a.title
                );
            }
            let from = cli
                .advisories
                .as_ref()
                .map_or("built in".to_string(), |d| d.display().to_string());
            println!("{} advisories OK ({from})", advisories.len());
            Ok(ExitCode::SUCCESS)
        }
        Command::Check {
            config,
            nodes,
            json,
            include_unverified,
        } => {
            let advisories = select(advisories, include_unverified);
            let (report, warnings) = match nodes {
                Some(path) => (
                    Report::build(
                        &load_snapshot(&path)?,
                        &advisories,
                        releases.as_ref(),
                        now(),
                    ),
                    Vec::new(),
                ),
                None => {
                    check_live(
                        &Config::load(&config::resolve(config))?.nodes,
                        &advisories,
                        releases.as_ref(),
                    )
                    .await
                }
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print_report(&report);
            }
            // After the report, so they're the last thing on screen; on
            // stderr, so `--json` output stays clean.
            for warning in warnings {
                eprintln!("warning: {warning}");
            }
            Ok(ExitCode::from(match report.worst() {
                Status::NotAffected => 0,
                Status::CantTell => 2,
                Status::Affected => 3,
            }))
        }
        Command::Watch {
            config,
            include_unverified,
            allow_remote,
            dashboard,
        } => {
            let config_path = config::resolve(config);
            let mut config = Config::load(&config_path)?;
            if dashboard.is_some() {
                config.watch.dashboard_dir = dashboard;
            }
            watch::run(
                config,
                &config_path,
                select(advisories, include_unverified),
                releases,
                allow_remote,
            )
            .await?;
            Ok(ExitCode::SUCCESS)
        }
        Command::TestAlert { config } => {
            let config = Config::load(&config::resolve(config))?;
            let text = test_alert_text(config.alerts.strings()?);
            let nostr = config
                .alerts
                .nostr
                .context("no [alerts.nostr] section in the config")?;
            let alerter = NostrAlerter::connect(&nostr).await?;
            let relays = alerter.send(&text).await?;
            println!(
                "Sent from {} to {} relay(s): {}",
                alerter.sender(),
                relays,
                alerter
                    .relays()
                    .iter()
                    .map(|r| r.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            Ok(ExitCode::SUCCESS)
        }
        Command::Init { .. } => unreachable!("handled above"),
    }
}

/// Only verified advisories run unless asked otherwise.
fn select(advisories: Vec<Advisory>, include_unverified: bool) -> Vec<Advisory> {
    advisories
        .into_iter()
        .filter(|a| include_unverified || a.verified)
        .collect()
}

/// Reads every configured node. A node that can't be read is reported as
/// "Can't tell" with its error, so one dead node doesn't hide the others.
/// Also returns the connectors' warnings, which are not part of the report.
pub(crate) async fn check_live(
    configs: &[NodeConfig],
    advisories: &[Advisory],
    releases: Option<&ReleaseTable>,
) -> (Report, Vec<String>) {
    let http = firewatch_connectors::http_client();
    // All nodes at once, so one node timing out doesn't delay the rest.
    let reads = futures::future::join_all(
        configs
            .iter()
            .map(|node| firewatch_connectors::fetch(node, &http)),
    )
    .await;
    let mut warnings = Vec::new();
    let nodes = configs
        .iter()
        .zip(reads)
        .map(|(node, read)| match read {
            Ok(info) => {
                warnings.extend(info.warnings.iter().cloned());
                NodeReport::from_node(&info, advisories, releases)
            }
            Err(e) => NodeReport::unreachable(node.name(), node.implementation(), e.to_string()),
        })
        .collect();
    let report = Report {
        checked_at: now(),
        nodes,
    };
    (report, warnings)
}

/// The release table. A missing file stops the run, as a missing advisory
/// directory does: without it an unofficial LND build would get normal
/// verdicts, and nothing on screen would say the build check was off.
fn load_releases(path: &Path) -> Result<ReleaseTable> {
    ReleaseTable::load(path).with_context(|| {
        format!(
            "loading the release table from {} (regenerate it with scripts/lnd-releases.sh, or pass --releases)",
            path.display()
        )
    })
}

fn load_snapshot(path: &Path) -> Result<Vec<NodeInfo>> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The `test-alert` message, marked as a draft if the language isn't reviewed.
fn test_alert_text(strings: &firewatch_alerts::Strings) -> String {
    if strings.reviewed {
        strings.test_alert.to_string()
    } else {
        format!(
            "[Draft {} translation, not reviewed yet]\n{}",
            strings.english_name, strings.test_alert
        )
    }
}

/// A Unix time as this machine's local time, short: `14:02` today, or
/// `27 Oct 14:02` on another day.
pub(crate) fn local_time(secs: u64) -> String {
    use chrono::{Local, TimeZone};
    let Some(at) = Local.timestamp_opt(secs as i64, 0).single() else {
        return format!("{secs}");
    };
    if at.date_naive() == Local::now().date_naive() {
        at.format("%H:%M").to_string()
    } else {
        at.format("%-d %b %H:%M").to_string()
    }
}

/// One timestamped line on stderr, in local time like the alerts.
pub(crate) fn log(line: &str) {
    eprintln!("[{}] {line}", chrono::Local::now().format("%H:%M:%S"));
}

fn print_report(report: &Report) {
    let color = std::io::stdout().is_terminal();
    let paint = |status: Status| {
        let (code, label) = match status {
            Status::Affected => ("31", "AFFECTED    "),
            Status::CantTell => ("33", "CAN'T TELL  "),
            Status::NotAffected => ("32", "NOT AFFECTED"),
        };
        if color {
            format!("\x1b[1;{code}m{label}\x1b[0m")
        } else {
            label.to_string()
        }
    };

    for node in &report.nodes {
        println!(
            "{}  {} ({} {})",
            paint(node.status),
            node.name,
            node.implementation,
            node.version.as_deref().unwrap_or("version unknown")
        );
        match &node.build_check {
            BuildCheck::Match => println!("    Build: the official release commit."),
            BuildCheck::Mismatch {
                release,
                expected,
                actual,
                build,
            } if expected.eq_ignore_ascii_case(actual) => println!(
                "    Build: NOT a clean {release} release (official commit, but built as \"{}\").",
                build.as_deref().unwrap_or_default()
            ),
            BuildCheck::Mismatch {
                release,
                expected,
                actual,
                ..
            } => println!(
                "    Build: NOT the official {release} release (commit {}, expected {}).",
                short_commit(actual),
                short_commit(expected)
            ),
            BuildCheck::Unreleased { release, build } => println!(
                "    Build: NOT the {release} release (\"{build}\" is a later commit)."
            ),
            BuildCheck::Modified { release, build } => println!(
                "    Build: {release} with local changes (\"{build}\"); nothing can be confirmed safe."
            ),
            BuildCheck::UnknownRelease => {
                println!("    Build: release not in Firewatch's table, so not verified.")
            }
            BuildCheck::NotChecked => {}
        }
        // Not a verdict: it never changes the node's colour, so it's shown
        // apart from the findings, and only when it needs a look.
        for e in node
            .exposure
            .iter()
            .filter(|e| e.status != ExposureStatus::Local)
        {
            println!("    Exposure: {}", e.reason);
            if let Some(check) = &e.manual_check {
                println!("        Check by hand: {check}");
            }
        }
        if let Some(error) = &node.error {
            println!("    Could not read this node: {error}");
        } else if node.findings.is_empty() {
            println!("    No advisories apply to this node.");
        }
        for f in &node.findings {
            println!("    {}  {}  {}", paint(f.status), f.advisory_id, f.title);
            println!("        {}", f.reason);
            for check in &f.manual_checks {
                println!("        Check by hand: {check}");
            }
            if f.status == Status::Affected {
                println!("        Developers say: \"{}\"", f.steps);
            }
            if f.status != Status::NotAffected {
                println!("        Source: {}", f.source);
            }
        }
        println!();
    }
    let exposed = report
        .nodes
        .iter()
        .flat_map(|n| &n.exposure)
        .any(|e| e.status != ExposureStatus::Local);
    if exposed {
        println!(
            "Why exposure matters: a BTCPay Server flaw let attackers obtain LND macaroons, and funds were stolen. \"{}\" {}",
            exposure::QUOTE,
            exposure::SOURCE
        );
    }
}
