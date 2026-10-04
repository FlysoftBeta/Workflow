//! The Chat service: backend registry, conversation policy, journal, index and send ledger. Every
//! method blocks the calling Server worker; vendor IO runs on the backends' transport threads.
//! Nothing here answers a vendor request on its own.
use super::{
    index::{ConversationIndex, take_utf16},
    ledger::{ComposerAttachment, SendLedger},
    paths,
    tools::ToolPolicy,
};
use crate::{
    backend::{Backend, EventSink, ThreadOptions, Token, spawn},
    claude::{
        self,
        backend::{ClaudeBackend, ClaudeConfig},
        launch::LaunchConfig,
    },
    codex::{self, backend::CodexBackend, launch::CodexConfig},
    config::{AgentPatch, BackendPatch},
    error::{ChatError, ErrorKind, Result},
    journal::Journal,
    model::*,
    ports::*,
    service::launch_env::Secret,
};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};
use workflow_environment::{access::AGENT_HOMES, config::FieldPatch, tools::ToolPhase};

/// 128 + SIGSYS: what a wait reports for a seccomp kill.
pub const SIGSYS_EXIT: i32 = 128 + 31;
const KINDS: [BackendKind; 2] = [BackendKind::Codex, BackendKind::Claude];
const TOOL_REFRESH: Duration = Duration::from_secs(3);
const INDEX_COALESCE: Duration = Duration::from_millis(250);

/// Everything Chat needs from other domains, implemented by Server composition.
#[derive(Clone)]
pub struct ChatPorts {
    pub runtime: Arc<dyn AgentRuntime>,
    pub guest_home: Arc<dyn GuestHome>,
    pub tools: Arc<dyn AgentTools>,
    pub store: Arc<dyn ChatStore>,
    pub preferences: Arc<dyn AgentPreferences>,
    pub workspace: Arc<dyn WorkspaceBridge>,
    pub clock: Arc<dyn Clock>,
    pub ids: Arc<dyn IdSource>,
}

pub fn login_methods(kind: BackendKind) -> Vec<LoginMethod> {
    match kind {
        BackendKind::Codex => vec![
            LoginMethod::CodexDeviceCode,
            LoginMethod::CodexBrowser,
            LoginMethod::CodexApiKey,
        ],
        BackendKind::Claude => vec![
            LoginMethod::ClaudeTerminalLogin,
            LoginMethod::ClaudeSetupToken,
            LoginMethod::ClaudeApiKey,
        ],
    }
}
pub fn backend_of(id: &str) -> Option<BackendKind> {
    match id {
        "codex" => Some(BackendKind::Codex),
        "claude" => Some(BackendKind::Claude),
        _ => None,
    }
}
fn preset_of(value: &str) -> Option<PermissionPreset> {
    match value.to_ascii_uppercase().as_str() {
        "ASK" => Some(PermissionPreset::Ask),
        "AUTO_EDIT" => Some(PermissionPreset::AutoEdit),
        "PLAN" => Some(PermissionPreset::Plan),
        "DENY_UNLISTED" => Some(PermissionPreset::DenyUnlisted),
        _ => None,
    }
}
pub fn preset_name(preset: PermissionPreset) -> &'static str {
    match preset {
        PermissionPreset::Ask => "ask",
        PermissionPreset::AutoEdit => "auto_edit",
        PermissionPreset::Plan => "plan",
        PermissionPreset::DenyUnlisted => "deny_unlisted",
    }
}
fn guest_home(id: &str) -> &'static str {
    AGENT_HOMES
        .iter()
        .find(|h| h.id == id)
        .map(|h| h.guest)
        .expect("declared agent home")
}

/// The service facade. Dropping it does not stop backends; call [`Chat::shutdown`].
pub struct Chat {
    pub(super) hub: Arc<Hub>,
}
pub(super) struct Hub {
    me: Weak<Hub>,
    pub(super) ports: ChatPorts,
    pub(super) journal: Journal,
    pub(super) index: ConversationIndex,
    pub(super) ledger: SendLedger,
    policy: ToolPolicy,
    tools_gate: Mutex<HashSet<BackendKind>>,
    setups: Mutex<HashSet<BackendKind>>,
    unsupported: Mutex<HashSet<BackendKind>>,
    ready_seen: Mutex<HashSet<BackendKind>>,
    backends: Mutex<HashMap<BackendKind, Arc<dyn Backend>>>,
    lifecycle: Mutex<()>,
    resumed: Mutex<HashSet<(BackendKind, String)>>,
    thread_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    dirty: Mutex<Option<mpsc::Sender<ThreadKey>>>,
    stopping: Token,
    closed: AtomicBool,
}

