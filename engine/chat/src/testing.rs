//! Test-only fakes (feature `testing`): in-memory pipes, a scripted guest process and runtime, the
//! recorded-transcript `ScriptedServer` ported from the retained Kotlin harness, and a reducing
//! event store. Fixture JSON is manipulated directly here because it is test data, not vendor IO.
use crate::{
    backend::EventSink,
    error::{ChatError, ErrorKind, Result},
    model::*,
    ports::{AgentProcess, AgentRuntime, AgentStdio, SpawnSpec},
    reducer,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    io::{Read, Write},
    sync::{Arc, Condvar, Mutex, Weak},
    time::{Duration, Instant},
};

#[derive(Default)]
struct PipeState {
    bytes: VecDeque<u8>,
    closed: bool,
}
type Shared = Arc<(Mutex<PipeState>, Condvar)>;
/// The reading end of an in-memory pipe; it blocks until data or the writer closes.
pub struct PipeReader(Shared);
/// The writing end; writes never block.
#[derive(Clone)]
pub struct PipeWriter(Shared);
pub fn pipe() -> (PipeWriter, PipeReader) {
    let shared: Shared = Default::default();
    (PipeWriter(shared.clone()), PipeReader(shared))
}
impl Read for PipeReader {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let mut state = self.0.0.lock().unwrap();
        while state.bytes.is_empty() && !state.closed {
            state = self.0.1.wait(state).unwrap();
        }
        let n = out.len().min(state.bytes.len());
        for (slot, byte) in out.iter_mut().zip(state.bytes.drain(..n)) {
            *slot = byte;
        }
        Ok(n)
    }
}
impl PipeWriter {
    pub fn push(&self, bytes: &[u8]) {
        let mut state = self.0.0.lock().unwrap();
        if !state.closed {
            state.bytes.extend(bytes);
        }
        self.0.1.notify_all();
    }
    pub fn close(&self) {
        self.0.0.lock().unwrap().closed = true;
        self.0.1.notify_all();
    }
}
impl Write for PipeWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.0.0.lock().unwrap().closed {
            return Err(std::io::ErrorKind::BrokenPipe.into());
        }
        self.push(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub type React = Arc<dyn Fn(&FakeProcess, &Value) + Send + Sync>;

/// An in-memory child. Every frame the adapter writes is recorded and handed to `react` on the
/// writer's thread, which answers by pushing frames to stdout.
pub struct FakeProcess {
    pub spec: SpawnSpec,
    out: PipeWriter,
    err: PipeWriter,
    written: Mutex<Vec<Value>>,
    exit: Mutex<Option<i32>>,
    exited: Condvar,
    stdio: Mutex<Option<AgentStdio>>,
}
struct Capture {
    buffer: Vec<u8>,
    process: Weak<FakeProcess>,
    react: React,
}
impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        for byte in bytes {
            if *byte == b'\n' {
                let line = std::mem::take(&mut self.buffer);
                let frame: Value = crate::wire::parse(std::str::from_utf8(&line).expect("UTF-8 frame"))
                    .expect("adapter wrote JSON")
                    .0;
                if let Some(process) = self.process.upgrade() {
                    if process.exit.lock().unwrap().is_some() {
                        return Err(std::io::ErrorKind::BrokenPipe.into());
                    }
                    process.written.lock().unwrap().push(frame.clone());
                    (self.react)(&process, &frame);
                }
            } else {
                self.buffer.push(*byte);
            }
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl FakeProcess {
    pub fn new(spec: SpawnSpec, react: React) -> Arc<Self> {
        let (out, stdout) = pipe();
        let (err, stderr) = pipe();
        Arc::new_cyclic(|weak| Self {
            spec,
            out,
            err,
            written: Mutex::new(Vec::new()),
            exit: Mutex::new(None),
            exited: Condvar::new(),
            stdio: Mutex::new(Some(AgentStdio {
                stdin: Box::new(Capture {
                    buffer: Vec::new(),
                    process: weak.clone(),
                    react,
                }),
                stdout: Box::new(stdout),
                stderr: Box::new(stderr),
            })),
        })
    }
    pub fn take_stdio(&self) -> AgentStdio {
        self.stdio.lock().unwrap().take().expect("stdio taken once")
    }
    pub fn emit(&self, frame: &Value) {
        let mut line = serde_json::to_vec(frame).unwrap();
        line.push(b'\n');
        self.out.push(&line);
    }
    pub fn emit_raw(&self, text: &str) {
        self.out.push(text.as_bytes());
    }
    pub fn emit_stderr(&self, text: &str) {
        self.err.push(text.as_bytes());
    }
    pub fn exit(&self, code: i32) {
        let mut exit = self.exit.lock().unwrap();
        if exit.is_none() {
            *exit = Some(code);
            self.out.close();
            self.err.close();
            self.exited.notify_all();
        }
    }
    pub fn written(&self) -> Vec<Value> {
        self.written.lock().unwrap().clone()
    }
    pub fn count(&self, method: &str) -> usize {
        self.written()
            .iter()
            .filter(|f| f["method"].as_str() == Some(method))
            .count()
    }
}
impl AgentProcess for FakeProcess {
    fn kill(&self, force: bool) {
        self.exit(if force { 137 } else { 143 });
    }
    fn wait(&self) -> i32 {
        let mut exit = self.exit.lock().unwrap();
        loop {
            if let Some(code) = *exit {
                return code;
            }
            exit = self.exited.wait(exit).unwrap();
        }
    }
    fn is_alive(&self) -> bool {
        self.exit.lock().unwrap().is_none()
    }
}

type Factory = Box<dyn Fn(usize, &SpawnSpec) -> Arc<FakeProcess> + Send + Sync>;
/// Hands out scripted processes in spawn order and records them.
pub struct FakeRuntime {
    factory: Factory,
    pub launched: Mutex<Vec<Arc<FakeProcess>>>,
    pub base: BTreeMap<String, String>,
    pub fail: Mutex<Option<String>>,
}
impl FakeRuntime {
    pub fn new(factory: impl Fn(usize, &SpawnSpec) -> Arc<FakeProcess> + Send + Sync + 'static) -> Arc<Self> {
        Arc::new(Self {
            factory: Box::new(factory),
            launched: Mutex::new(Vec::new()),
            base: BTreeMap::new(),
            fail: Mutex::new(None),
        })
    }
    /// A runtime whose processes all answer through `react`.
    pub fn reacting(react: impl Fn(&FakeProcess, &Value) + Send + Sync + 'static) -> Arc<Self> {
        let react: React = Arc::new(react);
        Self::new(move |_, spec| FakeProcess::new(spec.clone(), react.clone()))
    }
    pub fn process(&self, n: usize) -> Arc<FakeProcess> {
        self.launched.lock().unwrap()[n].clone()
    }
    pub fn last(&self) -> Arc<FakeProcess> {
        self.launched.lock().unwrap().last().cloned().expect("a process was launched")
    }
    pub fn count(&self) -> usize {
        self.launched.lock().unwrap().len()
    }
}
impl AgentRuntime for FakeRuntime {
    fn spawn(&self, spec: &SpawnSpec) -> Result<(Arc<dyn AgentProcess>, AgentStdio)> {
        if let Some(message) = self.fail.lock().unwrap().clone() {
            return Err(ChatError::new(ErrorKind::BackendUnavailable, message));
        }
        let mut launched = self.launched.lock().unwrap();
        let process = (self.factory)(launched.len(), spec);
        launched.push(process.clone());
        let stdio = process.take_stdio();
        Ok((process, stdio))
    }
    fn base_env(&self) -> BTreeMap<String, String> {
        self.base.clone()
    }
}

/// One recorded frame of a research transcript: `{t, phase, dir, msg}`.
#[derive(Clone, Debug)]
pub struct Envelope {
    pub phase: String,
    pub dir: String,
    pub msg: Value,
}
pub fn envelopes(text: &str) -> Vec<Envelope> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let v: Value = crate::wire::parse(l).expect("fixture line").0;
            Envelope {
                phase: v["phase"].as_str().unwrap_or("").into(),
                dir: v["dir"].as_str().unwrap_or("").into(),
                msg: if v["msg"].is_object() { v["msg"].clone() } else { json!({}) },
            }
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    JsonRpc,
    ClaudeControl,
}
struct Step {
    out: Value,
    inbound: Vec<Value>,
    consumed: bool,
}
/// Replays a recorded transcript as the server side of a fake process.
///
/// Each recorded client frame starts a step owning the server frames recorded after it. When the
/// adapter writes a frame, the first unconsumed step with the same key (JSON-RPC method, control
/// subtype, answered request ID or user message) is emitted, with recorded IDs of *our* requests
/// rewritten to the IDs actually used. Server-originated IDs are emitted verbatim. Unscripted
/// requests get the last recorded response of the same kind.
pub struct ScriptedServer {
    protocol: Protocol,
    preamble: Vec<Value>,
    steps: Mutex<Vec<Step>>,
    pub unmatched: Mutex<Vec<Value>>,
}
impl ScriptedServer {
    pub fn new(envelopes: Vec<Envelope>, protocol: Protocol) -> Arc<Self> {
        let mut preamble = Vec::new();
        let mut steps: Vec<Step> = Vec::new();
        for e in envelopes {
            match e.dir.as_str() {
                "out" => steps.push(Step {
                    out: e.msg,
                    inbound: Vec::new(),
                    consumed: false,
                }),
                "in" => match steps.last_mut() {
                    Some(step) => step.inbound.push(e.msg),
                    None => preamble.push(e.msg),
                },
                _ => (),
            }
        }
        Arc::new(Self {
            protocol,
            preamble,
            steps: Mutex::new(steps),
            unmatched: Mutex::new(Vec::new()),
        })
    }
    pub fn start(&self, process: &FakeProcess) {
        for frame in &self.preamble {
            process.emit(frame);
        }
    }
    pub fn remaining(&self) -> usize {
        self.steps.lock().unwrap().iter().filter(|s| !s.consumed).count()
    }
    /// A runtime whose n-th process replays `servers[n]`.
    pub fn runtime(servers: Vec<Arc<ScriptedServer>>) -> Arc<FakeRuntime> {
        FakeRuntime::new(move |n, spec| {
            let server = servers[n].clone();
            let replay = server.clone();
            let process = FakeProcess::new(spec.clone(), Arc::new(move |p, f| replay.react(p, f)));
            server.start(&process);
            process
        })
    }
    pub fn react(&self, process: &FakeProcess, frame: &Value) {
        let mut steps = self.steps.lock().unwrap();
        match self.protocol {
            Protocol::JsonRpc => self.json_rpc(&mut steps, process, frame),
            Protocol::ClaudeControl => self.control(&mut steps, process, frame),
        }
    }
    fn json_rpc(&self, steps: &mut [Step], process: &FakeProcess, frame: &Value) {
        let method = frame.get("method").and_then(Value::as_str);
        let id = frame.get("id");
        let index = steps.iter().position(|s| {
            !s.consumed
                && match (method, id) {
                    (Some(m), Some(_)) => s.out["method"].as_str() == Some(m) && s.out.get("id").is_some(),
                    (Some(m), None) => s.out["method"].as_str() == Some(m) && s.out.get("id").is_none(),
                    (None, Some(id)) => s.out.get("method").is_none() && s.out.get("id") == Some(id),
                    _ => false,
                }
        });
        let Some(index) = index else {
            self.unmatched.lock().unwrap().push(frame.clone());
            if let (Some(method), Some(id)) = (method, id) {
                let previous = steps
                    .iter()
                    .rev()
                    .find(|s| s.out["method"].as_str() == Some(method) && s.out.get("id").is_some());
                let response = previous.and_then(|p| {
                    p.inbound
                        .iter()
                        .find(|m| m.get("method").is_none() && m.get("id") == p.out.get("id"))
                });
                process.emit(&match response {
                    Some(r) => with(r, "id", id.clone()),
                    None => json!({"id": id, "error": {"code": -32601, "message": format!("unscripted {method}")}}),
                });
            }
            return;
        };
        let step = &mut steps[index];
        step.consumed = true;
        let recorded = step.out.get("id").cloned();
        for msg in &step.inbound {
            let ours = method.is_some()
                && id.is_some()
                && msg.get("method").is_none()
                && msg.get("id") == recorded.as_ref()
                && (msg.get("result").is_some() || msg.get("error").is_some());
            process.emit(&if ours { with(msg, "id", id.unwrap().clone()) } else { msg.clone() });
        }
    }
    fn control(&self, steps: &mut [Step], process: &FakeProcess, frame: &Value) {
        let kind = frame["type"].as_str();
        let subtype = |f: &Value| f["request"]["subtype"].as_str().map(str::to_owned);
        let index = steps.iter().position(|s| {
            !s.consumed
                && match kind {
                    Some("control_request") => {
                        s.out["type"].as_str() == Some("control_request") && subtype(&s.out) == subtype(frame)
                    }
                    Some("control_response") => {
                        s.out["type"].as_str() == Some("control_response")
                            && s.out["response"]["request_id"] == frame["response"]["request_id"]
                    }
                    Some("user") => s.out["type"].as_str() == Some("user"),
                    _ => false,
                }
        });
        let ours = frame.get("request_id").cloned();
        let Some(index) = index else {
            self.unmatched.lock().unwrap().push(frame.clone());
            if kind == Some("control_request") {
                let id = ours.unwrap_or(Value::Null);
                let previous = steps.iter().rev().find(|s| {
                    s.out["type"].as_str() == Some("control_request") && subtype(&s.out) == subtype(frame)
                });
                let response = previous.and_then(|p| {
                    p.inbound.iter().find(|m| {
                        m["type"].as_str() == Some("control_response")
                            && m["response"]["request_id"] == p.out["request_id"]
                    })
                });
                process.emit(&match response {
                    Some(r) => rewrite(r, id),
                    None => json!({"type": "control_response", "response": {"subtype": "error", "request_id": id,
                        "error": format!("unscripted {}", subtype(frame).unwrap_or_default())}}),
                });
            }
            return;
        };
        let step = &mut steps[index];
        step.consumed = true;
        let recorded = step.out["request_id"].clone();
        for msg in &step.inbound {
            let is_ours = kind == Some("control_request")
                && msg["type"].as_str() == Some("control_response")
                && msg["response"]["request_id"] == recorded;
            process.emit(&if is_ours { rewrite(msg, ours.clone().unwrap_or(Value::Null)) } else { msg.clone() });
        }
    }
}
fn with(frame: &Value, key: &str, value: Value) -> Value {
    let mut frame = frame.clone();
    frame[key] = value;
    frame
}
fn rewrite(response: &Value, id: Value) -> Value {
    let mut response = response.clone();
    response["response"]["request_id"] = id;
    response
}

/// Folds every event into `AgentState`, like the retained `AgentStateStore`, and lets tests wait.
#[derive(Default)]
pub struct EventStore {
    state: Mutex<(AgentState, Vec<AgentEvent>)>,
    changed: Condvar,
    listeners: Mutex<Vec<Box<dyn Fn(&AgentState, &AgentEvent) + Send + Sync>>>,
}
impl EventStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
    pub fn state(&self) -> AgentState {
        self.state.lock().unwrap().0.clone()
    }
    pub fn events(&self) -> Vec<AgentEvent> {
        self.state.lock().unwrap().1.clone()
    }
    pub fn listen(&self, listener: impl Fn(&AgentState, &AgentEvent) + Send + Sync + 'static) {
        self.listeners.lock().unwrap().push(Box::new(listener));
    }
    /// Waits up to ten seconds for `predicate`.
    pub fn wait(&self, what: &str, predicate: impl Fn(&AgentState) -> bool) -> AgentState {
        self.wait_for(Duration::from_secs(10), what, predicate)
    }
    pub fn wait_for(&self, timeout: Duration, what: &str, predicate: impl Fn(&AgentState) -> bool) -> AgentState {
        let deadline = Instant::now() + timeout;
        let mut guard = self.state.lock().unwrap();
        loop {
            if predicate(&guard.0) {
                return guard.0.clone();
            }
            let now = Instant::now();
            if now >= deadline {
                panic!("timed out waiting for {what}");
            }
            guard = self.changed.wait_timeout(guard, deadline - now).unwrap().0;
        }
    }
}
impl EventSink for EventStore {
    fn emit(&self, event: AgentEvent) {
        let mut guard = self.state.lock().unwrap();
        reducer::reduce(&mut guard.0, &event);
        for listener in self.listeners.lock().unwrap().iter() {
            listener(&guard.0, &event);
        }
        guard.1.push(event);
        self.changed.notify_all();
    }
}

