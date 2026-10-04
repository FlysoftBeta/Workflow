//! Claude Code adapter: one CLI process per session (`-p` stream-json plus the control protocol).
//! A spare process started by `start` answers account and model queries and can become the next
//! new thread. Listing, archival and deletion are Engine-level because Claude sessions are
//! transcript files.
use super::{
    launch::{self, ALLOWED_MODES, LaunchConfig, Session as Attach},
    mapper::{self, ClaudeMapper},
    requests::{self, ClaudeHook},
    transcript,
    wire::{self as cw},
};
use crate::{
    backend::{Backend, EventSink, ThreadOptions, spawn},
    codex::requests::UserReply,
    error::{ChatError, ErrorKind, Result},
    model::*,
    ports::{AgentProcess, AgentRuntime, Clock, IdSource},
    service::launch_env::Secret,
    transport::control::{ControlConnection, ControlInbound, default_ids},
    wire::{self, Json},
};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock, Weak},
};

const B: BackendKind = BackendKind::Claude;
const MAX_TRANSCRIPT: usize = 16 * 1024 * 1024;

/// Reads attachment bytes by guest path, at most `max` bytes. A missing file is an error.
pub type Attachments = Arc<dyn Fn(&str, usize) -> Result<Vec<u8>> + Send + Sync>;
/// Reads a transcript by guest path; `None` when it does not exist.
pub type Transcripts = Arc<dyn Fn(&str, usize) -> Result<Option<Vec<u8>>> + Send + Sync>;

pub struct ClaudeConfig {
    pub launch: LaunchConfig,
    pub cwd: String,
    /// Crate-private and test-only in practice; production registers none.
    pub hooks: Vec<ClaudeHook>,
    /// Largest file inlined as base64 (image or PDF).
    pub max_inline_bytes: usize,
}

struct SessionState {
    model: Option<String>,
    effort: Option<String>,
    mode: String,
    submitted: Vec<String>,
    initialized: bool,
}
struct Session {
    id: String,
    cwd: String,
    process: Arc<dyn AgentProcess>,
    connection: Arc<ControlConnection>,
    mapper: Mutex<ClaudeMapper>,
    state: Mutex<SessionState>,
}

pub struct ClaudeBackend(Arc<Inner>);
struct Inner {
    me: Weak<Inner>,
    runtime: Arc<dyn AgentRuntime>,
    config: ClaudeConfig,
    sink: Arc<dyn EventSink>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdSource>,
    attachments: Attachments,
    transcripts: Option<Transcripts>,
    lifecycle: Mutex<()>,
    sessions: Mutex<HashMap<String, Arc<Session>>>,
    spare: Mutex<Option<Arc<Session>>>,
    /// request_id → (session, card).
    open_requests: Mutex<HashMap<String, (String, PendingRequest)>>,
    /// Held in memory only; persisting it is never Chat's job.
    api_key: Mutex<Option<Secret>>,
}

fn notice(
    level: NoticeLevel,
    message: impl Into<String>,
    code: &str,
    detail: Option<String>,
) -> Notice {
    Notice {
        level,
        message: message.into(),
        code: Some(code.into()),
        detail,
        ..Default::default()
    }
}
fn fields<T: Serialize>(value: &T) -> OpaqueObject {
    wire::project(&wire::encode(value))
}
#[derive(Serialize)]
struct Model<'a> {
    model: &'a str,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Effort<'a> {
    effort_level: &'a str,
}
#[derive(Serialize)]
struct FlagSettings<'a> {
    settings: Effort<'a>,
}
#[derive(Serialize)]
struct Mode<'a> {
    mode: &'a str,
}
#[derive(Serialize)]
struct Rename<'a> {
    title: &'a str,
    source: &'static str,
}
#[derive(Serialize)]
struct CancelAsync<'a> {
    message_uuid: &'a str,
}
#[derive(Serialize)]
struct Interrupt {
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    cancel_queued: bool,
}

impl ClaudeBackend {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        runtime: Arc<dyn AgentRuntime>,
        config: ClaudeConfig,
        sink: Arc<dyn EventSink>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdSource>,
        attachments: Attachments,
        transcripts: Option<Transcripts>,
    ) -> Self {
        Self(Arc::new_cyclic(|me| Inner {
            me: me.clone(),
            runtime,
            config,
            sink,
            clock,
            ids,
            attachments,
            transcripts,
            lifecycle: Mutex::new(()),
            sessions: Mutex::new(HashMap::new()),
            spare: Mutex::new(None),
            open_requests: Mutex::new(HashMap::new()),
            api_key: Mutex::new(None),
        }))
    }
    /// Test hook: request IDs currently open on any session.
    pub fn open_request_ids(&self) -> Vec<String> {
        let mut ids: Vec<_> = self
            .0
            .open_requests
            .lock()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        ids.sort();
        ids
    }
}

