//! Port of the Kotlin `CodexBackendReplayTest`: drives the Rust Codex backend against the recorded
//! Codex 0.157.0 session (markdown and math turn, command approval, interrupt, thread operations).
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use workflow_chat::{
    backend::{Backend, ThreadOptions},
    codex::{backend::CodexBackend, launch::CodexConfig, params},
    error::ErrorKind,
    model::*,
    testing::*,
};

const THREAD: &str = "01a0ddb6-d670-70f3-9a62-1320d2a3b41a";
const T1: &str = "01a0ddb6-d741-7293-9b32-d5164341d611";
const T2: &str = "01a0ddb6-fe01-77a2-a0a1-deff66d7d2c2";
const T3: &str = "01a0ddb7-1f2b-7f51-a263-e57807c042d3";
const FORK: &str = "01a0ddb7-338f-73c0-a5c2-ee9522e2a52f";

fn fixture() -> Vec<Envelope> {
    envelopes(include_str!("fixtures/codex/session.jsonl"))
}
/// The research client did not send approvalsReviewer, so the server echoed config's auto_review.
fn as_if_reviewer_honoured(envelopes: Vec<Envelope>) -> Vec<Envelope> {
    fn fix(v: &mut Value) {
        match v {
            Value::Object(map) => {
                for (k, value) in map.iter_mut() {
                    if k == "approvalsReviewer" && value == "auto_review" {
                        *value = json!("user");
                    } else {
                        fix(value);
                    }
                }
            }
            Value::Array(items) => items.iter_mut().for_each(fix),
            _ => (),
        }
    }
    envelopes
        .into_iter()
        .map(|mut e| {
            if e.dir == "in" {
                fix(&mut e.msg);
            }
            e
        })
        .collect()
}
fn backend(envelopes: Vec<Envelope>) -> (CodexBackend, Arc<FakeRuntime>, Arc<EventStore>) {
    let server = ScriptedServer::new(envelopes, Protocol::JsonRpc);
    let runtime = ScriptedServer::runtime(vec![server]);
    let store = EventStore::new();
    let mut config = CodexConfig::new("/opt/workflow/libcodex.so", "/home/work/.codex-app");
    config.base_env = [
        ("PATH", "/usr/bin"),
        ("HOME", "/home/work"),
        ("OPENAI_API_KEY", "host-secret"),
        ("CLAUDE_CODE_ENTRYPOINT", "x"),
    ]
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .into();
    let codex = CodexBackend::new(
        runtime.clone(),
        config,
        store.clone(),
        Arc::new(FixedClock(1_790_426_138_000)),
        SequenceIds::new("client-msg"),
    );
    (codex, runtime, store)
}
fn t(state: &AgentState) -> ThreadState {
    thread(state, BackendKind::Codex, THREAD)
        .cloned()
        .expect("thread")
}
fn text(parts: &str) -> Vec<UserPart> {
    vec![UserPart::Text { text: parts.into() }]
}
fn settings() -> Option<TurnSettings> {
    Some(TurnSettings {
        model: Some("gpt-reserve".into()),
        effort: Some("low".into()),
        permissions: None,
    })
}

