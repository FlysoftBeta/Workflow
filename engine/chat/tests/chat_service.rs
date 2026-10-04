//! Command-level tests of the Rust Chat service over in-memory ports and a scripted Codex: the
//! `chat.command` names and shapes the unchanged Kotlin client sends, the send ledger with
//! composer-revision deduplication, approval epochs, the index and tool demand.
use serde_json::{Value, json};
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use workflow_chat::{
    error::ErrorKind,
    model::*,
    service::{Chat, ChatPorts},
    testing::{ports::*, *},
};
use workflow_environment::tools::ToolPhase;

struct Fixture {
    chat: Chat,
    runtime: Arc<FakeRuntime>,
    store: Arc<MemoryStore>,
    workspace: Arc<MemoryWorkspace>,
    tools: Arc<FakeTools>,
    preferences: Arc<MemoryPreferences>,
}
fn reviewer_user(text: &str) -> Vec<Envelope> {
    envelopes(&text.replace(r#""approvalsReviewer": "auto_review""#, r#""approvalsReviewer": "user""#))
}
fn fixture(runtime: Arc<FakeRuntime>, tools: FakeTools, default: &str) -> Fixture {
    let store = Arc::new(MemoryStore::default());
    let workspace = Arc::new(MemoryWorkspace::default());
    let tools = Arc::new(tools);
    let preferences = Arc::new(MemoryPreferences::default());
    preferences.0.lock().unwrap().backend = default.into();
    let chat = Chat::start(ChatPorts {
        runtime: runtime.clone(),
        guest_home: Arc::new(MemoryHome::default()),
        tools: tools.clone(),
        store: store.clone(),
        preferences: preferences.clone(),
        workspace: workspace.clone(),
        clock: Arc::new(FixedClock(1_790_426_138_000)),
        ids: SequenceIds::new("id"),
    })
    .unwrap();
    Fixture {
        chat,
        runtime,
        store,
        workspace,
        tools,
        preferences,
    }
}
fn codex_fixture() -> Fixture {
    let server = ScriptedServer::new(
        reviewer_user(include_str!("fixtures/codex/session.jsonl")),
        Protocol::JsonRpc,
    );
    // The recorded session never deleted a thread; this server accepts it.
    let runtime = FakeRuntime::new(move |_, spec| {
        let replay = server.clone();
        let p = FakeProcess::new(
            spec.clone(),
            Arc::new(move |p, f| {
                if f["method"] == "thread/delete" {
                    p.emit(&json!({"id": f["id"], "result": {}}));
                } else {
                    replay.react(p, f);
                }
            }),
        );
        server.start(&p);
        p
    });
    fixture(
        runtime,
        FakeTools::new(ToolPhase::Ready, ToolPhase::NotInstalled),
        "codex",
    )
}
fn command(chat: &Chat, name: &str, args: Value) -> workflow_chat::error::Result<Value> {
    let args: OpaqueObject = serde_json::from_value(args).unwrap();
    chat.command(name, &args).map(|v| v.0)
}
fn wait_snapshot(chat: &Chat, what: &str, f: impl Fn(&ChatSnapshot) -> bool) -> ChatSnapshot {
    let mut last = None;
    for _ in 0..1000 {
        let s = chat.snapshot();
        if f(&s) {
            return s;
        }
        last = Some(s);
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for {what}: revision {}", last.unwrap().revision)
}
fn submitted(id: &str, revision: u64, text: &str) -> Value {
    json!({"format":1,"conversationId":id,"revision":revision,"text":text,"attachments":[]})
}
fn send_args(id: &str, revision: u64, text: &str, operation: &str) -> Value {
    json!({"id":id,"text":text,"attachments":[],"settings":{"model":"gpt-reserve","effort":"low","permissions":null},
        "mode":"AUTO","operationId":operation,"submitted":submitted(id, revision, text)})
}

#[test]
fn metadata_reflects_measured_tools_and_conversation_commands() {
    let f = fixture(
        FakeRuntime::reacting(|_, _| {}),
        FakeTools::new(ToolPhase::Ready, ToolPhase::NotInstalled),
        "codex",
    );
    let snapshot = f.chat.snapshot();
    assert_eq!(snapshot.metadata.available, [BackendKind::Codex]);
    assert_eq!(snapshot.metadata.default_backend, BackendKind::Codex);
    assert_eq!(snapshot.metadata.login_methods[&BackendKind::Claude].len(), 3);
    assert_eq!(f.tools.installs.load(Ordering::SeqCst), 0, "Codex use never installs Claude");

    let id = command(&f.chat, "newConversation", json!({"id":"c1","backend":"codex"})).unwrap();
    assert_eq!(id, "c1");
    let entry = command(&f.chat, "ensureConversation", json!({"id":"c1"})).unwrap();
    assert_eq!(entry["backend"], "CODEX");
    assert_eq!(entry["cwd"], "/workspace");
    assert_eq!(entry["backendThreadId"], Value::Null, "ChatWire writes explicit nulls");
    command(&f.chat, "rename", json!({"id":"c1","title":"  Engine-owned conversation  "})).unwrap();
    command(&f.chat, "archive", json!({"id":"c1","archived":true})).unwrap();
    command(&f.chat, "rememberSelection", json!({"id":"c1","model":"m","effort":"high"})).unwrap();
    command(&f.chat, "setPermissions", json!({"id":"c1","preset":"AUTO_EDIT"})).unwrap();
    let snapshot = f.chat.snapshot();
    let c1 = &snapshot.metadata.conversations[0];
    assert_eq!(c1.title.as_deref(), Some("Engine-owned conversation"));
    assert!(c1.archived);
    assert_eq!((c1.model.as_deref(), c1.effort.as_deref()), (Some("m"), Some("high")));
    assert_eq!(snapshot.metadata.permissions, PermissionPreset::AutoEdit);
    let prefs = f.preferences.0.lock().unwrap().clone();
    assert_eq!(prefs.permissions.as_deref(), Some("auto_edit"));
    assert_eq!(prefs.backends["codex"].model.as_deref(), Some("m"));
    // The index document keeps the retained format-1 shape.
    let index: Value = serde_json::from_str(f.store.index.lock().unwrap().as_ref().unwrap()).unwrap();
    assert_eq!(index["format"], 1);
    assert_eq!(index["conversations"][0]["archived"], true);

    command(&f.chat, "setBackend", json!({"id":"c1","kind":"claude"})).unwrap();
    assert_eq!(f.chat.snapshot().metadata.default_backend, BackendKind::Claude);
    assert_eq!(f.tools.installs.load(Ordering::SeqCst), 1, "Claude demand installs once");
    command(&f.chat, "warmUp", json!({"kind":"claude"})).unwrap_err();
    assert_eq!(f.tools.installs.load(Ordering::SeqCst), 1, "a pending install is not repeated");

    let unknown = command(&f.chat, "future", json!({})).unwrap_err();
    assert_eq!(unknown.kind, ErrorKind::InvalidArgument);
    let missing = command(&f.chat, "rename", json!({"id":"c1"})).unwrap_err();
    assert_eq!(missing.kind, ErrorKind::InvalidArgument);
    command(&f.chat, "deleteConversation", json!({"id":"c1"})).unwrap();
    assert!(f.chat.snapshot().metadata.conversations.is_empty());
    assert_eq!(*f.workspace.removed.lock().unwrap(), ["c1"]);
    f.chat.shutdown();
}

#[test]
fn watch_replays_events_and_resnapshots_foreign_epochs() {
    let f = codex_fixture();
    let before = f.chat.snapshot();
    command(&f.chat, "warmUp", json!({"kind":"codex"})).unwrap();
    let update = f
        .chat
        .watch(&before.epoch, before.revision, Duration::from_secs(1))
        .unwrap();
    assert!(!update.resnapshot);
    assert!(update.revision > before.revision);
    assert!(update.events.iter().any(|e| matches!(e, AgentEvent::ProcessChanged { state: ProcessState::Starting {}, .. })));
    assert!(update.metadata.process_epochs.contains_key(&BackendKind::Codex));
    let foreign = f.chat.watch("other-epoch", 0, Duration::ZERO).unwrap();
    assert!(foreign.resnapshot);
    assert!(f.chat.watch(&before.epoch, 0, Duration::from_secs(31)).is_err());
    f.chat.shutdown();
}

#[test]
fn send_dispatches_once_per_composer_revision_and_answers_only_with_the_users_epoch() {
    let f = codex_fixture();
    command(&f.chat, "newConversation", json!({"id":"c","backend":"codex"})).unwrap();
    let client = command(&f.chat, "send", send_args("c", 1, "Answer in Markdown only…", "op-1")).unwrap();
    let client = client.as_str().unwrap().to_string();
    // A recreated client retries the same composer revision with a new operation ID.
    let again = command(&f.chat, "send", send_args("c", 1, "Answer in Markdown only…", "op-2")).unwrap();
    assert_eq!(again, client.as_str());
    let process = f.runtime.process(0);
    assert_eq!(process.count("turn/start"), 1, "a revision is dispatched once");
    assert_eq!(f.workspace.acknowledged.lock().unwrap().len(), 2);
    // Different content for the same revision is refused.
    let conflict = command(&f.chat, "send", send_args("c", 1, "something else", "op-3")).unwrap_err();
    assert_eq!(conflict.kind, ErrorKind::SendConflict);
    // The message must match the submitted composer exactly.
    let mut mismatch = send_args("c", 2, "x", "op-4");
    mismatch["text"] = json!("y");
    assert_eq!(command(&f.chat, "send", mismatch).unwrap_err().kind, ErrorKind::InvalidArgument);

    let thread = "01a0ddb6-d670-70f3-9a62-1320d2a3b41a";
    let snapshot = wait_snapshot(&f.chat, "turn 1", |s| {
        thread_done(s, thread, "01a0ddb6-d741-7293-9b32-d5164341d611")
    });
    let entry = snapshot.metadata.conversations.iter().find(|e| e.id == "c").unwrap();
    assert_eq!(entry.backend_thread_id.as_deref(), Some(thread));
    assert_eq!(entry.preview.as_deref(), Some("Answer in Markdown only…"));
    assert_eq!(entry.model.as_deref(), Some("gpt-reserve"));
    let record = f.store.sends.lock().unwrap().values().next().cloned().unwrap();
    assert_eq!(record.client_message_id.as_deref(), Some(client.as_str()));
    assert_eq!(record.conversation_id.as_deref(), Some("c"));

    // Turn 2 asks for approval; only the user's explicit answer with the current epoch reaches Codex.
    command(&f.chat, "send", send_args("c", 2, "Create a file named hello.txt…", "op-5")).unwrap();
    let key = RequestKey {
        backend: BackendKind::Codex,
        raw_id: opaque(json!(0)),
    };
    let snapshot = wait_snapshot(&f.chat, "approval", |s| {
        s.state.requests.get(&key).is_some_and(|r| r.status == RequestStatus::Pending)
    });
    let epoch = snapshot.metadata.process_epochs[&BackendKind::Codex].clone();
    let key_json = serde_json::to_value(&key).unwrap();
    let stale = command(
        &f.chat,
        "respond",
        json!({"key":key_json,"response":{"_type":"Decide","decisionId":"accept","message":null},"processEpoch":"old"}),
    )
    .unwrap_err();
    assert_eq!(stale.kind, ErrorKind::RequestExpired);
    assert!(process.written().iter().all(|f| f.get("id") != Some(&json!(0))));
    command(
        &f.chat,
        "respond",
        json!({"key":key_json,"response":{"_type":"Decide","decisionId":"accept","message":null},"processEpoch":epoch}),
    )
    .unwrap();
    let answers: Vec<Value> = process
        .written()
        .into_iter()
        .filter(|f| f.get("id") == Some(&json!(0)))
        .collect();
    assert_eq!(answers, [json!({"id":0,"result":{"decision":"accept"}})]);
    let replay = command(
        &f.chat,
        "respond",
        json!({"key":key_json,"response":{"_type":"Decide","decisionId":"accept","message":null},"processEpoch":epoch}),
    )
    .unwrap_err();
    assert_eq!(replay.kind, ErrorKind::RequestExpired, "an answered card cannot be answered again");

    // Deleting the conversation removes its ledger records.
    wait_snapshot(&f.chat, "turn 2", |s| thread_done(s, thread, "01a0ddb6-fe01-77a2-a0a1-deff66d7d2c2"));
    command(&f.chat, "deleteConversation", json!({"id":"c"})).unwrap();
    assert!(f.store.sends.lock().unwrap().is_empty());
    f.chat.shutdown();
    assert!(!process.is_alive_fake());
}
fn thread_done(s: &ChatSnapshot, thread_id: &str, turn_id: &str) -> bool {
    thread(&s.state, BackendKind::Codex, thread_id)
        .and_then(|t| turn(t, turn_id))
        .is_some_and(|t| t.status == TurnStatus::Completed)
}
trait Alive {
    fn is_alive_fake(&self) -> bool;
}
impl Alive for FakeProcess {
    fn is_alive_fake(&self) -> bool {
        workflow_chat::ports::AgentProcess::is_alive(self)
    }
}

#[test]
fn seccomp_killed_launch_marks_backend_unsupported() {
    let runtime = FakeRuntime::new(|_, spec| {
        let p = FakeProcess::new(spec.clone(), Arc::new(|_, _| {}));
        p.exit(workflow_chat::service::hub::SIGSYS_EXIT);
        p
    });
    let f = fixture(runtime, FakeTools::new(ToolPhase::Ready, ToolPhase::NotInstalled), "codex");
    assert!(command(&f.chat, "warmUp", json!({"kind":"codex"})).is_err());
    let snapshot = wait_snapshot(&f.chat, "unsupported", |s| s.metadata.available.is_empty());
    assert!(matches!(
        snapshot.state.backends[&BackendKind::Codex].process,
        ProcessState::Exited { .. } | ProcessState::Failed { .. }
    ));
    f.chat.shutdown();
}

#[test]
fn removed_tool_stops_its_backend_and_unknown_backend_is_refused() {
    let f = codex_fixture();
    command(&f.chat, "warmUp", json!({"kind":"codex"})).unwrap();
    let process = f.runtime.process(0);
    assert!(process.is_alive_fake());
    f.tools.set("codex", ToolPhase::Failed);
    command(&f.chat, "newConversation", json!({"id":"x"})).unwrap();
    eventually("backend stopped", || !process.is_alive_fake());
    assert!(f.chat.snapshot().metadata.available.is_empty());
    assert_eq!(
        command(&f.chat, "warmUp", json!({"kind":"codex"})).unwrap_err().kind,
        ErrorKind::BackendUnavailable
    );
    assert_eq!(
        command(&f.chat, "warmUp", json!({"kind":"gemini"})).unwrap_err().kind,
        ErrorKind::InvalidArgument
    );
    f.chat.shutdown();
}