impl Inner {
    fn emit(&self, event: AgentEvent) {
        self.sink.emit(event);
    }

    // ---------------------------------------------------------------- process lifecycle

    fn launch(
        &self,
        attach: Attach<'_>,
        cwd: &str,
        settings: &TurnSettings,
    ) -> Result<Arc<Session>> {
        let id = attach.id().to_string();
        let mut config = self.config.launch.clone();
        if let Some(key) = self.api_key.lock().unwrap().clone() {
            config.credentials.insert("ANTHROPIC_API_KEY".into(), key);
        }
        let spec = launch::spec(
            &config,
            cwd,
            &attach,
            settings.model.as_deref(),
            settings.effort.as_deref(),
            settings.permissions,
        )?;
        let (process, stdio) = self.runtime.spawn(&spec)?;
        let slot: Arc<OnceLock<Weak<Session>>> = Arc::new(OnceLock::new());
        let me = self.me.clone();
        let target = slot.clone();
        let connection = ControlConnection::open(
            stdio,
            default_ids(),
            Box::new(move |_, message| {
                let (Some(me), Some(session)) = (me.upgrade(), target.wait().upgrade()) else {
                    return;
                };
                me.handle(&session, message);
            }),
        );
        let session = Arc::new(Session {
            id: id.clone(),
            cwd: cwd.into(),
            process: process.clone(),
            connection: connection.clone(),
            mapper: Mutex::new(ClaudeMapper::new(Some(&id))),
            state: Mutex::new(SessionState {
                model: settings.model.clone(),
                effort: settings.effort.clone(),
                mode: launch::permission_mode(
                    settings
                        .permissions
                        .unwrap_or(self.config.launch.default_permissions),
                )
                .into(),
                submitted: Vec::new(),
                initialized: false,
            }),
        });
        let _ = slot.set(Arc::downgrade(&session));
        self.sessions
            .lock()
            .unwrap()
            .insert(id.clone(), session.clone());
        {
            let me = self.me.upgrade().expect("backend alive");
            let s = session.clone();
            spawn("claude-exit", move || {
                let code = s.process.wait();
                s.connection.join();
                me.exited(&s, code);
            });
        }
        match connection.control("initialize", &requests::initialize(&self.config.hooks)) {
            Ok(response) => {
                session.state.lock().unwrap().initialized = true;
                let init: cw::Initialized = wire::project(&response);
                self.emit(AgentEvent::ServerInfo {
                    backend: B,
                    info: response.clone(),
                });
                self.emit(AgentEvent::AccountChanged {
                    backend: B,
                    account: mapper::account(&response),
                });
                if let Some(models) = init.models.get() {
                    self.emit(AgentEvent::ModelsChanged {
                        backend: B,
                        catalog: mapper::models(Some(models)),
                    });
                }
                self.emit(AgentEvent::ProcessChanged {
                    backend: B,
                    state: ProcessState::Ready {},
                });
                // Re-arm prompts a previous worker left pending (CLI 2.1.268 and later).
                for pending in init.pending_permission_requests.items() {
                    let Some(frame) = pending.object() else {
                        continue;
                    };
                    let p: cw::PendingFrame = wire::project(frame);
                    let (Some(request_id), Some(request)) =
                        (p.request_id.owned(), p.request.object())
                    else {
                        continue;
                    };
                    let subtype = wire::project::<cw::ControlBody>(request).subtype.or("");
                    if !matches!(subtype.as_str(), "can_use_tool" | "elicitation") {
                        continue;
                    }
                    connection.adopt(&request_id, &subtype);
                    let card = requests::pending(
                        &request_id,
                        &subtype,
                        request,
                        frame,
                        Some(&id),
                        None,
                        self.clock.now_ms(),
                    );
                    self.open(&session, card);
                }
                Ok(session)
            }
            Err(e) => {
                self.emit(AgentEvent::BackendNotice {
                    backend: B,
                    notice: notice(
                        NoticeLevel::Error,
                        format!("Claude Code failed to start: {}", e.message),
                        "startFailed",
                        Some(connection.stderr_tail(4000)),
                    ),
                });
                process.kill(true);
                self.sessions.lock().unwrap().remove(&id);
                Err(e)
            }
        }
    }

