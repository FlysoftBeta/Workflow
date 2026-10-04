//! Typed Codex request bodies. Every request that can start, resume, fork or run a thread, or change
//! its settings, carries `approvalsReviewer: "user"` explicitly, because the server would otherwise
//! inherit the reviewer from `config.toml` (observed: `auto_review`, a model approving for the user).
use super::items;
use crate::{
    error::{ChatError, Result},
    model::*,
    wire,
};
use serde::Serialize;

pub const REVIEWER_USER: &str = "user";
/// Methods whose params must carry `approvalsReviewer: "user"`, including both settings methods.
pub const REVIEWER_METHODS: [&str; 6] = [
    "thread/start",
    "thread/resume",
    "thread/fork",
    "turn/start",
    "thread/settings/update",
    "turn/settings/update",
];

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SandboxPolicy {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub network_access: bool,
}
pub struct Policy {
    pub approval_policy: &'static str,
    pub sandbox_mode: &'static str,
    pub sandbox_policy: SandboxPolicy,
}
pub fn policy(preset: PermissionPreset) -> Policy {
    let (approval_policy, sandbox_mode, kind) = match preset {
        PermissionPreset::Ask | PermissionPreset::Plan => ("untrusted", "read-only", "readOnly"),
        PermissionPreset::AutoEdit => ("on-request", "workspace-write", "workspaceWrite"),
        PermissionPreset::DenyUnlisted => ("never", "read-only", "readOnly"),
    };
    Policy {
        approval_policy,
        sandbox_mode,
        sandbox_policy: SandboxPolicy {
            kind,
            network_access: false,
        },
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Initialize<'a> {
    client_info: ClientInfo<'a>,
    capabilities: Capabilities<'a>,
}
#[derive(Serialize)]
struct ClientInfo<'a> {
    name: &'a str,
    title: &'a str,
    version: &'a str,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Capabilities<'a> {
    experimental_api: bool,
    #[serde(skip_serializing_if = "<[String]>::is_empty")]
    opt_out_notification_methods: &'a [String],
}
pub fn initialize(name: &str, title: &str, version: &str, opt_out: &[String]) -> OpaqueJson {
    wire::encode(&Initialize {
        client_info: ClientInfo {
            name,
            title,
            version,
        },
        capabilities: Capabilities {
            experimental_api: true,
            opt_out_notification_methods: opt_out,
        },
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ThreadParams<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    thread_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cwd: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<&'a str>,
    approval_policy: &'static str,
    sandbox: &'static str,
    approvals_reviewer: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_turn_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ephemeral: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exclude_turns: Option<bool>,
}
fn thread_params<'a>(
    cwd: Option<&'a str>,
    settings: &'a TurnSettings,
    preset: PermissionPreset,
) -> ThreadParams<'a> {
    let p = policy(settings.permissions.unwrap_or(preset));
    ThreadParams {
        thread_id: None,
        cwd,
        model: settings.model.as_deref(),
        approval_policy: p.approval_policy,
        sandbox: p.sandbox_mode,
        approvals_reviewer: REVIEWER_USER,
        last_turn_id: None,
        ephemeral: None,
        exclude_turns: None,
    }
}
pub fn thread_start(
    cwd: &str,
    settings: &TurnSettings,
    preset: PermissionPreset,
    ephemeral: bool,
) -> OpaqueJson {
    let mut p = thread_params(Some(cwd), settings, preset);
    p.ephemeral = Some(ephemeral);
    wire::encode(&p)
}
pub fn thread_resume(
    thread: &str,
    cwd: Option<&str>,
    settings: &TurnSettings,
    preset: PermissionPreset,
) -> OpaqueJson {
    let mut p = thread_params(cwd, settings, preset);
    p.thread_id = Some(thread);
    p.exclude_turns = Some(true);
    wire::encode(&p)
}
pub fn thread_fork(
    thread: &str,
    last_turn: Option<&str>,
    cwd: Option<&str>,
    settings: &TurnSettings,
    preset: PermissionPreset,
    ephemeral: bool,
) -> OpaqueJson {
    let mut p = thread_params(cwd, settings, preset);
    p.thread_id = Some(thread);
    p.last_turn_id = last_turn;
    p.ephemeral = Some(ephemeral);
    p.exclude_turns = Some(true);
    wire::encode(&p)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TurnStart<'a> {
    thread_id: &'a str,
    input: OpaqueJson,
    client_user_message_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    effort: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    approval_policy: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sandbox_policy: Option<SandboxPolicy>,
    summary: &'static str,
    approvals_reviewer: &'static str,
}
pub fn turn_start(
    thread: &str,
    parts: &[UserPart],
    settings: Option<&TurnSettings>,
    client_message_id: &str,
) -> Result<OpaqueJson> {
    let policy = settings.and_then(|s| s.permissions).map(policy);
    Ok(wire::encode(&TurnStart {
        thread_id: thread,
        input: items::user_input(parts)?,
        client_user_message_id: client_message_id,
        model: settings.and_then(|s| s.model.as_deref()),
        effort: settings.and_then(|s| s.effort.as_deref()),
        approval_policy: policy.as_ref().map(|p| p.approval_policy),
        sandbox_policy: policy.map(|p| p.sandbox_policy),
        summary: "auto",
        approvals_reviewer: REVIEWER_USER,
    }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TurnSteer<'a> {
    thread_id: &'a str,
    expected_turn_id: &'a str,
    input: OpaqueJson,
    client_user_message_id: &'a str,
}
pub fn turn_steer(
    thread: &str,
    expected_turn: &str,
    parts: &[UserPart],
    client_message_id: &str,
) -> Result<OpaqueJson> {
    Ok(wire::encode(&TurnSteer {
        thread_id: thread,
        expected_turn_id: expected_turn,
        input: items::user_input(parts)?,
        client_user_message_id: client_message_id,
    }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct QueueAdd<'a> {
    thread_id: &'a str,
    client_user_message_id: &'a str,
    input: OpaqueJson,
}
pub fn queue_add(thread: &str, parts: &[UserPart], client_message_id: &str) -> Result<OpaqueJson> {
    Ok(wire::encode(&QueueAdd {
        thread_id: thread,
        client_user_message_id: client_message_id,
        input: items::user_input(parts)?,
    }))
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum Login<'a> {
    #[serde(rename = "chatgptDeviceCode")]
    DeviceCode,
    #[serde(rename = "chatgpt")]
    Browser,
    #[serde(rename = "apiKey")]
    ApiKey {
        #[serde(rename = "apiKey")]
        api_key: &'a str,
    },
}
/// The API key is placed only in this request body; it is never stored or logged.
pub fn login(method: LoginMethod, secret: Option<&str>) -> Result<OpaqueJson> {
    Ok(wire::encode(&match method {
        LoginMethod::CodexDeviceCode => Login::DeviceCode,
        LoginMethod::CodexBrowser => Login::Browser,
        LoginMethod::CodexApiKey => Login::ApiKey {
            api_key: secret
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| ChatError::invalid("API key required"))?,
        },
        other => {
            return Err(ChatError::invalid(format!(
                "{} is not a Codex login method",
                serde_json::to_string(&other)
                    .unwrap_or_default()
                    .trim_matches('"')
            )));
        }
    }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Permissions<'a> {
    thread_id: &'a str,
    approval_policy: &'static str,
    sandbox_policy: SandboxPolicy,
    approvals_reviewer: &'static str,
}
pub fn permissions(thread: &str, preset: PermissionPreset) -> OpaqueJson {
    let p = policy(preset);
    wire::encode(&Permissions {
        thread_id: thread,
        approval_policy: p.approval_policy,
        sandbox_policy: p.sandbox_policy,
        approvals_reviewer: REVIEWER_USER,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadRef<'a> {
    pub thread_id: &'a str,
}
pub fn thread_ref(thread: &str) -> OpaqueJson {
    wire::encode(&ThreadRef { thread_id: thread })
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TurnsList<'a> {
    thread_id: &'a str,
    limit: u32,
    items_view: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<&'a str>,
}
pub fn turns_list(thread: &str, limit: u32, cursor: Option<&str>) -> OpaqueJson {
    wire::encode(&TurnsList {
        thread_id: thread,
        limit,
        items_view: "full",
        cursor,
    })
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NameSet<'a> {
    thread_id: &'a str,
    name: &'a str,
}
pub fn name_set(thread: &str, name: &str) -> OpaqueJson {
    wire::encode(&NameSet {
        thread_id: thread,
        name,
    })
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct QueueDelete<'a> {
    thread_id: &'a str,
    queued_submission_id: &'a str,
}
pub fn queue_delete(thread: &str, submission: &str) -> OpaqueJson {
    wire::encode(&QueueDelete {
        thread_id: thread,
        queued_submission_id: submission,
    })
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct QueueList<'a> {
    thread_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<&'a str>,
}
pub fn queue_list(thread: &str, cursor: Option<&str>) -> OpaqueJson {
    wire::encode(&QueueList {
        thread_id: thread,
        cursor,
    })
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Interrupt<'a> {
    thread_id: &'a str,
    turn_id: &'a str,
}
pub fn interrupt(thread: &str, turn: &str) -> OpaqueJson {
    wire::encode(&Interrupt {
        thread_id: thread,
        turn_id: turn,
    })
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ModelList<'a> {
    include_hidden: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<&'a str>,
}
pub fn model_list(cursor: Option<&str>) -> OpaqueJson {
    wire::encode(&ModelList {
        include_hidden: true,
        cursor,
    })
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountRead {
    refresh_token: bool,
}
pub fn account_read() -> OpaqueJson {
    wire::encode(&AccountRead {
        refresh_token: false,
    })
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginCancel<'a> {
    login_id: &'a str,
}
pub fn login_cancel(login_id: &str) -> OpaqueJson {
    wire::encode(&LoginCancel { login_id })
}

/// Advanced-console guard: forces `approvalsReviewer: "user"` on every reviewer-bearing method.
/// Other members, including unknown ones and their exact number tokens, are preserved.
pub fn enforce_reviewer(method: &str, params: Option<OpaqueJson>) -> Option<OpaqueJson> {
    if !REVIEWER_METHODS.contains(&method) {
        return params;
    }
    let mut object: OpaqueObject = params
        .filter(wire::is_object)
        .map(|p| wire::project(&p))
        .unwrap_or_default();
    object.insert(
        "approvalsReviewer".into(),
        wire::string_value(REVIEWER_USER),
    );
    Some(wire::encode(&object))
}
