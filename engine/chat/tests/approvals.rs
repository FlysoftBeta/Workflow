use serde::Deserialize;
use workflow_chat::{
    claude::{self, launch::Session},
    codex::{
        params,
        requests::{self, Disposition, UserReply},
    },
    error::ErrorKind,
    model::*,
    transport::raw::{RawJson, RequestId},
};
use workflow_environment::json::strict_json;
#[derive(Deserialize)]
struct Case {
    case: String,
    backend: String,
    id: RawJson,
    method: String,
    params: RawJson,
    raw: RawJson,
    disposition: String,
    expected: PendingRequest,
    result: Option<RawJson>,
    code: Option<i64>,
    message: Option<String>,
}
#[test]
fn kotlin_approval_card_goldens() {
    let cases: Vec<Case> = strict_json(include_bytes!("golden/approvals.json")).unwrap();
    assert_eq!(cases.len(), 18);
    for case in cases {
        if case.backend == "codex" {
            match requests::pending(
                RequestId(case.id),
                &case.method,
                Some(&case.params),
                &case.raw,
                17000,
            )
            .unwrap()
            {
                Disposition::Ask(actual) => {
                    assert_eq!(case.disposition, "ask", "{}", case.case);
                    assert_eq!(actual, case.expected, "{}", case.case);
                }
                Disposition::Fact { record, result } => {
                    assert_eq!(case.disposition, "fact", "{}", case.case);
                    assert_eq!(record, case.expected, "{}", case.case);
                    assert_eq!(
                        result.opaque().unwrap(),
                        case.result.unwrap().opaque().unwrap(),
                        "{}",
                        case.case
                    );
                }
                Disposition::Unsupported {
                    record,
                    code,
                    message,
                } => {
                    assert_eq!(case.disposition, "error");
                    assert_eq!(record, case.expected, "{}", case.case);
                    assert_eq!(Some(code), case.code);
                    assert_eq!(Some(message), case.message);
                }
            }
        } else {
            let id: String = case.id.decode().unwrap();
            assert_eq!(
                claude::requests::pending(
                    &id,
                    &case.method,
                    &case.params,
                    &case.raw,
                    Some("s"),
                    Some("t"),
                    17000
                )
                .unwrap(),
                case.expected,
                "{}",
                case.case
            );
        }
    }
}
fn codex_card(method: &str, body: &str) -> PendingRequest {
    let body = RawJson::parse(body).unwrap();
    match requests::pending(
        RequestId::number(1),
        method,
        Some(&body),
        &RawJson::parse("{}").unwrap(),
        0,
    )
    .unwrap()
    {
        Disposition::Ask(card) => card,
        _ => panic!("permission must require a user"),
    }
}
#[test]
fn unknown_requests_are_cards_and_raw_answers_cannot_override_known_permissions() {
    let unknown = codex_card("future/approveEverything", r#"{"future":9007199254740993}"#);
    assert_eq!(unknown.status, RequestStatus::Pending);
    assert_eq!(unknown.decisions.len(), 1);
    assert_eq!(unknown.decisions[0].id, "reject");
    let known = codex_card(
        "item/commandExecution/requestApproval",
        r#"{"availableDecisions":["decline"]}"#,
    );
    assert_eq!(
        requests::answer(
            &known,
            &RequestResponse::Decide {
                decision_id: "accept".into(),
                message: None
            }
        )
        .unwrap_err()
        .kind,
        ErrorKind::DecisionNotOffered
    );
    assert!(
        requests::answer(
            &known,
            &RequestResponse::RawResult {
                result: strict_json(br#"{"decision":"accept"}"#).unwrap()
            }
        )
        .is_err()
    );
    assert!(matches!(
        requests::answer(
            &unknown,
            &RequestResponse::Decide {
                decision_id: "reject".into(),
                message: None
            }
        )
        .unwrap(),
        UserReply::Error { .. }
    ));
}
#[test]
fn codex_raw_settings_force_user_reviewer_and_preserve_extensions() {
    for method in params::REVIEWER_METHODS {
        let guarded = params::enforce_reviewer(
            method,
            Some(
                &RawJson::parse(
                    r#"{"approvalsReviewer":"auto_review","future":{"n":9007199254740993}}"#,
                )
                .unwrap(),
            ),
        )
        .unwrap()
        .unwrap();
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Check {
            approvals_reviewer: String,
            future: RawJson,
        }
        let parsed: Check = guarded.decode().unwrap();
        assert_eq!(parsed.approvals_reviewer, "user");
        assert_eq!(parsed.future.text(), r#"{"n":9007199254740993}"#);
    }
    let mut guard = params::ReviewerGuard::default();
    guard.observe("thread", Some("auto_review"));
    assert!(guard.validate_send("thread").is_err());
    guard.observe("thread", Some("user"));
    guard.validate_send("thread").unwrap();
}
#[test]
fn claude_launch_filters_inherited_credentials_and_rejects_permission_override_spellings() {
    let base = std::collections::BTreeMap::from([
        ("OPENAI_API_KEY".into(), "fixture".into()),
        ("CLAUDECODE".into(), "1".into()),
        ("LD_PRELOAD".into(), "fixture".into()),
        ("LANG".into(), "C.UTF-8".into()),
    ]);
    let spec = claude::launch::spec(
        &base,
        &Default::default(),
        Session::Resume("s"),
        Some("model"),
        Some("high"),
        PermissionPreset::AutoEdit,
        &[],
    )
    .unwrap();
    assert!(!spec.env.contains_key("OPENAI_API_KEY"));
    assert!(!spec.env.contains_key("LD_PRELOAD"));
    assert!(!spec.env.contains_key("CLAUDECODE"));
    assert_eq!(spec.env.get("LANG").unwrap(), "C.UTF-8");
    assert!(
        spec.argv
            .windows(2)
            .any(|v| v == ["--permission-mode", "acceptEdits"])
    );
    for flag in [
        "--dangerously-skip-permissions",
        "--permission-mode=auto",
        "--permission-mode=bypassPermissions",
        "--permission-prompt-tool=custom",
        "--allow-dangerously-skip-permissions=true",
    ] {
        assert!(
            claude::launch::spec(
                &base,
                &Default::default(),
                Session::New("s"),
                None,
                None,
                PermissionPreset::Ask,
                &[flag.into()]
            )
            .is_err(),
            "{flag}"
        );
    }
    assert!(claude::launch::transcript_path("/workspace", "../../secret").is_err());
}
#[test]
fn claude_undeclared_dialogs_and_unoffered_persistent_decisions_cannot_be_answered() {
    let raw=RawJson::parse(r#"{"type":"control_request","request_id":"r","request":{"subtype":"can_use_tool","tool_name":"Edit","input":{},"requires_user_interaction":true}}"#).unwrap();
    #[derive(Deserialize)]
    struct Frame {
        request: RawJson,
    }
    let request: Frame = raw.decode().unwrap();
    let card = claude::requests::pending(
        "r",
        "can_use_tool",
        &request.request,
        &raw,
        Some("s"),
        Some("t"),
        0,
    )
    .unwrap();
    for id in ["allow", "allowAlways:0"] {
        assert!(
            claude::requests::answer(
                &card,
                &RequestResponse::Decide {
                    decision_id: id.into(),
                    message: None
                }
            )
            .is_err()
        );
    }
    let dialog = claude::requests::pending(
        "r",
        "request_user_dialog",
        &RawJson::parse(r#"{"dialog_kind":"future"}"#).unwrap(),
        &raw,
        None,
        None,
        0,
    )
    .unwrap();
    assert!(
        claude::requests::answer(
            &dialog,
            &RequestResponse::RawResult {
                result: strict_json(b"{}").unwrap()
            }
        )
        .is_err()
    );
}