    fn exited(&self, s: &Arc<Session>, code: i32) {
        {
            let mut sessions = self.sessions.lock().unwrap();
            if sessions.get(&s.id).is_some_and(|x| Arc::ptr_eq(x, s)) {
                sessions.remove(&s.id);
            }
        }
        {
            let mut spare = self.spare.lock().unwrap();
            if spare.as_ref().is_some_and(|x| Arc::ptr_eq(x, s)) {
                *spare = None;
            }
        }
        let submitted = s.state.lock().unwrap().submitted.clone();
        for turn in submitted {
            let events = s.mapper.lock().unwrap().end_turn(
                &turn,
                TurnStatus::Failed,
                Some(TurnError {
                    message: format!("Claude Code exited ({code})"),
                    code: Some("processExited".into()),
                    ..Default::default()
                }),
            );
            events.into_iter().for_each(|e| self.emit(e));
        }
        let expired: Vec<String> = {
            let mut open = self.open_requests.lock().unwrap();
            let ids: Vec<String> = open
                .iter()
                .filter(|(_, (session, _))| *session == s.id)
                .map(|(id, _)| id.clone())
                .collect();
            for id in &ids {
                open.remove(id);
            }
            ids
        };
        for id in expired {
            self.emit(AgentEvent::RequestClosed {
                key: requests::key(&id),
                status: RequestStatus::Expired,
                answer: None,
            });
        }
        self.emit(AgentEvent::ThreadStatusChanged {
            backend: B,
            thread_id: s.id.clone(),
            run_state: RunState::NotLoaded,
        });
        if !matches!(code, 0 | 143 | 137) {
            self.emit(AgentEvent::ThreadNotice {
                backend: B,
                thread_id: s.id.clone(),
                notice: notice(
                    NoticeLevel::Warning,
                    format!("Claude Code exited with status {code}"),
                    "processExited",
                    Some(s.connection.stderr_tail(4000)),
                ),
            });
        }
    }

    fn start(&self) -> Result<()> {
        let _lifecycle = self.lifecycle.lock().unwrap();
        if self.spare.lock().unwrap().is_some() || !self.sessions.lock().unwrap().is_empty() {
            return Ok(());
        }
        self.emit(AgentEvent::ProcessChanged {
            backend: B,
            state: ProcessState::Starting {},
        });
        let id = self.ids.new_id();
        match self.launch(Attach::New(&id), &self.config.cwd, &TurnSettings::default()) {
            Ok(s) => {
                *self.spare.lock().unwrap() = Some(s);
                Ok(())
            }
            Err(e) => {
                self.emit(AgentEvent::ProcessChanged {
                    backend: B,
                    state: ProcessState::Failed {
                        message: e.message.clone(),
                        stderr_tail: String::new(),
                    },
                });
                Err(e)
            }
        }
    }

    fn stop(&self) {
        let _lifecycle = self.lifecycle.lock().unwrap();
        let sessions: Vec<_> = self.sessions.lock().unwrap().values().cloned().collect();
        for s in sessions {
            s.process.kill(false);
            s.connection.close_input();
        }
        *self.spare.lock().unwrap() = None;
        self.emit(AgentEvent::ProcessChanged {
            backend: B,
            state: ProcessState::Stopped {},
        });
    }

    fn any_session(&self) -> Result<Arc<Session>> {
        if let Some(s) = self.spare.lock().unwrap().clone() {
            return Ok(s);
        }
        self.sessions
            .lock()
            .unwrap()
            .values()
            .find(|s| s.state.lock().unwrap().initialized)
            .cloned()
            .ok_or_else(|| ChatError::unavailable("Claude Code is not running"))
    }
    fn session(&self, thread: &str) -> Result<Arc<Session>> {
        self.sessions
            .lock()
            .unwrap()
            .get(thread)
            .cloned()
            .ok_or_else(|| {
                ChatError::state(format!("session {thread} is not running; resume it first"))
            })
    }

    // ---------------------------------------------------------------- inbound

    fn handle(&self, s: &Arc<Session>, message: ControlInbound) {
        match message {
            ControlInbound::Message { raw, .. } => {
                let events = s.mapper.lock().unwrap().map(&raw);
                events.into_iter().for_each(|e| self.emit(e));
            }
            ControlInbound::Request {
                id,
                subtype,
                request,
                raw,
            } => self.control_request(s, &id, &subtype, &request, &raw),
            ControlInbound::Cancel { id, .. } => {
                self.open_requests.lock().unwrap().remove(&id);
                self.emit(AgentEvent::RequestClosed {
                    key: requests::key(&id),
                    status: RequestStatus::Cancelled,
                    answer: None,
                });
            }
            ControlInbound::Malformed { preview, reason } => self.emit(AgentEvent::ThreadNotice {
                backend: B,
                thread_id: s.id.clone(),
                notice: notice(
                    NoticeLevel::Warning,
                    format!("Malformed frame: {reason}"),
                    "malformed",
                    Some(preview),
                ),
            }),
        }
    }