/// Polls `condition` for up to five seconds.
pub fn eventually(what: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition() {
        if Instant::now() >= deadline {
            panic!("timed out waiting for {what}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
/// The JSON form of a model value, for assertions.
pub fn json_of<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap()
}
pub fn opaque(value: Value) -> OpaqueJson {
    OpaqueJson(value)
}

/// Fixed clock and sequential IDs for deterministic replays.
pub struct FixedClock(pub i64);
impl crate::ports::Clock for FixedClock {
    fn now_ms(&self) -> i64 {
        self.0
    }
}
pub struct SequenceIds {
    prefix: String,
    next: Mutex<usize>,
    listed: Mutex<VecDeque<String>>,
}
impl SequenceIds {
    /// `prefix-1`, `prefix-2`, ...
    pub fn new(prefix: &str) -> Arc<Self> {
        Arc::new(Self {
            prefix: prefix.into(),
            next: Mutex::new(0),
            listed: Mutex::new(VecDeque::new()),
        })
    }
    /// The given IDs in order, then `prefix-n`.
    pub fn listed(ids: &[&str]) -> Arc<Self> {
        let s = Self::new("id");
        *s.listed.lock().unwrap() = ids.iter().map(|s| s.to_string()).collect();
        s
    }
}
impl crate::ports::IdSource for SequenceIds {
    fn new_id(&self) -> String {
        if let Some(id) = self.listed.lock().unwrap().pop_front() {
            return id;
        }
        let mut n = self.next.lock().unwrap();
        *n += 1;
        format!("{}-{}", self.prefix, *n)
    }
}

pub fn backend_status(state: &AgentState, kind: BackendKind) -> BackendStatus {
    state.backends.get(&kind).cloned().unwrap_or_default()
}
pub fn thread<'a>(state: &'a AgentState, kind: BackendKind, id: &str) -> Option<&'a ThreadState> {
    state.threads.get(&ThreadKey {
        backend: kind,
        id: id.into(),
    })
}
pub fn turn<'a>(thread: &'a ThreadState, id: &str) -> Option<&'a Turn> {
    thread.turns.iter().find(|t| t.id == id)
}
pub fn final_message(turn: &Turn) -> Option<&AgentMessageItem> {
    turn.items.iter().rev().find_map(|i| match i {
        Item::AgentMessage(m) if m.phase == MessagePhase::Final => Some(m),
        _ => None,
    })
}
pub fn request<'a>(state: &'a AgentState, key: &RequestKey) -> Option<&'a PendingRequest> {
    state.requests.get(key)
}

