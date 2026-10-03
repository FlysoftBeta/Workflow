//! Original Android ChatWire envelope shapes; snapshot chunking belongs to Server.
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ChatMetadata {
    pub conversations: Vec<ConversationEntry>,
    pub available: Vec<BackendKind>,
    pub login_methods: BTreeMap<BackendKind, Vec<LoginMethod>>,
    pub permissions: PermissionPreset,
    pub default_backend: BackendKind,
    pub process_epochs: BTreeMap<BackendKind, String>,
}
impl Default for ChatMetadata {
    fn default() -> Self {
        Self {
            conversations: Default::default(),
            available: Default::default(),
            login_methods: Default::default(),
            permissions: PermissionPreset::Ask,
            default_backend: BackendKind::Codex,
            process_epochs: Default::default(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ChatSnapshot {
    pub epoch: String,
    pub revision: i64,
    pub state: AgentState,
    pub metadata: ChatMetadata,
}
impl Default for ChatSnapshot {
    fn default() -> Self {
        Self {
            epoch: Default::default(),
            revision: Default::default(),
            state: Default::default(),
            metadata: Default::default(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ChatUpdate {
    pub epoch: String,
    pub revision: i64,
    pub events: Vec<AgentEvent>,
    pub metadata: ChatMetadata,
    pub resnapshot: bool,
}
impl Default for ChatUpdate {
    fn default() -> Self {
        Self {
            epoch: Default::default(),
            revision: Default::default(),
            events: Default::default(),
            metadata: Default::default(),
            resnapshot: false,
        }
    }
}