/// Backends publish into the journal; the hub keeps cheap bookkeeping only.
struct HubSink(Weak<Hub>);
impl EventSink for HubSink {
    fn emit(&self, event: AgentEvent) {
        if let Some(hub) = self.0.upgrade() {
            let _ = hub.journal.event(event.clone());
            hub.observe(&event);
        }
    }
}

impl Chat {
    /// Loads the conversation index and starts the index-maintenance and tool-status threads.
    pub fn start(ports: ChatPorts) -> Result<Chat> {
        let index = ConversationIndex::load(ports.store.clone())?;
        let ledger = SendLedger::new(ports.store.clone(), ports.workspace.clone());
        let (sender, receiver) = mpsc::channel();
        let hub = Arc::new_cyclic(|me| Hub {
            me: me.clone(),
            ports,
            journal: Journal::default(),
            index,
            ledger,
            policy: ToolPolicy::default(),
            tools_gate: Mutex::new(HashSet::new()),
            setups: Mutex::new(HashSet::new()),
            unsupported: Mutex::new(HashSet::new()),
            ready_seen: Mutex::new(HashSet::new()),
            backends: Mutex::new(HashMap::new()),
            lifecycle: Mutex::new(()),
            resumed: Mutex::new(HashSet::new()),
            thread_locks: Mutex::new(HashMap::new()),
            dirty: Mutex::new(Some(sender)),
            stopping: Token::new(),
            closed: AtomicBool::new(false),
        });
        {
            let weak = Arc::downgrade(&hub);
            spawn("chat-index", move || maintain_index(weak, receiver));
        }
        // A selected tool's measured failure stays visible until an explicit retry.
        let _ = hub.refresh_tools(None);
        hub.publish_metadata();
        {
            let weak = Arc::downgrade(&hub);
            let stopping = hub.stopping.clone();
            spawn("chat-tools", move || {
                while stopping.sleep(TOOL_REFRESH) {
                    let Some(hub) = weak.upgrade() else { return };
                    let _ = hub.refresh_tools(None);
                    hub.publish_metadata();
                }
            });
        }
        Ok(Chat { hub })
    }
    /// Stops every backend; their open requests expire through the events of the process exit.
    pub fn shutdown(&self) {
        let hub = &self.hub;
        hub.closed.store(true, Ordering::SeqCst);
        hub.stopping.cancel();
        hub.dirty.lock().unwrap().take();
        let _gate = hub.tools_gate.lock().unwrap();
        for kind in KINDS {
            hub.replace_backend(kind, false);
        }
    }
    pub fn snapshot(&self) -> ChatSnapshot {
        self.hub.journal.snapshot()
    }
    /// Long-polls for ordered events after `after`; a gap or a foreign epoch asks for a snapshot.
    pub fn watch(&self, epoch: &str, after: i64, wait: Duration) -> Result<ChatUpdate> {
        self.hub.journal.watch(epoch, after, wait)
    }
    /// One `chat.command` in the retained `ChatWire` shape.
    pub fn command(&self, name: &str, args: &OpaqueObject) -> Result<OpaqueJson> {
        if self.hub.closed.load(Ordering::SeqCst) {
            return Err(ChatError::new(ErrorKind::Closed, "chat service stopped"));
        }
        let result = super::commands::dispatch(&self.hub, name, args);
        self.hub.publish_metadata();
        result
    }
}

impl Hub {
    // ---------------------------------------------------------------- tools and backends