    fn open(&self, s: &Session, request: PendingRequest) {
        let id = wire::decode::<String>(&request.key.raw_id).unwrap_or_default();
        self.open_requests
            .lock()
            .unwrap()
            .insert(id, (s.id.clone(), request.clone()));
        self.emit(AgentEvent::RequestOpened { request });
    }

    fn control_request(
        &self,
        s: &Arc<Session>,
        id: &str,
        subtype: &str,
        request: &OpaqueJson,
        raw: &OpaqueJson,
    ) {
        let turn = s.mapper.lock().unwrap().running_turn();
        let now = self.clock.now_ms();
        match subtype {
            "can_use_tool" | "elicitation" | "request_user_dialog" => {
                let card =
                    requests::pending(id, subtype, request, raw, Some(&s.id), turn.as_deref(), now);
                if subtype == "request_user_dialog" {
                    // It must never be answered by us.
                    s.connection.forget(id);
                }
                self.open(s, card);
            }
            "hook_callback" => {
                let body: cw::ControlBody = wire::project(request);
                let callback = body.callback_id.owned();
                match self
                    .config
                    .hooks
                    .iter()
                    .find(|h| Some(&h.callback_id) == callback.as_ref())
                {
                    None => {
                        let _ = s.connection.respond_error(
                            id,
                            &format!(
                                "no hook registered for {}",
                                callback.as_deref().unwrap_or("?")
                            ),
                        );
                        self.emit(AgentEvent::Unknown {
                            backend: B,
                            kind: "control_request/hook_callback".into(),
                            thread_id: Some(s.id.clone()),
                            raw: raw.clone(),
                        });
                    }
                    Some(hook) => {
                        let input = body
                            .input
                            .object()
                            .cloned()
                            .unwrap_or_else(wire::empty_object);
                        let events = s
                            .mapper
                            .lock()
                            .unwrap()
                            .hook_marker(&hook.callback_id, &input);
                        events.into_iter().for_each(|e| self.emit(e));
                        let _ = match (hook.handler)(&input) {
                            Ok(output) => s.connection.respond(id, Some(&output)),
                            Err(message) => s.connection.respond_error(id, &message),
                        };
                    }
                }
            }
            // SDK MCP messages are not supported: answered with an error and recorded as rejected.
            "mcp_message" => {
                self.emit(AgentEvent::RequestOpened {
                    request: requests::rejected(id, subtype, request, raw, Some(&s.id), now),
                });
                self.emit(AgentEvent::Unknown {
                    backend: B,
                    kind: format!("control_request/{subtype}"),
                    thread_id: Some(s.id.clone()),
                    raw: raw.clone(),
                });
                let _ = s
                    .connection
                    .respond_error(id, &format!("unsupported control request: {subtype}"));
            }
            // Unknown requests become a card whose only choice rejects them.
            _ => self.open(
                s,
                requests::pending(id, subtype, request, raw, Some(&s.id), turn.as_deref(), now),
            ),
        }
    }

    // ---------------------------------------------------------------- account

    /// Account state is reported only by `initialize`; an idle spare restarts to read new
    /// credentials.
    fn refresh_account(&self) -> Result<()> {
        let _lifecycle = self.lifecycle.lock().unwrap();
        {
            let mut spare = self.spare.lock().unwrap();
            if let Some(s) = spare.as_ref()
                && s.state.lock().unwrap().submitted.is_empty()
            {
                s.process.kill(false);
                *spare = None;
            }
        }
        let id = self.ids.new_id();
        let s = self.launch(Attach::New(&id), &self.config.cwd, &TurnSettings::default())?;
        *self.spare.lock().unwrap() = Some(s);
        Ok(())
    }

    fn refresh_rate_limits(&self) -> Result<()> {
        let usage = self
            .any_session()?
            .connection
            .control("get_usage", &OpaqueObject::new())?;
        let r: cw::UsageResponse = wire::project(&usage);
        let limits = r
            .rate_limits
            .0
            .unwrap_or_default()
            .into_iter()
            .filter(|(_, w)| wire::is_object(w))
            .map(|(id, w)| {
                let window: cw::UsageWindow = wire::project(&w);
                RateLimit {
                    id,
                    primary: Some(RateLimitWindow {
                        used_percent: window
                            .utilization
                            .get()
                            .filter(|v| {
                                !wire::is_object(v) && !wire::is_array(v) && !wire::is_null(v)
                            })
                            .and_then(|v| Json(Some(v.clone())).display_text())
                            .and_then(|t| t.parse().ok()),
                        window_minutes: None,
                        resets_at_epoch_sec: None,
                    }),
                    raw: Some(w),
                    ..Default::default()
                }
            })
            .collect();
        self.emit(AgentEvent::RateLimitsChanged {
            backend: B,
            limits: RateLimitState {
                limits,
                raw: Some(usage),
                ..Default::default()
            },
            merge: false,
        });
        Ok(())
    }