/// In-memory implementations of the remaining Chat ports.
pub mod ports {
    use crate::{
        config::{AgentConfig, AgentPatch},
        error::{ChatError, ErrorKind, Result},
        ports::*,
        service::{
            index::IndexDocument,
            ledger::{SendRecord, SubmittedComposer},
        },
    };
    use std::{
        collections::BTreeMap,
        sync::{Mutex, atomic::{AtomicUsize, Ordering}},
    };
    use workflow_environment::{
        config::ConfigSection,
        tools::{ToolPhase, ToolStatus, ToolsStatus},
    };

    #[derive(Default)]
    pub struct MemoryStore {
        pub index: Mutex<Option<String>>,
        pub sends: Mutex<BTreeMap<String, SendRecord>>,
    }
    impl ChatStore for MemoryStore {
        fn read_index(&self) -> Result<Option<IndexDocument>> {
            self.index
                .lock()
                .unwrap()
                .as_ref()
                .map(|s| IndexDocument::decode(s.as_bytes()))
                .transpose()
        }
        fn write_index(&self, document: &IndexDocument) -> Result<()> {
            *self.index.lock().unwrap() = Some(serde_json::to_string(document)?);
            Ok(())
        }
        fn quarantine_index(&self) -> Result<()> {
            self.index.lock().unwrap().take();
            Ok(())
        }
        fn read_send(&self, key: &str) -> Result<Option<SendRecord>> {
            Ok(self.sends.lock().unwrap().get(key).cloned())
        }
        fn write_send(&self, key: &str, record: &SendRecord) -> Result<()> {
            self.sends.lock().unwrap().insert(key.into(), record.clone());
            Ok(())
        }
        fn remove_sends(&self, conversation: &str) -> Result<()> {
            self.sends
                .lock()
                .unwrap()
                .retain(|_, r| r.conversation_id.as_deref() != Some(conversation));
            Ok(())
        }
    }

