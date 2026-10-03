use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use workflow_environment::{
    error::{Error, Result},
    json::OpaqueObject,
};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FileVersion {
    pub exists: bool,
    pub size: u64,
    pub modified_at: u64,
    pub sha256: Option<String>,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
impl FileVersion {
    pub fn same_content(&self, other: &Self) -> bool {
        if !self.exists && !other.exists {
            return true;
        }
        if self.exists != other.exists {
            return false;
        }
        match (&self.sha256, &other.sha256) {
            (Some(a), Some(b)) => a == b,
            _ => self.size == other.size && self.modified_at == other.modified_at,
        }
    }
    pub fn matches_text(&self, text: &str) -> bool {
        self.exists
            && self.sha256.as_deref() == Some(&workflow_environment::persist::hash(text.as_bytes()))
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub format: u64,
    pub path: String,
    pub text: String,
    pub base: FileVersion,
    pub edited_at: u64,
    pub revision: u64,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ComposerDraft {
    #[serde(default = "one")]
    pub format: u64,
    pub conversation_id: String,
    pub revision: u64,
    pub text: String,
    pub attachments: Vec<Attachment>,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
fn one() -> u64 {
    1
}
impl ComposerDraft {
    pub fn empty(id: &str) -> Self {
        Self {
            format: 1,
            conversation_id: id.into(),
            revision: 0,
            text: String::new(),
            attachments: vec![],
            extra: Default::default(),
        }
    }
    pub fn has_content(&self) -> bool {
        !self.text.is_empty() || !self.attachments.is_empty()
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FileWorkState {
    pub drafts: BTreeMap<String, Draft>,
    pub composers: BTreeMap<String, ComposerDraft>,
    pub disk: BTreeMap<String, FileVersion>,
}
impl FileWorkState {
    pub fn validate(&self) -> Result<()> {
        for (p, d) in &self.drafts {
            if d.format != 1 {
                return Err(Error::business(
                    "unsupported_format",
                    "unknown draft format; source preserved read-only",
                ));
            }
            if d.path != *p || !crate::valid_path(p) {
                return Err(Error::invalid("invalid draft"));
            }
        }
        for (id, d) in &self.composers {
            if d.format != 1 {
                return Err(Error::business(
                    "unsupported_format",
                    "unknown composer format; source preserved read-only",
                ));
            }
            if d.conversation_id != *id {
                return Err(Error::invalid("invalid composer"));
            }
        }
        Ok(())
    }
    pub fn composer(&self, id: &str) -> ComposerDraft {
        self.composers
            .get(id)
            .cloned()
            .unwrap_or_else(|| ComposerDraft::empty(id))
    }
    pub fn clear_composer(&mut self, id: &str) -> Result<ComposerDraft> {
        let mut d = self.composer(id);
        if d.has_content() {
            d.revision = d
                .revision
                .checked_add(1)
                .ok_or_else(|| Error::business("overflow", "composer revision overflow"))?;
            d.text.clear();
            d.attachments.clear();
            self.composers.insert(id.into(), d.clone());
        }
        Ok(d)
    }
    pub fn acknowledge_composer(&mut self, mut submitted: ComposerDraft) -> Result<ComposerDraft> {
        submitted.format = 1;
        let current = self.composer(&submitted.conversation_id);
        if current == submitted {
            self.clear_composer(&submitted.conversation_id)
        } else {
            Ok(current)
        }
    }
    pub fn dirty(&self) -> Vec<ResourceRef> {
        let mut out: Vec<_> = self
            .drafts
            .keys()
            .map(|p| ResourceRef::File { path: p.clone() })
            .collect();
        out.extend(
            self.composers
                .iter()
                .filter(|(_, d)| d.has_content())
                .map(|(id, _)| ResourceRef::Conversation { id: id.clone() }),
        );
        out
    }
    pub fn draft_paths(&self) -> HashSet<String> {
        self.drafts.keys().cloned().collect()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ResourceRef {
    File { path: String },
    Conversation { id: String },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OpenFile {
    pub path: String,
    pub disk: FileVersion,
    pub disk_text: Option<String>,
    pub binary: bool,
    pub too_large: bool,
    pub draft: Option<Draft>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SaveOutcome {
    Saved { version: FileVersion, text: String },
    Unchanged { version: FileVersion },
    Conflict { disk: FileVersion },
    Invalid { message: String },
    Failed { message: String },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    UseDisk,
    KeepMine,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveDecision {
    SaveAll,
    KeepDrafts,
    Discard,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ArchiveOutcome {
    Archived { saved_paths: Vec<String> },
    NeedsDecision { resources: Vec<ResourceRef> },
    SaveConflict { paths: Vec<String> },
    Invalid { path: String, message: String },
    Failed { message: String },
    NotFound,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryEntry {
    pub path: String,
    pub is_directory: bool,
    pub size: u64,
    pub modified_at: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TrashEntry {
    pub id: String,
    pub path: String,
    pub trashed_at: u64,
    pub is_directory: bool,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FileOutcome {
    Done,
    Trashed { entry: TrashEntry },
    Restored { path: String },
    Failed { message: String },
}
#[derive(Clone, Debug)]
pub enum FileOperation {
    CreateFile { path: String, data: Vec<u8> },
    CreateDirectory { path: String },
    Delete { path: String },
    Trash { path: String },
    Move { from: String, to: String },
    Copy { from: String, to: String },
    Restore { id: String },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FileChunk {
    pub data: Vec<u8>,
    pub next_offset: u64,
    pub eof: bool,
    pub size: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UploadStarted {
    pub upload_id: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UploadProgress {
    pub next_offset: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UploadOutcome {
    Done { path: String },
    Failed { message: String },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum UploadDestination {
    Exact { path: String },
    Allocate { directory: String, name: String },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Diff {
    pub path: String,
    pub disk: FileVersion,
    pub base: Option<FileVersion>,
    pub disk_text: Option<String>,
    pub draft_text: Option<String>,
    pub conflicted: bool,
    pub binary: bool,
    pub too_large: bool,
}
/// Only the newest live session referencing each dirty resource is protected from retention.
#[derive(Clone, Debug)]
pub struct ArchiveCandidate {
    pub id: String,
    pub last_used_at: u64,
    pub created_at: u64,
    pub resources: Vec<ResourceRef>,
}
pub fn protected_sessions(
    state: &FileWorkState,
    live_sessions: &[ArchiveCandidate],
) -> HashSet<String> {
    state
        .dirty()
        .iter()
        .filter_map(|r| {
            live_sessions
                .iter()
                .filter(|s| s.resources.contains(r))
                .max_by(|a, b| {
                    a.last_used_at
                        .cmp(&b.last_used_at)
                        .then(a.created_at.cmp(&b.created_at))
                        .then(a.id.cmp(&b.id))
                })
                .map(|s| s.id.clone())
        })
        .collect()
}