    fn refresh_models(&self) -> Result<ModelCatalog> {
        let response = self
            .any_session()?
            .connection
            .control("list_models", &OpaqueObject::new())?;
        let catalog = mapper::models(wire::project::<cw::Initialized>(&response).models.get());
        self.emit(AgentEvent::ModelsChanged {
            backend: B,
            catalog: catalog.clone(),
        });
        Ok(catalog)
    }

    fn terminal(&self, argv: Vec<String>) -> LoginFlow {
        LoginFlow::Terminal {
            argv,
            env: launch::env(&self.config.launch),
        }
    }

    fn login(&self, method: LoginMethod, secret: Option<Secret>) -> Result<LoginFlow> {
        let flow = match method {
            LoginMethod::ClaudeTerminalLogin => {
                self.terminal(launch::login_argv(&self.config.launch))
            }
            LoginMethod::ClaudeSetupToken => {
                self.terminal(launch::setup_token_argv(&self.config.launch))
            }
            LoginMethod::ClaudeApiKey => {
                let key = secret
                    .filter(|s| !s.0.trim().is_empty())
                    .ok_or_else(|| ChatError::invalid("API key required"))?;
                *self.api_key.lock().unwrap() = Some(key);
                self.refresh_account()?;
                LoginFlow::Completed {
                    login_id: None,
                    success: true,
                    error: None,
                }
            }
            other => {
                return Err(ChatError::invalid(format!(
                    "{} is not a Claude login method",
                    serde_json::to_string(&other)
                        .unwrap_or_default()
                        .trim_matches('"')
                )));
            }
        };
        self.emit(AgentEvent::LoginChanged {
            backend: B,
            flow: Some(flow.clone()),
        });
        Ok(flow)
    }

    // ---------------------------------------------------------------- threads

    fn upserted(
        &self,
        s: &Session,
        thread: &str,
        cwd: &str,
        forked_from: Option<&str>,
        settings: bool,
    ) -> Result<()> {
        let state = s.state.lock().unwrap();
        self.emit(AgentEvent::ThreadUpserted {
            backend: B,
            thread_id: thread.into(),
            title: None,
            preview: None,
            cwd: Some(cwd.into()),
            path: Some(launch::transcript_path(
                &self.config.launch.config_dir,
                cwd,
                thread,
            )?),
            forked_from: forked_from.map(str::to_owned),
            ephemeral: None,
            run_state: Some(RunState::Idle),
            settings: settings.then(|| ThreadSettings {
                model: state.model.clone(),
                effort: state.effort.clone(),
                approval_policy: Some(state.mode.clone()),
                ..Default::default()
            }),
            created_at_sec: None,
            updated_at_sec: None,
            raw: None,
        });
        Ok(())
    }

    fn start_thread(&self, options: &ThreadOptions) -> Result<String> {
        let reuse = {
            let _lifecycle = self.lifecycle.lock().unwrap();
            let mut spare = self.spare.lock().unwrap();
            if spare.as_ref().is_some_and(|s| {
                s.cwd == options.cwd && options.settings == TurnSettings::default()
            }) {
                spare.take()
            } else {
                None
            }
        };
        let s = match reuse {
            Some(s) => s,
            None => {
                let id = self.ids.new_id();
                self.launch(Attach::New(&id), &options.cwd, &options.settings)?
            }
        };
        self.upserted(&s, &s.id, &options.cwd, None, true)?;
        if let Some(title) = &options.title {
            self.rename(&s.id, title)?;
        }
        Ok(s.id.clone())
    }

    fn resume_thread(&self, thread: &str, options: &ThreadOptions) -> Result<String> {
        if self.sessions.lock().unwrap().contains_key(thread) {
            return Ok(thread.into());
        }
        let s = self.launch(Attach::Resume(thread), &options.cwd, &options.settings)?;
        self.upserted(&s, thread, &options.cwd, None, true)?;
        let _ = self.load_history(thread, None);
        Ok(thread.into())
    }

