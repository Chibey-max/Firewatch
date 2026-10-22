//! `firewatch`: check nodes against the advisory database.
//!
//! Nodes come from `firewatch.toml` (live, read-only connectors) or from a
//! JSON snapshot (a list of `NodeInfo`, e.g. `examples/nodes.json`).

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use firewatch_connectors::Config;
use firewatch_core::{load_dir, Advisory, NodeInfo, NodeReport, Report, Status};

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
    /// Load and validate every advisory file, then list them.
    Validate,
}

#[tokio::main]
async fn main() -> ExitCode {
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

    match cli.command {
        Command::Validate => {
            for a in &advisories {
                let mark = if a.verified {
                    "verified  "
                } else {
                    "UNVERIFIED"
                };
                println!(
                    "{} {mark} {:<4} {:<12} {}",
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
            let advisories: Vec<Advisory> = advisories
                .into_iter()
                .filter(|a| include_unverified || a.verified)
                .collect();
            let report = match nodes {
                Some(path) => Report::build(&load_snapshot(&path)?, &advisories, now()),
                None => check_live(&config, &advisories).await?,
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
    }
}

/// Reads every configured node. A node that can't be read is reported as
/// "Can't tell" with its error, so one dead node doesn't hide the others.
async fn check_live(config: &Path, advisories: &[Advisory]) -> Result<Report> {
    let config = Config::load(config)?;
    let http = firewatch_connectors::http_client();
    let mut nodes = Vec::with_capacity(config.nodes.len());
    for node in &config.nodes {
        nodes.push(match firewatch_connectors::fetch(node, &http).await {
            Ok(info) => NodeReport::from_node(&info, advisories),
            Err(e) => NodeReport::unreachable(node.name(), node.implementation(), e.to_string()),
        });
    }
    Ok(Report {
        checked_at: now(),
        nodes,
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
