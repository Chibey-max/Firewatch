//! What a connector reads from a node. Connectors fill this in; the engine
//! only ever sees this struct, so it never needs node credentials.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::advisory::Implementation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Operator-chosen label, e.g. "alice-cln".
    pub name: String,
    pub implementation: Implementation,
    /// Exactly as the node reported it, e.g. `v26.06.7` or `0.20.1-beta commit=...`.
    pub version: String,
    #[serde(default)]
    pub network: Option<String>,
    /// Option name to enabled. `None` means the config could not be read, which
    /// is different from an option being off.
    #[serde(default)]
    pub config: Option<BTreeMap<String, bool>>,
    /// Feature bits the node advertises. `None` means they could not be read.
    #[serde(default)]
    pub features: Option<BTreeSet<u32>>,
}
