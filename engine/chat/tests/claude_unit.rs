//! Port of the Kotlin `ClaudeUnitTest`: request cards, answers, attachments, launch contract,
//! transcript hydration and the initialize account and model projection.
use serde_json::{Value, json};
use std::collections::BTreeMap;
use workflow_chat::{
    claude::{
        launch::{self, LaunchConfig, Session},
        mapper, requests, transcript,
    },
    codex::requests::UserReply,
    error::ErrorKind,
    model::*,
    service::launch_env::Secret,
    testing::*,
};

fn o(v: Value) -> OpaqueJson {
    opaque(v)
}
fn can_use_tool(request: Value) -> PendingRequest {
    let raw = json!({"type":"control_request","request_id":"r1","request":request});
    requests::pending(
        "r1",
        "can_use_tool",
        &o(request),
        &o(raw),
        Some("s"),
        Some("t"),
        1,
    )
}
fn decide(id: &str, message: Option<&str>) -> RequestResponse {
    RequestResponse::Decide {
        decision_id: id.into(),
        message: message.map(str::to_owned),
    }
}
fn payload(request: &PendingRequest, response: &RequestResponse) -> Value {
    match requests::answer(request, response).unwrap().reply {
        UserReply::Result { value, .. } => value.0,
        other => panic!("{other:?}"),
    }
}
fn ids(request: &PendingRequest) -> Vec<&str> {
    request.decisions.iter().map(|d| d.id.as_str()).collect()
}

#[test]
fn tool_approval_offers_only_what_the_cli_allows() {
    let req = can_use_tool(
        json!({"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"rm x"},"tool_use_id":"tu",
        "decision_reason":"\u{1b}[31mdangerous\u{1b}[0m","default_to_no":true,"suppress_always_allow_rule":true,
        "permission_suggestions":[{"type":"addRules","rules":[{"toolName":"Bash","ruleContent":"rm x"}],"behavior":"allow","destination":"localSettings"}]}),
    );
    assert_eq!(ids(&req), ["allow", "deny", "denyAndStop"]);
    let RequestKind::ToolApproval {
        reason,
        default_to_no,
        ..
    } = &req.kind
    else {
        panic!()
    };
    assert_eq!(reason.as_deref(), Some("dangerous"));
    assert!(default_to_no);
    let interactive = can_use_tool(
        json!({"subtype":"can_use_tool","tool_name":"Bash","input":{},"tool_use_id":"tu","requires_user_interaction":true}),
    );
    assert_eq!(
        ids(&interactive),
        ["deny", "denyAndStop"],
        "no one-tap approve"
    );
}

#[test]
fn answers_are_built_from_the_offered_decision_only() {
    let req = can_use_tool(
        json!({"subtype":"can_use_tool","tool_name":"Edit","input":{"file_path":"/workspace/a"},"tool_use_id":"tu",
        "permission_suggestions":[{"type":"setMode","mode":"acceptEdits","destination":"session"}]}),
    );
    assert_eq!(
        payload(&req, &decide("allowAlways:0", None)),
        json!({"behavior":"allow","updatedInput":{"file_path":"/workspace/a"},
            "updatedPermissions":[{"type":"setMode","mode":"acceptEdits","destination":"session"}]})
    );
    assert_eq!(req.decisions[1].kind, DecisionKind::AllowSession);
    assert_eq!(
        payload(&req, &decide("denyAndStop", Some("no"))),
        json!({"behavior":"deny","message":"no","interrupt":true})
    );
    assert_eq!(
        requests::answer(&req, &decide("allowAlways:5", None))
            .unwrap_err()
            .kind,
        ErrorKind::DecisionNotOffered
    );
    assert!(
        requests::answer(
            &req,
            &RequestResponse::RawResult {
                result: o(json!({"behavior":"allow"}))
            }
        )
        .is_err()
    );
}

