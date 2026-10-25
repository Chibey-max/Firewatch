//! Firewatch core: advisory format, version matching and the verdict engine.
//!
//! Everything here is pure: no network, no credentials. Connectors produce a
//! [`NodeInfo`], the engine turns it into [`Verdict`]s, and [`Report`] is the
//! JSON the CLI, alerts and dashboard all share.

pub mod advisory;
pub mod changes;
pub mod engine;
pub mod exposure;
pub mod node;
pub mod releases;
pub mod report;
pub mod version;

pub use advisory::{
    load_dir, load_texts, Action, Advisory, Condition, Implementation, LoadError, Severity,
};
pub use changes::{changes, Change};
pub use engine::{evaluate, Status, Verdict};
pub use exposure::{Exposure, ExposureStatus};
pub use node::{Api, ApiListen, Listen, NodeInfo};
pub use releases::{build_check, short_commit, BuildCheck, Release, ReleaseTable};
pub use report::{Finding, NodeReport, Report};
pub use version::{Version, VersionError, VersionReq};
