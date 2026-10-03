use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use workflow_environment::json::OpaqueObject;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Starting,
    Running,
    #[default]
    Ended,
    Failed,
}

/// Persisted terminal identity and the current Engine-owned projection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub id: String,
    #[serde(default)]
    pub ordinal: u64,
    #[serde(default)]
    pub generation: u64,
    #[serde(default = "default_directory")]
    pub cwd: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub custom_title: Option<String>,
    #[serde(default)]
    pub status: Status,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default = "default_rows")]
    pub rows: u16,
    #[serde(default = "default_columns")]
    pub columns: u16,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
impl Metadata {
    pub(crate) fn new(id: String, ordinal: u64, cwd: String) -> Self {
        Self {
            id,
            ordinal,
            generation: 0,
            cwd,
            title: None,
            custom_title: None,
            status: Status::Starting,
            exit_code: None,
            error: None,
            rows: default_rows(),
            columns: default_columns(),
            extra: OpaqueObject::default(),
        }
    }
}
fn default_directory() -> String {
    "/workspace".into()
}
fn default_rows() -> u16 {
    24
}
fn default_columns() -> u16 {
    80
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StartOptions {
    /// Workspace-relative directory. None retains an existing terminal's cwd.
    #[serde(default)]
    pub directory: Option<String>,
    #[serde(default)]
    pub rows: Option<u16>,
    #[serde(default)]
    pub columns: Option<u16>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadOptions {
    #[serde(default)]
    pub generation: Option<u64>,
    #[serde(default)]
    pub offset: u64,
    #[serde(default = "default_max_bytes")]
    pub max_bytes: usize,
    #[serde(default = "default_wait_ms")]
    pub wait_ms: u64,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
impl Default for ReadOptions {
    fn default() -> Self {
        Self {
            generation: None,
            offset: 0,
            max_bytes: default_max_bytes(),
            wait_ms: default_wait_ms(),
            extra: OpaqueObject::default(),
        }
    }
}
fn default_max_bytes() -> usize {
    65536
}
fn default_wait_ms() -> u64 {
    1000
}

/// Raw PTY bytes. Binary transport encoding belongs to the Server.
#[derive(Clone, Debug)]
pub struct ReadResult {
    pub data: Vec<u8>,
    pub start_offset: u64,
    pub next_offset: u64,
    pub eof: bool,
    pub exit_code: Option<i32>,
    pub reset: bool,
    pub terminal: Metadata,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Saved {
    pub format: u32,
    pub terminals: Vec<Metadata>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
impl Default for Saved {
    fn default() -> Self {
        Self {
            format: 1,
            terminals: Vec::new(),
            extra: OpaqueObject::default(),
        }
    }
}
