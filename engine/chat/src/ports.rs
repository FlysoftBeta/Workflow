//! Chat's capabilities. Production implementations are provided by Server composition, which adapts
//! the Environment runtime, typed store, tools and the Workspace/FileWork transaction. Chat itself
//! never touches the filesystem, spawns a process or calls libc.
use crate::{
    config::{AgentConfig, AgentPatch},
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

/// What to start inside the environment. `env` is the complete environment of the child: the
/// runtime must not merge other variables into it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpawnSpec {
    pub argv: Vec<String>,
    pub cwd: String,
    pub env: BTreeMap<String, String>,
    /// Short name for diagnostics (`codex`, `claude:<session>`), never a credential.
    pub label: String,
}
pub struct AgentStdio {
    pub stdin: Box<dyn Write + Send>,
    pub stdout: Box<dyn Read + Send>,
    pub stderr: Box<dyn Read + Send>,
}
/// A running guest child with piped stdio. Framing belongs to the transport.
pub trait AgentProcess: Send + Sync {
    /// SIGTERM, or SIGKILL when `force`, to the whole guest process group. Idempotent.
    fn kill(&self, force: bool);
    /// Blocks until the process exits and returns its status (128 + signal for signals).
    fn wait(&self) -> i32;
    fn is_alive(&self) -> bool;
}
pub trait AgentRuntime: Send + Sync {
    /// Spawns inside the active verified environment with the workspace layout and registers the
    /// child with the environment's process accounting. It fails closed: there is no host fallback.
    fn spawn(&self, spec: &SpawnSpec) -> Result<(Arc<dyn AgentProcess>, AgentStdio)>;
    /// The guest base environment (PATH, LANG, TMPDIR, declared variables) that Chat filters.
    fn base_env(&self) -> BTreeMap<String, String>;
}
/// Bounded reads of vendor-owned files below the guest agent homes (Claude transcripts).
pub trait GuestHome: Send + Sync {
    /// `path` is a guest path below `/home/work/.claude` or `/home/work/.codex`. Symlink escapes and
    /// files larger than `max` fail; an absent file is `None`.
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
/// The `agent` section of the revisioned workspace configuration.
pub trait AgentPreferences: Send + Sync {
    fn get(&self) -> AgentConfig;
    fn update(&self, patch: AgentPatch) -> Result<()>;
}
pub trait WorkspaceBridge: Send + Sync {
    /// Reads a workspace-relative file through FileWork's allowlist; `None` when absent.
    fn read_attachment(&self, path: &str, max: usize) -> Result<Option<Vec<u8>>>;
    /// Clears only an exactly matching submitted composer.
    fn acknowledge_composer(&self, submitted: &SubmittedComposer) -> Result<()>;
    /// Closes the conversation's panels and clears its composer after a confirmed deletion.
    fn remove_conversation(&self, id: &str) -> Result<()>;
}
pub trait Clock: Send + Sync {
    fn now_ms(&self) -> i64;
}
pub trait IdSource: Send + Sync {
    fn new_id(&self) -> String;
}
pub struct SystemClock;
impl Clock for SystemClock {
    fn now_ms(&self) -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or_default()
    }
}
pub struct RandomIds;
impl IdSource for RandomIds {
    fn new_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}
