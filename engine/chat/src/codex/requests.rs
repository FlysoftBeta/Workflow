//! Codex server requests as cards, and the user's explicit answers. Mapping never answers an
//! approval: only factual or negative capability results are automatic.
use crate::{
    error::{ChatError, ErrorKind, Result},
    model::*,
    wire::{self, Arr, Json, Long, Obj, Str, Strings, project},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SERVER_REQUESTS: &[&str] = &[
    "item/commandExecution/requestApproval",
    "item/fileChange/requestApproval",
    "item/tool/requestUserInput",
    "mcpServer/elicitation/request",
    "item/permissions/requestApproval",
    "item/tool/call",
    "account/chatgptAuthTokens/refresh",
    "attestation/generate",
    "currentTime/read",
    "applyPatchApproval",
    "execCommandApproval",
];

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Params {
    thread_id: Str,
    conversation_id: Str,
    turn_id: Str,
    item_id: Str,
    call_id: Str,
    command: Json,
    cwd: Str,
    reason: Str,
    command_actions: Arr<Obj<Action>>,
    network_approval_context: Obj<Host>,
    available_decisions: Json,
    grant_root: Str,
    permissions: Json,
    questions: Arr<Obj<InputQuestion>>,
    auto_resolution_ms: Long,
    server_name: Str,
    #[serde(rename = "_meta")]
    meta: Obj<Meta>,
    message: Str,
    description: Str,
    title: Str,
    mode: Str,
    url: Str,
    requested_schema: Json,
    elicitation_id: Str,
}
#[derive(Clone, Default, Deserialize)]
#[serde(default)]
struct Action {
    #[serde(rename = "type")]
    kind: Str,
    command: Str,
    path: Str,
    query: Str,
}
#[derive(Clone, Default, Deserialize)]
#[serde(default)]
struct Host {
    host: Str,
}
#[derive(Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Meta {
    server_name: Str,
}
#[derive(Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct InputQuestion {
    id: Str,
    question: Str,
    header: Str,
    options: Json,
    is_other: crate::wire::Bool,
    is_secret: crate::wire::Bool,
}
#[derive(Clone, Default, Deserialize)]
#[serde(default)]
struct InputOption {
    label: Str,
    description: Str,
}
#[derive(Clone, Default, Deserialize)]
#[serde(default)]
struct ExecPolicy {
    execpolicy_amendment: Strings,
}
#[derive(Clone, Default, Deserialize)]
#[serde(default)]
struct NetworkPolicy {
    network_policy_amendment: Obj<NetworkRule>,
}
#[derive(Clone, Default, Deserialize)]
#[serde(default)]
struct NetworkRule {
    action: Str,
    host: Str,
}

/// What to do with a server request. Only `Ask` reaches the user; nothing here approves.
pub enum Disposition {
    Ask(PendingRequest),
    /// A harmless factual or negative capability answer (current time, no dynamic tools).
    Fact {
        record: PendingRequest,
        result: OpaqueJson,
    },
    /// Unsupported known method: answered with a JSON-RPC error and recorded as rejected.
    Unsupported {
        record: PendingRequest,
        code: i64,
        message: String,
    },
}

fn decision(id: &str, kind: DecisionKind, wire: Option<OpaqueJson>) -> Decision {
    Decision {
        id: id.into(),
        kind,
        detail: None,
        wire,
    }
}
fn simple(id: &str, kind: DecisionKind) -> Decision {
    decision(id, kind, Some(wire::string_value(id)))
}
fn one_shot() -> Vec<Decision> {
    vec![
        simple("accept", DecisionKind::AllowOnce),
        simple("decline", DecisionKind::Deny),
        simple("cancel", DecisionKind::Abort),
    ]
}

