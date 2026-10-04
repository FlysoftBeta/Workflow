//! Codex App-Server adapter. One process serves every thread. State goes to the event sink; the
//! backend keeps only what it needs to address requests: open server requests, the running turn
//! per thread, queued submission IDs, the observed reviewer and the current login attempt.
use super::{
    events,
    launch::{self, CodexConfig},
    params, requests,
    requests::{Disposition, UserReply},
    wire as codex_wire,
};
use crate::{
    backend::{Backend, EventSink, Shared, ThreadOptions, Token, spawn},
    error::{ChatError, ErrorKind, Result},
    model::*,
    ports::{AgentProcess, AgentRuntime, Clock, IdSource},
    service::launch_env::Secret,
    transport::jsonrpc::{RpcConnection, RpcInbound},
    wire,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
    time::{Duration, Instant},
};

const B: BackendKind = BackendKind::Codex;
/// Local ID of a login whose start request is unanswered; never sent to Codex.
pub const ATTEMPT_PREFIX: &str = "attempt:";
/// The cancelled-login marker the client's `LoginView` recognises.
pub const CANCELLED: &str = "cancelled";
const ACCOUNT_READ_TIMEOUT: &str = "Codex 没有及时返回账户状态";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    Starting,
    Waiting,
    Confirming,
}
/// One login attempt (docs/engine/chat.md "Codex login state machine"). Only short critical
/// sections hold the login lock, and no request is sent while holding it.
struct Attempt {
    serial: u64,
    id: String,
    connection: Arc<RpcConnection>,
    vendor_id: Option<String>,
    phase: Phase,
    job: Token,
    outcome: Arc<Shared<LoginFlow>>,
}
#[derive(Default)]
struct LoginSlot {
    next: u64,
    attempt: Option<Attempt>,
    /// Completions that arrived before their start answer, by login ID (at most eight).
    early: Vec<(String, LoginFlow)>,
}
impl LoginSlot {
    fn current(&mut self, serial: u64) -> Option<&mut Attempt> {
        self.attempt.as_mut().filter(|a| a.serial == serial)
    }
    fn is(&self, serial: u64, phase: Phase) -> bool {
        self.attempt
            .as_ref()
            .is_some_and(|a| a.serial == serial && a.phase == phase)
    }
}
#[derive(Clone)]
enum AccountRead {
    Answered(Box<AccountState>),
    Failed(String),
}
struct Session {
    connection: Arc<RpcConnection>,
    process: Arc<dyn AgentProcess>,
}
/// Handles a login attempt's thread owns; the attempt itself lives in the login slot.
#[derive(Clone)]
struct Handle {
    serial: u64,
    id: String,
    connection: Arc<RpcConnection>,
    outcome: Arc<Shared<LoginFlow>>,
}

/// The connection a shared account read was issued on, and its outcome.
type PendingRead = (Arc<RpcConnection>, Arc<Shared<AccountRead>>);

pub struct CodexBackend(Arc<Inner>);
struct Inner {
    me: Weak<Inner>,
    runtime: Arc<dyn AgentRuntime>,
    config: CodexConfig,
    sink: Arc<dyn EventSink>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdSource>,
    lifecycle: Mutex<()>,
    session: Mutex<Option<Session>>,
    login: Mutex<LoginSlot>,
    read: Mutex<Option<PendingRead>>,
    open_requests: Mutex<HashMap<String, PendingRequest>>,
    active_turn: Mutex<HashMap<String, String>>,
    /// clientMessageId → (queuedSubmissionId, threadId).
    queued: Mutex<HashMap<String, (String, String)>>,
    queue_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    reviewer: Mutex<HashMap<String, String>>,
}

fn describe(e: &ChatError) -> String {
    let text = if e.kind == ErrorKind::Closed {
        "Codex 连接已断开".to_string()
    } else if e.message.trim().is_empty() {
        "error".to_string()
    } else {
        e.message.clone()
    };
    text.chars().take(300).collect()
}
fn completed(login_id: Option<String>, success: bool, error: Option<&str>) -> LoginFlow {
    LoginFlow::Completed {
        login_id,
        success,
        error: error.map(str::to_owned),
    }
}
fn notice(level: NoticeLevel, message: &str, code: &str, detail: Option<String>) -> Notice {
    Notice {
        level,
        message: message.into(),
        code: Some(code.into()),
        detail,
        ..Default::default()
    }
}

