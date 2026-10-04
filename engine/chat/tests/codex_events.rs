//! Ports of the Kotlin `CodexEventsTest`, `CodexQueueTest` and `CodexReviewerTest`: pure mapper
//! replays over recorded frames, request dispositions, parameter invariants and queue paging.
use serde_json::{Value, json};
use std::sync::Arc;
use workflow_chat::{
    backend::Backend,
    codex::{
        backend::CodexBackend,
        events,
        launch::CodexConfig,
        params,
        requests::{self, Disposition, UserReply},
    },
    model::*,
    reducer,
    testing::*,
    wire,
};

fn replay_inbound(text: &str, limit: usize) -> AgentState {
    let mut state = AgentState::default();
    for e in envelopes(text).into_iter().take(limit) {
        if e.dir != "in" || e.msg.get("id").is_some() {
            continue;
        }
        let Some(method) = e.msg["method"].as_str() else {
            continue;
        };
        let params = e.msg.get("params").cloned().map(opaque);
        for event in events::notification(method, params.as_ref(), &opaque(e.msg.clone())) {
            reducer::reduce(&mut state, &event);
        }
    }
    state
}
fn o(v: Value) -> OpaqueJson {
    opaque(v)
}

#[test]
fn usage_limit_failure_shape() {
    let text = include_str!("fixtures/codex/usage-limit.jsonl");
    let first_failure = envelopes(text)
        .iter()
        .position(|e| e.msg["method"] == "turn/completed")
        .unwrap();
    // systemError survives the failed turn's completion (not reset to idle)...
    let state = replay_inbound(text, first_failure + 1);
    let thread = state.threads.values().find(|t| !t.turns.is_empty()).unwrap();
    assert_eq!(thread.run_state, RunState::Error);
    // ...and the research script archived the thread afterwards.
    let state = replay_inbound(text, usize::MAX);
    let thread = state.threads.values().find(|t| !t.turns.is_empty()).unwrap();
    assert!(thread.archived);
    for turn in &thread.turns {
        assert_eq!(turn.status, TurnStatus::Failed);
        assert_eq!(
            turn.error.as_ref().unwrap().code.as_deref(),
            Some("usageLimitExceeded")
        );
        let notices: Vec<_> = turn
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Notice(n) => Some(n),
                _ => None,
            })
            .collect();
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].notice.code.as_deref(), Some("usageLimitExceeded"));
    }
}

#[test]
fn inbound_only_replay_matches_backend_replay() {
    let state = replay_inbound(include_str!("fixtures/codex/session.jsonl"), usize::MAX);
    let thread = thread(&state, BackendKind::Codex, "01a0ddb6-d670-70f3-9a62-1320d2a3b41a").unwrap();
    assert_eq!(
        thread.turns.iter().map(|t| t.status).collect::<Vec<_>>(),
        [TurnStatus::Completed, TurnStatus::Completed, TurnStatus::Interrupted]
    );
    assert_eq!(thread.title.as_deref(), Some("workflow protocol research"));
    assert!(thread.archived);
    let b = backend_status(&state, BackendKind::Codex);
    assert_eq!(b.mcp_servers["node_repl"].status, "ready");
    assert!(b.unknown.iter().any(|u| u.kind == "remoteControl/status/changed"));
    assert_eq!(b.notices.len(), 1);
    assert_eq!(b.notices[0].code.as_deref(), Some("deprecationNotice"));
}

#[test]
fn command_decisions_follow_available_decisions_exactly() {
    let d = requests::command_decisions(Some(&o(json!(["accept",
        {"acceptWithExecpolicyAmendment":{"execpolicy_amendment":["git","status"]}},
        {"applyNetworkPolicyAmendment":{"network_policy_amendment":{"action":"allow","host":"example.com"}}},
        {"applyNetworkPolicyAmendment":{"network_policy_amendment":{"action":"deny","host":"evil.test"}}},
        "acceptForSession","decline","cancel","futureChoice"]))));
    assert_eq!(
        d.iter().map(|d| d.id.as_str()).collect::<Vec<_>>(),
        [
            "accept",
            "acceptWithExecpolicyAmendment",
            "applyNetworkPolicyAmendment",
            "applyNetworkPolicyAmendment#2",
            "acceptForSession",
            "decline",
            "cancel",
            "futureChoice"
        ]
    );
    use DecisionKind::*;
    assert_eq!(
        d.iter().map(|d| d.kind).collect::<Vec<_>>(),
        [AllowOnce, AllowPersistent, AllowPersistent, Deny, AllowSession, Deny, Abort, Other]
    );
    assert_eq!(d[1].detail.as_deref(), Some("git status"));
    assert_eq!(d[3].detail.as_deref(), Some("deny evil.test"));
    // Absent list: one-shot only, no persistent grant is invented.
    assert_eq!(
        requests::command_decisions(None)
            .iter()
            .map(|d| d.id.as_str())
            .collect::<Vec<_>>(),
        ["accept", "decline", "cancel"]
    );
}