/// `availableDecisions` as buttons in server order. Without a list only one-shot choices exist.
pub fn command_decisions(available: Option<&OpaqueJson>) -> Vec<Decision> {
    let Some(list) = available.filter(|v| wire::is_array(v)) else {
        return one_shot();
    };
    let mut used = BTreeMap::<String, usize>::new();
    let mut out = Vec::new();
    for member in Json(Some(list.clone())).elements() {
        let text = Json(Some(member.clone())).string();
        let keys = Json(Some(member.clone())).keys();
        let mut d = match text.as_deref() {
            Some("accept") => decision("accept", DecisionKind::AllowOnce, Some(member.clone())),
            Some("acceptForSession") => decision(
                "acceptForSession",
                DecisionKind::AllowSession,
                Some(member.clone()),
            ),
            Some("decline") => decision("decline", DecisionKind::Deny, Some(member.clone())),
            Some("cancel") => decision("cancel", DecisionKind::Abort, Some(member.clone())),
            _ if keys.iter().any(|k| k == "acceptWithExecpolicyAmendment") => {
                #[derive(Default, Deserialize)]
                #[serde(default, rename_all = "camelCase")]
                struct Body {
                    accept_with_execpolicy_amendment: Obj<ExecPolicy>,
                }
                let body: Body = project(&member);
                Decision {
                    id: "acceptWithExecpolicyAmendment".into(),
                    kind: DecisionKind::AllowPersistent,
                    detail: Some(
                        body.accept_with_execpolicy_amendment
                            .or_default()
                            .execpolicy_amendment
                            .0
                            .join(" "),
                    ),
                    wire: Some(member.clone()),
                }
            }
            _ if keys.iter().any(|k| k == "applyNetworkPolicyAmendment") => {
                #[derive(Default, Deserialize)]
                #[serde(default, rename_all = "camelCase")]
                struct Body {
                    apply_network_policy_amendment: Obj<NetworkPolicy>,
                }
                let body: Body = project(&member);
                let rule = body
                    .apply_network_policy_amendment
                    .or_default()
                    .network_policy_amendment
                    .or_default();
                let action = rule.action.owned();
                Decision {
                    id: "applyNetworkPolicyAmendment".into(),
                    kind: if action.as_deref() == Some("deny") {
                        DecisionKind::Deny
                    } else {
                        DecisionKind::AllowPersistent
                    },
                    detail: Some(
                        format!(
                            "{} {}",
                            action.as_deref().unwrap_or("allow"),
                            rule.host.or("")
                        )
                        .trim()
                        .to_string(),
                    ),
                    wire: Some(member.clone()),
                }
            }
            _ => Decision {
                id: text
                    .or_else(|| keys.into_iter().next())
                    .unwrap_or_else(|| "other".into()),
                kind: DecisionKind::Other,
                detail: Some(wire::text(&member)),
                wire: Some(member.clone()),
            },
        };
        let n = used.entry(d.id.clone()).or_default();
        *n += 1;
        if *n > 1 {
            d.id = format!("{}#{}", d.id, n);
        }
        out.push(d);
    }
    out
}

#[derive(Serialize)]
struct Grant<'a> {
    permissions: &'a OpaqueJson,
    scope: &'static str,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CurrentTime {
    current_time_at: i64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DynamicToolResult {
    success: bool,
    content_items: [ToolText; 1],
}
#[derive(Serialize)]
struct ToolText {
    #[serde(rename = "type")]
    kind: &'static str,
    text: &'static str,
}
#[derive(Serialize)]
struct Denied<'a> {
    denied: Rejection<'a>,
}
#[derive(Serialize)]
struct Rejection<'a> {
    rejection: &'a str,
}