    fn transcript(&self, cwd: &str, thread: &str) -> Result<Option<Vec<String>>> {
        let Some(read) = &self.transcripts else {
            return Ok(None);
        };
        let path = launch::transcript_path(&self.config.launch.config_dir, cwd, thread)?;
        Ok(read(&path, MAX_TRANSCRIPT)?.map(|bytes| {
            String::from_utf8_lossy(&bytes)
                .lines()
                .map(str::to_owned)
                .collect()
        }))
    }

    fn fork_thread(
        &self,
        thread: &str,
        at_turn: Option<&str>,
        options: &ThreadOptions,
    ) -> Result<String> {
        let anchor = match at_turn {
            None => None,
            Some(turn) => {
                let live = self
                    .sessions
                    .lock()
                    .unwrap()
                    .get(thread)
                    .and_then(|s| s.mapper.lock().unwrap().last_message_uuid(turn));
                let anchor = match live {
                    Some(a) => Some(a),
                    None => self
                        .transcript(&options.cwd, thread)?
                        .and_then(|lines| transcript::last_uuid(&lines, turn)),
                };
                Some(anchor.ok_or_else(|| {
                    ChatError::invalid(format!("turn {turn} not found in {thread}"))
                })?)
            }
        };
        let id = self.ids.new_id();
        let s = self.launch(
            Attach::Fork {
                from: thread,
                id: &id,
                at_message: anchor.as_deref(),
            },
            &options.cwd,
            &options.settings,
        )?;
        self.upserted(&s, &id, &options.cwd, Some(thread), false)?;
        let _ = self.load_history(&id, None);
        Ok(id)
    }

    fn load_history(&self, thread: &str, older_than: Option<&str>) -> Result<()> {
        // Transcripts are loaded whole.
        if older_than.is_some() {
            return Ok(());
        }
        let cwd = self
            .sessions
            .lock()
            .unwrap()
            .get(thread)
            .map(|s| s.cwd.clone())
            .unwrap_or_else(|| self.config.cwd.clone());
        let Some(lines) = self.transcript(&cwd, thread)? else {
            return Ok(());
        };
        let history = transcript::history(thread, &lines);
        self.emit(AgentEvent::HistoryLoaded {
            backend: B,
            thread_id: thread.into(),
            turns: history.turns,
            prepend: false,
            cursor: None,
        });
        if let Some(title) = history.title {
            self.emit(AgentEvent::ThreadRenamed {
                backend: B,
                thread_id: thread.into(),
                title: Some(title),
            });
        }
        Ok(())
    }

    fn rename(&self, thread: &str, title: &str) -> Result<()> {
        let session = self.sessions.lock().unwrap().get(thread).cloned();
        if let Some(s) = session {
            s.connection.control(
                "rename_session",
                &fields(&Rename {
                    title,
                    source: "host",
                }),
            )?;
        }
        self.emit(AgentEvent::ThreadRenamed {
            backend: B,
            thread_id: thread.into(),
            title: Some(title.into()),
        });
        Ok(())
    }

    fn kill(&self, thread: &str) {
        if let Some(s) = self.sessions.lock().unwrap().get(thread) {
            s.process.kill(false);
        }
    }

    // ---------------------------------------------------------------- turns

    fn apply_settings(&self, s: &Session, settings: Option<&TurnSettings>) -> Result<()> {
        let Some(settings) = settings else {
            return Ok(());
        };
        let current = {
            let st = s.state.lock().unwrap();
            (st.model.clone(), st.effort.clone())
        };
        if let Some(model) = settings
            .model
            .as_ref()
            .filter(|m| Some(*m) != current.0.as_ref())
        {
            s.connection
                .control("set_model", &fields(&Model { model }))?;
            s.state.lock().unwrap().model = Some(model.clone());
            self.emit(AgentEvent::ThreadSettingsChanged {
                backend: B,
                thread_id: s.id.clone(),
                settings: ThreadSettings {
                    model: Some(model.clone()),
                    ..Default::default()
                },
            });
        }
        if let Some(effort) = settings
            .effort
            .as_ref()
            .filter(|e| Some(*e) != current.1.as_ref())
        {
            s.connection.control(
                "apply_flag_settings",
                &fields(&FlagSettings {
                    settings: Effort {
                        effort_level: effort,
                    },
                }),
            )?;
            s.state.lock().unwrap().effort = Some(effort.clone());
            self.emit(AgentEvent::ThreadSettingsChanged {
                backend: B,
                thread_id: s.id.clone(),
                settings: ThreadSettings {
                    effort: Some(effort.clone()),
                    ..Default::default()
                },
            });
        }
        if let Some(preset) = settings.permissions {
            self.set_mode(s, launch::permission_mode(preset))?;
        }
        Ok(())
    }

