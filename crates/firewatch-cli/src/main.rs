//! `firewatch`: check nodes against the advisory database.
//!
//! Nodes come from `firewatch.toml` (live, read-only connectors) or from a
//! JSON snapshot (a list of `NodeInfo`, e.g. `examples/nodes.json`).

mod config;
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
    load_dir, short_commit, Advisory, BuildCheck, NodeInfo, NodeReport, ReleaseTable, Report,
    Status,
};

use crate::config::Config;

#[derive(Parser)]
#[command(
    name = "firewatch",
    version,
    about = "Security alerts for Lightning nodes"
)]
struct Cli {
    /// Directory of advisory TOML files.
    #[arg(long, global = true, default_value = "advisories")]
    advisories: PathBuf,

    /// Official release table for the build check. Required, like the
    /// advisories; regenerate it with scripts/lnd-releases.sh.
    #[arg(long, global = true, default_value = "releases/lnd.toml")]
    releases: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check nodes against the advisories.
    ///
    /// Exit code: 0 nothing found, 1 error, 2 something can't be told, 3 affected.
    Check {
        /// Node config with read-only credentials.
        #[arg(long, default_value = "firewatch.toml", conflicts_with = "nodes")]
        config: PathBuf,
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
        #[arg(long, default_value = "firewatch.toml")]
        config: PathBuf,
        /// Also run advisories not yet verified against an official source.
        #[arg(long)]
        include_unverified: bool,
        /// Allow `listen` on a non-loopback address. The report tells anyone
        /// who can reach it which nodes are vulnerable.
        #[arg(long)]
        allow_remote: bool,
        /// Serve a built dashboard at `/`, overriding `dashboard_dir`.
        #[arg(long)]
        dashboard: Option<PathBuf>,
    },
    /// Send a test alert to check that it reaches your phone.
    TestAlert {
        #[arg(long, default_value = "firewatch.toml")]
        config: PathBuf,
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
    let advisories = load_dir(&cli.advisories)
        .with_context(|| format!("loading advisories from {}", cli.advisories.display()))?;
    let releases = Some(load_releases(&cli.releases)?);

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
            println!("{} advisories OK", advisories.len());
            Ok(ExitCode::SUCCESS)
        }
        Command::Check {
            config,
            nodes,
            json,
            include_unverified,
        } => {
            let advisories = select(advisories, include_unverified);
            let report = match nodes {
                Some(path) => Report::build(
                    &load_snapshot(&path)?,
                    &advisories,
                    releases.as_ref(),
                    now(),
                ),
                None => {
                    check_live(
                        &Config::load(&config)?.nodes,
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
            let mut config = Config::load(&config)?;
            if dashboard.is_some() {
                config.watch.dashboard_dir = dashboard;
            }
            watch::run(
                config,
                select(advisories, include_unverified),
                releases,
                allow_remote,
            )
            .await?;
            Ok(ExitCode::SUCCESS)
        }
        Command::TestAlert { config } => {
            let config = Config::load(&config)?;
            let nostr = config
                .alerts
                .nostr
                .context("no [alerts.nostr] section in the config")?;
            let alerter = NostrAlerter::connect(&nostr).await?;
            let relays = alerter
                .send("Firewatch test alert: if you can read this, alerts reach you.")
                .await?;
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
pub(crate) async fn check_live(
    configs: &[NodeConfig],
    advisories: &[Advisory],
    releases: Option<&ReleaseTable>,
) -> Report {
    let http = firewatch_connectors::http_client();
    // All nodes at once, so one node timing out doesn't delay the rest.
    let reads = futures::future::join_all(
        configs
            .iter()
            .map(|node| firewatch_connectors::fetch(node, &http)),
    )
    .await;
    let nodes = configs
        .iter()
        .zip(reads)
        .map(|(node, read)| match read {
            Ok(info) => NodeReport::from_node(&info, advisories, releases),
            Err(e) => NodeReport::unreachable(node.name(), node.implementation(), e.to_string()),
        })
        .collect();
    Report {
        checked_at: now(),
        nodes,
    }
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

/// One timestamped line on stderr (UTC).
pub(crate) fn log(line: &str) {
    let secs = now() % 86_400;
    eprintln!(
        "[{:02}:{:02}:{:02}] {line}",
        secs / 3600,
        secs / 60 % 60,
        secs % 60
    );
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
            BuildCheck::UnknownRelease => {
                println!("    Build: release not in Firewatch's table, so not verified.")
            }
            BuildCheck::NotChecked => {}
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
}
