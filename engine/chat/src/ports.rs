//! Chat's capabilities. Production implementations are provided by Server composition.
use crate::{
    error::Result,
    service::{
        index::IndexDocument,
        ledger::{SendRecord, SubmittedComposer},
    },
};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    sync::Arc,
};
use workflow_environment::tools::ToolsStatus;
pub struct SpawnSpec {
    pub argv: Vec<String>,
    pub cwd: String,
    pub env: BTreeMap<String, String>,
    pub label: String,
}
pub struct AgentStdio {
    pub stdin: Box<dyn Write + Send>,
    pub stdout: Box<dyn Read + Send>,
    pub stderr: Box<dyn Read + Send>,
}
pub trait AgentProcess: Send + Sync {
    fn stop(&self, force: bool) -> Result<()>;
    fn wait(&self, timeout_ms: u64) -> Result<Option<i32>>;
}
pub trait AgentRuntime: Send + Sync {
    /// Must spawn within the active verified environment, register process lifetime and fail closed.
    fn spawn(&self, spec: SpawnSpec) -> Result<(Arc<dyn AgentProcess>, AgentStdio)>;
    fn base_env(&self) -> BTreeMap<String, String>;
}
pub trait GuestHome: Send + Sync {
    fn ensure_private_dir(&self, path: &str) -> Result<()>;
    fn read(&self, path: &str, max: usize) -> Result<Option<Vec<u8>>>;
}
pub trait AgentTools: Send + Sync {
    fn status(&self) -> Result<ToolsStatus>;
    fn install_claude(&self, retry: bool) -> Result<ToolsStatus>;
}
pub trait ChatStore: Send + Sync {
    fn read_index(&self) -> Result<Option<IndexDocument>>;
    fn write_index(&self, document: &IndexDocument) -> Result<()>;
    fn quarantine_index(&self) -> Result<()>;
    fn read_send(&self, key: &str) -> Result<Option<SendRecord>>;
    fn write_send(&self, key: &str, record: &SendRecord) -> Result<()>;
    fn remove_sends(&self, conversation: &str) -> Result<()>;
}
pub trait WorkspaceBridge: Send + Sync {
    fn read_attachment(&self, path: &str, max: usize) -> Result<Option<Vec<u8>>>;
    fn acknowledge_composer(&self, submitted: &SubmittedComposer) -> Result<()>;
    fn remove_conversation(&self, id: &str) -> Result<()>;
}
