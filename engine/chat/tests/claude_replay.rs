//! Port of the Kotlin `ClaudeBackendReplayTest`: drives the Rust Claude backend against recorded
//! Claude Code 2.1.283 frames (markdown, math and image; hook callback; can_use_tool allow and
//! deny; model, effort and mode switches; rename; interrupt; fork) and the not-logged-in run.
use base64::Engine;
use serde_json::{Value, json};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use workflow_chat::{
    backend::{Backend, ThreadOptions},
    claude::{
        backend::{ClaudeBackend, ClaudeConfig},
        launch::LaunchConfig,
        requests::{self, ClaudeHook},
    },
    error::ChatError,
    model::*,
    testing::*,
};

const SESSION: &str = "1577088f-7864-4263-aee6-9afe1b2f4a73";
const FORK: &str = "6810f1e3-226d-4f67-8c39-1a4e00ebab88";
const U1: &str = "47397f51-ee05-4bf1-912e-296866b9da74";
const U2: &str = "01b893ae-40c7-451c-b444-9f3f82ab80f2";
const U3: &str = "98c79d86-1546-45df-aeaa-07ac81e3f40a";
const U4: &str = "5eb19f10-e16a-4a65-98f0-8995aef2fac0";
const U5: &str = "284b1a4e-b70b-4a8d-b7e2-ca43830b8844";
const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAIAAACQkWg2AAAAF0lEQVR4nGP4z8BAEiJN9aiGUQ1DSgMAkPn/Afnh+ngAAAAASUVORK5CYII=";

fn fixture(name: &str) -> &'static str {
    match name {
        "session" => include_str!("fixtures/claude/session.jsonl"),
        "fork" => include_str!("fixtures/claude/fork.jsonl"),
        "noauth" => include_str!("fixtures/claude/noauth.jsonl"),
        _ => unreachable!(),
    }
}
fn config(hooks: Vec<ClaudeHook>) -> ClaudeConfig {
    ClaudeConfig {
        launch: LaunchConfig {
            executable: "claude".into(),
            config_dir: "/home/work/.claude-app".into(),
            tmp_dir: "/tmp".into(),
            base_env: [
                ("PATH", "/usr/bin"),
                ("ANTHROPIC_API_KEY", "host-secret"),
                ("CLAUDE_CODE_ENTRYPOINT", "desktop"),
            ]
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .into(),
            credentials: Default::default(),
            default_permissions: PermissionPreset::Ask,
            extra_args: vec![],
        },
        cwd: "/workspace".into(),
        hooks,
        max_inline_bytes: 20 * 1024 * 1024,
    }
}
struct Harness {
    claude: ClaudeBackend,
    runtime: Arc<FakeRuntime>,
    store: Arc<EventStore>,
    hook_calls: Arc<Mutex<Vec<Value>>>,
}
fn backend(scripts: &[&str], ids: &[&str]) -> Harness {
    let servers = scripts
        .iter()
        .map(|s| ScriptedServer::new(envelopes(fixture(s)), Protocol::ClaudeControl))
        .collect();
    let runtime = ScriptedServer::runtime(servers);
    let store = EventStore::new();
    let hook_calls = Arc::new(Mutex::new(Vec::new()));
    let calls = hook_calls.clone();
    let hooks = vec![ClaudeHook {
        event: "PreToolUse".into(),
        matcher: Some("Write|Bash".into()),
        callback_id: "hook_pre_1".into(),
        handler: Box::new(move |input| {
            calls.lock().unwrap().push(input.0.clone());
            Ok(opaque(json!({"continue": true})))
        }),
    }];
    let claude = ClaudeBackend::new(
        runtime.clone(),
        config(hooks),
        store.clone(),
        Arc::new(FixedClock(1_790_427_049_000)),
        SequenceIds::listed(ids),
        Arc::new(|path: &str, _max: usize| {
            assert_eq!(path, "/workspace/red.png");
            Ok(base64::engine::general_purpose::STANDARD.decode(PNG).unwrap())
        }),
        None,
    );
    Harness {
        claude,
        runtime,
        store,
        hook_calls,
    }
}
fn t(state: &AgentState, id: &str) -> ThreadState {
    thread(state, BackendKind::Claude, id).cloned().expect("thread")
}
fn turn_status(state: &AgentState, thread_id: &str, turn_id: &str) -> Option<TurnStatus> {
    thread(state, BackendKind::Claude, thread_id)
        .and_then(|t| turn(t, turn_id))
        .map(|t| t.status)
}
fn text(text: &str) -> Vec<UserPart> {
    vec![UserPart::Text { text: text.into() }]
}
fn decide(id: &str, message: Option<&str>) -> RequestResponse {
    RequestResponse::Decide {
        decision_id: id.into(),
        message: message.map(str::to_owned),
    }
}
fn ids(decisions: &[Decision]) -> Vec<&str> {
    decisions.iter().map(|d| d.id.as_str()).collect()
}

