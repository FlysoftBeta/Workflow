//! The backend-neutral control surface used only by the Chat service, plus small concurrency helpers.
use crate::{
    error::Result,
    model::*,
    service::launch_env::Secret,
};
use std::{
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};

/// Receives normalised events. It must be cheap and must not block on vendor IO.
pub trait EventSink: Send + Sync {
    fn emit(&self, event: AgentEvent);
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThreadOptions {
    /// Working directory inside the environment (`/workspace`).
    pub cwd: String,
    pub settings: TurnSettings,
    pub ephemeral: bool,
    pub title: Option<String>,
}
impl ThreadOptions {
    pub fn new(cwd: &str) -> Self {
        Self {
            cwd: cwd.into(),
            settings: TurnSettings::default(),
            ephemeral: false,
            title: None,
        }
    }
}

/// State is published through the [`EventSink`], never returned piecemeal. Every method blocks the
/// calling Server worker thread while vendor IO happens on the process transport.
pub trait Backend: Send + Sync {
    fn kind(&self) -> BackendKind;
    /// Spawns the backend process and performs the handshake. Idempotent.
    fn start(&self) -> Result<()>;
    fn stop(&self);
    fn refresh_account(&self) -> Result<()>;
    fn refresh_rate_limits(&self) -> Result<()>;
    fn refresh_models(&self) -> Result<ModelCatalog>;
    /// `secret` is used only for API-key methods; it is never stored by Chat or logged.
    fn login(&self, method: LoginMethod, secret: Option<Secret>) -> Result<LoginFlow>;
    fn cancel_login(&self, login_id: &str) -> Result<()>;
    fn logout(&self) -> Result<()>;
    fn start_thread(&self, options: &ThreadOptions) -> Result<String>;
    fn resume_thread(&self, thread: &str, options: &ThreadOptions) -> Result<String>;
    /// Branches `thread`; `at_turn` keeps history up to and including that turn.
    fn fork_thread(
        &self,
        thread: &str,
        at_turn: Option<&str>,
        options: &ThreadOptions,
    ) -> Result<String>;
    fn load_history(&self, thread: &str, older_than: Option<&str>) -> Result<()>;
    fn rename(&self, thread: &str, title: &str) -> Result<()>;
    fn archive(&self, thread: &str, archived: bool) -> Result<()>;
    fn delete(&self, thread: &str) -> Result<()>;
    fn compact(&self, thread: &str) -> Result<()>;
    /// Returns the client message ID that correlates the turn.
    fn send(
        &self,
        thread: &str,
        parts: Vec<UserPart>,
        settings: Option<TurnSettings>,
        mode: SendMode,
    ) -> Result<String>;
    fn cancel_queued(&self, thread: &str, client_message_id: &str) -> Result<()>;
    fn interrupt(&self, thread: &str, cancel_queued: bool) -> Result<()>;
    fn set_permissions(&self, thread: &str, preset: PermissionPreset) -> Result<()>;
    /// Answers an open request with the user's explicit choice, validated against the offer.
    fn respond(&self, key: &RequestKey, response: &RequestResponse) -> Result<()>;
    /// The explicit advanced console. Safety fields are still enforced.
    fn raw_request(
        &self,
        method: &str,
        params: Option<OpaqueJson>,
        thread: Option<&str>,
    ) -> Result<OpaqueJson>;
}

/// A cancellation token whose sleeps wake early when it is cancelled.
#[derive(Clone, Default)]
pub struct Token(Arc<(Mutex<bool>, Condvar)>);
impl Token {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        *self.0.0.lock().unwrap() = true;
        self.0.1.notify_all();
    }
    pub fn is_cancelled(&self) -> bool {
        *self.0.0.lock().unwrap()
    }
    /// Sleeps for `duration`; returns false when cancelled first.
    pub fn sleep(&self, duration: Duration) -> bool {
        let deadline = Instant::now() + duration;
        let mut cancelled = self.0.0.lock().unwrap();
        while !*cancelled {
            let now = Instant::now();
            if now >= deadline {
                return true;
            }
            cancelled = self.0.1.wait_timeout(cancelled, deadline - now).unwrap().0;
        }
        false
    }
}

/// A value produced once by a background thread and awaited by several callers.
pub struct Shared<T> {
    value: Mutex<Option<T>>,
    ready: Condvar,
}
impl<T: Clone> Shared<T> {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            value: Mutex::new(None),
            ready: Condvar::new(),
        })
    }
    pub fn set(&self, value: T) {
        *self.value.lock().unwrap() = Some(value);
        self.ready.notify_all();
    }
    pub fn is_done(&self) -> bool {
        self.value.lock().unwrap().is_some()
    }
    /// Waits up to `timeout`; `None` means the value is not available yet.
    pub fn wait(&self, timeout: Duration) -> Option<T> {
        let deadline = Instant::now() + timeout;
        let mut value = self.value.lock().unwrap();
        loop {
            if let Some(v) = value.as_ref() {
                return Some(v.clone());
            }
            let now = Instant::now();
            if now >= deadline {
                return None;
            }
            value = self.ready.wait_timeout(value, deadline - now).unwrap().0;
        }
    }
}

/// Runs `job` on a named background thread.
pub fn spawn(name: &str, job: impl FnOnce() + Send + 'static) {
    let _ = std::thread::Builder::new().name(name.into()).spawn(job);
}
