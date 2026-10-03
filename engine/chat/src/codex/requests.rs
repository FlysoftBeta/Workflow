//! Codex request cards and explicit user answers. No approval is answered by mapping.
use crate::{
    error::{ChatError, ErrorKind, Result},
    model::*,
    transport::raw::{RawJson, RequestId},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Params {
    thread_id: Option<String>,
    conversation_id: Option<String>,
    turn_id: Option<String>,
    item_id: Option<String>,
    call_id: Option<String>,
    command: Option<Command>,
    cwd: Option<String>,
    reason: Option<String>,
    command_actions: Option<Vec<Action>>,
    network_approval_context: Option<Network>,
    available_decisions: Option<Vec<RawJson>>,
    grant_root: Option<String>,
    permissions: Option<RawJson>,
    questions: Vec<InputQuestion>,
    auto_resolution_ms: Option<i64>,
    server_name: Option<String>,
    #[serde(rename = "_meta")]
    meta: Option<Meta>,
    message: Option<String>,
    description: Option<String>,
    title: Option<String>,
    mode: Option<String>,
    url: Option<String>,
    requested_schema: Option<RawJson>,
    elicitation_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Command {
    Text(String),
    Arguments(Vec<String>),
}
impl Command {
    fn text(self) -> String {
        match self {
            Self::Text(s) => s,
            Self::Arguments(v) => v.join(" "),
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Action {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    command: String,
    path: Option<String>,
    query: Option<String>,
}
#[derive(Deserialize)]
struct Network {
    host: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Meta {
    server_name: Option<String>,
}
#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct InputQuestion {
    id: String,
    question: String,
    header: Option<String>,
    options: Vec<QuestionOption>,
    is_other: bool,
    is_secret: bool,
}
pub enum Disposition {
    Ask(PendingRequest),
    Fact {
        record: PendingRequest,
        result: RawJson,
    },
    Unsupported {
        record: PendingRequest,
        code: i64,
        message: String,
    },
}
fn decision(id: &str, kind: DecisionKind, wire: RawJson) -> Result<Decision> {
    Ok(Decision {
        id: id.into(),
        kind,
        wire: Some(wire.opaque()?),
        ..Default::default()
    })
}
fn simple(id: &str, kind: DecisionKind) -> Result<Decision> {
    decision(id, kind, RawJson::encode(id)?)
}
fn fallback() -> Result<Vec<Decision>> {
    Ok(vec![
        simple("accept", DecisionKind::AllowOnce)?,
        simple("decline", DecisionKind::Deny)?,
        simple("cancel", DecisionKind::Abort)?,
    ])
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Amendment {
    accept_with_execpolicy_amendment: Option<ExecPolicy>,
    apply_network_policy_amendment: Option<NetworkPolicy>,
}
#[derive(Deserialize)]
struct ExecPolicy {
    #[serde(default)]
    execpolicy_amendment: Vec<String>,
}
#[derive(Deserialize)]
struct NetworkPolicy {
    network_policy_amendment: NetworkRule,
}
#[derive(Deserialize)]
struct NetworkRule {
    action: Option<String>,
    host: Option<String>,
}
pub fn command_decisions(available: Option<&[RawJson]>) -> Result<Vec<Decision>> {
    let Some(available) = available else {
        return fallback();
    };
    let mut used = BTreeMap::<String, usize>::new();
    let mut out = vec![];
    for wire in available {
        let value = wire.decode::<String>().ok();
        let mut d = match value.as_deref() {
            Some("accept") => simple("accept", DecisionKind::AllowOnce)?,
            Some("acceptForSession") => simple("acceptForSession", DecisionKind::AllowSession)?,
            Some("decline") => simple("decline", DecisionKind::Deny)?,
            Some("cancel") => simple("cancel", DecisionKind::Abort)?,
            _ => {
                let amendment = wire.decode::<Amendment>().ok();
                if let Some(p) = amendment
                    .as_ref()
                    .and_then(|a| a.accept_with_execpolicy_amendment.as_ref())
                {
                    Decision {
                        id: "acceptWithExecpolicyAmendment".into(),
                        kind: DecisionKind::AllowPersistent,
                        detail: Some(p.execpolicy_amendment.join(" ")),
                        wire: None,
                    }
                } else if let Some(p) = amendment
                    .as_ref()
                    .and_then(|a| a.apply_network_policy_amendment.as_ref())
                {
                    let p = &p.network_policy_amendment;
                    Decision {
                        id: "applyNetworkPolicyAmendment".into(),
                        kind: if p.action.as_deref() == Some("deny") {
                            DecisionKind::Deny
                        } else {
                            DecisionKind::AllowPersistent
                        },
                        detail: Some(
                            format!(
                                "{} {}",
                                p.action.as_deref().unwrap_or("allow"),
                                p.host.as_deref().unwrap_or("")
                            )
                            .trim()
                            .into(),
                        ),
                        wire: None,
                    }
                } else {
                    Decision {
                        id: value
                            .or_else(|| {
                                wire.decode::<BTreeMap<String, RawJson>>()
                                    .ok()
                                    .and_then(|v| v.keys().next().cloned())
                            })
                            .unwrap_or("other".into()),
                        kind: DecisionKind::Other,
                        detail: Some(wire.text().into()),
                        wire: None,
                    }
                }
            }
        };
        d.wire = Some(wire.opaque()?);
        let n = used.entry(d.id.clone()).or_default();
        *n += 1;
        if *n > 1 {
            d.id = format!("{}#{}", d.id, n);
        }
        out.push(d);
    }
    Ok(out)
}
#[derive(Serialize)]
struct Permissions<'a> {
    permissions: &'a RawJson,
    scope: &'static str,
}
pub fn pending(
    id: RequestId,
    method: &str,
    params: Option<&RawJson>,
    raw: &RawJson,
    now_ms: i64,
) -> Result<Disposition> {
    let unknown = || RequestKind::Unknown {
        method: method.into(),
        params: params.map(RawJson::opaque).transpose().ok().flatten(),
    };
    let mut card = PendingRequest {
        key: RequestKey {
            backend: BackendKind::Codex,
            raw_id: id.0.opaque()?,
        },
        method: method.into(),
        kind: unknown(),
        decisions: vec![simple("reject", DecisionKind::Deny)?],
        received_at_ms: Some(now_ms),
        raw: Some(raw.opaque()?),
        ..Default::default()
    };
    let p = match params {
        Some(params) => match params.decode::<Params>() {
            Ok(p) => p,
            Err(_) => return Ok(Disposition::Ask(card)),
        },
        None => Params::default(),
    };
    card.thread_id = p.thread_id.or(p.conversation_id);
    card.turn_id = p.turn_id;
    card.item_id = p.item_id;
    match method {
        "item/commandExecution/requestApproval" => {
            card.kind = RequestKind::CommandApproval {
                command: p.command.map(Command::text),
                cwd: p.cwd,
                reason: p.reason,
                actions: p
                    .command_actions
                    .unwrap_or_default()
                    .into_iter()
                    .map(|a| CommandAction {
                        kind: a.kind,
                        command: a.command,
                        path: a.path,
                        query: a.query,
                    })
                    .collect(),
                network_host: p.network_approval_context.and_then(|n| n.host),
            };
            card.decisions = command_decisions(p.available_decisions.as_deref())?;
        }
        "item/fileChange/requestApproval" => {
            card.kind = RequestKind::FileChangeApproval {
                reason: p.reason,
                grant_root: p.grant_root,
                changes: vec![],
            };
            card.decisions = vec![
                simple("accept", DecisionKind::AllowOnce)?,
                simple("acceptForSession", DecisionKind::AllowSession)?,
                simple("decline", DecisionKind::Deny)?,
                simple("cancel", DecisionKind::Abort)?,
            ];
        }
        "item/permissions/requestApproval" => {
            let permissions = p.permissions.unwrap_or(RawJson::parse("{}")?);
            let empty = RawJson::parse("{}")?;
            card.kind = RequestKind::PermissionsApproval {
                reason: p.reason,
                cwd: p.cwd,
                permissions: permissions.opaque()?,
            };
            card.decisions = vec![
                decision(
                    "grantTurn",
                    DecisionKind::AllowOnce,
                    RawJson::encode(&Permissions {
                        permissions: &permissions,
                        scope: "turn",
                    })?,
                )?,
                decision(
                    "grantSession",
                    DecisionKind::AllowSession,
                    RawJson::encode(&Permissions {
                        permissions: &permissions,
                        scope: "session",
                    })?,
                )?,
                decision(
                    "decline",
                    DecisionKind::Deny,
                    RawJson::encode(&Permissions {
                        permissions: &empty,
                        scope: "turn",
                    })?,
                )?,
            ];
        }
        "item/tool/requestUserInput" => {
            card.kind = RequestKind::UserInput {
                questions: p
                    .questions
                    .into_iter()
                    .map(|q| Question {
                        id: q.id,
                        question: q.question,
                        header: q.header,
                        allow_free_text: q.is_other || q.options.is_empty(),
                        options: q.options,
                        secret: q.is_secret,
                        ..Default::default()
                    })
                    .collect(),
                auto_resolution_ms: p.auto_resolution_ms,
            };
            card.decisions.clear();
        }
        "mcpServer/elicitation/request" => {
            card.kind = RequestKind::Elicitation {
                server: p.server_name.or_else(|| p.meta.and_then(|m| m.server_name)),
                message: p.message.or(p.description).or(p.title).unwrap_or_default(),
                mode: p.mode.unwrap_or("form".into()),
                url: p.url,
                schema: p.requested_schema.map(|r| r.opaque()).transpose()?,
                elicitation_id: p.elicitation_id,
            };
            card.decisions = fallback()?;
            card.item_id = None;
        }
        "execCommandApproval" | "applyPatchApproval" => {
            card.kind = if method == "execCommandApproval" {
                RequestKind::CommandApproval {
                    command: Some(p.command.map(Command::text).unwrap_or_default()),
                    cwd: p.cwd,
                    reason: p.reason,
                    actions: vec![],
                    network_host: None,
                }
            } else {
                RequestKind::FileChangeApproval {
                    reason: p.reason,
                    grant_root: p.grant_root,
                    changes: vec![],
                }
            };
            card.item_id = p.call_id;
            card.decisions = vec![
                simple("approved", DecisionKind::AllowOnce)?,
                simple("approved_for_session", DecisionKind::AllowSession)?,
                decision(
                    "denied",
                    DecisionKind::Deny,
                    RawJson::parse(r#"{"denied":{"rejection":"Declined by user"}}"#)?,
                )?,
                simple("abort", DecisionKind::Abort)?,
            ];
        }
        "currentTime/read" => {
            #[derive(Serialize)]
            #[serde(rename_all = "camelCase")]
            struct Time {
                current_time_at: i64,
            }
            card.status = RequestStatus::Answered;
            card.decisions.clear();
            return Ok(Disposition::Fact {
                record: card,
                result: RawJson::encode(&Time {
                    current_time_at: now_ms / 1000,
                })?,
            });
        }
        "item/tool/call" => {
            card.status = RequestStatus::Rejected;
            card.decisions.clear();
            return Ok(Disposition::Fact {
                record: card,
                result: RawJson::parse(
                    r#"{"success":false,"contentItems":[{"type":"inputText","text":"Workflow registers no dynamic tools."}]}"#,
                )?,
            });
        }
        "account/chatgptAuthTokens/refresh" | "attestation/generate" => {
            card.status = RequestStatus::Rejected;
            card.decisions.clear();
            return Ok(Disposition::Unsupported {
                record: card,
                code: -32601,
                message: format!("{method} is not supported by this client"),
            });
        }
        _ => (),
    }
    Ok(Disposition::Ask(card))
}
#[derive(Debug)]
pub enum UserReply {
    Result {
        value: RawJson,
        summary: String,
    },
    Error {
        code: i64,
        message: String,
        summary: String,
    },
}
fn invalid(message: &str) -> ChatError {
    ChatError::new(ErrorKind::DecisionNotOffered, message)
}
pub fn answer(request: &PendingRequest, response: &RequestResponse) -> Result<UserReply> {
    let result = |value, summary| Ok(UserReply::Result { value, summary });
    match response {
        RequestResponse::Decide {
            decision_id,
            message,
        } => {
            let decision = request
                .decisions
                .iter()
                .find(|d| &d.id == decision_id)
                .ok_or_else(|| invalid("decision was not offered"))?;
            if matches!(request.kind, RequestKind::Unknown { .. }) {
                return Ok(UserReply::Error {
                    code: -32601,
                    message: message.clone().unwrap_or("Rejected by user".into()),
                    summary: decision_id.clone(),
                });
            }
            let wire = decision
                .wire
                .as_ref()
                .map(RawJson::encode)
                .transpose()?
                .unwrap_or_else(RawJson::null);
            #[derive(Serialize)]
            struct DecisionValue<'a> {
                decision: &'a RawJson,
            }
            let payload = match request.method.as_str() {
                "item/permissions/requestApproval" => wire,
                "mcpServer/elicitation/request" => {
                    #[derive(Serialize)]
                    struct Elicit<'a> {
                        action: &'a RawJson,
                        content: Option<()>,
                    }
                    RawJson::encode(&Elicit {
                        action: &wire,
                        content: None,
                    })?
                }
                "execCommandApproval" | "applyPatchApproval"
                    if decision.id == "denied" && message.is_some() =>
                {
                    #[derive(Serialize)]
                    struct Rejection<'a> {
                        rejection: &'a str,
                    }
                    #[derive(Serialize)]
                    struct Denied<'a> {
                        denied: Rejection<'a>,
                    }
                    let wire = RawJson::encode(&Denied {
                        denied: Rejection {
                            rejection: message.as_deref().unwrap(),
                        },
                    })?;
                    RawJson::encode(&DecisionValue { decision: &wire })?
                }
                _ => RawJson::encode(&DecisionValue { decision: &wire })?,
            };
            result(payload, decision_id.clone())
        }
        RequestResponse::Answer { answers } => {
            let RequestKind::UserInput { questions, .. } = &request.kind else {
                return Err(invalid("request is not a question"));
            };
            if answers
                .keys()
                .any(|id| !questions.iter().any(|q| &q.id == id))
            {
                return Err(invalid("unknown question ID"));
            }
            #[derive(Serialize)]
            struct Values<'a> {
                answers: &'a Vec<String>,
            }
            #[derive(Serialize)]
            struct Answers<'a> {
                answers: BTreeMap<&'a str, Values<'a>>,
            }
            result(
                RawJson::encode(&Answers {
                    answers: answers
                        .iter()
                        .map(|(id, a)| (id.as_str(), Values { answers: a }))
                        .collect(),
                })?,
                "answered".into(),
            )
        }
        RequestResponse::Elicit { action, content } => {
            if !matches!(request.kind, RequestKind::Elicitation { .. })
                || !matches!(action.as_str(), "accept" | "decline" | "cancel")
            {
                return Err(invalid("invalid elicitation action"));
            }
            #[derive(Serialize)]
            struct Elicit<'a> {
                action: &'a str,
                content: &'a Option<OpaqueObject>,
            }
            result(
                RawJson::encode(&Elicit { action, content })?,
                action.clone(),
            )
        }
        RequestResponse::RawResult { result: raw } => {
            if !matches!(request.kind, RequestKind::Unknown { .. }) {
                return Err(invalid("raw results are only allowed for unknown requests"));
            }
            result(RawJson::encode(raw)?, "raw".into())
        }
        RequestResponse::Reject { message } => Ok(UserReply::Error {
            code: -32000,
            message: message.clone(),
            summary: "rejected".into(),
        }),
    }
}