#[test]
fn full_session_replay() {
    let h = backend(&["session", "fork"], &[SESSION, U1, U2, U3, U4, FORK, U5]);
    h.claude.start().unwrap();
    let spec = h.runtime.process(0).spec.clone();
    let argv = spec.argv.join(" ");
    assert!(argv.starts_with(
        "claude -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages"
    ));
    assert!(argv.contains("--permission-prompt-tool stdio --permission-mode default"));
    assert!(argv.ends_with(&format!("--session-id {SESSION}")));
    assert!(!argv.contains("dangerously") && !argv.contains("bypassPermissions"));
    assert_eq!(spec.env["CLAUDE_CONFIG_DIR"], "/home/work/.claude-app");
    assert!(!spec.env.contains_key("ANTHROPIC_API_KEY"), "host credentials are not forwarded");
    assert!(!spec.env.contains_key("CLAUDE_CODE_ENTRYPOINT"));

    let status = backend_status(&h.store.state(), BackendKind::Claude);
    // The research run used an API key.
    assert_eq!(status.account.state, LoginState::LoggedIn);
    let models = status.models.unwrap();
    let default = models.models.iter().find(|m| m.is_default).unwrap_or(&models.models[0]);
    assert_eq!(default.id, "default");
    let effort = |id: &str| {
        models
            .models
            .iter()
            .find(|m| m.id == id)
            .unwrap()
            .efforts
            .iter()
            .map(|e| e.id.clone())
            .collect::<Vec<_>>()
    };
    assert!(effort("haiku").is_empty());
    assert_eq!(effort("sonnet"), ["low", "medium", "high", "xhigh", "max"]);

    assert_eq!(h.claude.start_thread(&ThreadOptions::new("/workspace")).unwrap(), SESSION);
    h.claude.raw_request("mcp_status", None, None).unwrap();
    h.claude.refresh_models().unwrap();
    h.claude.raw_request("get_settings", None, None).unwrap();
    h.claude
        .raw_request("get_context_usage", Some(opaque(json!({"detail":"summary"}))), None)
        .unwrap();

    // ---- turn 1: markdown + math + image attachment
    h.claude
        .send(
            SESSION,
            vec![
                UserPart::Text {
                    text: "MATH: answer in markdown with a formula; what colour is the image?".into(),
                },
                UserPart::Image {
                    path: "/workspace/red.png".into(),
                    mime_type: Some("image/png".into()),
                },
            ],
            None,
            SendMode::Auto,
        )
        .unwrap();
    h.store.wait("turn 1", |s| turn_status(s, SESSION, U1) == Some(TurnStatus::Completed));
    let p0 = h.runtime.process(0);
    let written = p0.written().into_iter().find(|f| f["type"] == "user").unwrap();
    assert_eq!(written["message"]["content"][1]["source"]["data"], PNG);
    let state = h.store.state();
    let turn1 = turn(&t(&state, SESSION), U1).unwrap().clone();
    let users: Vec<_> = turn1
        .items
        .iter()
        .filter_map(|i| match i {
            Item::UserMessage(u) => Some(u),
            _ => None,
        })
        .collect();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].id, U1);
    assert!(!users[0].local);
    // The composed parts are kept over the base64 echo.
    assert_eq!(
        users[0].parts[1],
        UserPart::Image {
            path: "/workspace/red.png".into(),
            mime_type: Some("image/png".into())
        }
    );
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
        reasoning[0].content,
        ["The user wants markdown with a formula; the image is a red square."]
    );
    assert!(
        final_message(&turn1)
            .unwrap()
            .text
            .contains("$$x=\\frac{-b\\pm\\sqrt{b^2-4ac}}{2a}$$")
    );
    assert!((turn1.usage.as_ref().unwrap().cost_usd.unwrap() - 0.00128).abs() < 1e-9);
    assert_eq!(t(&state, SESSION).settings.model.as_deref(), Some("claude-opus-5-5[1m]"));

    // ---- turn 2: hook callback (app-registered) + permission allowed by the user
    h.claude
        .send(SESSION, text("WRITE: create hello.txt"), None, SendMode::Auto)
        .unwrap();
    let allow = requests::key("c071187f-d192-476b-942a-e990906bfdd6");
    h.store.wait("write permission", |s| {
        request(s, &allow).is_some_and(|r| r.status == RequestStatus::Pending)
    });
    assert_eq!(
        h.hook_calls.lock().unwrap().iter().map(|c| c["hook_event_name"].clone()).collect::<Vec<_>>(),
        [json!("PreToolUse")]
    );
    let state = h.store.state();
    let write = request(&state, &allow).unwrap().clone();
    assert_eq!(ids(&write.decisions), ["allow", "allowAlways:0", "deny", "denyAndStop"]);
    assert_eq!(write.decisions[1].kind, DecisionKind::AllowSession);
    assert_eq!(write.decisions[1].detail.as_deref(), Some("mode acceptEdits → session"));
    assert!(matches!(&write.kind, RequestKind::ToolApproval { tool, .. } if tool == "Write"));
    assert_eq!(t(&state, SESSION).run_state, RunState::WaitingApproval);
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        p0.written()
            .iter()
            .all(|f| f["response"]["request_id"] != "c071187f-d192-476b-942a-e990906bfdd6"),
        "no permission answer before the user"
    );
    h.claude.respond(&allow, &decide("allow", None)).unwrap();
    h.store.wait("turn 2", |s| turn_status(s, SESSION, U2) == Some(TurnStatus::Completed));
    let state = h.store.state();
    let turn2 = turn(&t(&state, SESSION), U2).unwrap().clone();
    let files: Vec<_> = turn2
        .items
        .iter()
        .filter_map(|i| match i {
            Item::FileChange(f) => Some(f),
            _ => None,
        })
        .collect();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].status, ItemStatus::Completed);
    assert_eq!(files[0].changes[0].kind, FileChangeKind::Add);
    assert_eq!(files[0].changes[0].path, "/workspace/hello.txt");
    assert_eq!(
        turn2
            .items
            .iter()
            .filter_map(|i| match i {
                Item::AgentMessage(m) => Some(m.phase),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [MessagePhase::Commentary, MessagePhase::Final]
    );
    assert!(
        turn2
            .items
            .iter()
            .any(|i| matches!(i, Item::Marker(m) if m.kind == MarkerKind::Hook))
    );
    assert_eq!(request(&state, &allow).unwrap().status, RequestStatus::Answered);

    // ---- turn 3: deny
    h.claude
        .send(SESSION, text("BASH: write bash.txt"), None, SendMode::Auto)
        .unwrap();
    let deny = requests::key("3d376a32-71e6-44e8-bc27-a708d72b6482");
    h.store.wait("bash permission", |s| {
        request(s, &deny).is_some_and(|r| r.status == RequestStatus::Pending)
    });
    let bash = request(&h.store.state(), &deny).unwrap().clone();
    assert_eq!(
        ids(&bash.decisions),
        ["allow", "allowAlways:0", "allowAlways:1", "deny", "denyAndStop"]
    );
    // A localSettings rule is written to disk; a session rule is not.
    assert_eq!(bash.decisions[1].kind, DecisionKind::AllowPersistent);
    assert_eq!(bash.decisions[2].kind, DecisionKind::AllowSession);
    assert!(
        matches!(&bash.kind, RequestKind::ToolApproval { blocked_path: Some(p), .. } if p == "/workspace/bash.txt")
    );
    h.claude
        .respond(&deny, &decide("deny", Some("Declined by research client")))
        .unwrap();
    let denial = p0
        .written()
        .into_iter()
        .find(|f| f["response"]["request_id"] == "3d376a32-71e6-44e8-bc27-a708d72b6482")
        .unwrap();
    assert_eq!(denial["response"]["response"]["behavior"], "deny");
    h.store.wait("turn 3", |s| turn_status(s, SESSION, U3) == Some(TurnStatus::Completed));
    let state = h.store.state();
    let turn3 = turn(&t(&state, SESSION), U3).unwrap().clone();
    assert!(
        turn3
            .items
            .iter()
            .any(|i| matches!(i, Item::Command(c) if c.status == ItemStatus::Declined))
    );

    // ---- mid-session controls, then turn 4 with a model and effort switch, interrupted
    h.claude
        .raw_request("set_max_thinking_tokens", Some(opaque(json!({"max_thinking_tokens":4096}))), None)
        .unwrap();
    h.claude.set_permissions(SESSION, PermissionPreset::Plan).unwrap();
    h.claude.set_permissions(SESSION, PermissionPreset::Ask).unwrap();
    h.claude.rename(SESSION, "workflow protocol research").unwrap();
    h.claude
        .send(
            SESSION,
            text("SLOW: count to 300"),
            Some(TurnSettings {
                model: Some("sonnet".into()),
                effort: Some("high".into()),
                permissions: None,
            }),
            SendMode::Auto,
        )
        .unwrap();
    h.store.wait("turn 4 streaming", |s| {
        thread(s, BackendKind::Claude, SESSION)
            .and_then(|t| turn(t, U4))
            .is_some_and(|t| t.items.iter().any(|i| matches!(i, Item::AgentMessage(m) if m.text.len() > 10)))
    });
    h.claude.interrupt(SESSION, false).unwrap();
    h.store
        .wait("turn 4 interrupted", |s| turn_status(s, SESSION, U4) == Some(TurnStatus::Interrupted));
    let state = h.store.state();
    let turn4 = turn(&t(&state, SESSION), U4).unwrap().clone();
    assert!(
        turn4
            .items
            .iter()
            .any(|i| matches!(i, Item::AgentMessage(m) if m.status == ItemStatus::Incomplete))
    );
    assert!(
        turn4
            .items
            .iter()
            .any(|i| matches!(i, Item::Marker(m) if m.kind == MarkerKind::Interrupted))
    );
    let thread_state = t(&state, SESSION);
    assert_eq!(thread_state.title.as_deref(), Some("workflow protocol research"));
    // Resolved by the CLI in system/init.
    assert_eq!(thread_state.settings.model.as_deref(), Some("claude-sonnet-5"));
    assert_eq!(thread_state.settings.effort.as_deref(), Some("high"));
    assert_eq!(thread_state.settings.approval_policy.as_deref(), Some("default"));
    let subtypes: Vec<String> = p0
        .written()
        .iter()
        .filter(|f| f["type"] == "control_request")
        .filter_map(|f| f["request"]["subtype"].as_str().map(str::to_owned))
        .collect();
    for subtype in ["set_model", "apply_flag_settings", "set_permission_mode", "rename_session", "interrupt"] {
        assert!(subtypes.iter().any(|s| s == subtype), "{subtype}");
    }

    // ---- fork into a new session ID we chose
    assert_eq!(
        h.claude.fork_thread(SESSION, None, &ThreadOptions::new("/workspace")).unwrap(),
        FORK
    );
    assert!(
        h.runtime
            .process(1)
            .spec
            .argv
            .join(" ")
            .contains(&format!("--resume {SESSION} --fork-session --session-id {FORK}"))
    );
    h.claude
        .send(FORK, text("MATH again after fork"), None, SendMode::Auto)
        .unwrap();
    h.store.wait("fork turn", |s| turn_status(s, FORK, U5) == Some(TurnStatus::Completed));
    let state = h.store.state();
    assert_eq!(t(&state, FORK).forked_from.as_deref(), Some(SESSION));
    assert_eq!(t(&state, FORK).settings.model.as_deref(), Some("claude-sonnet-5"));

    // Exactly the two answers the user gave, plus the app hook responses.
    let answers: Vec<Value> = p0
        .written()
        .into_iter()
        .filter(|f| f["type"] == "control_response")
        .collect();
    assert_eq!(answers.len(), 2 + 2);
    assert_eq!(
        answers
            .iter()
            .filter(|f| f["response"]["response"].get("behavior").is_some())
            .count(),
        2
    );
    assert!(h.claude.open_request_ids().is_empty());
}

#[test]
fn not_logged_in() {
    let noauth = "c595e3cd-450f-43ab-99a8-3df521adf5d1";
    let turn_id = "ffd7f76f-d373-4734-a221-69cb322c89a8";
    let h = backend(&["noauth"], &[noauth, turn_id]);
    h.claude.start().unwrap();
    assert_eq!(
        backend_status(&h.store.state(), BackendKind::Claude).account.state,
        LoginState::LoggedOut
    );
    h.claude.start_thread(&ThreadOptions::new("/workspace")).unwrap();
    h.claude
        .send(
            noauth,
            vec![
                UserPart::Text {
                    text: "MATH: answer in markdown with a formula; what colour is the image?".into(),
                },
                UserPart::Image {
                    path: "/workspace/red.png".into(),
                    mime_type: Some("image/png".into()),
                },
            ],
            None,
            SendMode::Auto,
        )
        .unwrap();
    h.store.wait("failed turn", |s| turn_status(s, noauth, turn_id) == Some(TurnStatus::Failed));
    let state = h.store.state();
    let failed = turn(&t(&state, noauth), turn_id).unwrap().clone();
    assert_eq!(failed.error.as_ref().unwrap().code.as_deref(), Some("api_error"));
    assert_eq!(failed.error.as_ref().unwrap().message, "Not logged in · Please run /login");
    let notices: Vec<_> = failed
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Notice(n) => Some(n.notice.code.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(notices, [Some("authentication_failed".to_string())]);
    assert!(!failed.items.iter().any(|i| matches!(i, Item::AgentMessage(_))));
}

#[test]
fn process_exit_fails_turns_and_expires_requests() {
    let h = backend(&["session"], &[SESSION, U1, U2]);
    h.claude.start().unwrap();
    h.claude.start_thread(&ThreadOptions::new("/workspace")).unwrap();
    // The scripted server plays recorded turns in order: turn 1, then the write turn's request.
    h.claude.send(SESSION, text("first"), None, SendMode::Auto).unwrap();
    h.store.wait("turn 1", |s| turn_status(s, SESSION, U1) == Some(TurnStatus::Completed));
    h.claude.send(SESSION, text("WRITE"), None, SendMode::Auto).unwrap();
    let allow = requests::key("c071187f-d192-476b-942a-e990906bfdd6");
    h.store.wait("write permission", |s| {
        request(s, &allow).is_some_and(|r| r.status == RequestStatus::Pending)
    });
    h.runtime.process(0).exit(1);
    let state = h.store.wait("expired", |s| {
        t(s, SESSION).run_state == RunState::NotLoaded
            && request(s, &allow).is_some_and(|r| r.status == RequestStatus::Expired)
    });
    assert_eq!(turn_status(&state, SESSION, U2), Some(TurnStatus::Failed));
    assert!(
        t(&state, SESSION)
            .notices
            .iter()
            .any(|n| n.code.as_deref() == Some("processExited"))
    );
    let error: ChatError = h.claude.respond(&allow, &decide("allow", None)).unwrap_err();
    assert_eq!(error.kind, workflow_chat::error::ErrorKind::RequestExpired);
    assert!(
        h.runtime
            .process(0)
            .written()
            .iter()
            .all(|f| f["type"] != "control_response" || f["response"]["request_id"] != "c071187f-d192-476b-942a-e990906bfdd6"),
        "an expired card is never answered"
    );
}