/// Maps a server request. Approvals and questions become cards; nothing is answered with consent.
pub fn pending(
    id: &OpaqueJson,
    method: &str,
    params: Option<&OpaqueJson>,
    raw: &OpaqueJson,
    now_ms: i64,
) -> Disposition {
    let p: Params = wire::project_opt(params.filter(|v| wire::is_object(v)));
    let key = RequestKey {
        backend: BackendKind::Codex,
        raw_id: id.clone(),
    };
    let card = |kind: RequestKind, decisions: Vec<Decision>, item_id: Option<String>| {
        Disposition::Ask(PendingRequest {
            key: key.clone(),
            method: method.into(),
            kind,
            decisions,
            thread_id: p.thread_id.owned().or(p.conversation_id.owned()),
            turn_id: p.turn_id.owned(),
            item_id,
            status: RequestStatus::Pending,
            answer: None,
            received_at_ms: Some(now_ms),
            raw: Some(raw.clone()),
        })
    };
    let record = |status: RequestStatus| PendingRequest {
        key: key.clone(),
        method: method.into(),
        kind: RequestKind::Unknown {
            method: method.into(),
            params: params.filter(|v| !wire::is_null(v)).cloned(),
        },
        decisions: vec![],
        thread_id: p.thread_id.owned(),
        turn_id: p.turn_id.owned(),
        item_id: None,
        status,
        answer: None,
        received_at_ms: Some(now_ms),
        raw: Some(raw.clone()),
    };
    match method {
        "item/commandExecution/requestApproval" => card(
            RequestKind::CommandApproval {
                command: p.command.string(),
                cwd: p.cwd.owned(),
                reason: p.reason.owned(),
                actions: p
                    .command_actions
                    .items()
                    .iter()
                    .map(|a| {
                        let a = a.0.clone().unwrap_or_default();
                        CommandAction {
                            kind: a.kind.or("unknown"),
                            command: a.command.or(""),
                            path: a.path.owned(),
                            query: a.query.owned(),
                        }
                    })
                    .collect(),
                network_host: p
                    .network_approval_context
                    .get()
                    .and_then(|n| n.host.owned()),
            },
            command_decisions(p.available_decisions.get()),
            p.item_id.owned(),
        ),
        "item/fileChange/requestApproval" => card(
            RequestKind::FileChangeApproval {
                reason: p.reason.owned(),
                grant_root: p.grant_root.owned(),
                changes: vec![],
            },
            vec![
                simple("accept", DecisionKind::AllowOnce),
                simple("acceptForSession", DecisionKind::AllowSession),
                simple("decline", DecisionKind::Deny),
                simple("cancel", DecisionKind::Abort),
            ],
            p.item_id.owned(),
        ),
        "item/permissions/requestApproval" => {
            let requested = p
                .permissions
                .get()
                .cloned()
                .unwrap_or_else(wire::empty_object);
            let empty = wire::empty_object();
            card(
                RequestKind::PermissionsApproval {
                    reason: p.reason.owned(),
                    cwd: p.cwd.owned(),
                    permissions: requested.clone(),
                },
                vec![
                    decision(
                        "grantTurn",
                        DecisionKind::AllowOnce,
                        Some(wire::encode(&Grant {
                            permissions: &requested,
                            scope: "turn",
                        })),
                    ),
                    decision(
                        "grantSession",
                        DecisionKind::AllowSession,
                        Some(wire::encode(&Grant {
                            permissions: &requested,
                            scope: "session",
                        })),
                    ),
                    decision(
                        "decline",
                        DecisionKind::Deny,
                        Some(wire::encode(&Grant {
                            permissions: &empty,
                            scope: "turn",
                        })),
                    ),
                ],
                p.item_id.owned(),
            )
        }
        "item/tool/requestUserInput" => card(
            RequestKind::UserInput {
                questions: p
                    .questions
                    .items()
                    .iter()
                    .map(|q| {
                        let q = q.0.clone().unwrap_or_default();
                        let options: Arr<Obj<InputOption>> = wire::project_opt(q.options.get());
                        Question {
                            id: q.id.or(""),
                            question: q.question.or(""),
                            header: q.header.owned(),
                            allow_free_text: q.is_other.is_true() || options.items().is_empty(),
                            options: options
                                .items()
                                .iter()
                                .map(|o| {
                                    let o = o.0.clone().unwrap_or_default();
                                    QuestionOption {
                                        label: o.label.or(""),
                                        description: o.description.owned(),
                                        preview: None,
                                    }
                                })
                                .collect(),
                            multi_select: false,
                            secret: q.is_secret.is_true(),
                        }
                    })
                    .collect(),
                auto_resolution_ms: p.auto_resolution_ms.get(),
            },
            vec![],
            p.item_id.owned(),
        ),
        "mcpServer/elicitation/request" => card(
            RequestKind::Elicitation {
                server: p
                    .server_name
                    .owned()
                    .or_else(|| p.meta.get().and_then(|m| m.server_name.owned())),
                message: p
                    .message
                    .owned()
                    .or(p.description.owned())
                    .or(p.title.owned())
                    .unwrap_or_default(),
                mode: p.mode.or("form"),
                url: p.url.owned(),
                schema: p.requested_schema.value().cloned(),
                elicitation_id: p.elicitation_id.owned(),
            },
            one_shot(),
            None,
        ),
        "execCommandApproval" => card(
            RequestKind::CommandApproval {
                command: Some(
                    project::<Strings>(p.command.get().unwrap_or(&OpaqueJson::default()))
                        .0
                        .join(" "),
                ),
                cwd: p.cwd.owned(),
                reason: p.reason.owned(),
                actions: vec![],
                network_host: None,
            },
            legacy_decisions(),
            p.call_id.owned(),
        ),
        "applyPatchApproval" => card(
            RequestKind::FileChangeApproval {
                reason: p.reason.owned(),
                grant_root: p.grant_root.owned(),
                changes: vec![],
            },
            legacy_decisions(),
            p.call_id.owned(),
        ),
        "currentTime/read" => Disposition::Fact {
            record: record(RequestStatus::Answered),
            result: wire::encode(&CurrentTime {
                current_time_at: now_ms / 1000,
            }),
        },
        "item/tool/call" => Disposition::Fact {
            record: record(RequestStatus::Rejected),
            result: wire::encode(&DynamicToolResult {
                success: false,
                content_items: [ToolText {
                    kind: "inputText",
                    text: "Workflow registers no dynamic tools.",
                }],
            }),
        },
        "account/chatgptAuthTokens/refresh" | "attestation/generate" => Disposition::Unsupported {
            record: record(RequestStatus::Rejected),
            code: -32601,
            message: format!("{method} is not supported by this client"),
        },
        // Unknown methods become a card whose only choice rejects them; nothing is approved.
        _ => Disposition::Ask(PendingRequest {
            status: RequestStatus::Pending,
            decisions: vec![decision("reject", DecisionKind::Deny, None)],
            ..record(RequestStatus::Pending)
        }),
    }
}