    /// Measures tools, demands the optional Claude install once per generation, and installs or
    /// removes backends to match the verified tool status.
    pub(super) fn refresh_tools(&self, required: Option<BackendKind>) -> Result<()> {
        let mut installed = self.tools_gate.lock().unwrap();
        if self.closed.load(Ordering::SeqCst) {
            return Ok(());
        }
        let status = self
            .policy
            .refresh(&*self.ports.tools, self.default_backend(), required)?;
        for kind in KINDS {
            if self.closed.load(Ordering::SeqCst) {
                return Ok(());
            }
            let ready = status
                .tools
                .iter()
                .any(|t| t.id == kind.id() && t.phase == ToolPhase::Ready);
            if ready && !installed.contains(&kind) {
                self.replace_backend(kind, true);
                installed.insert(kind);
            } else if !ready && installed.contains(&kind) {
                self.replace_backend(kind, false);
                installed.remove(&kind);
            }
        }
        Ok(())
    }

    /// Installs or removes how `kind` runs. A running backend of that kind is stopped first.
    fn replace_backend(&self, kind: BackendKind, available: bool) {
        let _lifecycle = self.lifecycle.lock().unwrap();
        if let Some(backend) = self.backends.lock().unwrap().remove(&kind) {
            backend.stop();
        }
        self.resumed.lock().unwrap().retain(|(k, _)| *k != kind);
        self.ready_seen.lock().unwrap().remove(&kind);
        self.unsupported.lock().unwrap().remove(&kind);
        let mut setups = self.setups.lock().unwrap();
        if available {
            setups.insert(kind);
        } else {
            setups.remove(&kind);
        }
    }

    fn create(&self, kind: BackendKind) -> Arc<dyn Backend> {
        let p = &self.ports;
        let sink: Arc<dyn EventSink> = Arc::new(HubSink(self.me.clone()));
        let base = p.runtime.base_env();
        let permissions = self.default_permissions();
        match kind {
            BackendKind::Codex => {
                let mut config = CodexConfig::new(codex::launch::EXECUTABLE, guest_home("codex"));
                config.cwd = paths::ROOT.into();
                config.base_env = base;
                config.default_permissions = permissions;
                Arc::new(CodexBackend::new(
                    p.runtime.clone(),
                    config,
                    sink,
                    p.clock.clone(),
                    p.ids.clone(),
                ))
            }
            BackendKind::Claude => {
                let workspace = p.workspace.clone();
                let home = p.guest_home.clone();
                Arc::new(ClaudeBackend::new(
                    p.runtime.clone(),
                    ClaudeConfig {
                        launch: LaunchConfig {
                            executable: claude::launch::EXECUTABLE.into(),
                            config_dir: guest_home("claude").into(),
                            tmp_dir: "/tmp".into(),
                            base_env: base,
                            credentials: BTreeMap::new(),
                            default_permissions: permissions,
                            extra_args: vec![],
                        },
                        cwd: paths::ROOT.into(),
                        hooks: vec![],
                        max_inline_bytes: 20 * 1024 * 1024,
                    },
                    sink,
                    p.clock.clone(),
                    p.ids.clone(),
                    Arc::new(move |path: &str, max: usize| {
                        let relative = paths::to_workspace(path).ok_or_else(|| {
                            ChatError::invalid("Agent resource is outside permitted guest roots")
                        })?;
                        workspace
                            .read_attachment(&relative, max)?
                            .ok_or_else(|| ChatError::new(ErrorKind::NotFound, path.to_string()))
                    }),
                    Some(Arc::new(move |path: &str, max: usize| home.read(path, max))),
                ))
            }
        }
    }

    /// Starts the backend process if needed and returns it (idempotent).
    pub(super) fn connect(&self, kind: BackendKind) -> Result<Arc<dyn Backend>> {
        let backend = {
            let _lifecycle = self.lifecycle.lock().unwrap();
            let mut backends = self.backends.lock().unwrap();
            match backends.get(&kind) {
                Some(b) => b.clone(),
                None => {
                    if !self.setups.lock().unwrap().contains(&kind) {
                        return Err(ChatError::unavailable(format!(
                            "{} is not available",
                            kind.id()
                        )));
                    }
                    let b = self.create(kind);
                    backends.insert(kind, b.clone());
                    b
                }
            }
        };
        backend.start()?;
        Ok(backend)
    }
    pub(super) fn running(&self, kind: BackendKind) -> Option<Arc<dyn Backend>> {
        self.backends.lock().unwrap().get(&kind).cloned()
    }