impl CodexBackend {
    pub fn new(
        runtime: Arc<dyn AgentRuntime>,
        config: CodexConfig,
        sink: Arc<dyn EventSink>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdSource>,
    ) -> Self {
        Self(Arc::new_cyclic(|me| Inner {
            me: me.clone(),
            runtime,
            config,
            sink,
            clock,
            ids,
            lifecycle: Mutex::new(()),
            session: Mutex::new(None),
            login: Mutex::new(LoginSlot::default()),
            read: Mutex::new(None),
            open_requests: Mutex::new(HashMap::new()),
            active_turn: Mutex::new(HashMap::new()),
            queued: Mutex::new(HashMap::new()),
            queue_locks: Mutex::new(HashMap::new()),
            reviewer: Mutex::new(HashMap::new()),
        }))
    }
    /// Test hook: currently open (unanswered) server request IDs.
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
    fn arc(&self) -> Arc<Inner> {
        self.me.upgrade().expect("backend alive")
    }
    fn connection(&self) -> Result<Arc<RpcConnection>> {
        self.session
            .lock()
            .unwrap()
            .as_ref()
            .map(|s| s.connection.clone())
            .ok_or_else(|| ChatError::unavailable("Codex is not running"))
    }
    fn is_current_connection(&self, connection: &Arc<RpcConnection>) -> bool {
        self.session
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|s| Arc::ptr_eq(&s.connection, connection))
    }
    fn background(&self, name: &str, job: impl FnOnce(Arc<Inner>) + Send + 'static) {
        let me = self.arc();
        spawn(name, move || job(me));
    }

    // ---------------------------------------------------------------- lifecycle

    fn start(&self) -> Result<()> {
        let _lifecycle = self.lifecycle.lock().unwrap();
        if self
            .session
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|s| s.process.is_alive() && s.connection.closed_reason().is_none())
        {
            return Ok(());
        }
        self.emit(AgentEvent::ProcessChanged {
            backend: B,
            state: ProcessState::Starting {},
        });
        let (process, stdio) = match self.runtime.spawn(&launch::spec(&self.config)) {
            Ok(v) => v,
            Err(e) => {
                self.emit(AgentEvent::ProcessChanged {
                    backend: B,
                    state: ProcessState::Failed {
                        message: e.message.clone(),
                        stderr_tail: String::new(),
                    },
                });
                return Err(e);
            }
        };
        let me = self.me.clone();
        let connection = RpcConnection::open(
            stdio,
            Box::new(move |connection, message| {
                if let Some(me) = me.upgrade() {
                    me.handle(connection, message);
                }
            }),
        );
        *self.session.lock().unwrap() = Some(Session {
            connection: connection.clone(),
            process: process.clone(),
        });
        {
            let connection = connection.clone();
            let process = process.clone();
            self.background("codex-exit", move |me| {
                let code = process.wait();
                connection.join();
                me.exited(&connection, code);
            });
        }
        let init = params::initialize(
            &self.config.client_name,
            &self.config.client_title,
            &self.config.client_version,
            &self.config.opt_out_notifications,
        );
        match connection
            .request("initialize", Some(init))
            .and_then(|info| {
                self.emit(AgentEvent::ServerInfo { backend: B, info });
                connection.notify("initialized", None)
            }) {
            Ok(()) => self.emit(AgentEvent::ProcessChanged {
                backend: B,
                state: ProcessState::Ready {},
            }),
            Err(e) => {
                self.emit(AgentEvent::ProcessChanged {
                    backend: B,
                    state: ProcessState::Failed {
                        message: e.message.clone(),
                        stderr_tail: connection.stderr_tail(4000),
                    },
                });
                process.kill(true);
                return Err(e);
            }
        }
        self.background("codex-refresh", |me| {
            let _ = me.refresh_account();
            let _ = me.refresh_models();
            let _ = me.refresh_rate_limits();
        });
        Ok(())
    }

    fn exited(&self, connection: &Arc<RpcConnection>, code: i32) {
        let was_current = {
            let mut session = self.session.lock().unwrap();
            match session.as_ref() {
                Some(s) if Arc::ptr_eq(&s.connection, connection) => {
                    *session = None;
                    true
                }
                Some(_) => false,
                None => true,
            }
        };
        self.finish_login(Some(connection), "Codex 进程已退出，请重新登录");
        if was_current {
            self.open_requests.lock().unwrap().clear();
            self.active_turn.lock().unwrap().clear();
            self.queued.lock().unwrap().clear();
        }
        self.emit(AgentEvent::ProcessChanged {
            backend: B,
            state: ProcessState::Exited {
                exit_code: Some(code),
                stderr_tail: connection.stderr_tail(4000),
            },
        });
    }

    fn stop(&self) {
        let _lifecycle = self.lifecycle.lock().unwrap();
        let Some(session) = self.session.lock().unwrap().take() else {
            return;
        };
        self.finish_login(None, "Codex 已停止，请重新登录");
        self.open_requests.lock().unwrap().clear();
        self.active_turn.lock().unwrap().clear();
        self.queued.lock().unwrap().clear();
        session.process.kill(false);
        session.connection.close_input();
    }

    // ---------------------------------------------------------------- inbound

    fn handle(&self, connection: &Arc<RpcConnection>, message: RpcInbound) {
        match message {
            RpcInbound::Notification {
                method,
                params,
                raw,
            } => {
                let evs = events::notification(&method, params.as_ref(), &raw);
                self.track(connection, &evs);
                for event in evs {
                    match event {
                        AgentEvent::LoginChanged {
                            flow: Some(flow @ LoginFlow::Completed { .. }),
                            ..
                        } => self.login_completed(connection, flow),
                        other => self.emit(other),
                    }
                }
                let p: codex_wire::Notification =
                    wire::project_opt(params.as_ref().filter(|v| wire::is_object(v)));
                if method == "account/updated" {
                    let signed_out = p.auth_mode.get().is_some_and(wire::is_null);
                    let connection = connection.clone();
                    self.background("codex-account", move |me| {
                        if signed_out && me.login.lock().unwrap().attempt.is_none() {
                            me.emit(AgentEvent::LoginChanged {
                                backend: B,
                                flow: None,
                            });
                        }
                        me.refresh(&connection);
                    });
                }
                if method == "thread/queue/changed"
                    && let Some(thread) = p.thread_id.owned()
                {
                    let connection = connection.clone();
                    self.background("codex-queue", move |me| {
                        if me.refresh_queue(&thread, &connection).is_err() {
                            me.emit(AgentEvent::ThreadNotice {
                                backend: B,
                                thread_id: thread,
                                notice: notice(
                                    NoticeLevel::Warning,
                                    "队列更新失败",
                                    "queueRefreshFailed",
                                    None,
                                ),
                            });
                        }
                    });
                }
            }
            RpcInbound::Request {
                id,
                method,
                params,
                raw,
            } => self.server_request(connection, id, &method, params, raw),
            RpcInbound::Stray { reason, raw } => self.emit(AgentEvent::Unknown {
                backend: B,
                kind: format!("stray: {reason}"),
                thread_id: None,
                raw,
            }),
            RpcInbound::Malformed { preview, reason } => self.emit(AgentEvent::BackendNotice {
                backend: B,
                notice: notice(
                    NoticeLevel::Warning,
                    &format!("Malformed frame: {reason}"),
                    "malformed",
                    Some(preview),
                ),
            }),
        }
    }

    fn track(&self, connection: &Arc<RpcConnection>, evs: &[AgentEvent]) {
        for event in evs {
            match event {
                AgentEvent::TurnStarted {
                    thread_id, turn_id, ..
                } => {
                    self.active_turn
                        .lock()
                        .unwrap()
                        .insert(thread_id.clone(), turn_id.clone());
                }
                AgentEvent::TurnCompleted {
                    thread_id, turn_id, ..
                } => {
                    let mut active = self.active_turn.lock().unwrap();
                    if active.get(thread_id) == Some(turn_id) {
                        active.remove(thread_id);
                    }
                    self.open_requests.lock().unwrap().retain(|_, r| {
                        !(r.thread_id.as_ref() == Some(thread_id)
                            && r.turn_id.as_ref() == Some(turn_id))
                    });
                }
                AgentEvent::RequestClosed { key, .. } => {
                    self.open_requests
                        .lock()
                        .unwrap()
                        .remove(&wire::text(&key.raw_id));
                    connection.forget(&key.raw_id);
                }
                AgentEvent::ThreadSettingsChanged {
                    thread_id,
                    settings,
                    ..
                } => {
                    if let Some(value) = &settings.approvals_reviewer {
                        self.check_reviewer(thread_id, value);
                    }
                }
                AgentEvent::ItemStarted { item, .. } | AgentEvent::ItemCompleted { item, .. } => {
                    // A queued submission started: its user message carries our clientMessageId.
                    if let Item::UserMessage(UserMessageItem {
                        client_message_id: Some(client),
                        ..
                    }) = item
                    {
                        self.queued.lock().unwrap().remove(client);
                    }
                }
                _ => (),
            }
        }
    }

    fn check_reviewer(&self, thread: &str, value: &str) {
        self.reviewer
            .lock()
            .unwrap()
            .insert(thread.into(), value.into());
        if value != params::REVIEWER_USER {
            self.emit(AgentEvent::ThreadNotice {
                backend: B,
                thread_id: thread.into(),
                notice: notice(
                    NoticeLevel::Error,
                    &format!(
                        "Approvals for this thread are not routed to you (approvalsReviewer={value}). Sending is blocked."
                    ),
                    "approvalsReviewerNotUser",
                    None,
                ),
            });
        }
    }

    fn server_request(
        &self,
        connection: &Arc<RpcConnection>,
        id: OpaqueJson,
        method: &str,
        params: Option<OpaqueJson>,
        raw: OpaqueJson,
    ) {
        match requests::pending(&id, method, params.as_ref(), &raw, self.clock.now_ms()) {
            Disposition::Ask(request) => {
                self.open_requests
                    .lock()
                    .unwrap()
                    .insert(wire::text(&id), request.clone());
                self.emit(AgentEvent::RequestOpened { request });
            }
            Disposition::Fact { record, result } => {
                self.emit(AgentEvent::RequestOpened { request: record });
                let _ = connection.respond(&id, &result);
            }
            Disposition::Unsupported {
                record,
                code,
                message,
            } => {
                let thread = record.thread_id.clone();
                self.emit(AgentEvent::RequestOpened { request: record });
                self.emit(AgentEvent::Unknown {
                    backend: B,
                    kind: format!("serverRequest:{method}"),
                    thread_id: thread,
                    raw,
                });
                let _ = connection.respond_error(&id, code, &message, None);
            }
        }
    }

    // ---------------------------------------------------------------- account

    /// Codex serialises account requests and an authenticated read can wait for workspace-routing
    /// discovery. At most one `account/read` is outstanding: callers share it, and a caller that
    /// stops waiting never resends.
    fn read_account(&self, connection: &Arc<RpcConnection>, wait: Duration) -> AccountRead {
        let shared = {
            let mut slot = self.read.lock().unwrap();
            match slot.as_ref() {
                Some((c, shared)) if Arc::ptr_eq(c, connection) && !shared.is_done() => {
                    shared.clone()
                }
                _ => {
                    let shared = Shared::new();
                    *slot = Some((connection.clone(), shared.clone()));
                    let (c, s) = (connection.clone(), shared.clone());
                    spawn("codex-account-read", move || {
                        s.set(
                            match c.request("account/read", Some(params::account_read())) {
                                Ok(result) => {
                                    AccountRead::Answered(Box::new(events::account(&result)))
                                }
                                Err(e) => AccountRead::Failed(describe(&e)),
                            },
                        );
                    });
                    shared
                }
            }
        };
        shared
            .wait(wait)
            .unwrap_or_else(|| AccountRead::Failed(ACCOUNT_READ_TIMEOUT.into()))
    }

    /// Applies an answered read. An authenticated account also ends a pending attempt.
    fn observe_account(&self, connection: &Arc<RpcConnection>, account: AccountState) {
        let mut slot = self.login.lock().unwrap();
        if account.state == LoginState::LoggedIn
            && slot
                .attempt
                .as_ref()
                .is_some_and(|a| Arc::ptr_eq(&a.connection, connection))
        {
            let attempt = slot.attempt.take().unwrap();
            let done = completed(
                Some(attempt.vendor_id.clone().unwrap_or(attempt.id.clone())),
                true,
                None,
            );
            if attempt.phase != Phase::Confirming {
                self.emit(AgentEvent::LoginChanged {
                    backend: B,
                    flow: Some(done.clone()),
                });
            }
            attempt.job.cancel();
            attempt.outcome.set(done);
        }
        self.emit(AgentEvent::AccountChanged {
            backend: B,
            account,
        });
    }

    fn refresh(&self, connection: &Arc<RpcConnection>) {
        match self.read_account(connection, self.config.timings.read_timeout) {
            AccountRead::Answered(account) => self.observe_account(connection, *account),
            AccountRead::Failed(message) => self.emit(AgentEvent::AccountCheckFailed {
                backend: B,
                message,
            }),
        }
    }

    /// A failed read is recorded as `AccountCheckFailed`; it never changes the account state.
    fn refresh_account(&self) -> Result<()> {
        let connection = self.connection()?;
        self.refresh(&connection);
        Ok(())
    }

    fn refresh_rate_limits(&self) -> Result<()> {
        let result = self
            .connection()?
            .request("account/rateLimits/read", None)?;
        self.emit(AgentEvent::RateLimitsChanged {
            backend: B,
            limits: events::rate_limits(&result),
            merge: false,
        });
        Ok(())
    }

    fn refresh_models(&self) -> Result<ModelCatalog> {
        let mut pages = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let page = self
                .connection()?
                .request("model/list", Some(params::model_list(cursor.as_deref())))?;
            cursor = events::next_cursor(&page);
            pages.push(page);
            if cursor.is_none() || pages.len() >= 20 {
                break;
            }
        }
        let catalog = events::models(&pages);
        self.emit(AgentEvent::ModelsChanged {
            backend: B,
            catalog: catalog.clone(),
        });
        Ok(catalog)
    }

    /// Starts a login and returns its first flow. The attempt belongs to the backend: it continues
    /// when the caller goes away, and `cancel_login` with the attempt ID ends a slow start. Vendor
    /// failures are returned and emitted as a failed `Completed` flow, not thrown.
    fn login(&self, method: LoginMethod, secret: Option<Secret>) -> Result<LoginFlow> {
        let connection = self.connection()?;
        let outcome = Shared::new();
        let (handle, superseded) = {
            let mut slot = self.login.lock().unwrap();
            slot.next += 1;
            let handle = Handle {
                serial: slot.next,
                id: format!("{ATTEMPT_PREFIX}{}", self.ids.new_id()),
                connection: connection.clone(),
                outcome: outcome.clone(),
            };
            let previous = slot.attempt.take();
            let superseded = previous.and_then(|p| {
                p.job.cancel();
                p.outcome.set(completed(
                    Some(p.vendor_id.clone().unwrap_or(p.id.clone())),
                    false,
                    Some(CANCELLED),
                ));
                p.vendor_id
                    .filter(|_| Arc::ptr_eq(&p.connection, &connection))
            });
            slot.early.clear();
            slot.attempt = Some(Attempt {
                serial: handle.serial,
                id: handle.id.clone(),
                connection: connection.clone(),
                vendor_id: None,
                phase: Phase::Starting,
                job: Token::new(),
                outcome: outcome.clone(),
            });
            self.emit(AgentEvent::LoginChanged {
                backend: B,
                flow: Some(LoginFlow::Progress {
                    login_id: Some(handle.id.clone()),
                    output: vec![],
                    error: None,
                }),
            });
            (handle, superseded)
        };
        if let Some(id) = superseded {
            let connection = connection.clone();
            self.background("codex-login-cancel", move |me| {
                me.cancel_on_server(&connection, &id)
            });
        }
        {
            let handle = handle.clone();
            self.background("codex-login", move |me| {
                me.run_attempt(handle, method, secret)
            });
        }
        loop {
            if let Some(flow) = outcome.wait(Duration::from_secs(3600)) {
                return Ok(flow);
            }
        }
    }

    fn run_attempt(&self, handle: Handle, method: LoginMethod, secret: Option<Secret>) {
        let started = params::login(method, secret.as_ref().map(|s| s.0.as_str()))
            .and_then(|body| handle.connection.request("account/login/start", Some(body)));
        drop(secret);
        let result = match started {
            Ok(r) => r,
            Err(e) => return self.fail_start(&handle, &describe(&e)),
        };
        let flow = events::login_flow(&result).unwrap_or(LoginFlow::Progress {
            login_id: events::login_id(&result),
            output: vec![],
            error: None,
        });
        let mut read_after = false;
        {
            let mut slot = self.login.lock().unwrap();
            if slot.current(handle.serial).is_none() {
                // Cancelled or superseded while starting: release the server-side login it created.
                if !matches!(flow, LoginFlow::Completed { .. })
                    && let Some(id) = flow.login_id().map(str::to_owned)
                {
                    let connection = handle.connection.clone();
                    self.background("codex-login-cancel", move |me| {
                        me.cancel_on_server(&connection, &id)
                    });
                }
                return;
            }
            if matches!(flow, LoginFlow::Completed { .. }) {
                slot.attempt = None;
                self.emit(AgentEvent::LoginChanged {
                    backend: B,
                    flow: Some(flow.clone()),
                });
                read_after = true;
            } else {
                let vendor = flow.login_id().map(str::to_owned);
                {
                    let attempt = slot.current(handle.serial).unwrap();
                    attempt.vendor_id = vendor.clone();
                    attempt.phase = Phase::Waiting;
                }
                self.emit(AgentEvent::LoginChanged {
                    backend: B,
                    flow: Some(flow.clone()),
                });
                let early = vendor.and_then(|id| {
                    slot.early
                        .iter()
                        .position(|(k, _)| *k == id)
                        .map(|i| slot.early.remove(i).1)
                });
                slot.early.clear();
                match early {
                    Some(done) => self.resolve_completion(&mut slot, done),
                    None => {
                        let token = Token::new();
                        slot.current(handle.serial).unwrap().job = token.clone();
                        let handle = handle.clone();
                        self.background("codex-login-wait", move |me| me.monitor(&handle, &token));
                    }
                }
            }
            handle.outcome.set(flow);
        }
        if read_after {
            self.refresh(&handle.connection);
        }
    }

    fn fail_start(&self, handle: &Handle, message: &str) {
        let mut slot = self.login.lock().unwrap();
        let failed = completed(Some(handle.id.clone()), false, Some(message));
        if slot.current(handle.serial).is_some() {
            slot.attempt = None;
            self.emit(AgentEvent::LoginChanged {
                backend: B,
                flow: Some(failed.clone()),
            });
        }
        handle.outcome.set(failed);
    }

    /// WAITING: bounded by the login deadline; afterwards the attempt fails and the server login
    /// is cancelled.
    fn monitor(&self, handle: &Handle, token: &Token) {
        let deadline = Instant::now() + self.config.timings.timeout;
        if !self.poll(handle, token, deadline) {
            return;
        }
        let expired = {
            let mut slot = self.login.lock().unwrap();
            if slot.is(handle.serial, Phase::Waiting) {
                let attempt = slot.attempt.take().unwrap();
                self.emit(AgentEvent::LoginChanged {
                    backend: B,
                    flow: Some(completed(
                        attempt.vendor_id.clone(),
                        false,
                        Some("登录等待超时，请重试"),
                    )),
                });
                attempt.vendor_id
            } else {
                None
            }
        };
        if let Some(id) = expired {
            self.cancel_on_server(&handle.connection, &id);
        }
    }

    /// One read at a time; failures back off and are reported without ending the attempt.
    /// Returns true when the deadline passed.
    fn poll(&self, handle: &Handle, token: &Token, deadline: Instant) -> bool {
        let timings = &self.config.timings;
        let mut wait = timings.poll;
        let mut reported = false;
        loop {
            let now = Instant::now();
            if now >= deadline {
                return true;
            }
            if !token.sleep(wait.min(deadline - now)) {
                return false;
            }
            let now = Instant::now();
            if now >= deadline {
                return true;
            }
            if !self.login.lock().unwrap().is(handle.serial, Phase::Waiting) {
                return false;
            }
            let read =
                self.read_account(&handle.connection, timings.read_timeout.min(deadline - now));
            if Instant::now() >= deadline {
                return true;
            }
            match read {
                AccountRead::Answered(account) => {
                    wait = timings.poll;
                    if account.state == LoginState::LoggedIn {
                        if self.login.lock().unwrap().is(handle.serial, Phase::Waiting) {
                            self.observe_account(&handle.connection, *account);
                        }
                        return false;
                    }
                    // Clear a reported check failure; the reducer keeps the pending flow.
                    if reported && self.login.lock().unwrap().is(handle.serial, Phase::Waiting) {
                        reported = false;
                        self.emit(AgentEvent::AccountChanged {
                            backend: B,
                            account: *account,
                        });
                    }
                }
                AccountRead::Failed(message) => {
                    wait = (wait * 2).min(timings.max_poll);
                    if self.login.lock().unwrap().is(handle.serial, Phase::Waiting) {
                        reported = true;
                        self.emit(AgentEvent::AccountCheckFailed {
                            backend: B,
                            message,
                        });
                    }
                }
            }
        }
    }

    fn login_completed(&self, connection: &Arc<RpcConnection>, done: LoginFlow) {
        let LoginFlow::Completed {
            login_id, success, ..
        } = &done
        else {
            return;
        };
        let check = {
            let mut slot = self.login.lock().unwrap();
            let current = slot
                .attempt
                .as_ref()
                .filter(|a| Arc::ptr_eq(&a.connection, connection))
                .map(|a| (a.vendor_id.clone(), a.phase));
            match current {
                Some((Some(vendor), Phase::Waiting)) if login_id.as_ref() == Some(&vendor) => {
                    self.resolve_completion(&mut slot, done.clone());
                    false
                }
                // The start answer has not been processed yet; keep it for that attempt.
                Some((_, Phase::Starting)) if login_id.is_some() => {
                    slot.early.push((login_id.clone().unwrap(), done.clone()));
                    while slot.early.len() > 8 {
                        slot.early.remove(0);
                    }
                    false
                }
                // Stale or unattributed: it never ends the current attempt, but credentials may
                // have changed.
                current => *success || (current.is_some() && login_id.is_none()),
            }
        };
        if check {
            let connection = connection.clone();
            self.background("codex-account", move |me| me.refresh(&connection));
        }
    }

    /// Caller holds the login lock.
    fn resolve_completion(&self, slot: &mut LoginSlot, done: LoginFlow) {
        let Some(attempt) = slot.attempt.as_mut() else {
            return;
        };
        attempt.job.cancel();
        self.emit(AgentEvent::LoginChanged {
            backend: B,
            flow: Some(done.clone()),
        });
        let handle = Handle {
            serial: attempt.serial,
            id: attempt.id.clone(),
            connection: attempt.connection.clone(),
            outcome: attempt.outcome.clone(),
        };
        if matches!(done, LoginFlow::Completed { success: true, .. }) {
            attempt.phase = Phase::Confirming;
            let token = Token::new();
            attempt.job = token.clone();
            self.background("codex-login-confirm", move |me| me.confirm(&handle, &token));
        } else {
            slot.attempt = None;
            // Codex can persist credentials and still report failure (workspace-routing
            // discovery): one read shows the real account, or why it cannot be read.
            self.background("codex-account", move |me| me.refresh(&handle.connection));
        }
    }

    /// CONFIRMING: the completion already counts. Reads only fill in the account; null or failed
    /// reads never undo it.
    fn confirm(&self, handle: &Handle, token: &Token) {
        let mut last_error = None;
        for wait in self.config.timings.confirm.clone() {
            if !token.sleep(wait) {
                return;
            }
            if !self
                .login
                .lock()
                .unwrap()
                .is(handle.serial, Phase::Confirming)
            {
                return;
            }
            match self.read_account(&handle.connection, self.config.timings.read_timeout) {
                AccountRead::Answered(account) if account.state == LoginState::LoggedIn => {
                    self.observe_account(&handle.connection, *account);
                    return;
                }
                AccountRead::Answered(_) => (),
                AccountRead::Failed(message) => last_error = Some(message),
            }
        }
        let unconfirmed = {
            let mut slot = self.login.lock().unwrap();
            if slot.current(handle.serial).is_some() {
                slot.attempt = None;
                true
            } else {
                false
            }
        };
        if !unconfirmed {
            return;
        }
        self.emit(AgentEvent::BackendNotice {
            backend: B,
            notice: notice(
                NoticeLevel::Warning,
                "Codex 已报告登录成功，但暂时无法读取账户信息",
                "loginAccountUnconfirmed",
                last_error.clone(),
            ),
        });
        if let Some(message) = last_error {
            self.emit(AgentEvent::AccountCheckFailed {
                backend: B,
                message,
            });
        }
    }

    fn cancel_on_server(&self, connection: &Arc<RpcConnection>, login_id: &str) {
        let _ = connection.request_timeout(
            "account/login/cancel",
            Some(params::login_cancel(login_id)),
            self.config.timings.cancel_timeout,
        );
    }

    /// Ends the attempt bound to `connection` (any attempt when `None`). A waiting flow is never
    /// kept after its process disappears.
    fn finish_login(&self, connection: Option<&Arc<RpcConnection>>, error: &str) {
        let mut slot = self.login.lock().unwrap();
        self.finish_locked(&mut slot, connection, error);
    }
    fn finish_locked(
        &self,
        slot: &mut LoginSlot,
        connection: Option<&Arc<RpcConnection>>,
        error: &str,
    ) {
        if !slot
            .attempt
            .as_ref()
            .is_some_and(|a| connection.is_none_or(|c| Arc::ptr_eq(&a.connection, c)))
        {
            return;
        }
        let attempt = slot.attempt.take().unwrap();
        slot.early.clear();
        attempt.job.cancel();
        let ended = completed(
            Some(attempt.vendor_id.clone().unwrap_or(attempt.id.clone())),
            false,
            Some(error),
        );
        if attempt.phase != Phase::Confirming {
            self.emit(AgentEvent::LoginChanged {
                backend: B,
                flow: Some(ended.clone()),
            });
        }
        attempt.outcome.set(ended);
    }

    fn cancel_login(&self, login_id: &str) -> Result<()> {
        // Local waiting state ends at once, even if the server never answers the cancellation.
        let target = {
            let mut slot = self.login.lock().unwrap();
            let matches = slot.attempt.as_ref().is_some_and(|a| {
                a.phase != Phase::Confirming
                    && (login_id == a.id || Some(login_id) == a.vendor_id.as_deref())
            });
            if matches {
                let a = slot.attempt.as_ref().unwrap();
                let target = a.vendor_id.clone().map(|id| (a.connection.clone(), id));
                self.finish_locked(&mut slot, None, CANCELLED);
                target
            } else if login_id.starts_with(ATTEMPT_PREFIX) {
                None
            } else {
                self.connection().ok().map(|c| (c, login_id.to_owned()))
            }
        };
        if let Some((connection, id)) = target {
            self.cancel_on_server(&connection, &id);
        }
        Ok(())
    }

    fn logout(&self) -> Result<()> {
        let connection = self.connection()?;
        self.finish_login(None, CANCELLED);
        connection.request("account/logout", None)?;
        self.emit(AgentEvent::LoginChanged {
            backend: B,
            flow: None,
        });
        self.refresh(&connection);
        Ok(())
    }

    // ---------------------------------------------------------------- threads

    fn thread_result(&self, result: &OpaqueJson) -> Result<String> {
        let r: codex_wire::ThreadResult = wire::project(result);
        let thread = r
            .thread
            .object()
            .ok_or_else(|| ChatError::new(ErrorKind::Vendor, "response has no thread"))?;
        let event = events::thread_upserted(thread, Some(result));
        let AgentEvent::ThreadUpserted { thread_id, .. } = &event else {
            unreachable!()
        };
        let id = thread_id.clone();
        self.emit(event);
        if let Some(reviewer) = r.approvals_reviewer.get() {
            self.check_reviewer(&id, reviewer);
        }
        Ok(id)
    }

    fn start_thread(&self, options: &ThreadOptions) -> Result<String> {
        let result = self.connection()?.request(
            "thread/start",
            Some(params::thread_start(
                &options.cwd,
                &options.settings,
                self.config.default_permissions,
                options.ephemeral,
            )),
        )?;
        let id = self.thread_result(&result)?;
        if let Some(title) = &options.title {
            self.rename(&id, title)?;
        }
        Ok(id)
    }

    fn resume_thread(&self, thread: &str, options: &ThreadOptions) -> Result<String> {
        let result = self.connection()?.request(
            "thread/resume",
            Some(params::thread_resume(
                thread,
                Some(&options.cwd),
                &options.settings,
                self.config.default_permissions,
            )),
        )?;
        let id = self.thread_result(&result)?;
        self.load_history(&id, None)?;
        Ok(id)
    }

    fn fork_thread(
        &self,
        thread: &str,
        at_turn: Option<&str>,
        options: &ThreadOptions,
    ) -> Result<String> {
        let result = self.connection()?.request(
            "thread/fork",
            Some(params::thread_fork(
                thread,
                at_turn,
                Some(&options.cwd),
                &options.settings,
                self.config.default_permissions,
                options.ephemeral,
            )),
        )?;
        let id = self.thread_result(&result)?;
        if !options.ephemeral {
            let _ = self.load_history(&id, None);
        }
        Ok(id)
    }

    fn load_history(&self, thread: &str, older_than: Option<&str>) -> Result<()> {
        let result = self.connection()?.request(
            "thread/turns/list",
            Some(params::turns_list(
                thread,
                self.config.history_page_size,
                older_than,
            )),
        )?;
        // The server pages newest first; the model is oldest first.
        let mut turns: Vec<Turn> = events::page_items(&result)
            .iter()
            .map(super::items::turn)
            .collect();
        turns.reverse();
        self.emit(AgentEvent::HistoryLoaded {
            backend: B,
            thread_id: thread.into(),
            turns,
            prepend: older_than.is_some(),
            cursor: events::next_cursor(&result),
        });
        Ok(())
    }

    fn rename(&self, thread: &str, title: &str) -> Result<()> {
        self.connection()?
            .request("thread/name/set", Some(params::name_set(thread, title)))
            .map(|_| ())
    }
    fn simple(&self, method: &str, thread: &str) -> Result<()> {
        self.connection()?
            .request(method, Some(params::thread_ref(thread)))
            .map(|_| ())
    }

    // ---------------------------------------------------------------- turns

    fn queue_lock(&self, thread: &str) -> Arc<Mutex<()>> {
        self.queue_locks
            .lock()
            .unwrap()
            .entry(thread.into())
            .or_default()
            .clone()
    }

    fn send(
        &self,
        thread: &str,
        parts: Vec<UserPart>,
        settings: Option<TurnSettings>,
        mode: SendMode,
    ) -> Result<String> {
        if let Some(value) = self.reviewer.lock().unwrap().get(thread)
            && value != params::REVIEWER_USER
        {
            return Err(ChatError::state(format!(
                "approvalsReviewer is {value}; refusing to send"
            )));
        }
        let client = self.ids.new_id();
        let running = self.active_turn.lock().unwrap().get(thread).cloned();
        let effective = match mode {
            SendMode::Auto if running.is_some() => SendMode::Queue,
            SendMode::Auto => SendMode::Start,
            SendMode::Steer if running.is_none() => SendMode::Start,
            other => other,
        };
        self.emit(AgentEvent::TurnSubmitted {
            backend: B,
            thread_id: thread.into(),
            client_message_id: client.clone(),
            parts: parts.clone(),
            settings: settings.clone(),
            at_ms: Some(self.clock.now_ms()),
        });
        let sent = (|| -> Result<()> {
            match effective {
                SendMode::Steer => {
                    let running = running.clone().unwrap_or_default();
                    let result = self.connection()?.request(
                        "turn/steer",
                        Some(params::turn_steer(thread, &running, &parts, &client)?),
                    )?;
                    let r: codex_wire::TurnSteerResult = wire::project(&result);
                    self.emit(AgentEvent::TurnBound {
                        backend: B,
                        thread_id: thread.into(),
                        client_message_id: client.clone(),
                        turn_id: r.turn_id.owned().unwrap_or(running),
                    });
                }
                SendMode::Queue => {
                    let lock = self.queue_lock(thread);
                    let _guard = lock.lock().unwrap();
                    let result = self.connection()?.request(
                        "thread/queue/add",
                        Some(params::queue_add(thread, &parts, &client)?),
                    )?;
                    let r: codex_wire::QueueAddResult = wire::project(&result);
                    if let Some(id) = r.queued_submission.get().and_then(|q| q.id.owned()) {
                        self.queued
                            .lock()
                            .unwrap()
                            .insert(client.clone(), (id, thread.into()));
                    }
                }
                _ => {
                    let result = self.connection()?.request(
                        "turn/start",
                        Some(params::turn_start(
                            thread,
                            &parts,
                            settings.as_ref(),
                            &client,
                        )?),
                    )?;
                    let r: codex_wire::TurnStartResult = wire::project(&result);
                    if let Some(turn) = r.turn.get().and_then(|t| t.id.owned()) {
                        self.emit(AgentEvent::TurnBound {
                            backend: B,
                            thread_id: thread.into(),
                            client_message_id: client.clone(),
                            turn_id: turn,
                        });
                    }
                }
            }
            Ok(())
        })();
        if let Err(e) = sent {
            self.emit(AgentEvent::TurnCompleted {
                backend: B,
                thread_id: thread.into(),
                turn_id: client.clone(),
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

    fn cancel_queued(&self, thread: &str, client: &str) -> Result<()> {
        let lock = self.queue_lock(thread);
        let _guard = lock.lock().unwrap();
        let (submission, owner) = self
            .queued
            .lock()
            .unwrap()
            .get(client)
            .cloned()
            .ok_or_else(|| ChatError::state(format!("{client} is not queued")))?;
        if owner != thread {
            return Err(ChatError::state(
                "queued message belongs to a different thread",
            ));
        }
        self.connection()?.request(
            "thread/queue/delete",
            Some(params::queue_delete(thread, &submission)),
        )?;
        self.queued.lock().unwrap().remove(client);
        self.emit(AgentEvent::TurnCancelled {
            backend: B,
            thread_id: thread.into(),
            turn_id: client.into(),
        });
        Ok(())
    }

    fn refresh_queue(&self, thread: &str, connection: &Arc<RpcConnection>) -> Result<()> {
        let lock = self.queue_lock(thread);
        let _guard = lock.lock().unwrap();
        let mut previous: Vec<String> = self
            .queued
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, (_, t))| t == thread)
            .map(|(c, _)| c.clone())
            .collect();
        previous.sort();
        let mut messages = Vec::new();
        let mut submissions = Vec::new();
        let mut cursor: Option<String> = None;
        let mut seen = std::collections::HashSet::new();
        loop {
            let page = connection.request(
                "thread/queue/list",
                Some(params::queue_list(thread, cursor.as_deref())),
            )?;
            for item in events::page_items(&page) {
                let q: codex_wire::QueuedSubmission = wire::project(&item);
                let (Some(client), Some(submission)) =
                    (q.client_user_message_id.owned(), q.id.owned())
                else {
                    continue;
                };
                submissions.push((client.clone(), submission));
                messages.push(QueuedMessage {
                    client_message_id: client,
                    parts: q
                        .input
                        .items()
                        .iter()
                        .map(|p| super::items::user_part(&p.0.clone().unwrap_or_default()))
                        .collect(),
                });
            }
            cursor = events::next_cursor(&page);
            match &cursor {
                None => break,
                Some(c) if seen.insert(c.clone()) && seen.len() < 50 => (),
                Some(_) => return Err(ChatError::state("queue pagination did not finish")),
            }
        }
        if !self.is_current_connection(connection) {
            return Ok(());
        }
        {
            let mut queued = self.queued.lock().unwrap();
            for client in &previous {
                queued.remove(client);
            }
            for (client, submission) in submissions {
                queued.insert(client, (submission, thread.into()));
            }
        }
        self.emit(AgentEvent::QueueUpdated {
            backend: B,
            thread_id: thread.into(),
            messages,
            previously_queued: previous,
        });
        Ok(())
    }

    fn interrupt(&self, thread: &str, cancel_queued: bool) -> Result<()> {
        if cancel_queued {
            let clients: Vec<String> = self
                .queued
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, (_, t))| t == thread)
                .map(|(c, _)| c.clone())
                .collect();
            for client in clients {
                let _ = self.cancel_queued(thread, &client);
            }
        }
        let Some(turn) = self.active_turn.lock().unwrap().get(thread).cloned() else {
            return Ok(());
        };
        self.connection()?
            .request("turn/interrupt", Some(params::interrupt(thread, &turn)))
            .map(|_| ())
    }

    // ---------------------------------------------------------------- requests

    fn respond(&self, key: &RequestKey, response: &RequestResponse) -> Result<()> {
        if key.backend != B {
            return Err(ChatError::invalid("request belongs to another backend"));
        }
        let id = wire::text(&key.raw_id);
        let request = self
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
        let connection = self.connection()?;
        let summary = match requests::answer(&request, response)? {
            UserReply::Result { value, summary } => {
                connection.respond(&key.raw_id, &value)?;
                summary
            }
            UserReply::Error {
                code,
                message,
                summary,
            } => {
                connection.respond_error(&key.raw_id, code, &message, None)?;
                summary
            }
        };
        self.open_requests.lock().unwrap().remove(&id);
        self.emit(AgentEvent::RequestClosed {
            key: key.clone(),
            status: RequestStatus::Answered,
            answer: Some(summary),
        });
        Ok(())
    }
}

