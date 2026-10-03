//! Agent defaults owned by Chat.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use workflow_environment::{
    Error, Result,
    config::{ConfigSection, FieldPatch, merge_extra},
    json::OpaqueObject,
};
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct BackendDefaults {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentConfig {
    pub backend: String,
    pub backends: BTreeMap<String, BackendDefaults>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<String>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            backend: "codex".into(),
            backends: Default::default(),
            permissions: None,
            extra: Default::default(),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct BackendPatch {
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub model: FieldPatch<Option<String>>,
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub effort: FieldPatch<Option<String>>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentPatch {
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub backend: FieldPatch<String>,
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub backends: FieldPatch<BTreeMap<String, BackendPatch>>,
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub permissions: FieldPatch<Option<String>>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}

impl ConfigSection for AgentConfig {
    type Patch = AgentPatch;
    fn validate(&self) -> Result<()> {
        if self.backend.trim().is_empty() {
            return Err(Error::invalid("agent.backend: expected nonempty string"));
        }
        Ok(())
    }
    fn apply(&mut self, p: AgentPatch) -> Result<()> {
        p.backend.assign(&mut self.backend);
        p.permissions.assign(&mut self.permissions);
        if let FieldPatch::Value(backends) = p.backends {
            for (id, p) in backends {
                let b = self.backends.entry(id).or_default();
                p.model.assign(&mut b.model);
                p.effort.assign(&mut b.effort);
                merge_extra(&mut b.extra, p.extra);
            }
        }
        merge_extra(&mut self.extra, p.extra);
        self.validate()
    }
}
