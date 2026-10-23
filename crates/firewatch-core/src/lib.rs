//! Firewatch core: advisory format, version matching and the verdict engine.
//!
//! Everything here is pure: no network, no credentials. Connectors produce a
//! [`NodeInfo`], the engine turns it into [`Verdict`]s, and [`Report`] is the
//! JSON the CLI, alerts and dashboard all share.

pub mod advisory;
pub mod changes;
pub mod engine;
pub mod node;
pub mod releases;
pub mod report;
pub mod version;

pub use advisory::{load_dir, Action, Advisory, Condition, Implementation, LoadError, Severity};
pub use changes::{changes, Change};
pub use engine::{evaluate, Status, Verdict};
pub use node::NodeInfo;
pub use releases::{short_commit, BuildCheck, Release, ReleaseTable};
pub use report::{Finding, NodeReport, Report};
pub use version::{Version, VersionError, VersionReq};