#[test]
fn ask_user_question_and_plan_approval() {
    let ask = can_use_tool(
        json!({"subtype":"can_use_tool","tool_name":"AskUserQuestion","tool_use_id":"tu","input":{"questions":[
        {"question":"Which DB?","header":"DB","multiSelect":false,"options":[{"label":"Postgres","description":"p"},{"label":"SQLite","description":"s"}]},
        {"question":"Features?","header":"F","multiSelect":true,"options":[{"label":"A","description":""},{"label":"B","description":""}]}]}}),
    );
    let RequestKind::UserInput { questions, .. } = &ask.kind else {
        panic!()
    };
    assert_eq!(
        questions.iter().map(|q| q.id.as_str()).collect::<Vec<_>>(),
        ["Which DB?", "Features?"]
    );
    assert!(questions[1].multi_select);
    let answer = payload(
        &ask,
        &RequestResponse::Answer {
            answers: BTreeMap::from([
                ("Which DB?".to_string(), vec!["SQLite".to_string()]),
                (
                    "Features?".to_string(),
                    vec!["A".to_string(), "B".to_string()],
                ),
            ]),
        },
    );
    assert_eq!(answer["behavior"], "allow");
    assert_eq!(
        answer["updatedInput"]["answers"],
        json!({"Which DB?":"SQLite","Features?":"A, B"})
    );
    assert_eq!(
        answer["updatedInput"]["questions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let plan = can_use_tool(
        json!({"subtype":"can_use_tool","tool_name":"ExitPlanMode","tool_use_id":"tu","input":{"plan":"1. do it"}}),
    );
    assert!(matches!(&plan.kind, RequestKind::PlanApproval { plan } if plan == "1. do it"));
    assert_eq!(ids(&plan), ["allow", "deny"]);
}

#[test]
fn elicitation_and_undeclared_dialogs() {
    let elicit = requests::pending(
        "e1",
        "elicitation",
        &o(
            json!({"subtype":"elicitation","mcp_server_name":"gh","message":"Token?","mode":"form","requested_schema":{"type":"object"}}),
        ),
        &o(json!({})),
        Some("s"),
        None,
        1,
    );
    assert!(matches!(&elicit.kind, RequestKind::Elicitation { server: Some(s), .. } if s == "gh"));
    assert_eq!(
        payload(
            &elicit,
            &RequestResponse::Elicit {
                action: "accept".into(),
                content: Some(BTreeMap::from([("token".to_string(), o(json!("x")))])),
            }
        ),
        json!({"action":"accept","content":{"token":"x"}})
    );
    let dialog = requests::pending(
        "d1",
        "request_user_dialog",
        &o(
            json!({"subtype":"request_user_dialog","dialog_kind":"refusal_fallback_prompt","payload":{}}),
        ),
        &o(json!({})),
        Some("s"),
        None,
        1,
    );
    assert!(dialog.decisions.is_empty());
    assert_eq!(
        requests::answer(&dialog, &decide("x", None))
            .unwrap_err()
            .kind,
        ErrorKind::State
    );
}

#[test]
fn attachments_map_to_content_blocks() {
    let reader = |path: &str, _: usize| Ok(path.as_bytes().to_vec());
    let content = requests::content(
        &[
            UserPart::Text {
                text: "look".into(),
            },
            UserPart::Image {
                path: "/workspace/a.jpg".into(),
                mime_type: None,
            },
            UserPart::File {
                path: "/workspace/doc.pdf".into(),
                mime_type: None,
            },
            UserPart::File {
                path: "/workspace/src/Main.kt".into(),
                mime_type: None,
            },
        ],
        &reader,
        1024,
    )
    .unwrap()
    .0;
    assert_eq!(content[0]["text"], "look\n\n@/workspace/src/Main.kt");
    assert_eq!(content[1]["source"]["media_type"], "image/jpeg");
    assert_eq!(content[2]["type"], "document");
    assert_eq!(content[2]["source"]["media_type"], "application/pdf");
    assert_eq!(content.as_array().unwrap().len(), 3);
    let only = requests::content(
        &[UserPart::File {
            path: "/workspace/notes.txt".into(),
            mime_type: None,
        }],
        &reader,
        1024,
    )
    .unwrap()
    .0;
    assert_eq!(
        only,
        json!([{"type":"text","text":"@/workspace/notes.txt"}])
    );
}

#[test]
fn launch_contract_never_weakens_permissions() {
    let mut config = LaunchConfig {
        executable: "claude".into(),
        config_dir: "/home/work/.claude-app".into(),
        tmp_dir: "/tmp".into(),
        base_env: [
            ("ANTHROPIC_API_KEY", "leak"),
            ("CLAUDECODE", "1"),
            ("LANG", "C.UTF-8"),
        ]
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .into(),
        credentials: BTreeMap::from([(
            "CLAUDE_CODE_OAUTH_TOKEN".to_string(),
            Secret("app-token".into()),
        )]),
        default_permissions: PermissionPreset::Ask,
        extra_args: vec![],
    };
    let argv = launch::argv(
        &config,
        &Session::Resume("s1"),
        Some("sonnet"),
        Some("high"),
        Some(PermissionPreset::AutoEdit),
    )
    .unwrap();
    let at = argv.iter().position(|a| a == "--permission-mode").unwrap();
    assert_eq!(argv[at + 1], "acceptEdits");
    for arg in [
        "--resume",
        "s1",
        "--model",
        "sonnet",
        "--effort",
        "high",
        "--replay-user-messages",
    ] {
        assert!(argv.iter().any(|a| a == arg), "{arg}");
    }
    let env = launch::env(&config);
    assert!(!env.contains_key("ANTHROPIC_API_KEY"));
    assert!(!env.contains_key("CLAUDECODE"));
    assert_eq!(env["CLAUDE_CODE_OAUTH_TOKEN"], "app-token");
    assert_eq!(env["DISABLE_AUTOUPDATER"], "1");
    assert_eq!(
        format!("{:?}", config.credentials),
        r#"{"CLAUDE_CODE_OAUTH_TOKEN": <redacted>}"#
    );
    for bad in [
        "--dangerously-skip-permissions",
        "--dangerously-skip-permissions=true",
        "--permission-mode=auto",
        "--permission-mode",
        "--permission-prompt-tool=custom",
        "bypassPermissions",
        "auto",
    ] {
        config.extra_args = vec![bad.into()];
        assert!(
            launch::argv(&config, &Session::New("s"), None, None, None).is_err(),
            "{bad}"
        );
    }
    assert_eq!(
        launch::transcript_path("/home/work/.claude-app", "/workspace", "s1").unwrap(),
        "/home/work/.claude-app/projects/-workspace/s1.jsonl"
    );
}

#[test]
fn transcript_hydration() {
    let lines: Vec<String> = include_str!("fixtures/claude/transcript-1577088f.jsonl")
        .lines()
        .map(str::to_owned)
        .collect();
    let history = transcript::history("1577088f-7864-4263-aee6-9afe1b2f4a73", &lines);
    assert_eq!(history.title.as_deref(), Some("workflow protocol research"));
    assert_eq!(
        history
            .turns
            .iter()
            .map(|t| t.id.as_str())
            .collect::<Vec<_>>(),
        [
            "47397f51-ee05-4bf1-912e-296866b9da74",
            "01b893ae-40c7-451c-b444-9f3f82ab80f2",
            "98c79d86-1546-45df-aeaa-07ac81e3f40a",
            "5eb19f10-e16a-4a65-98f0-8995aef2fac0"
        ]
    );
    assert_eq!(
        history.turns.iter().map(|t| t.status).collect::<Vec<_>>(),
        [
            TurnStatus::Completed,
            TurnStatus::Completed,
            TurnStatus::Completed,
            TurnStatus::Interrupted
        ]
    );
    let [t1, t2, t3, t4] = &history.turns[..] else {
        panic!()
    };
    let users: Vec<_> = t1
        .items
        .iter()
        .filter_map(|i| match i {
            Item::UserMessage(u) => Some(u),
            _ => None,
        })
        .collect();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].parts.len(), 2);
    assert!(matches!(users[0].parts[1], UserPart::InlineData { .. }));
    assert_eq!(
        t1.items
            .iter()
            .filter(|i| matches!(i, Item::Reasoning(_)))
            .count(),
        1
    );
    assert_eq!(final_message(t1).unwrap().phase, MessagePhase::Final);
    let writes: Vec<_> = t2
        .items
        .iter()
        .filter_map(|i| match i {
            Item::FileChange(f) => Some(f),
            _ => None,
        })
        .collect();
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0].changes[0].kind, FileChangeKind::Add);
    assert_eq!(writes[0].status, ItemStatus::Completed);
    assert!(
        t3.items
            .iter()
            .any(|i| matches!(i, Item::Command(c) if c.status == ItemStatus::Declined))
    );
    assert!(
        !t3.items
            .iter()
            .any(|i| matches!(i, Item::UserMessage(u) if u.text().contains("command-name"))),
        "the local /model command is between turns, not a user message"
    );
    assert!(
        t4.items
            .iter()
            .any(|i| matches!(i, Item::AgentMessage(m) if m.status == ItemStatus::Incomplete))
    );
    assert!(
        t4.items
            .iter()
            .any(|i| matches!(i, Item::Marker(m) if m.kind == MarkerKind::Interrupted))
    );
    // Fork anchor for "branch after turn 2".
    assert!(
        transcript::last_uuid(&lines, &t2.id)
            .unwrap()
            .starts_with("5366cf09")
    );
}

#[test]
fn models_and_account_from_initialize() {
    let init = envelopes(include_str!("fixtures/claude/noauth.jsonl"))
        .into_iter()
        .find(|e| e.msg["response"]["response"].get("models").is_some())
        .unwrap()
        .msg["response"]["response"]
        .clone();
    let catalog = mapper::models(Some(&o(init["models"].clone())));
    assert!(catalog.models.iter().all(|m| {
        m.input_modalities.iter().any(|x| x == "image")
            && m.input_modalities.iter().any(|x| x == "pdf")
    }));
    assert_eq!(
        catalog
            .models
            .iter()
            .find(|m| m.id == "opus")
            .unwrap()
            .default_effort
            .as_deref(),
        Some("medium")
    );
    assert!(
        catalog
            .models
            .iter()
            .find(|m| m.id == "haiku")
            .unwrap()
            .efforts
            .is_empty()
    );
    assert_eq!(mapper::account(&o(init)).state, LoginState::LoggedOut);
}