    fn set_mode(&self, s: &Session, mode: &str) -> Result<()> {
        if !ALLOWED_MODES.contains(&mode) {
            return Err(ChatError::state(format!(
                "permission mode {mode} is not allowed"
            )));
        }
        if s.state.lock().unwrap().mode == mode {
            return Ok(());
        }
        s.connection
            .control("set_permission_mode", &fields(&Mode { mode }))?;
        s.state.lock().unwrap().mode = mode.into();
        Ok(())
    }

    fn send(
        &self,
        thread: &str,
        parts: Vec<UserPart>,
        settings: Option<TurnSettings>,
        mode: SendMode,
    ) -> Result<String> {
        let existing = self.sessions.lock().unwrap().get(thread).cloned();
        let s = match existing {
            Some(s) => s,
            None => {
                let id = self.resume_thread(thread, &ThreadOptions::new(&self.config.cwd))?;
                self.session(&id)?
            }
        };
        self.apply_settings(&s, settings.as_ref())?;
        let client = self.ids.new_id();
        self.emit(AgentEvent::TurnSubmitted {
            backend: B,
            thread_id: thread.into(),
            client_message_id: client.clone(),
            parts: parts.clone(),
            settings: settings.clone(),
            at_ms: Some(self.clock.now_ms()),
        });
        let sent = requests::content(&parts, &*self.attachments, self.config.max_inline_bytes)
            .and_then(|content| {
                s.state.lock().unwrap().submitted.push(client.clone());
                s.connection.send(&requests::user_message(
                    &client,
                    &content,
                    (mode == SendMode::Steer).then_some("now"),
                ))
            });
        if let Err(e) = sent {
            s.state.lock().unwrap().submitted.retain(|c| *c != client);
            self.emit(AgentEvent::TurnCompleted {
                backend: B,
                thread_id: thread.into(),
                turn_id: client,
                status: TurnStatus::Failed,
                error: Some(TurnError {
                    message: if e.message.is_empty() {
                        "send failed".into()
                    } else {
                        e.message.clone()
                    },
                    code: Some("sendFailed".into()),
                    ..Default::default()
                }),
                items: vec![],
                duration_ms: None,
                usage: None,
                at_ms: None,
            });
            return Err(e);
        }
        Ok(client)
    }

    // ---------------------------------------------------------------- requests

    fn respond(&self, key: &RequestKey, response: &RequestResponse) -> Result<()> {
        if key.backend != B {
            return Err(ChatError::invalid("request belongs to another backend"));
        }
        let id: String =
            wire::decode(&key.raw_id).map_err(|_| ChatError::state("bad request key"))?;
        let (session, request) = self
            .open_requests
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| {
                ChatError::new(
                    ErrorKind::RequestExpired,
                    format!("request {id} is not open"),
                )
            })?;
        let s = self.session(&session)?;
        let answer = requests::answer(&request, response)?;
        let summary = match answer.reply {
            UserReply::Result { value, summary } => {
                s.connection.respond(&id, Some(&value))?;
                summary
            }
            UserReply::Error {
                message, summary, ..
            } => {
                s.connection.respond_error(&id, &message)?;
                summary
            }
        };
        self.open_requests.lock().unwrap().remove(&id);
        if answer.denied
            && matches!(request.kind, RequestKind::ToolApproval { .. })
            && let Some(item) = &request.item_id
        {
            s.mapper.lock().unwrap().mark_denied(item);
            self.emit(AgentEvent::ItemDeclined {
                backend: B,
                thread_id: session.clone(),
                turn_id: request.turn_id.clone(),
                item_id: item.clone(),
            });
        }
        self.emit(AgentEvent::RequestClosed {
            key: key.clone(),
            status: RequestStatus::Answered,
            answer: Some(summary),
        });
        Ok(())
    }

    fn raw_request(
        &self,
        method: &str,
        params: Option<OpaqueJson>,
        thread: Option<&str>,
    ) -> Result<OpaqueJson> {
        if method == "initialize" {
            return Err(ChatError::invalid("initialize is sent once per process"));
        }
        let body: OpaqueObject = params
            .filter(wire::is_object)
            .map(|p| wire::project(&p))
            .unwrap_or_default();
        if method == "set_permission_mode" {
            let mode = body
                .get("mode")
                .and_then(|m| Json(Some(m.clone())).string());
            if !mode.as_deref().is_some_and(|m| ALLOWED_MODES.contains(&m)) {
                return Err(ChatError::state("permission mode not allowed"));
            }
        }
        let session = match thread {
            None => self.any_session()?,
            Some(t) => self
                .sessions
                .lock()
                .unwrap()
                .get(t)
                .cloned()
                .ok_or_else(|| ChatError::state("session is not open"))?,
        };
        session.connection.control(method, &body)
    }
}

