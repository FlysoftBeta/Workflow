//! Typed Codex request builders and the invariant for every reviewer-bearing method.
use crate::{
    error::{ChatError, ErrorKind, Result},
    model::{PermissionPreset, TurnSettings},
    transport::raw::RawJson,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
pub const REVIEWER_METHODS: [&str; 6] = [
    "thread/start",
    "thread/resume",
    "thread/fork",
    "turn/start",
    "thread/settings/update",
    "turn/settings/update",
];
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sandbox {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub network_access: bool,
}
pub struct Policy {
    pub approval_policy: &'static str,
    pub sandbox_mode: &'static str,
    pub sandbox_policy: Sandbox,
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
        sandbox_policy: Sandbox {
            kind,
            network_access: false,
        },
    }
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
    ephemeral: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exclude_turns: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_turn_id: Option<&'a str>,
}
fn thread_params<'a>(
    cwd: Option<&'a str>,
    settings: &'a TurnSettings,
    default: PermissionPreset,
) -> ThreadParams<'a> {
    let p = policy(settings.permissions.unwrap_or(default));
    ThreadParams {
        thread_id: None,
        cwd,
        model: settings.model.as_deref(),
        approval_policy: p.approval_policy,
        sandbox: p.sandbox_mode,
        approvals_reviewer: "user",
        ephemeral: None,
        exclude_turns: None,
        last_turn_id: None,
    }
}
pub fn thread_start(
    cwd: &str,
    settings: &TurnSettings,
    default: PermissionPreset,
    ephemeral: bool,
) -> Result<RawJson> {
    let mut p = thread_params(Some(cwd), settings, default);
    p.ephemeral = Some(ephemeral);
    RawJson::encode(&p)
}
pub fn thread_resume(
    id: &str,
    cwd: Option<&str>,
    settings: &TurnSettings,
    default: PermissionPreset,
) -> Result<RawJson> {
    let mut p = thread_params(cwd, settings, default);
    p.thread_id = Some(id);
    p.exclude_turns = Some(true);
    RawJson::encode(&p)
}
pub fn thread_fork(
    id: &str,
    last: Option<&str>,
    cwd: Option<&str>,
    settings: &TurnSettings,
    default: PermissionPreset,
    ephemeral: bool,
) -> Result<RawJson> {
    let mut p = thread_params(cwd, settings, default);
    p.thread_id = Some(id);
    p.last_turn_id = last;
    p.ephemeral = Some(ephemeral);
    p.exclude_turns = Some(true);
    RawJson::encode(&p)
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Permissions<'a> {
    thread_id: &'a str,
    approval_policy: &'static str,
    sandbox_policy: Sandbox,
    approvals_reviewer: &'static str,
}
pub fn permissions(id: &str, preset: PermissionPreset) -> Result<RawJson> {
    let p = policy(preset);
    RawJson::encode(&Permissions {
        thread_id: id,
        approval_policy: p.approval_policy,
        sandbox_policy: p.sandbox_policy,
        approvals_reviewer: "user",
    })
}
/// Advanced-console params are intentionally opaque. Unknown fields and tokens survive.
pub fn enforce_reviewer(method: &str, params: Option<&RawJson>) -> Result<Option<RawJson>> {
    if !REVIEWER_METHODS.contains(&method) {
        return Ok(params.cloned());
    }
    let mut object = match params {
        Some(raw) if raw.text().starts_with('{') => raw.decode::<BTreeMap<String, RawJson>>()?,
        _ => BTreeMap::new(),
    };
    object.insert("approvalsReviewer".into(), RawJson::encode("user")?);
    Ok(Some(RawJson::encode(&object)?))
}
#[derive(Default)]
pub struct ReviewerGuard {
    blocked: BTreeSet<String>,
}
impl ReviewerGuard {
    pub fn observe(&mut self, thread: &str, reviewer: Option<&str>) {
        match reviewer {
            Some("user") => {
                self.blocked.remove(thread);
            }
            Some(_) => {
                self.blocked.insert(thread.into());
            }
            None => (),
        }
    }
    pub fn validate_send(&self, thread: &str) -> Result<()> {
        if self.blocked.contains(thread) {
            Err(ChatError::new(
                ErrorKind::BackendUnavailable,
                "Backend did not honor approvalsReviewer=user",
            ))
        } else {
            Ok(())
        }
    }
}
