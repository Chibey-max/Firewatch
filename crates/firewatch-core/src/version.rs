//! Version parsing and range matching for Lightning implementations.
//!
//! Handles the formats nodes actually report:
//! - Core Lightning: `v26.06.7`, `v26.06`, `v26.06rc1`, `v26.06.8-modded`
//! - LND: `0.21.0-beta`, `0.21.0-beta.rc1`, `0.21.0-beta commit=v0.21.0-beta`
//! - Packaged builds that append a suffix, e.g. `v26.06.8-umbrel`
//!
//! Anything that does not start with at least `major.minor` is rejected, and
//! the matching engine turns that into "Can't tell", never "Not affected".

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VersionError {
    #[error("unparseable version '{0}'")]
    Unparseable(String),
    #[error("invalid range '{0}': {1}")]
    InvalidRange(String, &'static str),
}

/// A pre-release tag such as `rc1`. Any pre-release sorts before the release.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PreRelease {
    pub label: String,
    pub number: u64,
}

#[derive(Debug, Clone)]
pub struct Version {
    parts: Vec<u64>,
    pre: Option<PreRelease>,
    /// The numeric core as written, so CLN's `26.06.7` keeps its zero padding.
    core: String,
}

impl Version {
    pub fn parse(raw: &str) -> Result<Self, VersionError> {
        let err = || VersionError::Unparseable(raw.to_string());

        // LND appends ` commit=...`; only the first token is the version.
        let token = raw.split_whitespace().next().ok_or_else(err)?;
        let token = token.strip_prefix(['v', 'V']).unwrap_or(token);

        let core_len = token
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(token.len());
        let (core, rest) = token.split_at(core_len);
        let core = core.trim_end_matches('.');

        let parts = core
            .split('.')
            .map(|p| p.parse::<u64>().map_err(|_| err()))
            .collect::<Result<Vec<_>, _>>()?;
        if parts.len() < 2 {
            return Err(err());
        }

        Ok(Version {
            parts,
            pre: parse_pre_release(rest),
            core: core.to_string(),
        })
    }

    pub fn is_pre_release(&self) -> bool {
        self.pre.is_some()
    }

    fn part(&self, i: usize) -> u64 {
        self.parts.get(i).copied().unwrap_or(0)
    }
}

/// Reads the suffix after the numeric core. `rc`, `alpha` and `pre` tags mark
/// a pre-release; LND's `beta` (on every release) and packaging suffixes such
/// as `modded` or `umbrel` do not.
fn parse_pre_release(rest: &str) -> Option<PreRelease> {
    rest.split(['-', '.', '+', '_'])
        .filter(|t| !t.is_empty())
        .find_map(|t| {
            let t = t.to_ascii_lowercase();
            let label_len = t.find(|c: char| c.is_ascii_digit()).unwrap_or(t.len());
            let (label, digits) = t.split_at(label_len);
            if !matches!(label, "rc" | "alpha" | "pre") {
                return None;
            }
            Some(PreRelease {
                label: label.to_string(),
                number: digits.parse().unwrap_or(0),
            })
        })
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let len = self.parts.len().max(other.parts.len());
        for i in 0..len {
            match self.part(i).cmp(&other.part(i)) {
                Ordering::Equal => continue,
                ord => return ord,
            }
        }
        match (&self.pre, &other.pre) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(a), Some(b)) => a.cmp(b),
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Version {}

impl FromStr for Version {
    type Err = VersionError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Version::parse(s)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.core)?;
        if let Some(pre) = &self.pre {
            write!(f, "-{}{}", pre.label, pre.number)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
}

#[derive(Debug, Clone)]
struct Comparator {
    op: Op,
    version: Version,
}

impl Comparator {
    fn matches(&self, v: &Version) -> bool {
        let ord = v.cmp(&self.version);
        match self.op {
            Op::Lt => ord == Ordering::Less,
            Op::Le => ord != Ordering::Greater,
            Op::Gt => ord == Ordering::Greater,
            Op::Ge => ord != Ordering::Less,
            Op::Eq => ord == Ordering::Equal,
        }
    }
}

/// A version range such as `< 26.06.8` or `>= 0.18.0, < 0.21.0`.
/// Comma-separated comparators must all hold. Every comparator needs an
/// explicit operator, so a bare `26.06.7` in an advisory is a load error.
#[derive(Debug, Clone)]
pub struct VersionReq {
    raw: String,
    comparators: Vec<Comparator>,
}