    /// Called for every reduced event: cheap bookkeeping only.
    fn observe(&self, event: &AgentEvent) {
        match event {
            AgentEvent::ProcessChanged { backend, state } => match state {
                ProcessState::Ready {} => {
                    self.ready_seen.lock().unwrap().insert(*backend);
                }
                ProcessState::Exited { .. } | ProcessState::Failed { .. } => {
                    // Killed by a syscall restriction before its handshake: this launcher cannot
                    // run here.
                    if matches!(
                        state,
                        ProcessState::Exited {
                            exit_code: Some(SIGSYS_EXIT),
                            ..
                        }
                    ) && !self.ready_seen.lock().unwrap().contains(backend)
                    {
                        self.unsupported.lock().unwrap().insert(*backend);
                        self.publish_metadata();
                    }
                    self.resumed.lock().unwrap().retain(|(k, _)| k != backend);
                }
                _ => (),
            },
            AgentEvent::ThreadUpserted {
                backend, thread_id, ..
            }
            | AgentEvent::ThreadRenamed {
                backend, thread_id, ..
            }
            | AgentEvent::TurnSubmitted {
                backend, thread_id, ..
            }
            | AgentEvent::TurnCompleted {
                backend, thread_id, ..
            }
            | AgentEvent::HistoryLoaded {
                backend, thread_id, ..
            } => {
                if let Some(sender) = self.dirty.lock().unwrap().as_ref() {
                    let _ = sender.send(ThreadKey {
                        backend: *backend,
                        id: thread_id.clone(),
                    });
                }
            }
            _ => (),
        }
    }

    pub(super) fn publish_metadata(&self) {
        let setups = self.setups.lock().unwrap().clone();
        let unsupported = self.unsupported.lock().unwrap().clone();
        let _ = self.journal.metadata(ChatMetadata {
            conversations: self.index.entries(),
            available: KINDS
                .into_iter()
                .filter(|k| setups.contains(k) && !unsupported.contains(k))
                .collect(),
            login_methods: KINDS.into_iter().map(|k| (k, login_methods(k))).collect(),
            permissions: self.default_permissions(),
            default_backend: self.default_backend(),
            process_epochs: BTreeMap::new(),
        });
    }

    // ---------------------------------------------------------------- preferences

    pub(super) fn default_backend(&self) -> BackendKind {
        backend_of(&self.ports.preferences.get().backend).unwrap_or(BackendKind::Codex)
    }
    pub(super) fn default_permissions(&self) -> PermissionPreset {
        self.ports
            .preferences
            .get()
            .permissions
            .as_deref()
            .and_then(preset_of)
            .unwrap_or(PermissionPreset::Ask)
    }
    fn remembered(&self, kind: BackendKind) -> (Option<String>, Option<String>) {
        self.ports
            .preferences
            .get()
            .backends
            .get(kind.id())
            .map(|d| (d.model.clone(), d.effort.clone()))
            .unwrap_or_default()
    }

    // ---------------------------------------------------------------- conversations

    pub(super) fn entry(&self, id: &str) -> Option<ConversationEntry> {
        self.index.entries().into_iter().find(|e| e.id == id)
    }
    fn thread_key(entry: &ConversationEntry) -> Option<ThreadKey> {
        entry.backend_thread_id.as_ref().map(|id| ThreadKey {
            backend: entry.backend,
            id: id.clone(),
        })
    }
    fn lock(&self, id: &str) -> Arc<Mutex<()>> {
        self.thread_locks
            .lock()
            .unwrap()
            .entry(id.into())
            .or_default()
            .clone()
    }
    fn options(&self, settings: &TurnSettings) -> ThreadOptions {
        let mut options = ThreadOptions::new(paths::ROOT);
        options.settings = TurnSettings {
            permissions: settings.permissions.or(Some(self.default_permissions())),
            ..settings.clone()
        };
        options
    }
    fn logged_in(&self, kind: BackendKind) -> bool {
        self.journal.backend(kind).account.state != LoginState::LoggedOut
    }
    fn active_turn(&self, key: &ThreadKey) -> bool {
        self.journal
            .thread(key)
            .is_some_and(|t| t.turns.iter().any(|t| t.status == TurnStatus::Running))
    }