impl Backend for ClaudeBackend {
    fn kind(&self) -> BackendKind {
        B
    }
    fn start(&self) -> Result<()> {
        self.0.start()
    }
    fn stop(&self) {
        self.0.stop()
    }
    fn refresh_account(&self) -> Result<()> {
        self.0.refresh_account()
    }
    fn refresh_rate_limits(&self) -> Result<()> {
        self.0.refresh_rate_limits()
    }
    fn refresh_models(&self) -> Result<ModelCatalog> {
        self.0.refresh_models()
    }
    fn login(&self, method: LoginMethod, secret: Option<Secret>) -> Result<LoginFlow> {
        self.0.login(method, secret)
    }
    fn cancel_login(&self, _login_id: &str) -> Result<()> {
        self.0.emit(AgentEvent::LoginChanged {
            backend: B,
            flow: None,
        });
        Ok(())
    }
    fn logout(&self) -> Result<()> {
        *self.0.api_key.lock().unwrap() = None;
        let flow = self.0.terminal(launch::logout_argv(&self.0.config.launch));
        self.0.emit(AgentEvent::LoginChanged {
            backend: B,
            flow: Some(flow),
        });
        Ok(())
    }
    fn start_thread(&self, options: &ThreadOptions) -> Result<String> {
        self.0.start_thread(options)
    }
    fn resume_thread(&self, thread: &str, options: &ThreadOptions) -> Result<String> {
        self.0.resume_thread(thread, options)
    }
    fn fork_thread(
        &self,
        thread: &str,
        at_turn: Option<&str>,
        options: &ThreadOptions,
    ) -> Result<String> {
        self.0.fork_thread(thread, at_turn, options)
    }
    fn load_history(&self, thread: &str, older_than: Option<&str>) -> Result<()> {
        self.0.load_history(thread, older_than)
    }
    fn rename(&self, thread: &str, title: &str) -> Result<()> {
        self.0.rename(thread, title)
    }
    fn archive(&self, thread: &str, archived: bool) -> Result<()> {
        if archived {
            self.0.kill(thread);
        }
        self.0.emit(AgentEvent::ThreadArchived {
            backend: B,
            thread_id: thread.into(),
            archived,
        });
        Ok(())
    }
    fn delete(&self, thread: &str) -> Result<()> {
        self.0.kill(thread);
        self.0.emit(AgentEvent::ThreadDeleted {
            backend: B,
            thread_id: thread.into(),
        });
        Ok(())
    }
    fn compact(&self, thread: &str) -> Result<()> {
        self.0
            .send(
                thread,
                vec![UserPart::Text {
                    text: "/compact".into(),
                }],
                None,
                SendMode::Start,
            )
            .map(|_| ())
    }
    fn send(
        &self,
        thread: &str,
        parts: Vec<UserPart>,
        settings: Option<TurnSettings>,
        mode: SendMode,
    ) -> Result<String> {
        self.0.send(thread, parts, settings, mode)
    }
    fn cancel_queued(&self, thread: &str, client_message_id: &str) -> Result<()> {
        let s = self.0.session(thread)?;
        s.connection.control(
            "cancel_async_message",
            &fields(&CancelAsync {
                message_uuid: client_message_id,
            }),
        )?;
        let events =
            s.mapper
                .lock()
                .unwrap()
                .end_turn(client_message_id, TurnStatus::Cancelled, None);
        events.into_iter().for_each(|e| self.0.emit(e));
        Ok(())
    }
    fn interrupt(&self, thread: &str, cancel_queued: bool) -> Result<()> {
        self.0
            .session(thread)?
            .connection
            .control("interrupt", &fields(&Interrupt { cancel_queued }))
            .map(|_| ())
    }
    fn set_permissions(&self, thread: &str, preset: PermissionPreset) -> Result<()> {
        let s = self.0.session(thread)?;
        self.0.set_mode(&s, launch::permission_mode(preset))
    }
    fn respond(&self, key: &RequestKey, response: &RequestResponse) -> Result<()> {
        self.0.respond(key, response)
    }
    fn raw_request(
        &self,
        method: &str,
        params: Option<OpaqueJson>,
        thread: Option<&str>,
    ) -> Result<OpaqueJson> {
        self.0.raw_request(method, params, thread)
    }
}
