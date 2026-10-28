//! What `watch` knows, kept across restarts (task A4).
//!
//! After every check `watch` writes each node's last good report to
//! `state.json`, next to the config. After a restart (a power cut, a reboot)
//! the first check is compared with it, so a node that changed while
//! Firewatch was off gets one catch-up message, and an unchanged node gets
//! none. The file names vulnerable nodes, so it's readable only by its owner.
//! A missing or unreadable file means a fresh start, never a crash.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use firewatch_core::NodeReport;
use serde::{Deserialize, Serialize};

/// Bumped if the layout changes, so an old file is ignored, not misread.
const VERSION: u32 = 1;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub version: u32,
    /// Unix seconds of the check this was saved after.
    pub saved_at: u64,
    /// Each node's last successful read.
    pub known: HashMap<String, NodeReport>,
    /// Nodes whose "can't read" alert has been sent and not yet followed by
    /// "can be read again".
    #[serde(default)]
    pub down_alerted: HashSet<String>,
}

/// `state.json` in the same folder as the config.
pub fn path_for(config: &Path) -> PathBuf {
    config
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .join("state.json")
}

/// The saved state, or `None` (with the reason) for a fresh start.
pub fn load(path: &Path) -> std::result::Result<Option<State>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("can't read {}: {e}", path.display())),
    };
    let state: State = serde_json::from_str(&text)
        .map_err(|e| format!("{} is unreadable ({e})", path.display()))?;
    if state.version != VERSION {
        return Err(format!(
            "{} is from another version of Firewatch",
            path.display()
        ));
    }
    Ok(Some(state))
}

/// Writes the state atomically: a temporary file, made private before
/// anything is written, then renamed over the old one. A power cut mid-write
/// leaves the previous state, never half a file.
pub fn save(path: &Path, state: &State) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&tmp)
        .with_context(|| format!("writing {}", tmp.display()))?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(serde_json::to_string(state)?.as_bytes())?;
    file.sync_all()?;
    std::fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))
}

impl State {
    pub fn new(saved_at: u64) -> Self {
        State {
            version: VERSION,
            saved_at,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use firewatch_core::{Implementation, Status};

    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fw-state-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn node(name: &str) -> NodeReport {
        NodeReport {
            name: name.into(),
            implementation: Implementation::Cln,
            version: Some("v26.06.7".into()),
            network: None,
            status: Status::Affected,
            error: None,
            build_check: Default::default(),
            exposure: vec![],
            findings: vec![],
        }
    }

    #[test]
    fn saves_privately_and_loads_back() {
        let d = dir("roundtrip");
        let path = d.join("state.json");
        let mut state = State::new(1_800_000_000);
        state.known.insert("old-cln".into(), node("old-cln"));
        save(&path, &state).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        let back = load(&path).unwrap().unwrap();
        assert_eq!(back.saved_at, 1_800_000_000);
        assert_eq!(back.known["old-cln"], node("old-cln"));
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn missing_is_a_fresh_start_and_corrupt_says_why() {
        let d = dir("corrupt");
        let path = d.join("state.json");
        assert!(matches!(load(&path), Ok(None)));
        std::fs::write(&path, "{ not json").unwrap();
        let why = load(&path).unwrap_err();
        assert!(why.contains("is unreadable"), "{why}");
        std::fs::write(&path, r#"{"version":99,"saved_at":0,"known":{}}"#).unwrap();
        assert!(load(&path).unwrap_err().contains("another version"));
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn sits_next_to_the_config() {
        assert_eq!(
            path_for(Path::new("/home/x/.config/firewatch/firewatch.toml")),
            PathBuf::from("/home/x/.config/firewatch/state.json")
        );
        assert_eq!(
            path_for(Path::new("firewatch.toml")),
            PathBuf::from("./state.json")
        );
    }
}