fn legacy_decisions() -> Vec<Decision> {
    vec![
        simple("approved", DecisionKind::AllowOnce),
        simple("approved_for_session", DecisionKind::AllowSession),
        decision(
            "denied",
            DecisionKind::Deny,
            Some(wire::encode(&Denied {
                denied: Rejection {
                    rejection: "Declined by user",
                },
            })),
        ),
        simple("abort", DecisionKind::Abort),
    ]
}

/// The JSON-RPC answer for a user's response on `request`.
#[derive(Debug, PartialEq)]
pub enum UserReply {
    Result {
        value: OpaqueJson,
        summary: String,
    },
    Error {
        code: i64,
        message: String,
        summary: String,
    },
}

fn not_offered(request: &PendingRequest, id: &str) -> ChatError {
    ChatError::new(
        ErrorKind::DecisionNotOffered,
        format!(
            "decision {id} was not offered (offered: [{}])",
            request
                .decisions
                .iter()
                .map(|d| d.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    )
}

#[derive(Serialize)]
struct DecisionValue<'a> {
    decision: &'a OpaqueJson,
}
#[derive(Serialize)]
struct Elicitation<'a> {
    action: &'a OpaqueJson,
    content: Option<&'a OpaqueObject>,
}
#[derive(Serialize)]
struct AnswerValues<'a> {
    answers: &'a Vec<String>,
}
#[derive(Serialize)]
struct Answers<'a> {
    answers: BTreeMap<&'a str, AnswerValues<'a>>,
}

pub fn answer(request: &PendingRequest, response: &RequestResponse) -> Result<UserReply> {
    match response {
        RequestResponse::Decide {
            decision_id,
            message,
        } => {
            let decision = request
                .decisions
                .iter()
                .find(|d| &d.id == decision_id)
                .ok_or_else(|| not_offered(request, decision_id))?;
            if matches!(request.kind, RequestKind::Unknown { .. }) {
                return Ok(UserReply::Error {
                    code: -32601,
                    message: message.clone().unwrap_or_else(|| "Rejected by user".into()),
                    summary: decision.id.clone(),
                });
            }
            let null = OpaqueJson::default();
            let wire_value = decision.wire.as_ref().unwrap_or(&null);
            let value = match request.method.as_str() {
                "item/permissions/requestApproval" => wire_value.clone(),
                "mcpServer/elicitation/request" => {
                    let action = decision
                        .wire
                        .clone()
                        .unwrap_or_else(|| wire::string_value(&decision.id));
                    wire::encode(&Elicitation {
                        action: &action,
                        content: None,
                    })
                }
                "execCommandApproval" | "applyPatchApproval" => match message {
                    Some(m) if decision.id == "denied" => wire::encode(&DecisionValue {
                        decision: &wire::encode(&Denied {
                            denied: Rejection { rejection: m },
                        }),
                    }),
                    _ => wire::encode(&DecisionValue {
                        decision: wire_value,
                    }),
                },
                _ => wire::encode(&DecisionValue {
                    decision: wire_value,
                }),
            };
            Ok(UserReply::Result {
                value,
                summary: decision.id.clone(),
            })
        }
        RequestResponse::Answer { answers } => {
            let RequestKind::UserInput { questions, .. } = &request.kind else {
                return Err(ChatError::invalid("request is not a question".to_string()));
            };
            if let Some(unknown) = answers
                .keys()
                .find(|id| !questions.iter().any(|q| &q.id == *id))
            {
                return Err(ChatError::invalid(format!("unknown question id {unknown}")));
            }
            Ok(UserReply::Result {
                value: wire::encode(&Answers {
                    answers: answers
                        .iter()
                        .map(|(id, a)| (id.as_str(), AnswerValues { answers: a }))
                        .collect(),
                }),
                summary: "answered".into(),
            })
        }
        RequestResponse::Elicit { action, content } => {
            if !matches!(request.kind, RequestKind::Elicitation { .. }) {
                return Err(ChatError::invalid("request is not an elicitation"));
            }
            if !matches!(action.as_str(), "accept" | "decline" | "cancel") {
                return Err(ChatError::invalid(format!("invalid action {action}")));
            }
            Ok(UserReply::Result {
                value: wire::encode(&Elicitation {
                    action: &wire::string_value(action),
                    content: content.as_ref(),
                }),
                summary: action.clone(),
            })
        }
        RequestResponse::RawResult { result } => {
            if !matches!(request.kind, RequestKind::Unknown { .. }) {
                return Err(ChatError::invalid(
                    "raw results are only allowed for unknown requests",
                ));
            }
            Ok(UserReply::Result {
                value: result.clone(),
                summary: "raw".into(),
            })
        }
        RequestResponse::Reject { message } => Ok(UserReply::Error {
            code: -32000,
            message: message.clone(),
            summary: "rejected".into(),
        }),
    }
}
