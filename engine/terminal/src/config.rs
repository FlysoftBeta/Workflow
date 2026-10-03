//! Workspace-delivered terminal preferences.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use workflow_environment::{
    Result,
    config::{ConfigSection, FieldPatch, merge_extra},
    json::OpaqueObject,
};
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct TerminalConfig {
    pub extra_keys_pinned: bool,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct TerminalPatch {
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub extra_keys_pinned: FieldPatch<bool>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}

impl ConfigSection for TerminalConfig {
    type Patch = TerminalPatch;
    fn validate(&self) -> Result<()> {
        Ok(())
    }
    fn apply(&mut self, p: TerminalPatch) -> Result<()> {
        p.extra_keys_pinned.assign(&mut self.extra_keys_pinned);
        merge_extra(&mut self.extra, p.extra);
        Ok(())
    }
}
