//! In-process Rust Chat composition. Server implements Chat's ports over Environment (runtime,
//! typed store, tools, agent homes), FileWork (attachments) and the Workspace transaction
//! (composer acknowledgement, conversation removal, agent configuration), and owns the frozen
//! chunked snapshot transfer of the retained `chat.*` wire.
use crate::{
    Server,
    contracts::documents::{self, Operation},
    protocol::{
        ChatCommandParams, ChatSnapshotParams, ChatWatchParams, DocumentParams, Error, OpaqueJson,
        Result,
    },
    state,
};
use base64::Engine;
use serde::Serialize;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex, Weak},
    time::Duration,
};
use workflow_chat::{
    config::{AgentConfig, AgentPatch},
    error::{ChatError, ErrorKind},
    ports::{
        AgentPreferences, AgentProcess, AgentRuntime, AgentStdio, AgentTools, ChatStore, GuestHome,
        RandomIds, SpawnSpec, SystemClock, WorkspaceBridge,
    },
    service::{
        Chat, ChatPorts,
        index::IndexDocument,
        ledger::{SendRecord, SubmittedComposer},
        paths,
    },
};
use workflow_environment::{
    Environment,
    config::{ConfigOutcome, FieldPatch},
    runtime::{self, CommandSpec, ProcessHandle},
    tools::{self, ToolsStatus},
};

type ChatResult<T> = workflow_chat::error::Result<T>;
const UPDATE_BYTES: usize = 1024 * 1024;
const CHUNK_BYTES: usize = 65_536;
const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;

/// Chat failures keep the retained service's codes: invalid arguments are -32602, everything else
/// -32000, both with error kind `chat`. Messages go to the requester only.
pub fn error(e: ChatError) -> Error {
    let invalid = matches!(e.kind, ErrorKind::InvalidArgument | ErrorKind::DecisionNotOffered);
    Error {
        code: if invalid { -32602 } else { -32000 },
        kind: "chat".into(),
        message: e.message,
    }
}
fn store_error(e: impl std::fmt::Display) -> ChatError {
    ChatError::new(ErrorKind::Store, e.to_string())
}

