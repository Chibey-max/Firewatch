//! `firewatch.toml`: the nodes to watch, how often, and where alerts go.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use firewatch_alerts::NostrConfig;
use firewatch_connectors::NodeConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub watch: WatchConfig,
    #[serde(default)]
    pub alerts: AlertsConfig,
    #[serde(default)]
    pub nodes: Vec<NodeConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct WatchConfig {
    /// Seconds between checks.
    pub interval_secs: u64,
    /// Where the dashboard API listens. Keep it on localhost: the report
    /// says which nodes are vulnerable.
    pub listen: String,
    /// A built dashboard to serve at `/`, if any.
    pub dashboard_dir: Option<PathBuf>,
}

impl Default for WatchConfig {
    fn default() -> Self {
        WatchConfig {
            interval_secs: 30,
            listen: "127.0.0.1:7070".into(),
            dashboard_dir: None,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertsConfig {
    pub nostr: Option<NostrConfig>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            anyhow::bail!(
                "no config at {}. Run `firewatch init` to set one up, or pass --config",
                path.display()
            );
        }
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }
}

/// The config to use: `--config`, else `./firewatch.toml` if it exists (a
/// checkout of the repo), else the per-user one that `firewatch init` writes.
pub fn resolve(flag: Option<PathBuf>) -> PathBuf {
    flag.unwrap_or_else(|| {
        let here = PathBuf::from("firewatch.toml");
        if here.exists() {
            here
        } else {
            default_path()
        }
    })
}

/// `$XDG_CONFIG_HOME/firewatch/firewatch.toml`, or `~/.config/...`.
pub fn default_path() -> PathBuf {
    config_dir().join("firewatch.toml")
}

pub fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("firewatch")
}
