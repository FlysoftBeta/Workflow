//! Port of the Kotlin `CodexLoginTest`: login races replayed against the frame order the pinned
//! Codex 0.157.1 app-server produced with a local mock issuer (docs/engine/chat.md "Codex login
//! state machine"). `account/login/completed` comes first, then `account/updated`; a failed
//! workspace-routing discovery reports `success:false` and makes `account/read` answer with a
//! JSON-RPC error; account requests are served one at a time.
use serde_json::{Value, json};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use workflow_chat::{
    backend::Backend,
    codex::{
        backend::{CANCELLED, CodexBackend},
        launch::{CodexConfig, LoginTimings},
    },
    model::*,
    testing::*,
};

/// What the next `account/read` answers.
#[derive(Clone)]
enum Read {
    Anonymous,
    Authenticated,
    Error(&'static str),
    /// Not answered until released.
    Hold,
}
type Reads = Box<dyn Fn(usize) -> Read + Send + Sync>;

#[derive(Default)]
struct Script {
    reads: Mutex<Option<Reads>>,
    read_count: AtomicUsize,
    outstanding: AtomicUsize,
    max_outstanding: AtomicUsize,
    held: Mutex<Vec<Value>>,
    hold_start: Mutex<bool>,
    held_start: Mutex<Option<(Value, Value)>>,
    start_error: Mutex<Option<&'static str>>,
    login_ids: AtomicUsize,
}
impl Script {
    fn reads(&self, f: impl Fn(usize) -> Read + Send + Sync + 'static) {
        *self.reads.lock().unwrap() = Some(Box::new(f));
    }
    fn next_read(&self) -> Read {
        let n = self.read_count.fetch_add(1, Ordering::SeqCst) + 1;
        match self.reads.lock().unwrap().as_ref() {
            Some(f) => f(n),
            None => Read::Anonymous,
        }
    }
    fn answer_read(&self, p: &FakeProcess, id: Value, read: Read) {
        match read {
            Read::Anonymous => p.emit(&json!({"id":id,"result":{"account":null,"requiresOpenaiAuth":true,"workspaceRouting":null}})),
            Read::Authenticated => p.emit(&json!({"id":id,"result":{"account":{"type":"chatgpt","email":"tester@example.test","planType":"plus"},"requiresOpenaiAuth":true,"workspaceRouting":null}})),
            Read::Error(message) => p.emit(&json!({"id":id,"error":{"code":-32603,"message":message}})),
            Read::Hold => {
                self.held.lock().unwrap().push(id);
                return;
            }
        }
        self.outstanding.fetch_sub(1, Ordering::SeqCst);
    }
    fn react(&self, p: &FakeProcess, request: &Value) {
        let Some(id) = request.get("id").cloned() else {
            return;
        };
        match request["method"].as_str().unwrap_or("") {
            "account/read" => {
                let now = self.outstanding.fetch_add(1, Ordering::SeqCst) + 1;
                self.max_outstanding.fetch_max(now, Ordering::SeqCst);
                let read = self.next_read();
                self.answer_read(p, id, read);
            }
            "account/login/start" => {
                let login = format!("login-{}", self.login_ids.fetch_add(1, Ordering::SeqCst) + 1);
                let result = if request["params"]["type"] == "chatgpt" {
                    json!({"type":"chatgpt","loginId":login,"authUrl":"https://auth.example.test/oauth/authorize"})
                } else {
                    json!({"type":"chatgptDeviceCode","loginId":login,"verificationUrl":"https://auth.example.test/codex/device","userCode":"TEST-0000"})
                };
                if let Some(message) = *self.start_error.lock().unwrap() {
                    p.emit(&json!({"id":id,"error":{"code":-32603,"message":message}}));
                } else if *self.hold_start.lock().unwrap() {
                    *self.held_start.lock().unwrap() = Some((id, result));
                } else {
                    p.emit(&json!({"id":id,"result":result}));
                }
            }
            "account/login/cancel" => p.emit(&json!({"id":id,"result":{"status":"canceled"}})),
            "model/list" => p.emit(&json!({"id":id,"result":{"data":[],"nextCursor":null}})),
            _ => p.emit(&json!({"id":id,"result":{}})),
        }
    }
}

struct Harness {
    script: Arc<Script>,
    runtime: Arc<FakeRuntime>,
    store: Arc<EventStore>,
    backend: CodexBackend,
    history: Arc<Mutex<Vec<AccountState>>>,
}
impl Drop for Harness {
    fn drop(&mut self) {
        if self.runtime.count() > 0 {
            self.runtime.last().exit(0);
        }
    }
}
fn timings(timeout: u64, poll: u64, read_timeout: u64, confirm: &[u64]) -> LoginTimings {
    LoginTimings {
        poll: Duration::from_millis(poll),
        max_poll: Duration::from_millis(poll * 4),
        timeout: Duration::from_millis(timeout),
        read_timeout: Duration::from_millis(read_timeout),
        confirm: confirm.iter().map(|ms| Duration::from_millis(*ms)).collect(),
        cancel_timeout: Duration::from_secs(10),
    }
}
fn harness(timings: LoginTimings, script: Arc<Script>) -> Harness {
    let react = script.clone();
    let runtime = FakeRuntime::reacting(move |p, f| react.react(p, f));
    let store = EventStore::new();
    let history = Arc::new(Mutex::new(Vec::new()));
    let recorded = history.clone();
    store.listen(move |state, event| {
        if matches!(
            event,
            AgentEvent::AccountChanged { .. }
                | AgentEvent::LoginChanged { .. }
                | AgentEvent::AccountCheckFailed { .. }
        ) {
            recorded
                .lock()
                .unwrap()
                .push(backend_status(state, BackendKind::Codex).account);
        }
    });
    let mut config = CodexConfig::new("codex", "/test/home");
    config.timings = timings;
    let backend = CodexBackend::new(
        runtime.clone(),
        config,
        store.clone(),
        Arc::new(workflow_chat::ports::SystemClock),
        SequenceIds::new("attempt"),
    );
    Harness {
        script,
        runtime,
        store,
        backend,
        history,
    }
}
fn start(timeout: u64, poll: u64, read_timeout: u64, confirm: &[u64]) -> Harness {
    let h = harness(timings(timeout, poll, read_timeout, confirm), Arc::new(Script::default()));
    h.backend.start().unwrap();
    h.store.wait("initial account", |s| account_of(s).state == LoginState::LoggedOut);
    h
}
fn start_default() -> Harness {
    start(5_000, 20, 2_000, &[0, 20, 40])
}
fn account_of(state: &AgentState) -> AccountState {
    backend_status(state, BackendKind::Codex).account
}
impl Harness {
    fn account(&self) -> AccountState {
        account_of(&self.store.state())
    }
    fn process(&self) -> Arc<FakeProcess> {
        self.runtime.last()
    }
    fn notify(&self, method: &str, params: Value) {
        self.process().emit(&json!({"method": method, "params": params}));
    }
    fn completed(&self, login: Option<&str>, success: bool, error: Option<&str>) {
        self.notify(
            "account/login/completed",
            json!({"loginId":login,"success":success,"error":error,"onboardingEntrypoint":null}),
        );
    }
    fn updated(&self) {
        self.notify("account/updated", json!({"authMode":"chatgpt","planType":"plus"}));
    }
    fn wait_written(&self, method: &str, count: usize) {
        let p = self.process();
        eventually(method, || p.count(method) >= count);
    }
    fn release(&self, read: Read) {
        let held = std::mem::take(&mut *self.script.held.lock().unwrap());
        for id in held {
            self.script.answer_read(&self.process(), id, read.clone());
        }
    }
    fn login(&self, method: LoginMethod) -> LoginFlow {
        self.backend.login(method, None).unwrap()
    }
    fn wait_account(&self, what: &str, f: impl Fn(&AccountState) -> bool) -> AccountState {
        account_of(&self.store.wait(what, |s| f(&account_of(s))))
    }
    fn has_notice(&self, code: &str) -> bool {
        backend_status(&self.store.state(), BackendKind::Codex)
            .notices
            .iter()
            .any(|n| n.code.as_deref() == Some(code))
    }
}
fn completed_error(account: &AccountState) -> Option<String> {
    match &account.login {
        Some(LoginFlow::Completed { error, .. }) => error.clone(),
        _ => None,
    }
}
fn sleep(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

#[test]
fn real_device_code_sequence_ends_logged_in() {
    let h = start(5_000, 60_000, 2_000, &[0, 20, 40]);
    let flow = h.login(LoginMethod::CodexDeviceCode);
    assert_eq!(flow.login_id(), Some("login-1"));
    assert!(matches!(h.account().login, Some(LoginFlow::DeviceCode { .. })));
    assert_eq!(h.account().state, LoginState::LoggingIn);
    h.script.reads(|_| Read::Authenticated);
    h.completed(Some("login-1"), true, None);
    h.updated();
    let done = h.wait_account("confirmed account", |a| {
        a.state == LoginState::LoggedIn && a.login.is_none()
    });
    assert_eq!(done.email.as_deref(), Some("tester@example.test"));
    assert!(
        h.history
            .lock()
            .unwrap()
            .iter()
            .any(|a| matches!(a.login, Some(LoginFlow::Progress { .. })) && a.state == LoginState::LoggingIn),
        "the start shows a pending attempt first"
    );
}

#[test]
fn null_reads_after_completion_never_reset_to_logged_out() {
    let h = start(5_000, 60_000, 2_000, &[0, 20, 40]);
    h.login(LoginMethod::CodexDeviceCode);
    let base = h.script.read_count.load(Ordering::SeqCst);
    // Codex has not reloaded its credentials yet for the first two reads after completion.
    h.script
        .reads(move |n| if n <= base + 2 { Read::Anonymous } else { Read::Authenticated });
    h.completed(Some("login-1"), true, None);
    h.updated();
    h.wait_account("confirmed account", |a| a.email.is_some());
    let history = h.history.lock().unwrap().clone();
    let after: Vec<_> = history
        .iter()
        .skip_while(|a| !matches!(a.login, Some(LoginFlow::Completed { success: true, .. })))
        .collect();
    assert!(!after.is_empty());
    assert!(
        after.iter().all(|a| a.state == LoginState::LoggedIn),
        "a null read overrode the completion: {after:?}"
    );
}

#[test]
fn completion_that_never_reads_back_stays_logged_in() {
    let h = start(5_000, 60_000, 2_000, &[0, 20, 40]);
    h.login(LoginMethod::CodexDeviceCode);
    h.completed(Some("login-1"), true, None);
    h.store.wait("unconfirmed notice", |s| {
        backend_status(s, BackendKind::Codex)
            .notices
            .iter()
            .any(|n| n.code.as_deref() == Some("loginAccountUnconfirmed"))
    });
    assert_eq!(h.account().state, LoginState::LoggedIn);
}

#[test]
fn poll_that_authenticates_before_the_notification_wins() {
    let h = start_default();
    h.login(LoginMethod::CodexDeviceCode);
    h.script.reads(|_| Read::Authenticated);
    h.wait_account("authenticated by polling", |a| {
        a.state == LoginState::LoggedIn && a.login.is_none()
    });
    // A late notification for an attempt the poll already finished.
    h.completed(Some("login-1"), true, None);
    h.updated();
    sleep(150);
    assert_eq!(h.account().state, LoginState::LoggedIn);
    assert!(h.account().login.is_none());
}

#[test]
fn unattributed_success_is_confirmed_by_an_account_read() {
    let h = start(5_000, 60_000, 2_000, &[0, 20, 40]);
    h.login(LoginMethod::CodexBrowser);
    assert!(matches!(h.account().login, Some(LoginFlow::Browser { .. })));
    h.script.reads(|_| Read::Authenticated);
    h.completed(None, true, None);
    h.wait_account("confirmed by read", |a| {
        a.state == LoginState::LoggedIn && a.login.is_none()
    });
}

#[test]
fn stale_completion_cannot_end_current_login() {
    let h = start_default();
    h.login(LoginMethod::CodexDeviceCode);
    h.completed(Some("older-login"), false, Some("old failure"));
    sleep(100);
    assert!(matches!(h.account().login, Some(LoginFlow::DeviceCode { .. })));
    h.completed(Some("login-1"), false, Some("test failure"));
    h.wait_account("matching failure", |a| {
        completed_error(a).as_deref() == Some("test failure")
    });
    assert_eq!(h.account().state, LoginState::LoggedOut);
}

#[test]
fn routing_discovery_failure_shows_the_reason_and_the_account_error() {
    let h = start(5_000, 60_000, 2_000, &[0, 20, 40]);
    h.login(LoginMethod::CodexDeviceCode);
    // 0.157.1: credentials were stored, discovery failed, every read now errors.
    h.script.reads(|_| Read::Error("workspace routing discovery failed"));
    h.completed(Some("login-1"), false, Some("workspace routing discovery failed"));
    let failed = h.wait_account("account error", |a| a.check_error.is_some());
    assert_eq!(failed.state, LoginState::LoggedOut);
    assert_eq!(
        completed_error(&failed).as_deref(),
        Some("workspace routing discovery failed")
    );
    assert_eq!(
        failed.check_error.as_deref(),
        Some("workspace routing discovery failed")
    );
    // With the network back, Check again signs in without a new login.
    h.script.reads(|_| Read::Authenticated);
    h.backend.refresh_account().unwrap();
    h.wait_account("recovered", |a| {
        a.state == LoginState::LoggedIn && a.check_error.is_none() && a.login.is_none()
    });
}

#[test]
fn slow_reads_stay_single_and_do_not_delay_cancellation() {
    let h = start(5_000, 10, 100, &[0, 20, 40]);
    h.login(LoginMethod::CodexDeviceCode);
    // After the browser step Codex waits up to 15 s in routing discovery for each read.
    h.script.reads(|_| Read::Hold);
    h.wait_account("check failure reported", |a| a.check_error.is_some());
    for _ in 0..3 {
        // Resume re-checks join the outstanding read.
        h.backend.refresh_account().unwrap();
    }
    sleep(200);
    assert_eq!(
        h.script.max_outstanding.load(Ordering::SeqCst),
        1,
        "one account/read outstanding at most"
    );
    assert_eq!(h.account().state, LoginState::LoggingIn);
    assert!(LoginFlow::is_pending(h.account().login.as_ref()));
    h.backend.cancel_login("login-1").unwrap();
    assert_eq!(completed_error(&h.account()).as_deref(), Some(CANCELLED));
    assert_eq!(h.account().state, LoginState::LoggedOut);
    h.wait_written("account/login/cancel", 1);
    h.release(Read::Error("workspace routing discovery timed out"));
    sleep(100);
    assert_eq!(h.account().state, LoginState::LoggedOut);
}

#[test]
fn cancellation_while_starting_releases_the_server_login() {
    let h = Arc::new(start_default());
    *h.script.hold_start.lock().unwrap() = true;
    let login = {
        let h = h.clone();
        std::thread::spawn(move || h.login(LoginMethod::CodexDeviceCode))
    };
    let starting = h.wait_account("starting", |a| matches!(a.login, Some(LoginFlow::Progress { .. })));
    let attempt = starting.login.as_ref().unwrap().login_id().unwrap().to_string();
    assert!(attempt.starts_with("attempt:"));
    h.backend.cancel_login(&attempt).unwrap();
    let flow = login.join().unwrap();
    assert!(matches!(&flow, LoginFlow::Completed { error: Some(e), .. } if e == CANCELLED));
    assert_eq!(h.account().state, LoginState::LoggedOut);
    // The start answer arrives afterwards; the adapter cancels the login it created.
    eventually("start sent", || h.script.held_start.lock().unwrap().is_some());
    let (id, result) = h.script.held_start.lock().unwrap().take().unwrap();
    h.process().emit(&json!({"id":id,"result":result}));
    h.wait_written("account/login/cancel", 1);
    let cancel = h
        .process()
        .written()
        .into_iter()
        .rfind(|f| f["method"] == "account/login/cancel")
        .unwrap();
    assert_eq!(cancel["params"]["loginId"], "login-1");
    assert!(matches!(h.account().login, Some(LoginFlow::Completed { .. })));
}

#[test]
fn start_failure_is_returned_as_visible_failure() {
    let h = start_default();
    *h.script.start_error.lock().unwrap() = Some("failed to request device code");
    let flow = h.login(LoginMethod::CodexDeviceCode);
    let LoginFlow::Completed { success, error, .. } = flow else {
        panic!("completed")
    };
    assert!(!success);
    assert_eq!(error.as_deref(), Some("failed to request device code"));
    assert_eq!(
        completed_error(&h.account()).as_deref(),
        Some("failed to request device code")
    );
    assert_eq!(h.account().state, LoginState::LoggedOut);
    assert!(h.account().check_error.is_none());
}

#[test]
fn empty_api_key_is_a_visible_start_failure() {
    let h = start_default();
    let flow = h.backend.login(LoginMethod::CodexApiKey, None).unwrap();
    assert!(matches!(&flow, LoginFlow::Completed { success: false, error: Some(e), .. } if e == "API key required"));
    assert_eq!(h.process().count("account/login/start"), 0);
}

#[test]
fn resume_read_finishes_a_waiting_login() {
    let h = start(5_000, 60_000, 2_000, &[0, 20, 40]);
    h.login(LoginMethod::CodexDeviceCode);
    h.script.reads(|_| Read::Authenticated);
    // What the app does when it returns from the browser.
    h.backend.refresh_account().unwrap();
    assert_eq!(h.account().state, LoginState::LoggedIn);
    assert!(h.account().login.is_none());
    let count = h.script.read_count.load(Ordering::SeqCst);
    sleep(150);
    assert_eq!(
        h.script.read_count.load(Ordering::SeqCst),
        count,
        "no polling after the attempt ended"
    );
}

#[test]
fn account_polling_recovers_missing_completion_notification() {
    let h = start_default();
    h.login(LoginMethod::CodexDeviceCode);
    h.script.reads(|_| Read::Authenticated);
    h.wait_account("authenticated without notification", |a| {
        a.state == LoginState::LoggedIn && a.login.is_none()
    });
}

#[test]
fn timeout_ends_waiting_and_cancels_server_login() {
    let h = start(100, 20, 2_000, &[0, 20, 40]);
    h.login(LoginMethod::CodexDeviceCode);
    h.wait_account("login timeout", |a| {
        completed_error(a).is_some_and(|e| e.contains("超时"))
    });
    h.wait_written("account/login/cancel", 1);
    assert_eq!(h.account().state, LoginState::LoggedOut);
}

#[test]
fn cancellation_and_process_exit_end_waiting() {
    let h = start_default();
    h.login(LoginMethod::CodexDeviceCode);
    h.backend.cancel_login("login-1").unwrap();
    assert_eq!(completed_error(&h.account()).as_deref(), Some(CANCELLED));
    h.login(LoginMethod::CodexDeviceCode);
    h.process().exit(1);
    h.wait_account("login process exit", |a| {
        matches!(a.login, Some(LoginFlow::Completed { .. })) && a.state != LoginState::LoggingIn
    });
}

#[test]
fn sign_out_notification_lets_the_logged_out_read_apply() {
    let h = start(5_000, 60_000, 2_000, &[0, 20, 40]);
    h.login(LoginMethod::CodexDeviceCode);
    h.completed(Some("login-1"), true, None);
    eventually("unconfirmed", || h.has_notice("loginAccountUnconfirmed"));
    h.notify("account/updated", json!({"authMode":null,"planType":null}));
    h.wait_account("signed out", |a| {
        a.state == LoginState::LoggedOut && a.login.is_none()
    });
}

#[test]
fn completion_before_the_start_answer_is_kept_for_that_attempt() {
    let h = Arc::new(start(5_000, 60_000, 2_000, &[0, 20, 40]));
    *h.script.hold_start.lock().unwrap() = true;
    let login = {
        let h = h.clone();
        std::thread::spawn(move || h.login(LoginMethod::CodexDeviceCode))
    };
    eventually("start sent", || h.script.held_start.lock().unwrap().is_some());
    h.script.reads(|_| Read::Authenticated);
    h.completed(Some("login-1"), true, None);
    sleep(50);
    let (id, result) = h.script.held_start.lock().unwrap().take().unwrap();
    h.process().emit(&json!({"id":id,"result":result}));
    assert_eq!(login.join().unwrap().login_id(), Some("login-1"));
    h.wait_account("confirmed after early completion", |a| {
        a.state == LoginState::LoggedIn && a.login.is_none() && a.email.is_some()
    });
}

#[test]
fn startup_read_failure_is_visible_instead_of_unknown() {
    let script = Arc::new(Script::default());
    script.reads(|_| Read::Error("workspace routing discovery timed out"));
    let h = harness(LoginTimings::default(), script);
    h.backend.start().unwrap();
    let failed = h.wait_account("startup check failure", |a| a.check_error.is_some());
    assert_eq!(failed.state, LoginState::Unknown);
    assert_eq!(
        failed.check_error.as_deref(),
        Some("workspace routing discovery timed out")
    );
}