impl Backend for CodexBackend {
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
    fn cancel_login(&self, login_id: &str) -> Result<()> {
        self.0.cancel_login(login_id)
    }
    fn logout(&self) -> Result<()> {
        self.0.logout()
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
        self.0.simple(
            if archived {
                "thread/archive"
            } else {
                "thread/unarchive"
            },
            thread,
        )
    }
    fn delete(&self, thread: &str) -> Result<()> {
        self.0.simple("thread/delete", thread)
    }
    fn compact(&self, thread: &str) -> Result<()> {
        self.0.simple("thread/compact/start", thread)
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
        self.0.cancel_queued(thread, client_message_id)
    }
    fn interrupt(&self, thread: &str, cancel_queued: bool) -> Result<()> {
        self.0.interrupt(thread, cancel_queued)
    }
    fn set_permissions(&self, thread: &str, preset: PermissionPreset) -> Result<()> {
        self.0
            .connection()?
            .request(
                "thread/settings/update",
                Some(params::permissions(thread, preset)),
            )
            .map(|_| ())
    }
    fn respond(&self, key: &RequestKey, response: &RequestResponse) -> Result<()> {
        self.0.respond(key, response)
    }
    fn raw_request(
        &self,
        method: &str,
        params: Option<OpaqueJson>,
        _thread: Option<&str>,
    ) -> Result<OpaqueJson> {
        self.0
            .connection()?
            .request(method, params::enforce_reviewer(method, params))
    }
}