    #[derive(Default)]
    pub struct MemoryPreferences(pub Mutex<AgentConfig>);
    impl AgentPreferences for MemoryPreferences {
        fn get(&self) -> AgentConfig {
            self.0.lock().unwrap().clone()
        }
        fn update(&self, patch: AgentPatch) -> Result<()> {
            self.0
                .lock()
                .unwrap()
                .apply(patch)
                .map_err(|e| ChatError::new(ErrorKind::Store, e.message))
        }
    }

    #[derive(Default)]
    pub struct MemoryWorkspace {
        pub files: Mutex<BTreeMap<String, Vec<u8>>>,
        pub acknowledged: Mutex<Vec<SubmittedComposer>>,
        pub removed: Mutex<Vec<String>>,
    }
    impl WorkspaceBridge for MemoryWorkspace {
        fn read_attachment(&self, path: &str, max: usize) -> Result<Option<Vec<u8>>> {
            Ok(self.files.lock().unwrap().get(path).map(|b| b[..b.len().min(max)].to_vec()))
        }
        fn acknowledge_composer(&self, submitted: &SubmittedComposer) -> Result<()> {
            self.acknowledged.lock().unwrap().push(submitted.clone());
            Ok(())
        }
        fn remove_conversation(&self, id: &str) -> Result<()> {
            self.removed.lock().unwrap().push(id.into());
            Ok(())
        }
    }