impl VersionReq {
    pub fn parse(raw: &str) -> Result<Self, VersionError> {
        let invalid = |why| VersionError::InvalidRange(raw.to_string(), why);
        let mut comparators = Vec::new();
        for piece in raw.split(',') {
            let piece = piece.trim();
            let (op, rest) = if let Some(r) = piece.strip_prefix("<=") {
                (Op::Le, r)
            } else if let Some(r) = piece.strip_prefix(">=") {
                (Op::Ge, r)
            } else if let Some(r) = piece.strip_prefix('<') {
                (Op::Lt, r)
            } else if let Some(r) = piece.strip_prefix('>') {
                (Op::Gt, r)
            } else if let Some(r) = piece.strip_prefix('=') {
                (Op::Eq, r)
            } else {
                return Err(invalid("each part needs an operator: <, <=, >, >= or ="));
            };
            let version = Version::parse(rest.trim()).map_err(|_| invalid("bad version"))?;
            comparators.push(Comparator { op, version });
        }
        if comparators.is_empty() {
            return Err(invalid("empty range"));
        }
        Ok(VersionReq {
            raw: raw.trim().to_string(),
            comparators,
        })
    }

    pub fn matches(&self, v: &Version) -> bool {
        self.comparators.iter().all(|c| c.matches(v))
    }
}

impl fmt::Display for VersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}

impl TryFrom<String> for VersionReq {
    type Error = VersionError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        VersionReq::parse(&s)
    }
}

impl From<VersionReq> for String {
    fn from(r: VersionReq) -> String {
        r.raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn parses_cln_formats() {
        assert_eq!(v("v26.06.7"), v("26.06.7"));
        assert_eq!(v("v26.06"), v("26.06.0"));
        assert!(v("v26.06rc1").is_pre_release());
        assert!(!v("v26.06.8-modded").is_pre_release());
        assert_eq!(v("v26.06.8-umbrel"), v("26.06.8"));
    }

    #[test]
    fn parses_lnd_formats() {
        assert_eq!(v("0.21.0-beta"), v("0.21.0"));
        assert!(!v("0.21.0-beta").is_pre_release());
        assert!(v("0.21.0-beta.rc1").is_pre_release());
        assert_eq!(v("0.20.1-beta commit=v0.20.1-beta"), v("0.20.1"));
    }

    #[test]
    fn rejects_garbage() {
        for bad in [
            "",
            "   ",
            "beta",
            "v",
            "26",
            "unknown-26.06",
            "v.26.06",
            "26..06",
        ] {
            assert!(Version::parse(bad).is_err(), "{bad:?} should not parse");
        }
    }

    #[test]
    fn orders_versions() {
        assert!(v("26.06.7") < v("26.06.8"));
        assert!(v("26.04.1") < v("26.06"));
        assert!(v("26.06rc2") < v("26.06"));
        assert!(v("26.06rc1") < v("26.06rc2"));
        assert!(v("0.21.0-beta.rc1") < v("0.21.0-beta"));
        assert!(v("0.20.1-beta") < v("0.21.0-beta.rc1"));
        assert!(v("26.06.10") > v("26.06.9"));
    }

    #[test]
    fn matches_ranges() {
        let r = VersionReq::parse("< 26.06.8").unwrap();
        assert!(r.matches(&v("v26.06.7")));
        assert!(r.matches(&v("v26.06.8rc1")));
        assert!(!r.matches(&v("v26.06.8")));
        assert!(!r.matches(&v("v26.06.9")));

        let r = VersionReq::parse(">= 0.18.0, < 0.21.0").unwrap();
        assert!(r.matches(&v("0.20.1-beta")));
        assert!(r.matches(&v("0.21.0-beta.rc1")));
        assert!(!r.matches(&v("0.21.0-beta")));
        assert!(!r.matches(&v("0.17.5-beta")));
    }

    #[test]
    fn rejects_bad_ranges() {
        for bad in ["", "26.06.7", "< ", "<= banana", "< 26.06.8,"] {
            assert!(VersionReq::parse(bad).is_err(), "{bad:?} should not parse");
        }
    }
}