    /// Creates a thread-less conversation on `backend` (default: the last chosen one).
    pub(super) fn new_conversation(
        &self,
        backend: Option<BackendKind>,
        id: String,
    ) -> Result<String> {
        let kind = backend.unwrap_or_else(|| self.default_backend());
        let (model, effort) = self.remembered(kind);
        let now = self.ports.clock.now_ms();
        self.index.upsert(ConversationEntry {
            id: id.clone(),
            backend: kind,
            cwd: paths::ROOT.into(),
            created_at_ms: now,
            updated_at_ms: now,
            model,
            effort,
            ..Default::default()
        })?;
        Ok(id)
    }
    /// The entry of `id`; an ID the index does not know becomes a new conversation.
    pub(super) fn ensure_conversation(&self, id: &str) -> Result<ConversationEntry> {
        if let Some(entry) = self.entry(id) {
            return Ok(entry);
        }
        self.new_conversation(None, id.into())?;
        self.entry(id)
            .ok_or_else(|| ChatError::state("conversation was not created"))
    }
    /// Switches a conversation without a thread to another backend and remembers the choice.
    pub(super) fn set_backend(&self, id: &str, kind: BackendKind) -> Result<()> {
        let (model, effort) = self.remembered(kind);
        self.index.update(id, |e| {
            if e.backend_thread_id.is_none() {
                e.backend = kind;
                e.model = model;
                e.effort = effort;
                e.cwd = paths::ROOT.into();
            }
        })?;
        self.ports.preferences.update(AgentPatch {
            backend: FieldPatch::Value(kind.id().into()),
            ..Default::default()
        })
    }
    /// Makes `id`'s thread live: starts the process and resumes the thread with its history once
    /// per process. A conversation without a thread has nothing to resume.
    pub(super) fn open(&self, id: &str) -> Result<()> {
        let entry = self.ensure_conversation(id)?;
        if !self.setups.lock().unwrap().contains(&entry.backend) {
            return Ok(());
        }
        let backend = self.connect(entry.backend)?;
        let Some(key) = Self::thread_key(&entry) else {
            return Ok(());
        };
        let lock = self.lock(id);
        let _guard = lock.lock().unwrap();
        if self
            .resumed
            .lock()
            .unwrap()
            .contains(&(key.backend, key.id.clone()))
        {
            return Ok(());
        }
        if !self.logged_in(entry.backend) {
            return Ok(());
        }
        backend.resume_thread(&key.id, &self.options(&TurnSettings::default()))?;
        self.resumed.lock().unwrap().insert((key.backend, key.id));
        Ok(())
    }
    pub(super) fn load_earlier(&self, id: &str) -> Result<()> {
        let Some(entry) = self.entry(id) else {
            return Ok(());
        };
        let Some(key) = Self::thread_key(&entry) else {
            return Ok(());
        };
        let Some(cursor) = self.journal.thread(&key).and_then(|t| t.history_cursor) else {
            return Ok(());
        };
        self.connect(entry.backend)?
            .load_history(&key.id, Some(&cursor))
    }
    /// Sends a user message, creating the backend thread on the first message. Returns the
    /// client message ID.
    pub(super) fn send(
        &self,
        id: &str,
        text: &str,
        attachments: &[ComposerAttachment],
        settings: &TurnSettings,
        mode: SendMode,
    ) -> Result<String> {
        let entry = self.ensure_conversation(id)?;
        let backend = self.connect(entry.backend)?;
        let mut parts = Vec::new();
        if !text.trim().is_empty() {
            parts.push(UserPart::Text { text: text.into() });
        }
        for a in attachments {
            let path = paths::to_agent(&a.path)?;
            let mime = a
                .mime_type
                .clone()
                .or_else(|| paths::guess_mime(&a.path).map(str::to_owned));
            parts.push(
                if mime.as_deref().is_some_and(|m| m.starts_with("image/")) {
                    UserPart::Image {
                        path,
                        mime_type: mime,
                    }
                } else {
                    UserPart::File {
                        path,
                        mime_type: mime,
                    }
                },
            );
        }
        if parts.is_empty() {
            return Err(ChatError::invalid("Nothing to send"));
        }
        let thread = {
            let lock = self.lock(id);
            let _guard = lock.lock().unwrap();
            let current = self.entry(id).unwrap_or(entry);
            match Self::thread_key(&current) {
                None => {
                    let created = backend.start_thread(&self.options(settings))?;
                    self.resumed
                        .lock()
                        .unwrap()
                        .insert((current.backend, created.clone()));
                    let now = self.ports.clock.now_ms();
                    let thread = created.clone();
                    self.index.update(id, |e| {
                        e.backend_thread_id = Some(thread);
                        e.updated_at_ms = now;
                        if e.preview.is_none() {
                            e.preview = Some(take_utf16(text, 200));
                        }
                    })?;
                    created
                }
                Some(key) => {
                    let resumed = (key.backend, key.id.clone());
                    if !self.resumed.lock().unwrap().contains(&resumed) {
                        backend.resume_thread(&key.id, &self.options(&TurnSettings::default()))?;
                        self.resumed.lock().unwrap().insert(resumed);
                    }
                    key.id
                }
            }
        };
        backend.send(&thread, parts, Some(settings.clone()), mode)
    }
    pub(super) fn with_thread(
        &self,
        id: &str,
        action: impl FnOnce(Arc<dyn Backend>, &str) -> Result<()>,
    ) -> Result<()> {
        let Some(entry) = self.entry(id) else {
            return Ok(());
        };
        let Some(key) = Self::thread_key(&entry) else {
            return Ok(());
        };
        action(self.connect(entry.backend)?, &key.id)
    }
    pub(super) fn respond(&self, key: &RequestKey, response: &RequestResponse) -> Result<()> {
        self.running(key.backend)
            .ok_or_else(|| ChatError::state("backend not running"))?
            .respond(key, response)
    }
    pub(super) fn rename(&self, id: &str, title: &str) -> Result<()> {
        let clean = take_utf16(title.trim(), 200);
        let Some(entry) = self.index.update(id, |e| {
            e.title = Some(clean.clone()).filter(|t| !t.is_empty());
        })?
        else {
            return Ok(());
        };
        if let (Some(key), Some(backend)) = (Self::thread_key(&entry), self.running(entry.backend))
            && !clean.is_empty()
        {
            let _ = backend.rename(&key.id, &clean);
        }
        Ok(())
    }
    pub(super) fn archive(&self, id: &str, archived: bool) -> Result<()> {
        let now = self.ports.clock.now_ms();
        let Some(entry) = self.index.update(id, |e| {
            e.archived = archived;
            e.updated_at_ms = now;
        })?
        else {
            return Ok(());
        };
        if let (Some(key), Some(backend)) = (Self::thread_key(&entry), self.running(entry.backend))
        {
            let _ = backend.archive(&key.id, archived);
        }
        Ok(())
    }
    /// Called only after the user confirmed deletion, including the composer draft.
    pub(super) fn delete_conversation(&self, id: &str) -> Result<()> {
        let Some(entry) = self.entry(id) else {
            return Ok(());
        };
        if let Some(key) = Self::thread_key(&entry) {
            if self.active_turn(&key) {
                return Err(ChatError::new(
                    ErrorKind::TurnActive,
                    "请先停止正在进行的回答",
                ));
            }
            self.connect(entry.backend)?.delete(&key.id)?;
            self.resumed.lock().unwrap().remove(&(key.backend, key.id));
        }
        self.ports.workspace.remove_conversation(id)?;
        self.index.remove(id)?;
        // Ledger records are keyed by conversation; they are not kept after deletion.
        let _ = self.ports.store.remove_sends(id);
        Ok(())
    }
    pub(super) fn compact(&self, id: &str) -> Result<()> {
        let Some(entry) = self.entry(id) else {
            return Ok(());
        };
        let Some(key) = Self::thread_key(&entry) else {
            return Ok(());
        };
        if self.active_turn(&key) {
            return Err(ChatError::new(
                ErrorKind::TurnActive,
                "请先停止正在进行的回答",
            ));
        }
        self.open(id)?;
        self.connect(entry.backend)?.compact(&key.id)
    }
    /// The explicit advanced console keeps the backend's approval policy checks.
    pub(super) fn raw_request(
        &self,
        id: &str,
        method: &str,
        params: Option<OpaqueJson>,
    ) -> Result<OpaqueJson> {
        let entry = self.ensure_conversation(id)?;
        self.open(id)?;
        let method = method.trim();
        if method.is_empty() {
            return Err(ChatError::invalid("method is required"));
        }
        self.connect(entry.backend)?
            .raw_request(method, params, entry.backend_thread_id.as_deref())
    }
    /// Branches `id` after `at_turn` (whole conversation when absent) into a new conversation.
    pub(super) fn fork(&self, id: &str, at_turn: Option<&str>) -> Result<String> {
        let entry = self
            .entry(id)
            .ok_or_else(|| ChatError::invalid("unknown conversation"))?;
        let key =
            Self::thread_key(&entry).ok_or_else(|| ChatError::state("nothing to fork yet"))?;
        let backend = self.connect(entry.backend)?;
        let created =
            backend.fork_thread(&key.id, at_turn, &self.options(&TurnSettings::default()))?;
        self.resumed
            .lock()
            .unwrap()
            .insert((entry.backend, created.clone()));
        let now = self.ports.clock.now_ms();
        let new_id = self.ports.ids.new_id();
        self.index.upsert(ConversationEntry {
            id: new_id.clone(),
            backend_thread_id: Some(created),
            title: entry.title.as_ref().map(|t| format!("{t}（分叉）")),
            created_at_ms: now,
            updated_at_ms: now,
            archived: false,
            forked_from: Some(entry.id.clone()),
            forked_at: at_turn.map(str::to_owned),
            ..entry
        })?;
        Ok(new_id)
    }
    pub(super) fn set_permissions(&self, id: &str, preset: PermissionPreset) -> Result<()> {
        self.ports.preferences.update(AgentPatch {
            permissions: FieldPatch::Value(Some(preset_name(preset).into())),
            ..Default::default()
        })?;
        let Some(entry) = self.entry(id) else {
            return Ok(());
        };
        let Some(key) = Self::thread_key(&entry) else {
            return Ok(());
        };
        match self.running(entry.backend) {
            Some(backend) => backend.set_permissions(&key.id, preset),
            None => Ok(()),
        }
    }
    /// Remembers the slider position for this conversation and as the backend default.
    pub(super) fn remember_selection(
        &self,
        id: &str,
        model: Option<String>,
        effort: Option<String>,
    ) -> Result<()> {
        let Some(entry) = self.index.update(id, |e| {
            e.model = model.clone();
            e.effort = effort.clone();
        })?
        else {
            return Ok(());
        };
        self.ports.preferences.update(AgentPatch {
            backends: FieldPatch::Value(BTreeMap::from([(
                entry.backend.id().to_string(),
                BackendPatch {
                    model: FieldPatch::Value(model),
                    effort: FieldPatch::Value(effort),
                    ..Default::default()
                },
            )])),
            ..Default::default()
        })
    }
    pub(super) fn login(
        &self,
        kind: BackendKind,
        method: LoginMethod,
        secret: Option<Secret>,
    ) -> Result<LoginFlow> {
        self.connect(kind)?.login(method, secret)
    }
}

/// Folds thread titles, previews and activity times into the index, coalesced.
fn maintain_index(hub: Weak<Hub>, receiver: mpsc::Receiver<ThreadKey>) {
    while let Ok(first) = receiver.recv() {
        let mut batch = vec![first];
        std::thread::sleep(INDEX_COALESCE);
        while let Ok(next) = receiver.try_recv() {
            if !batch.contains(&next) {
                batch.push(next);
            }
        }
        let Some(hub) = hub.upgrade() else { return };
        let now = hub.ports.clock.now_ms();
        let mut changed = false;
        for key in batch {
            if let Some(thread) = hub.journal.thread(&key) {
                let before = hub.index.entries();
                if hub.index.refresh_thread(&thread, now).is_ok() && hub.index.entries() != before {
                    changed = true;
                }
            }
        }
        if changed {
            hub.publish_metadata();
        }
    }
}

impl Drop for Chat {
    fn drop(&mut self) {
        self.hub.stopping.cancel();
    }
}