    #[derive(Default)]
    pub struct MemoryHome(pub Mutex<BTreeMap<String, Vec<u8>>>);
    impl GuestHome for MemoryHome {
        fn read(&self, path: &str, max: usize) -> Result<Option<Vec<u8>>> {
            match self.0.lock().unwrap().get(path) {
                Some(b) if b.len() > max => Err(ChatError::invalid("too large")),
                other => Ok(other.cloned()),
            }
        }
    }

    /// Tool status with a settable phase per tool; installs mark Claude installing.
    pub struct FakeTools {
        pub phases: Mutex<BTreeMap<String, ToolPhase>>,
        pub installs: AtomicUsize,
    }
    impl FakeTools {
        pub fn new(codex: ToolPhase, claude: ToolPhase) -> Self {
            Self {
                phases: Mutex::new(BTreeMap::from([("codex".into(), codex), ("claude".into(), claude)])),
                installs: AtomicUsize::new(0),
            }
        }
        pub fn set(&self, id: &str, phase: ToolPhase) {
            self.phases.lock().unwrap().insert(id.into(), phase);
        }
        fn snapshot(&self) -> ToolsStatus {
            ToolsStatus {
                revision: 1,
                tools: self
                    .phases
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|(id, phase)| {
                        serde_json::from_value::<ToolStatus>(serde_json::json!({
                            "id": id,
                            "binary": format!("/opt/workflow/tools/{id}/bin/{id}"),
                            "phase": phase,
                        }))
                        .unwrap()
                    })
                    .collect(),
            }
        }
    }
    impl AgentTools for FakeTools {
        fn status(&self) -> Result<ToolsStatus> {
            Ok(self.snapshot())
        }
        fn install_claude(&self, _retry: bool) -> Result<ToolsStatus> {
            self.installs.fetch_add(1, Ordering::SeqCst);
            self.set("claude", ToolPhase::Installing);
            Ok(self.snapshot())
        }
    }
}