#[test]
fn full_session_replay() {
    let (codex, runtime, store) = backend(as_if_reviewer_honoured(fixture()));
    codex.start().unwrap();

    // Launch contract: app-owned CODEX_HOME, host credentials never forwarded.
    let spec = runtime.process(0).spec.clone();
    assert_eq!(spec.argv, ["/opt/workflow/libcodex.so", "app-server"]);
    assert_eq!(spec.env["CODEX_HOME"], "/home/work/.codex-app");
    assert!(!spec.env.contains_key("OPENAI_API_KEY"));
    assert!(!spec.env.keys().any(|k| k.starts_with("CLAUDE_")));
    assert_eq!(spec.cwd, "/workspace");

    store.wait("account, models, limits", |s| {
        let b = backend_status(s, BackendKind::Codex);
        b.account.state == LoginState::LoggedIn && b.models.is_some() && b.rate_limits.is_some()
    });
    let status = backend_status(&store.state(), BackendKind::Codex);
    assert_eq!(status.account.method.as_deref(), Some("chatgpt"));
    assert_eq!(status.account.plan.as_deref(), Some("prolite"));
    let limits = status.rate_limits.unwrap();
    assert_eq!(limits.ordinary_usage_allowed, Some(false));
    assert!(limits.upsell.is_some());
    let models = status.models.unwrap();
    assert_eq!(models.models.len(), 7);
    let astra = models
        .models
        .iter()
        .find(|m| m.id == "gpt-6-astra")
        .unwrap();
    assert_eq!(
        astra
            .efforts
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>(),
        ["low", "medium", "high", "xhigh", "max", "ultra"]
    );

    let mut options = ThreadOptions::new("/workspace");
    options.settings.model = Some("gpt-reserve".into());
    assert_eq!(codex.start_thread(&options).unwrap(), THREAD);

    // ---- turn 1: markdown + math
    codex
        .send(
            THREAD,
            text("Answer in Markdown only…"),
            settings(),
            SendMode::Auto,
        )
        .unwrap();
    store.wait("turn 1 completed", |s| {
        thread(s, BackendKind::Codex, THREAD)
            .and_then(|t| turn(t, T1))
            .is_some_and(|t| t.status == TurnStatus::Completed)
    });
    let state = store.state();
    let turn1 = turn(&t(&state), T1).unwrap().clone();
    let users: Vec<_> = turn1
        .items
        .iter()
        .filter_map(|i| match i {
            Item::UserMessage(u) => Some(u),
            _ => None,
        })
        .collect();
    assert_eq!(users.len(), 1);
    assert!(!users[0].local);
    assert_eq!(turn1.client_message_id.as_deref(), Some("client-msg-1"));
    let reasoning: Vec<_> = turn1
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Reasoning(r) => Some(r),
            _ => None,
        })
        .collect();
    assert_eq!(reasoning.len(), 1);
    assert_eq!(
        reasoning[0].summary,
        ["**Planning concise markdown response**"]
    );
    let answer = final_message(&turn1).unwrap();
    assert_eq!(answer.status, ItemStatus::Completed);
    assert!(answer.text.contains("$e^{i\\pi}+1=0$"));
    assert!(
        answer
            .text
            .contains("$$x=\\frac{-b\\pm\\sqrt{b^2-4ac}}{2a}$$")
    );
    assert_eq!(turn1.usage.as_ref().unwrap().total_tokens, 14781);

    // ---- turn 2: command approval, never answered without the user
    codex
        .send(
            THREAD,
            text("Create a file named hello.txt…"),
            settings(),
            SendMode::Auto,
        )
        .unwrap();
    let key = RequestKey {
        backend: BackendKind::Codex,
        raw_id: opaque(json!(0)),
    };
    store.wait("approval request", |s| {
        request(s, &key).is_some_and(|r| r.status == RequestStatus::Pending)
    });
    let state = store.state();
    let pending = request(&state, &key).unwrap().clone();
    assert_eq!(pending.turn_id.as_deref(), Some(T2));
    assert_eq!(
        pending.item_id.as_deref(),
        Some("exec-ce9fed29-e98f-49e8-8463-345c427750b8")
    );
    assert_eq!(
        pending
            .decisions
            .iter()
            .map(|d| d.id.as_str())
            .collect::<Vec<_>>(),
        ["accept", "acceptWithExecpolicyAmendment", "cancel"]
    );
    assert_eq!(pending.decisions[1].kind, DecisionKind::AllowPersistent);
    assert_eq!(
        pending.decisions[1].detail.as_deref(),
        Some("/bin/zsh -lc printf '%s\\n' 'hi from codex' > hello.txt")
    );
    let RequestKind::CommandApproval { command, .. } = &pending.kind else {
        panic!("command approval")
    };
    assert!(command.as_deref().unwrap().contains("hello.txt"));
    assert_eq!(t(&state).run_state, RunState::WaitingApproval);
    std::thread::sleep(Duration::from_millis(200));
    let process = runtime.process(0);
    assert!(
        process
            .written()
            .iter()
            .all(|f| f.get("id") != Some(&json!(0))),
        "nothing may answer id 0 before the user"
    );
    assert_eq!(
        request(&store.state(), &key).unwrap().status,
        RequestStatus::Pending
    );
    let refused = codex
        .respond(
            &key,
            &RequestResponse::Decide {
                decision_id: "decline".into(),
                message: None,
            },
        )
        .unwrap_err();
    assert_eq!(refused.kind, ErrorKind::DecisionNotOffered);
    codex
        .respond(
            &key,
            &RequestResponse::Decide {
                decision_id: "accept".into(),
                message: None,
            },
        )
        .unwrap();
    let answers: Vec<Value> = process
        .written()
        .into_iter()
        .filter(|f| f.get("id") == Some(&json!(0)))
        .collect();
    assert_eq!(answers, [json!({"id":0,"result":{"decision":"accept"}})]);
    store.wait("turn 2 completed", |s| {
        thread(s, BackendKind::Codex, THREAD)
            .and_then(|t| turn(t, T2))
            .is_some_and(|t| t.status == TurnStatus::Completed)
    });
    let state = store.state();
    let turn2 = turn(&t(&state), T2).unwrap().clone();
    let commands: Vec<_> = turn2
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Command(c) => Some(c),
            _ => None,
        })
        .collect();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].status, ItemStatus::Completed);
    assert_eq!(commands[0].exit_code, Some(0));
    let phases: Vec<_> = turn2
        .items
        .iter()
        .filter_map(|i| match i {
            Item::AgentMessage(m) => Some(m.phase),
            _ => None,
        })
        .collect();
    assert_eq!(phases, [MessagePhase::Commentary, MessagePhase::Final]);
    assert_eq!(
        final_message(&turn2).unwrap().text,
        "Created `hello.txt` containing `hi from codex`."
    );
    let closed = request(&state, &key).unwrap();
    assert_eq!(closed.status, RequestStatus::Answered);
    assert_eq!(closed.answer.as_deref(), Some("accept"));

    // ---- turn 3: interrupt closes the half-streamed message
    codex
        .send(
            THREAD,
            text("Without using tools, write the integers from 1 to 300…"),
            settings(),
            SendMode::Auto,
        )
        .unwrap();
    store.wait("streaming", |s| {
        thread(s, BackendKind::Codex, THREAD)
            .and_then(|t| turn(t, T3))
            .is_some_and(|t| {
                t.items
                    .iter()
                    .any(|i| matches!(i, Item::AgentMessage(m) if m.text.len() > 20))
            })
    });
    codex.interrupt(THREAD, false).unwrap();
    store.wait("turn 3 interrupted", |s| {
        thread(s, BackendKind::Codex, THREAD)
            .and_then(|t| turn(t, T3))
            .is_some_and(|t| t.status == TurnStatus::Interrupted)
    });
    let state = store.state();
    let partial: Vec<_> = turn(&t(&state), T3)
        .unwrap()
        .items
        .iter()
        .filter_map(|i| match i {
            Item::AgentMessage(m) => Some(m.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(partial.len(), 1);
    assert_eq!(partial[0].status, ItemStatus::Incomplete);
    assert!(partial[0].text.starts_with("1 2 3 4 5"));
    assert_eq!(t(&state).run_state, RunState::Idle);

    // ---- thread operations
    codex.load_history(THREAD, None).unwrap();
    let state = store.state();
    assert_eq!(
        t(&state)
            .turns
            .iter()
            .map(|t| t.id.as_str())
            .collect::<Vec<_>>(),
        [T1, T2, T3]
    );
    assert!(
        turn(&t(&state), T2)
            .unwrap()
            .items
            .iter()
            .any(|i| matches!(i, Item::Command(_))),
        "live turn kept over the summary"
    );
    codex.rename(THREAD, "workflow protocol research").unwrap();
    store.wait("renamed", |s| {
        t(s).title.as_deref() == Some("workflow protocol research")
    });
    let mut ephemeral = ThreadOptions::new("/workspace");
    ephemeral.ephemeral = true;
    assert_eq!(codex.fork_thread(THREAD, None, &ephemeral).unwrap(), FORK);
    assert_eq!(
        thread(&store.state(), BackendKind::Codex, FORK)
            .unwrap()
            .forked_from
            .as_deref(),
        Some(THREAD)
    );
    codex.archive(THREAD, true).unwrap();
    store.wait("archived", |s| t(s).archived);
    assert_eq!(t(&store.state()).run_state, RunState::NotLoaded);

    // ---- invariants over everything the client wrote
    let written = process.written();
    for frame in &written {
        if let Some(method) = frame["method"].as_str()
            && params::REVIEWER_METHODS.contains(&method)
        {
            assert_eq!(
                frame["params"]["approvalsReviewer"], "user",
                "{method} must route approvals to the user"
            );
        }
    }
    assert_eq!(
        written
            .iter()
            .filter(|f| f.get("method").is_none() && f["result"].get("decision").is_some())
            .count(),
        1,
        "exactly one answer, the user's"
    );
    assert!(
        written
            .iter()
            .filter(|f| f["method"] == "turn/start")
            .all(|f| f["params"]["clientUserMessageId"]
                .as_str()
                .unwrap()
                .starts_with("client-msg-"))
    );
    assert!(codex.open_request_ids().is_empty());
}

#[test]
fn inherited_auto_reviewer_blocks_sending() {
    let (codex, _runtime, store) = backend(fixture());
    codex.start().unwrap();
    codex
        .start_thread(&ThreadOptions::new("/workspace"))
        .unwrap();
    let state = store.state();
    let notice = t(&state)
        .notices
        .iter()
        .find(|n| n.code.as_deref() == Some("approvalsReviewerNotUser"))
        .cloned()
        .expect("reviewer notice");
    assert_eq!(notice.level, NoticeLevel::Error);
    let refused = codex
        .send(THREAD, text("hi"), None, SendMode::Auto)
        .unwrap_err();
    assert_eq!(refused.kind, ErrorKind::State);
    assert!(
        t(&store.state())
            .turns
            .iter()
            .all(|t| t.status != TurnStatus::Running)
    );
}

#[test]
fn unavailable_environment_fails_without_host_fallback() {
    let (codex, runtime, store) = backend(fixture());
    *runtime.fail.lock().unwrap() = Some("environment is not ready".into());
    let error = codex.start().unwrap_err();
    assert_eq!(error.kind, ErrorKind::BackendUnavailable);
    assert_eq!(runtime.count(), 0);
    let status = backend_status(&store.state(), BackendKind::Codex);
    assert!(matches!(status.process, ProcessState::Failed { .. }));
}
