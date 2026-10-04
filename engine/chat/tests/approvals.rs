use serde::Deserialize;
use workflow_chat::{
    claude::{self, launch::Session},
    codex::{
        params,
        requests::{self, Disposition, UserReply},
    },
    error::ErrorKind,
    model::*,
    wire,
};
use workflow_environment::json::strict_json;
#[derive(Deserialize)]
struct Case {
    case: String,
    backend: String,
    id: OpaqueJson,
    method: String,
    params: OpaqueJson,
    raw: OpaqueJson,
    disposition: String,
    expected: PendingRequest,
    result: Option<OpaqueJson>,
    code: Option<i64>,
    message: Option<String>,
}
#[test]
fn kotlin_approval_card_goldens() {
    let cases: Vec<Case> = strict_json(include_bytes!("golden/approvals.json")).unwrap();
    assert_eq!(cases.len(), 18);
    for case in cases {
        if case.backend == "codex" {
            match requests::pending(&case.id, &case.method, Some(&case.params), &case.raw, 17000) {
                Disposition::Ask(actual) => {
                    assert_eq!(case.disposition, "ask", "{}", case.case);
                    assert_eq!(actual, case.expected, "{}", case.case);
                }
                Disposition::Fact { record, result } => {
                    assert_eq!(case.disposition, "fact", "{}", case.case);
                    assert_eq!(record, case.expected, "{}", case.case);
                    assert_eq!(result, case.result.unwrap(), "{}", case.case);
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
            let id: String = wire::decode(&case.id).unwrap();
            assert_eq!(
                claude::requests::pending(
                    &id,
                    &case.method,
                    &case.params,
                    &case.raw,
                    Some("s"),
                    Some("t"),
                    17000
                ),
                case.expected,
                "{}",
                case.case
            );
        }
    }
}
fn parse(text: &str) -> OpaqueJson {
    strict_json(text.as_bytes()).unwrap()
}
fn codex_card(method: &str, body: &str) -> PendingRequest {
    let body = parse(body);
    match requests::pending(&parse("1"), method, Some(&body), &parse("{}"), 0) {
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
            Some(parse(r#"{"approvalsReviewer":"auto_review","future":{"n":9007199254740993}}"#)),
        )
        .unwrap();
        assert_eq!(
            wire::text(&guarded),
            r#"{"approvalsReviewer":"user","future":{"n":9007199254740993}}"#
        );
        assert_eq!(
            wire::text(&params::enforce_reviewer(method, None).unwrap()),
            r#"{"approvalsReviewer":"user"}"#
        );
    }
    assert!(params::enforce_reviewer("model/list", None).is_none());
    assert_eq!(
        wire::text(&params::enforce_reviewer("model/list", Some(parse("{}"))).unwrap()),
        "{}"
    );
}
fn launch_config(extra: &[&str]) -> claude::launch::LaunchConfig {
    claude::launch::LaunchConfig {
        executable: claude::launch::EXECUTABLE.into(),
        config_dir: "/home/work/.claude".into(),
        tmp_dir: "/tmp".into(),
        base_env: std::collections::BTreeMap::from([
            ("OPENAI_API_KEY".into(), "fixture".into()),
            ("CLAUDECODE".into(), "1".into()),
            ("LD_PRELOAD".into(), "fixture".into()),
            ("LANG".into(), "C.UTF-8".into()),
        ]),
        credentials: Default::default(),
        default_permissions: PermissionPreset::Ask,
        extra_args: extra.iter().map(|s| s.to_string()).collect(),
    }
}
#[test]
fn claude_launch_filters_inherited_credentials_and_rejects_permission_override_spellings() {
    let spec = claude::launch::spec(
        &launch_config(&[]),
        "/workspace",
        &Session::Resume("s"),
        Some("model"),
        Some("high"),
        Some(PermissionPreset::AutoEdit),
    )
    .unwrap();
    assert!(!spec.env.contains_key("OPENAI_API_KEY"));
    assert!(!spec.env.contains_key("LD_PRELOAD"));
    assert!(!spec.env.contains_key("CLAUDECODE"));
    assert_eq!(spec.env.get("LANG").unwrap(), "C.UTF-8");
    assert_eq!(spec.env.get("CLAUDE_CONFIG_DIR").unwrap(), "/home/work/.claude");
    assert!(
        spec.argv
            .windows(2)
            .any(|v| v == ["--permission-mode", "acceptEdits"])
    );
    for flag in [
        "--dangerously-skip-permissions",
        "--permission-mode=auto",
        "--permission-mode=bypassPermissions",
        "--permission-mode",
        "--permission-prompt-tool=custom",
        "--allow-dangerously-skip-permissions=true",
        "bypassPermissions",
        "auto",
    ] {
        assert!(
            claude::launch::spec(
                &launch_config(&[flag]),
                "/workspace",
                &Session::New("s"),
                None,
                None,
                None
            )
            .is_err(),
            "{flag}"
        );
    }
    assert!(claude::launch::transcript_path("/home/work/.claude", "/workspace", "../../secret").is_err());
    assert_eq!(
        claude::launch::transcript_path("/home/work/.claude", "/workspace", "s1").unwrap(),
        "/home/work/.claude/projects/-workspace/s1.jsonl"
    );
}
#[test]
fn claude_undeclared_dialogs_and_unoffered_persistent_decisions_cannot_be_answered() {
    let raw = parse(
        r#"{"type":"control_request","request_id":"r","request":{"subtype":"can_use_tool","tool_name":"Edit","input":{},"requires_user_interaction":true}}"#,
    );
    let request = parse(r#"{"subtype":"can_use_tool","tool_name":"Edit","input":{},"requires_user_interaction":true}"#);
    let card = claude::requests::pending("r", "can_use_tool", &request, &raw, Some("s"), Some("t"), 0);
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
        &parse(r#"{"dialog_kind":"future"}"#),
        &raw,
        None,
        None,
        0,
    );
    assert!(dialog.decisions.is_empty());
    assert!(
        claude::requests::answer(
            &dialog,
            &RequestResponse::RawResult {
                result: parse("{}")
            }
        )
        .is_err()
    );
}
