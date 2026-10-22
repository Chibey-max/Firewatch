//! The advisory file format and its loader. One TOML file per advisory.

use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::version::{Version, VersionReq};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Implementation {
    Cln,
    Lnd,
    Ldk,
}

impl Implementation {
    /// The short tag used in advisory files: `cln`, `lnd` or `ldk`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Implementation::Cln => "cln",
            Implementation::Lnd => "lnd",
            Implementation::Ldk => "ldk",
        }
    }
}

impl fmt::Display for Implementation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Implementation::Cln => "Core Lightning",
            Implementation::Lnd => "LND",
            Implementation::Ldk => "LDK",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Every condition must hold for "Affected". A condition that cannot be read
/// makes the verdict "Can't tell".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Condition {
    /// The node's version is inside `affected`. Always checked; listing it is
    /// optional and only documents that the advisory is version-only.
    VersionRange,
    /// A config option is on. CLN: `listconfigs`.
    ConfigEnabled { option: String },
    /// A feature bit is set (either bit of the even/odd pair). LND: `GetInfo`.
    FeatureBit { bit: u32 },
    /// Depends on something no RPC exposes. Always "Can't tell".
    Manual { check_hint: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Upgrade,
    Offline,
    DisableFeature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Remediation {
    pub action: Action,
    /// Quoted from the developers' source, never written by us.
    pub steps: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Advisory {
    pub id: String,
    pub implementation: Implementation,
    pub title: String,
    #[serde(with = "req_string")]
    pub affected: VersionReq,
    /// `None` when no fix existed at the time of the advisory.
    #[serde(default)]
    pub fixed: Option<String>,
    pub severity: Severity,
    #[serde(default)]
    pub exploited_in_wild: bool,
    /// The developers' own release notes or statement.
    pub source: String,
    pub reviewed_by: String,
    /// Only verified advisories run by default.
    #[serde(default)]
    pub verified: bool,
    #[serde(default)]
    pub conditions: Vec<Condition>,
    pub remediation: Remediation,
}

mod req_string {
    use serde::{Deserialize, Deserializer, Serializer};

    use crate::version::VersionReq;

    pub fn serialize<S: Serializer>(req: &VersionReq, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(req)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<VersionReq, D::Error> {
        let raw = String::deserialize(d)?;
        VersionReq::parse(&raw).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("reading {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("parsing {path}: {source}")]
    Parse {
        path: PathBuf,
        source: Box<toml::de::Error>,
    },
    #[error("{path}: {message}")]
    Invalid { path: PathBuf, message: String },
}

impl Advisory {
    pub fn from_toml(path: &Path, text: &str) -> Result<Self, LoadError> {
        let advisory: Advisory = toml::from_str(text).map_err(|e| LoadError::Parse {
            path: path.to_path_buf(),
            source: Box::new(e),
        })?;
        advisory.validate().map_err(|message| LoadError::Invalid {
            path: path.to_path_buf(),
            message,
        })?;
        Ok(advisory)
    }

    fn validate(&self) -> Result<(), String> {
        if !self.id.starts_with("FW-") {
            return Err(format!("id '{}' must start with FW-", self.id));
        }
        if !self.source.starts_with("https://") {
            return Err("source must be an https:// link to the developers' own notes".into());
        }
        if self.reviewed_by.trim().is_empty() {
            return Err("reviewed_by must name the team member who checked it".into());
        }
        if self.remediation.steps.trim().is_empty() {
            return Err("remediation.steps must quote the source".into());
        }
        if let Some(fixed) = &self.fixed {
            let fixed = Version::parse(fixed).map_err(|e| format!("fixed: {e}"))?;
            if self.affected.matches(&fixed) {
                return Err("the fixed version is inside the affected range".into());
            }
        }
        Ok(())
    }
}

/// Loads every `*.toml` file in `dir`, sorted by id. Fails on the first bad
/// file, so a broken advisory can never be silently skipped.
pub fn load_dir(dir: &Path) -> Result<Vec<Advisory>, LoadError> {
    let io = |source| LoadError::Io {
        path: dir.to_path_buf(),
        source,
    };
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(io)?
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<_, _>>()
        .map_err(io)?;
    paths.retain(|p| p.extension().is_some_and(|ext| ext == "toml"));
    paths.sort();

    let mut advisories = Vec::with_capacity(paths.len());
    let mut seen = HashSet::new();
    for path in paths {
        let text = std::fs::read_to_string(&path).map_err(|source| LoadError::Io {
            path: path.clone(),
            source,
        })?;
        let advisory = Advisory::from_toml(&path, &text)?;
        if !seen.insert(advisory.id.clone()) {
            return Err(LoadError::Invalid {
                path,
                message: format!("duplicate id {}", advisory.id),
            });
        }
        advisories.push(advisory);
    }
    advisories.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(advisories)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
        id = "FW-2026-999"
        implementation = "cln"
        title = "Test"
        affected = "< 26.06.8"
        fixed = "26.06.8"
        severity = "critical"
        source = "https://example.com/release"
        reviewed_by = "Dave"

        [[conditions]]
        kind = "config_enabled"
        option = "experimental-splicing"

        [remediation]
        action = "upgrade"
        steps = "Upgrade."
    "#;

    fn load(text: &str) -> Result<Advisory, LoadError> {
        Advisory::from_toml(Path::new("test.toml"), text)
    }

    #[test]
    fn loads_valid_advisory() {
        let a = load(VALID).unwrap();
        assert_eq!(a.implementation, Implementation::Cln);
        assert!(!a.verified);
        assert_eq!(
            a.conditions,
            vec![Condition::ConfigEnabled {
                option: "experimental-splicing".into()
            }]
        );
    }

    #[test]
    fn rejects_unknown_condition_kind() {
        let text = VALID.replace("config_enabled", "moon_phase");
        assert!(matches!(load(&text), Err(LoadError::Parse { .. })));
    }

    #[test]
    fn rejects_range_without_operator() {
        let text = VALID.replace("< 26.06.8", "26.06.7");
        assert!(matches!(load(&text), Err(LoadError::Parse { .. })));
    }

    #[test]
    fn rejects_fixed_inside_affected_range() {
        let text = VALID.replace("fixed = \"26.06.8\"", "fixed = \"26.06.7\"");
        assert!(matches!(load(&text), Err(LoadError::Invalid { .. })));
    }

    #[test]
    fn rejects_non_https_source() {
        let text = VALID.replace("https://example.com/release", "news site");
        assert!(matches!(load(&text), Err(LoadError::Invalid { .. })));
    }
}