#[derive(Default)]
pub struct ChatHost {
    current: Mutex<Option<(Arc<Chat>, Arc<GuestRuntime>)>>,
    transfers: Mutex<VecDeque<(String, Arc<Vec<u8>>)>>,
}
impl ChatHost {
    fn ensure(&self, server: &Arc<Server>) -> Result<Arc<Chat>> {
        let mut current = self.current.lock().unwrap();
        if let Some((chat, _)) = current.as_ref() {
            return Ok(chat.clone());
        }
        let store = server.workspace.lock().unwrap().store.clone();
        let runtime = Arc::new(GuestRuntime {
            environment: server.environment.clone(),
            children: Mutex::new((false, Vec::new())),
        });
        let weak = Arc::downgrade(server);
        let chat = Chat::start(ChatPorts {
            runtime: runtime.clone(),
            guest_home: Arc::new(Home(store)),
            tools: Arc::new(Tools(weak.clone())),
            store: Arc::new(Documents(weak.clone())),
            preferences: Arc::new(Preferences(weak.clone())),
            workspace: Arc::new(Bridge(weak)),
            clock: Arc::new(SystemClock),
            ids: Arc::new(RandomIds),
        })
        .map_err(error)?;
        let chat = Arc::new(chat);
        *current = Some((chat.clone(), runtime));
        Ok(chat)
    }
    /// Stops agents and every guest child they spawned. The next request starts a fresh service
    /// with a new journal epoch.
    pub fn stop(&self) {
        if let Some((chat, runtime)) = self.current.lock().unwrap().take() {
            chat.shutdown();
            runtime.kill_all();
        }
        self.transfers.lock().unwrap().clear();
    }
    pub fn snapshot(&self, server: &Arc<Server>, p: ChatSnapshotParams) -> Result<OpaqueJson> {
        if let Some(id) = p.transfer_id {
            let bytes = self
                .transfers
                .lock()
                .unwrap()
                .iter()
                .find(|(t, _)| *t == id)
                .map(|(_, b)| b.clone())
                .ok_or_else(|| error(ChatError::state("Snapshot transfer expired; request a new snapshot")))?;
            return chunk(&id, &bytes, p.offset.unwrap_or(0));
        }
        if p.offset.unwrap_or(0) != 0 {
            return Err(error(ChatError::invalid("offset requires a transfer")));
        }
        let frozen = self.ensure(server)?.snapshot();
        let bytes = serde_json::to_vec(&frozen).map_err(|e| Error::business("chat", &e.to_string()))?;
        if bytes.len() <= UPDATE_BYTES {
            return crate::protocol::strict_json(&bytes).map_err(|e| Error::business("chat", &e.to_string()));
        }
        if bytes.len() > MAX_SNAPSHOT_BYTES {
            return Err(error(ChatError::invalid(
                "Chat snapshot exceeds transfer limit; load less history",
            )));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let bytes = Arc::new(bytes);
        {
            let mut transfers = self.transfers.lock().unwrap();
            while transfers.len() >= 2 {
                transfers.pop_front();
            }
            transfers.push_back((id.clone(), bytes.clone()));
        }
        chunk(&id, &bytes, 0)
    }
    pub fn watch(&self, server: &Arc<Server>, p: ChatWatchParams) -> Result<OpaqueJson> {
        let after = i64::try_from(p.after_revision)
            .map_err(|_| error(ChatError::invalid("invalid chat cursor")))?;
        let update = self
            .ensure(server)?
            .watch(&p.epoch, after, Duration::from_millis(p.timeout_ms))
            .map_err(error)?;
        crate::protocol::opaque(&update)
    }
    pub fn command(&self, server: &Arc<Server>, p: ChatCommandParams) -> Result<OpaqueJson> {
        self.ensure(server)?.command(&p.name, &p.args).map_err(error)
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Chunk<'a> {
    transfer_id: &'a str,
    offset: u64,
    data: String,
    next_offset: u64,
    eof: bool,
}
fn chunk(id: &str, bytes: &[u8], offset: u64) -> Result<OpaqueJson> {
    let start = usize::try_from(offset)
        .ok()
        .filter(|o| *o <= bytes.len())
        .ok_or_else(|| error(ChatError::invalid("offset is outside the snapshot")))?;
    let end = (start + CHUNK_BYTES).min(bytes.len());
    crate::protocol::opaque(&Chunk {
        transfer_id: id,
        offset,
        data: base64::engine::general_purpose::STANDARD.encode(&bytes[start..end]),
        next_offset: end as u64,
        eof: end == bytes.len(),
    })
}

/// Guest execution through the Environment runtime. There is no host fallback; children are
/// counted by the environment, die with the Server and are killed when Chat stops.
struct GuestRuntime {
    environment: Arc<Mutex<Environment>>,
    children: Mutex<(bool, Vec<ProcessHandle>)>,
}
impl GuestRuntime {
    fn kill_all(&self) {
        let mut children = self.children.lock().unwrap();
        children.0 = true;
        for child in children.1.drain(..) {
            let _ = child.stop(true);
        }
    }
}
struct Guest(ProcessHandle);
impl AgentProcess for Guest {
    fn kill(&self, force: bool) {
        let _ = self.0.stop(force);
    }
    fn wait(&self) -> i32 {
        loop {
            match self.0.wait(30_000) {
                Ok(status) if !status.running => return status.exit_code.unwrap_or(-1),
                Ok(_) => (),
                Err(_) => return -1,
            }
        }
    }
    fn is_alive(&self) -> bool {
        self.0.running()
    }
}
impl AgentRuntime for GuestRuntime {
    fn spawn(&self, spec: &SpawnSpec) -> ChatResult<(Arc<dyn AgentProcess>, AgentStdio)> {
        let mut children = self.children.lock().unwrap();
        if children.0 {
            return Err(ChatError::unavailable("chat service stopped"));
        }
        let piped = runtime::spawn_piped(
            &self.environment,
            &CommandSpec {
                argv: spec.argv.clone(),
                cwd: spec.cwd.clone(),
                env: spec.env.clone(),
            },
        )
        .map_err(|e| ChatError::unavailable(e.message))?;
        children.1.retain(|h| h.running());
        children.1.push(piped.handle.clone());
        Ok((
            Arc::new(Guest(piped.handle)),
            AgentStdio {
                stdin: Box::new(piped.stdin),
                stdout: Box::new(piped.stdout),
                stderr: Box::new(piped.stderr),
            },
        ))
    }
    fn base_env(&self) -> BTreeMap<String, String> {
        [
            ("HOME", "/home/work"),
            ("USER", "work"),
            ("LOGNAME", "work"),
            ("SHELL", "/bin/bash"),
            ("TERM", "dumb"),
            ("LANG", "C.UTF-8"),
        ]
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .into()
    }
}

/// Vendor-owned files below the agent homes, read through the Environment store without
/// following symbolic links. Credentials are refused.
struct Home(workflow_environment::Store);
impl GuestHome for Home {
    fn read(&self, path: &str, max: usize) -> ChatResult<Option<Vec<u8>>> {
        let key = paths::agent_home_key(path)
            .ok_or_else(|| ChatError::invalid("Agent resource is outside permitted guest roots"))?;
        self.0.read_bytes(&key, max).map_err(store_error)
    }
}

struct Tools(Weak<Server>);
impl Tools {
    fn server(&self) -> ChatResult<Arc<Server>> {
        self.0.upgrade().ok_or_else(|| ChatError::new(ErrorKind::Closed, "workspace stopped"))
    }
}
impl AgentTools for Tools {
    fn status(&self) -> ChatResult<ToolsStatus> {
        tools::status(&self.server()?.environment).map_err(|e| ChatError::unavailable(e.message))
    }
    fn install_claude(&self, retry: bool) -> ChatResult<ToolsStatus> {
        let s = self.server()?;
        tools::install(&s.environment, s.processes.clone(), "claude", retry)
            .map_err(|e| ChatError::unavailable(e.message))
    }
}

/// Chat documents through the same Environment document store and Workspace commit as the
/// former `documents.*` callbacks, in namespace `chat`.
struct Documents(Weak<Server>);
impl Documents {
    fn call(&self, op: Operation, key: &str, document: Option<String>) -> ChatResult<Option<String>> {
        let server = self.0.upgrade().ok_or_else(|| ChatError::new(ErrorKind::Closed, "workspace stopped"))?;
        let params = DocumentParams {
            namespace: "chat".into(),
            key: key.into(),
            document,
            expected_revision: None,
            extra: Default::default(),
        };
        let mut w = server.workspace.lock().unwrap();
        let result = documents::call(&w.store, op, &params, w.writable).map_err(store_error)?;
        if result.changed {
            let mut next = w.clone();
            next.refresh().map_err(store_error)?;
            w.commit(next).map_err(store_error)?;
            server.changed.notify_all();
        }
        Ok(result.document.document)
    }
}
impl ChatStore for Documents {
    fn read_index(&self) -> ChatResult<Option<IndexDocument>> {
        self.call(Operation::Read, "conversations", None)?
            .map(|s| IndexDocument::decode(s.as_bytes()))
            .transpose()
    }
    fn write_index(&self, document: &IndexDocument) -> ChatResult<()> {
        self.call(Operation::Write, "conversations", Some(serde_json::to_string(document)?))
            .map(|_| ())
    }
    fn quarantine_index(&self) -> ChatResult<()> {
        self.call(Operation::Quarantine, "conversations", None).map(|_| ())
    }
    fn read_send(&self, key: &str) -> ChatResult<Option<SendRecord>> {
        self.call(Operation::Read, key, None)?
            .map(|s| workflow_environment::json::strict_json(s.as_bytes()).map_err(Into::into))
            .transpose()
    }
    fn write_send(&self, key: &str, record: &SendRecord) -> ChatResult<()> {
        self.call(Operation::Write, key, Some(serde_json::to_string(record)?))
            .map(|_| ())
    }
    fn remove_sends(&self, conversation: &str) -> ChatResult<()> {
        let server = self.0.upgrade().ok_or_else(|| ChatError::new(ErrorKind::Closed, "workspace stopped"))?;
        let keys = server
            .workspace
            .lock()
            .unwrap()
            .store
            .list("documents/chat")
            .map_err(store_error)?;
        for path in keys {
            let Some(name) = path
                .rsplit('/')
                .next()
                .and_then(|s| s.strip_suffix(".json"))
                .filter(|s| s.starts_with("send-"))
            else {
                continue;
            };
            if self
                .read_send(name)?
                .is_some_and(|r| r.conversation_id.as_deref() == Some(conversation))
            {
                let w = server.workspace.lock().unwrap();
                if w.writable {
                    w.store.remove(&path).map_err(store_error)?;
                }
            }
        }
        Ok(())
    }
}

/// The `agent` section of the revisioned workspace configuration.
struct Preferences(Weak<Server>);
impl AgentPreferences for Preferences {
    fn get(&self) -> AgentConfig {
        self.0
            .upgrade()
            .map(|s| s.workspace.lock().unwrap().state.config.agent.clone())
            .unwrap_or_default()
    }
    fn update(&self, patch: AgentPatch) -> ChatResult<()> {
        let server = self.0.upgrade().ok_or_else(|| ChatError::new(ErrorKind::Closed, "workspace stopped"))?;
        for _ in 0..3 {
            let mut w = server.workspace.lock().unwrap();
            let revision = w.revision;
            let reply = w
                .command_typed(state::Command::UpdateConfig {
                    config: state::ConfigPatch {
                        agent: FieldPatch::Value(patch.clone()),
                        ..Default::default()
                    },
                    expected_revision: revision,
                })
                .map_err(store_error)?;
            server.changed.notify_all();
            match reply.value {
                state::CommandValue::Config(ConfigOutcome::Updated { .. }) => return Ok(()),
                state::CommandValue::Config(ConfigOutcome::Conflict { .. }) => continue,
                state::CommandValue::Config(ConfigOutcome::Blocked { problem }) => {
                    return Err(ChatError::new(ErrorKind::Store, problem));
                }
                state::CommandValue::Config(ConfigOutcome::Failed { message }) => {
                    return Err(ChatError::new(ErrorKind::Store, message));
                }
                _ => return Err(ChatError::new(ErrorKind::Store, "unexpected configuration reply")),
            }
        }
        Err(ChatError::new(ErrorKind::Store, "configuration kept changing"))
    }
}

/// Composer acknowledgement, conversation removal and attachment reads through the Workspace
/// transaction and FileWork's allowlist.
struct Bridge(Weak<Server>);
impl Bridge {
    fn server(&self) -> ChatResult<Arc<Server>> {
        self.0.upgrade().ok_or_else(|| ChatError::new(ErrorKind::Closed, "workspace stopped"))
    }
    fn command(&self, command: state::Command) -> ChatResult<()> {
        let server = self.server()?;
        server
            .workspace
            .lock()
            .unwrap()
            .command_typed(command)
            .map_err(store_error)?;
        server.changed.notify_all();
        Ok(())
    }
}
impl WorkspaceBridge for Bridge {
    fn read_attachment(&self, path: &str, max: usize) -> ChatResult<Option<Vec<u8>>> {
        let server = self.server()?;
        let w = server.workspace.lock().unwrap();
        if !matches!(
            w.filework.existing_path_kind(path).map_err(store_error)?,
            Some(workflow_filework::ExistingPathKind::File)
        ) {
            return Ok(None);
        }
        let mut data = Vec::new();
        let mut offset = 0;
        loop {
            let chunk = w.filework.read_chunk(path, offset, 65_536).map_err(store_error)?;
            if chunk.size > max as u64 {
                return Err(ChatError::invalid("Agent resource exceeds read limit"));
            }
            data.extend_from_slice(&chunk.data);
            offset = chunk.next_offset;
            if chunk.eof || chunk.data.is_empty() {
                return Ok(Some(data));
            }
        }
    }
    fn acknowledge_composer(&self, submitted: &SubmittedComposer) -> ChatResult<()> {
        let submitted = workflow_environment::json::strict_json(&serde_json::to_vec(submitted)?)?;
        self.command(state::Command::AcknowledgeComposer { submitted })
    }
    fn remove_conversation(&self, id: &str) -> ChatResult<()> {
        self.command(state::Command::RemoveConversation {
            conversation_id: id.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn errors_keep_retained_codes_and_chunks_cover_the_frozen_snapshot() {
        let e = error(ChatError::invalid("bad"));
        assert_eq!((e.code, e.kind.as_str()), (-32602, "chat"));
        assert_eq!(error(ChatError::new(ErrorKind::DecisionNotOffered, "x")).code, -32602);
        assert_eq!(error(ChatError::new(ErrorKind::RequestExpired, "x")).code, -32000);
        let bytes = vec![7u8; CHUNK_BYTES + 3];
        let first = chunk("t", &bytes, 0).unwrap();
        assert_eq!(first.0["nextOffset"], CHUNK_BYTES);
        assert_eq!(first.0["eof"], false);
        let last = chunk("t", &bytes, CHUNK_BYTES as u64).unwrap();
        assert_eq!(last.0["eof"], true);
        assert_eq!(last.0["data"], "BwcH");
        assert!(chunk("t", &bytes, bytes.len() as u64 + 1).is_err());
    }
    #[test]
    fn guest_home_reads_refuse_credentials_and_foreign_paths() {
        let dir = tempfile::tempdir().unwrap();
        let store = workflow_environment::Store::open(dir.path()).unwrap();
        store
            .write_bytes("agents/claude/projects/-workspace/s.jsonl", b"{}\n")
            .unwrap();
        let home = Home(store);
        assert_eq!(
            home.read("/home/work/.claude/projects/-workspace/s.jsonl", 10).unwrap().unwrap(),
            b"{}\n"
        );
        assert!(home.read("/home/work/.claude/projects/-workspace/missing.jsonl", 10).unwrap().is_none());
        assert!(home.read("/home/work/.claude/.credentials.json", 10).is_err());
        assert!(home.read("/workspace/.workspace/state/x", 10).is_err());
        assert!(home.read("/home/work/.claude/projects/-workspace/s.jsonl", 1).is_err());
    }
}
