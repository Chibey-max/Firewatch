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
    /// The commit the node's binary was built from, if it reports one (LND).
    #[serde(default)]
    pub commit_hash: Option<String>,
    /// Where each of the node's APIs listens, for the exposure check. Empty
    /// when the connector didn't look (e.g. a snapshot without it).
    #[serde(default)]
    pub apis: Vec<ApiListen>,
}

/// One API of a node and the addresses it listens on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiListen {
    pub api: Api,
    pub listen: Listen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Api {
    /// clnrest, or LND's REST proxy.
    Rest,
    /// cln-grpc, or LND's gRPC server.
    Grpc,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Listen {
    /// `host:port` as the node's config gives them, e.g. `0.0.0.0:8080`.
    Read { addresses: Vec<String> },
    /// The addresses couldn't be read, and why.
    Unknown { why: String },
}
