use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use workflow_chat::{
    error::{ChatError, ErrorKind, Result},
    journal::Journal,
    model::*,
    ports::{AgentTools, ChatStore, WorkspaceBridge},
    service::{
        index::{ConversationIndex, IndexDocument},
        ledger::{SendLedger, SendRecord, SendRequest, SubmittedComposer},
        tools::ToolPolicy,
    },
};
use workflow_environment::{
    json::strict_json,
    tools::{ToolPhase, ToolStatus, ToolsStatus},
};
#[derive(Default)]
struct Memory {
    sends: Mutex<BTreeMap<String, SendRecord>>,
    index: Mutex<Option<String>>,
    quarantined: AtomicBool,
    fail_write_receipt: AtomicBool,
}
impl ChatStore for Memory {
    fn read_index(&self) -> Result<Option<IndexDocument>> {
        self.index
            .lock()
            .unwrap()
            .as_ref()
            .map(|s| IndexDocument::decode(s.as_bytes()))
            .transpose()
    }
    fn write_index(&self, d: &IndexDocument) -> Result<()> {
        *self.index.lock().unwrap() = Some(serde_json::to_string(d)?);
        Ok(())
    }
    fn quarantine_index(&self) -> Result<()> {
        self.index.lock().unwrap().take();
        self.quarantined.store(true, Ordering::SeqCst);
        Ok(())
    }
    fn read_send(&self, k: &str) -> Result<Option<SendRecord>> {
        Ok(self.sends.lock().unwrap().get(k).cloned())
    }
    fn write_send(&self, k: &str, v: &SendRecord) -> Result<()> {
        self.sends.lock().unwrap().insert(k.into(), v.clone());
        if self.fail_write_receipt.load(Ordering::SeqCst) {
            Err(error("lost durable write receipt"))
        } else {
            Ok(())
        }
    }
    fn remove_sends(&self, id: &str) -> Result<()> {
        self.sends
            .lock()
            .unwrap()
            .retain(|_, r| r.conversation_id.as_deref() != Some(id));
        Ok(())
    }
}
#[derive(Default)]
struct Ack {
    drafts: Mutex<Vec<SubmittedComposer>>,
    fail: AtomicBool,
}
impl WorkspaceBridge for Ack {
    fn read_attachment(&self, _: &str, _: usize) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }
    fn acknowledge_composer(&self, d: &SubmittedComposer) -> Result<()> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(error("ack disconnected"));
        }
        self.drafts.lock().unwrap().push(d.clone());
        Ok(())
    }
    fn remove_conversation(&self, _: &str) -> Result<()> {
        Ok(())
    }
}
fn error(message: &str) -> ChatError {
    ChatError::new(ErrorKind::Store, message)
}
fn request() -> SendRequest {
    SendRequest {
        operation: "op".into(),
        submitted: SubmittedComposer {
            format: 1,
            conversation_id: "c".into(),
            revision: 3,
            text: "hello".into(),
            attachments: vec![],
            extra: Default::default(),
        },
        settings: TurnSettings::default(),
        mode: SendMode::Auto,
    }
}
#[test]
fn concurrent_clients_and_reopen_dispatch_one_composer_once() {
    let store = Arc::new(Memory::default());
    let ack = Arc::new(Ack::default());
    let ledger = SendLedger::new(store.clone(), ack.clone());
    let calls = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let mut jobs = vec![];
        for n in 0..10 {
            let ledger = &ledger;
            let calls = &calls;
            jobs.push(scope.spawn(move || {
                let mut r = request();
                r.operation = format!("client-{n}");
                ledger
                    .send(&r, || {
                        calls.fetch_add(1, Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(2));
                        Ok("accepted".into())
                    })
                    .unwrap()
            }));
        }
        for job in jobs {
            assert_eq!(job.join().unwrap(), "accepted");
        }
    });
    let reopened = SendLedger::new(store, ack.clone());
    assert_eq!(
        reopened
            .send(&request(), || panic!("must not replay"))
            .unwrap(),
        "accepted"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(
        ack.drafts
            .lock()
            .unwrap()
            .iter()
            .all(|d| *d == request().submitted)
    );
}
#[test]
fn ambiguous_submission_cannot_replay_with_new_client_id() {
    let store = Arc::new(Memory::default());
    let ack = Arc::new(Ack::default());
    let ledger = SendLedger::new(store.clone(), ack.clone());
    assert!(
        ledger
            .send(&request(), || Err(error("lost vendor reply")))
            .is_err()
    );
    let mut r = request();
    r.operation = "new-client".into();
    let reopened = SendLedger::new(store, ack.clone());
    assert_eq!(
        reopened
            .send(&r, || panic!("must not replay"))
            .unwrap_err()
            .kind,
        ErrorKind::SendAmbiguous
    );
    assert!(ack.drafts.lock().unwrap().is_empty());
}
#[test]
fn changed_content_settings_or_mode_cannot_reuse_a_revision() {
    let ledger = SendLedger::new(Arc::new(Memory::default()), Arc::new(Ack::default()));
    ledger.send(&request(), || Ok("accepted".into())).unwrap();
    for variant in 0..3 {
        let mut r = request();
        match variant {
            0 => r.submitted.text = "changed".into(),
            1 => r.settings.model = Some("other".into()),
            _ => r.mode = SendMode::Queue,
        };
        assert_eq!(
            ledger
                .send(&r, || panic!("must not replay"))
                .unwrap_err()
                .kind,
            ErrorKind::SendConflict
        );
    }
}
#[test]
fn new_revision_and_other_conversation_can_dispatch() {
    let ledger = SendLedger::new(Arc::new(Memory::default()), Arc::new(Ack::default()));
    ledger.send(&request(), || Ok("one".into())).unwrap();
    let mut r = request();
    r.submitted.revision += 1;
    assert_eq!(ledger.send(&r, || Ok("two".into())).unwrap(), "two");
    r.submitted.conversation_id = "other".into();
    assert_eq!(ledger.send(&r, || Ok("three".into())).unwrap(), "three");
}
#[test]
fn accepted_dispatch_survives_ack_failure_and_reopen() {
    let store = Arc::new(Memory::default());
    let ack = Arc::new(Ack::default());
    ack.fail.store(true, Ordering::SeqCst);
    let ledger = SendLedger::new(store.clone(), ack.clone());
    assert!(ledger.send(&request(), || Ok("accepted".into())).is_err());
    ack.fail.store(false, Ordering::SeqCst);
    assert_eq!(
        SendLedger::new(store, ack)
            .send(&request(), || panic!("must not replay"))
            .unwrap(),
        "accepted"
    );
}
#[test]
fn interrupted_intent_receipt_never_leaves_dispatch_gap() {
    let store = Arc::new(Memory::default());
    store.fail_write_receipt.store(true, Ordering::SeqCst);
    let ack = Arc::new(Ack::default());
    let ledger = SendLedger::new(store.clone(), ack.clone());
    assert!(
        ledger
            .send(&request(), || panic!("must not dispatch"))
            .is_err()
    );
    store.fail_write_receipt.store(false, Ordering::SeqCst);
    assert_eq!(
        SendLedger::new(store.clone(), ack)
            .send(&request(), || panic!("must not replay"))
            .unwrap_err()
            .kind,
        ErrorKind::SendAmbiguous
    );
    assert_eq!(store.sends.lock().unwrap().len(), 1);
}
fn entry() -> ConversationEntry {
    ConversationEntry {
        id: "c".into(),
        backend: BackendKind::Codex,
        backend_thread_id: Some("t".into()),
        cwd: "/workspace".into(),
        ..Default::default()
    }
}
#[test]
fn concurrent_index_updates_survive_reopen() {
    let store = Arc::new(Memory::default());
    let index = ConversationIndex::load(store.clone()).unwrap();
    index.upsert(entry()).unwrap();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            index
                .update("c", |v| v.title = Some("renamed".into()))
                .unwrap()
        });
        scope.spawn(|| index.update("c", |v| v.archived = true).unwrap());
        scope.spawn(|| {
            index
                .update("c", |v| {
                    v.model = Some("model".into());
                    v.effort = Some("high".into());
                })
                .unwrap()
        });
    });
    let reopened = ConversationIndex::load(store).unwrap().entries();
    let e = &reopened[0];
    assert_eq!(e.title.as_deref(), Some("renamed"));
    assert!(e.archived);
    assert_eq!(e.model.as_deref(), Some("model"));
    assert_eq!(e.effort.as_deref(), Some("high"));
}
#[test]
fn refresh_preserves_user_title_and_archive_and_never_recreates_removed_entry() {
    let index = ConversationIndex::load(Arc::new(Memory::default())).unwrap();
    index.upsert(entry()).unwrap();
    index
        .update("c", |v| {
            v.title = Some("User title".into());
            v.archived = true;
        })
        .unwrap();
    let t = ThreadState {
        key: ThreadKey {
            backend: BackendKind::Codex,
            id: "t".into(),
        },
        title: Some("Backend title".into()),
        ..Default::default()
    };
    index.refresh_thread(&t, 10).unwrap();
    assert_eq!(index.entries()[0].title.as_deref(), Some("User title"));
    assert!(index.entries()[0].archived);
    index.remove("c").unwrap();
    index.refresh_thread(&t, 20).unwrap();
    assert!(index.entries().is_empty());
}
#[test]
fn newer_index_is_preserved_and_malformed_index_quarantined() {
    let store = Arc::new(Memory::default());
    *store.index.lock().unwrap() = Some("{\"format\":99,\"conversations\":[]}".into());
    assert!(matches!(
        ConversationIndex::load(store.clone()),
        Err(ChatError {
            kind: ErrorKind::UnsupportedFormat,
            ..
        })
    ));
    assert!(!store.quarantined.load(Ordering::SeqCst));
    assert!(store.index.lock().unwrap().is_some());
    *store.index.lock().unwrap() = Some("bad json".into());
    assert!(
        ConversationIndex::load(store.clone())
            .unwrap()
            .entries()
            .is_empty()
    );
    assert!(store.quarantined.load(Ordering::SeqCst));
}
#[test]
fn index_codec_retains_fields_skips_unknown_backends_and_duplicate_ids() {
    let mut e = entry();
    e.title = Some("标题".into());
    e.preview = Some("你好".into());
    e.archived = true;
    e.forked_at = Some("turn".into());
    let bytes = serde_json::to_vec(&IndexDocument::from_entries(&[e.clone()])).unwrap();
    let store = Arc::new(Memory::default());
    *store.index.lock().unwrap() = Some(String::from_utf8(bytes).unwrap());
    assert_eq!(ConversationIndex::load(store).unwrap().entries(), vec![e]);
    let doc=IndexDocument::decode(br#"{"format":1,"conversations":[{"id":"a","backend":"codex"},{"id":"a","backend":"claude"},{"id":"b","backend":"future"},42]}"#).unwrap();
    assert_eq!(doc.conversations.len(), 1);
    assert_eq!(doc.conversations[0].backend, "codex");
}
fn pending() -> PendingRequest {
    PendingRequest {
        key: RequestKey {
            backend: BackendKind::Codex,
            raw_id: strict_json(b"7").unwrap(),
        },
        method: "future/request".into(),
        ..Default::default()
    }
}
fn start(j: &Journal) {
    j.event(AgentEvent::ProcessChanged {
        backend: BackendKind::Codex,
        state: ProcessState::Starting {},
    })
    .unwrap();
}
#[test]
fn journal_replays_ordered_events_and_expires_old_cursors() {
    let j = Journal::new(2, 1024 * 1024);
    let before = j.snapshot();
    start(&j);
    let epoch = j.process_epoch(BackendKind::Codex).unwrap();
    j.event(AgentEvent::ProcessChanged {
        backend: BackendKind::Codex,
        state: ProcessState::Ready {},
    })
    .unwrap();
    let update = j.watch(&before.epoch, 0, Duration::ZERO).unwrap();
    assert!(!update.resnapshot);
    let mut state = before.state;
    for e in update.events {
        workflow_chat::reducer::reduce(&mut state, &e);
    }
    assert_eq!(state, j.snapshot().state);
    start(&j);
    assert_ne!(Some(epoch), j.process_epoch(BackendKind::Codex));
    assert!(
        j.watch(&before.epoch, 0, Duration::ZERO)
            .unwrap()
            .resnapshot
    );
    assert!(j.watch("old", 0, Duration::ZERO).unwrap().resnapshot);
}
#[test]
fn journal_rejects_stale_process_epoch_even_if_vendor_reuses_request_id() {
    let j = Journal::default();
    start(&j);
    let old = j.process_epoch(BackendKind::Codex).unwrap();
    j.event(AgentEvent::RequestOpened { request: pending() })
        .unwrap();
    j.validate_response(&pending().key, &old).unwrap();
    j.event(AgentEvent::ProcessChanged {
        backend: BackendKind::Codex,
        state: ProcessState::Exited {
            exit_code: Some(1),
            stderr_tail: String::new(),
        },
    })
    .unwrap();
    start(&j);
    j.event(AgentEvent::RequestOpened { request: pending() })
        .unwrap();
    assert!(j.validate_response(&pending().key, &old).is_err());
    j.validate_response(
        &pending().key,
        &j.process_epoch(BackendKind::Codex).unwrap(),
    )
    .unwrap();
}
#[test]
fn environment_activation_rotates_service_epoch_and_expires_requests() {
    let j = Journal::default();
    start(&j);
    j.event(AgentEvent::RequestOpened { request: pending() })
        .unwrap();
    let old = j.snapshot();
    j.environment_changed().unwrap();
    let new = j.snapshot();
    assert_ne!(old.epoch, new.epoch);
    assert!(new.metadata.process_epochs.is_empty());
    assert_eq!(
        new.state.requests.get(&pending().key).unwrap().status,
        RequestStatus::Expired
    );
    assert!(
        j.watch(&old.epoch, old.revision, Duration::ZERO)
            .unwrap()
            .resnapshot
    );
}
#[test]
fn journal_batch_metadata_belongs_to_returned_revision() {
    let j = Journal::default();
    let initial = j.snapshot();
    for _ in 0..512 {
        start(&j);
    }
    let expected = j.snapshot();
    start(&j);
    let update = j.watch(&initial.epoch, 0, Duration::ZERO).unwrap();
    assert_eq!(update.revision, 512);
    assert_eq!(update.events.len(), 512);
    assert_eq!(update.metadata, expected.metadata);
}
#[test]
fn oversized_single_event_requires_resnapshot_and_long_poll_wakes() {
    let j = Arc::new(Journal::default());
    let before = j.snapshot();
    let copy = j.clone();
    let epoch = before.epoch.clone();
    let waiting =
        std::thread::spawn(move || copy.watch(&epoch, 0, Duration::from_secs(1)).unwrap());
    start(&j);
    assert_eq!(waiting.join().unwrap().revision, 1);
    j.event(AgentEvent::Unknown {
        backend: BackendKind::Codex,
        kind: "large".into(),
        thread_id: None,
        raw: strict_json(
            serde_json::to_string(&"界".repeat(400_000))
                .unwrap()
                .as_bytes(),
        )
        .unwrap(),
    })
    .unwrap();
    assert!(
        j.watch(&before.epoch, 1, Duration::ZERO)
            .unwrap()
            .resnapshot
    );
}
struct Tools {
    phase: Mutex<ToolPhase>,
    installs: AtomicUsize,
    fail: AtomicBool,
}
impl Tools {
    fn new(phase: ToolPhase) -> Self {
        Self {
            phase: Mutex::new(phase),
            installs: AtomicUsize::new(0),
            fail: AtomicBool::new(false),
        }
    }
}
impl AgentTools for Tools {
    fn status(&self) -> Result<ToolsStatus> {
        Ok(ToolsStatus {
            revision: 0,
            tools: vec![ToolStatus {
                id: "claude".into(),
                phase: *self.phase.lock().unwrap(),
                version: None,
                architecture: None,
                binary: Some("/opt/workflow/tools/claude/bin/claude".into()),
                progress: None,
                error: None,
                operation_id: None,
                extra: Default::default(),
            }],
        })
    }
    fn install_claude(&self, retry: bool) -> Result<ToolsStatus> {
        assert!(!retry);
        self.installs.fetch_add(1, Ordering::SeqCst);
        if self.fail.load(Ordering::SeqCst) {
            return Err(error("lost install receipt"));
        }
        *self.phase.lock().unwrap() = ToolPhase::Installing;
        self.status()
    }
}
#[test]
fn claude_default_installs_once_even_with_lagging_status_and_concurrent_demand() {
    let p = ToolPolicy::default();
    let tools = Tools::new(ToolPhase::NotInstalled);
    p.refresh(&tools, BackendKind::Claude, None).unwrap();
    *tools.phase.lock().unwrap() = ToolPhase::NotInstalled;
    std::thread::scope(|s| {
        for _ in 0..12 {
            s.spawn(|| {
                p.refresh(&tools, BackendKind::Codex, Some(BackendKind::Claude))
                    .unwrap()
            });
        }
    });
    assert_eq!(tools.installs.load(Ordering::SeqCst), 1);
}
#[test]
fn opening_existing_claude_conversation_demands_tool_with_codex_default() {
    let p = ToolPolicy::default();
    let tools = Tools::new(ToolPhase::NotInstalled);
    p.refresh(&tools, BackendKind::Codex, None).unwrap();
    assert_eq!(tools.installs.load(Ordering::SeqCst), 0);
    p.refresh(&tools, BackendKind::Codex, Some(BackendKind::Claude))
        .unwrap();
    assert_eq!(tools.installs.load(Ordering::SeqCst), 1);
}
#[test]
fn codex_only_use_never_installs_claude() {
    let p = ToolPolicy::default();
    let tools = Tools::new(ToolPhase::NotInstalled);
    for _ in 0..3 {
        p.refresh(&tools, BackendKind::Codex, Some(BackendKind::Codex))
            .unwrap();
    }
    assert_eq!(tools.installs.load(Ordering::SeqCst), 0);
}
#[test]
fn failed_optional_tool_requires_explicit_retry() {
    let p = ToolPolicy::default();
    let tools = Tools::new(ToolPhase::Failed);
    for _ in 0..3 {
        p.refresh(&tools, BackendKind::Claude, Some(BackendKind::Claude))
            .unwrap();
    }
    assert_eq!(tools.installs.load(Ordering::SeqCst), 0);
}
#[test]
fn ambiguous_install_is_not_replayed_until_environment_generation_changes() {
    let p = ToolPolicy::default();
    let tools = Tools::new(ToolPhase::NotInstalled);
    tools.fail.store(true, Ordering::SeqCst);
    assert!(p.refresh(&tools, BackendKind::Claude, None).is_err());
    p.refresh(&tools, BackendKind::Claude, None).unwrap();
    assert_eq!(tools.installs.load(Ordering::SeqCst), 1);
    p.environment_changed();
    assert!(p.refresh(&tools, BackendKind::Claude, None).is_err());
    assert_eq!(tools.installs.load(Ordering::SeqCst), 2);
}
