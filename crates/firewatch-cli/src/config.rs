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
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }
}