#[test]
fn server_request_dispositions_never_grant_consent() {
    let Disposition::Ask(approval) = requests::pending(
        &o(json!("x1")),
        "item/fileChange/requestApproval",
        Some(&o(json!({"threadId":"t","turnId":"u","itemId":"i","reason":"write","grantRoot":"/workspace","startedAtMs":1}))),
        &o(json!({})),
        5,
    ) else {
        panic!()
    };
    assert_eq!(approval.key.raw_id, o(json!("x1")));
    assert_eq!(
        approval.decisions.iter().map(|d| d.id.as_str()).collect::<Vec<_>>(),
        ["accept", "acceptForSession", "decline", "cancel"]
    );
    assert!(matches!(&approval.kind, RequestKind::FileChangeApproval { grant_root: Some(r), .. } if r == "/workspace"));

    let Disposition::Ask(questions) = requests::pending(
        &o(json!(3)),
        "item/tool/requestUserInput",
        Some(&o(json!({"threadId":"t","turnId":"u","itemId":"i","isBlocking":true,
            "questions":[{"id":"q1","header":"H","question":"Which?","isOther":true,"isSecret":false,"options":[{"label":"A","description":"a"}]}]}))),
        &o(json!({})),
        5,
    ) else {
        panic!()
    };
    let RequestKind::UserInput { questions, .. } = questions.kind else { panic!() };
    assert_eq!(questions[0].id, "q1");
    assert!(questions[0].allow_free_text);

    let Disposition::Fact { result, record } = requests::pending(
        &o(json!(4)),
        "currentTime/read",
        Some(&o(json!({"threadId":"t"}))),
        &o(json!({})),
        1_790_000_000_999,
    ) else {
        panic!()
    };
    assert_eq!(wire::text(&result), r#"{"currentTimeAt":1790000000}"#);
    assert_eq!(record.status, RequestStatus::Answered);

    // The production policy shows unknown methods as cards whose only choice rejects them.
    let Disposition::Ask(unknown) =
        requests::pending(&o(json!(5)), "future/approveEverything", Some(&o(json!({}))), &o(json!({})), 5)
    else {
        panic!()
    };
    assert_eq!(unknown.decisions.len(), 1);
    assert_eq!(unknown.decisions[0].kind, DecisionKind::Deny);
    assert!(matches!(
        requests::answer(
            &unknown,
            &RequestResponse::Decide { decision_id: "reject".into(), message: None }
        )
        .unwrap(),
        UserReply::Error { code: -32601, .. }
    ));

    let Disposition::Fact { result, record } = requests::pending(
        &o(json!(6)),
        "item/tool/call",
        Some(&o(json!({"threadId":"t","turnId":"u","callId":"c","tool":"x","arguments":{}}))),
        &o(json!({})),
        5,
    ) else {
        panic!()
    };
    assert_eq!(result.0["success"], json!(false));
    assert_eq!(record.status, RequestStatus::Rejected);

    let Disposition::Unsupported { code, record, .. } = requests::pending(
        &o(json!(7)),
        "account/chatgptAuthTokens/refresh",
        None,
        &o(json!({})),
        5,
    ) else {
        panic!()
    };
    assert_eq!(code, -32601);
    assert_eq!(record.status, RequestStatus::Rejected);
}

#[test]
fn decision_results_use_server_wire_values() {
    let Disposition::Ask(ask) = requests::pending(
        &o(json!(0)),
        "item/permissions/requestApproval",
        Some(&o(json!({"threadId":"t","turnId":"u","itemId":"i","cwd":"/workspace","permissions":{"network":{"enabled":true}},"startedAtMs":1}))),
        &o(json!({})),
        5,
    ) else {
        panic!()
    };
    let decide = |id: &str| match requests::answer(
        &ask,
        &RequestResponse::Decide { decision_id: id.into(), message: None },
    )
    .unwrap()
    {
        UserReply::Result { value, .. } => value.0,
        other => panic!("{other:?}"),
    };
    assert_eq!(decide("grantSession"), json!({"permissions":{"network":{"enabled":true}},"scope":"session"}));
    assert_eq!(decide("decline"), json!({"permissions":{},"scope":"turn"}));
}

#[test]
fn params_always_route_approvals_to_the_user() {
    for preset in [
        PermissionPreset::Ask,
        PermissionPreset::AutoEdit,
        PermissionPreset::Plan,
        PermissionPreset::DenyUnlisted,
    ] {
        let settings = TurnSettings { permissions: Some(preset), ..Default::default() };
        for body in [
            params::thread_start("/workspace", &settings, PermissionPreset::Ask, false),
            params::thread_resume("t", None, &TurnSettings::default(), preset),
            params::thread_fork("t", Some("u"), None, &TurnSettings::default(), preset, true),
            params::permissions("t", preset),
        ] {
            assert_eq!(body.0["approvalsReviewer"], "user");
        }
    }
    let turn = params::turn_start(
        "t",
        &[
            UserPart::Text { text: "hi".into() },
            UserPart::Image { path: "/workspace/a.png".into(), mime_type: None },
            UserPart::File { path: "/workspace/spec.pdf".into(), mime_type: None },
        ],
        Some(&TurnSettings {
            model: Some("m".into()),
            effort: Some("high".into()),
            permissions: Some(PermissionPreset::AutoEdit),
        }),
        "cm",
    )
    .unwrap()
    .0;
    assert_eq!(turn["approvalsReviewer"], "user");
    assert_eq!(turn["approvalPolicy"], "on-request");
    assert_eq!(turn["sandboxPolicy"]["type"], "workspaceWrite");
    assert_eq!(
        turn["input"],
        json!([{"type":"text","text":"hi","text_elements":[]},{"type":"localImage","path":"/workspace/a.png"},
            {"type":"text","text":"Attached file: /workspace/spec.pdf","text_elements":[]}])
    );
    // The advanced console cannot bypass it either, including both settings methods.
    for method in params::REVIEWER_METHODS {
        let guarded = params::enforce_reviewer(
            method,
            Some(o(json!({"approvalsReviewer":"auto_review","futureField":"preserved"}))),
        )
        .unwrap();
        assert_eq!(guarded.0["approvalsReviewer"], "user");
        assert_eq!(guarded.0["futureField"], "preserved");
    }
    assert!(
        !wire::text(&params::enforce_reviewer("model/list", Some(o(json!({})))).unwrap())
            .contains("approvalsReviewer")
    );
}

#[test]
fn model_catalog_keeps_hidden_models_and_modalities() {
    let result = envelopes(include_str!("fixtures/codex/handshake-hidden.jsonl"))
        .into_iter()
        .find(|e| e.msg["result"].get("data").is_some())
        .unwrap()
        .msg["result"]
        .clone();
    let catalog = events::models(&[o(result)]);
    assert!(catalog.models.iter().any(|m| m.hidden));
    let astra = catalog.models.iter().find(|m| m.id == "gpt-6-astra").unwrap();
    assert_eq!(
        astra.efforts.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
        ["low", "medium", "high", "xhigh", "max", "ultra"]
    );
    assert_eq!(astra.default_effort.as_deref(), Some("medium"));
    assert!(astra.input_modalities.iter().any(|m| m == "image"));
}

#[test]
fn queue_notifications_fetch_pages_and_allow_cancelling_external_submissions() {
    let runtime = FakeRuntime::reacting(|process, frame| {
        let Some(id) = frame.get("id").cloned() else { return };
        let result = match frame["method"].as_str() {
            Some("thread/queue/list") if frame["params"].get("cursor").is_none() => {
                json!({"data":[{"id":"q1","clientUserMessageId":"c1","input":[{"type":"text","text":"first"}]}],"nextCursor":"page2"})
            }
            Some("thread/queue/list") => {
                json!({"data":[{"id":"q2","clientUserMessageId":"c2","input":[{"type":"text","text":"second"}]}]})
            }
            _ => json!({}),
        };
        process.emit(&json!({"id": id, "result": result}));
    });
    let store = EventStore::new();
    let backend = CodexBackend::new(
        runtime.clone(),
        CodexConfig::new("/guest/codex", "/home/work/.codex"),
        store.clone(),
        Arc::new(FixedClock(0)),
        SequenceIds::new("id"),
    );
    backend.start().unwrap();
    let child = runtime.process(0);
    child.emit(&json!({"method":"thread/queue/changed","params":{"threadId":"th"}}));
    let state = store.wait("queue pages", |s| {
        thread(s, BackendKind::Codex, "th").is_some_and(|t| {
            t.turns.iter().filter(|t| t.status == TurnStatus::Queued).count() == 2
        })
    });
    assert_eq!(
        thread(&state, BackendKind::Codex, "th")
            .unwrap()
            .turns
            .iter()
            .filter(|t| t.status == TurnStatus::Queued)
            .map(|t| t.client_message_id.clone().unwrap_or(t.id.clone()))
            .collect::<Vec<_>>(),
        ["c1", "c2"]
    );
    backend.cancel_queued("th", "c2").unwrap();
    let deletion = child
        .written()
        .into_iter()
        .find(|f| f["method"] == "thread/queue/delete")
        .unwrap();
    assert_eq!(deletion["params"]["queuedSubmissionId"], "q2");
    assert_eq!(
        turn(thread(&store.state(), BackendKind::Codex, "th").unwrap(), "c2")
            .unwrap()
            .status,
        TurnStatus::Cancelled
    );
    backend.stop();
}
