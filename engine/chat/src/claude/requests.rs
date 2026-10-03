//! Claude request-card policy. Permission suggestions are data until the user selects one.
use crate::{
    codex::requests::UserReply,
    error::{ChatError, ErrorKind, Result},
    model::*,
    transport::raw::RawJson,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Default, Deserialize)]
#[serde(default)]
struct Params {
    tool_name: String,
    input: Option<RawJson>,
    tool_use_id: Option<String>,
    permission_suggestions: Vec<RawJson>,
    suppress_always_allow_rule: bool,
    display_name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    blocked_path: Option<String>,
    decision_reason: Option<String>,
    decision_reason_type: Option<String>,
    default_to_no: bool,
    requires_user_interaction: bool,
    agent_id: Option<String>,
    mcp_server: Option<RawJson>,
    mcp_server_name: Option<String>,
    message: Option<String>,
    mode: Option<String>,
    url: Option<String>,
    requested_schema: Option<RawJson>,
    elicitation_id: Option<String>,
    dialog_kind: String,
    payload: Option<RawJson>,
}
#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Input {
    questions: Vec<QuestionInput>,
    plan: String,
}
#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct QuestionInput {
    question: String,
    header: Option<String>,
    options: Vec<QuestionOption>,
    multi_select: bool,
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct Suggestion {
    destination: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    behavior: Option<String>,
    rules: Vec<Rule>,
    mode: Option<String>,
    directories: Vec<String>,
}
#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Rule {
    tool_name: Option<String>,
    rule_content: Option<String>,
}
fn decision(id: &str, kind: DecisionKind) -> Decision {
    Decision {
        id: id.into(),
        kind,
        ..Default::default()
    }
}
fn describe(raw: &RawJson) -> String {
    let Ok(s) = raw.decode::<Suggestion>() else {
        return raw.text().into();
    };
    let destination = s.destination.as_deref().unwrap_or("session");
    match s.kind.as_deref() {
        Some("addRules" | "replaceRules") => format!(
            "{} {} → {destination}",
            s.behavior.as_deref().unwrap_or("allow"),
            s.rules
                .iter()
                .map(|r| format!(
                    "{}{}",
                    r.tool_name.as_deref().unwrap_or(""),
                    r.rule_content
                        .as_ref()
                        .map(|v| format!("({v})"))
                        .unwrap_or_default()
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Some("setMode") => format!(
            "mode {} → {destination}",
            s.mode.as_deref().unwrap_or("null")
        ),
        Some("addDirectories") => {
            format!("directories {} → {destination}", s.directories.join(", "))
        }
        Some("removeRules" | "removeDirectories") => format!("{} → {destination}", s.kind.unwrap()),
        _ => raw.text().into(),
    }
}
pub fn pending(
    id: &str,
    subtype: &str,
    request: &RawJson,
    raw: &RawJson,
    thread: Option<&str>,
    turn: Option<&str>,
    now: i64,
) -> Result<PendingRequest> {
    let mut card = PendingRequest {
        key: RequestKey {
            backend: BackendKind::Claude,
            raw_id: RawJson::encode(id)?.opaque()?,
        },
        method: subtype.into(),
        kind: RequestKind::Unknown {
            method: subtype.into(),
            params: Some(request.opaque()?),
        },
        decisions: vec![decision("reject", DecisionKind::Deny)],
        thread_id: thread.map(str::to_owned),
        turn_id: turn.map(str::to_owned),
        received_at_ms: Some(now),
        raw: Some(raw.opaque()?),
        ..Default::default()
    };
    let Ok(p) = request.decode::<Params>() else {
        return Ok(card);
    };
    match subtype {
        "can_use_tool" => {
            let input = p.input.unwrap_or(RawJson::parse("{}")?);
            let mut remember = Vec::new();
            if !p.suppress_always_allow_rule {
                for (n, suggestion) in p.permission_suggestions.iter().enumerate() {
                    let destination = suggestion
                        .decode::<Suggestion>()
                        .ok()
                        .and_then(|s| s.destination);
                    remember.push(Decision {
                        id: format!("allowAlways:{n}"),
                        kind: if matches!(destination.as_deref(), Some("session" | "cliArg")) {
                            DecisionKind::AllowSession
                        } else {
                            DecisionKind::AllowPersistent
                        },
                        detail: Some(describe(suggestion)),
                        wire: Some(suggestion.opaque()?),
                    });
                }
            }
            card.item_id = p.tool_use_id.clone();
            match p.tool_name.as_str() {
                "AskUserQuestion" => {
                    let input: Input = input.decode()?;
                    card.kind = RequestKind::UserInput {
                        questions: input
                            .questions
                            .into_iter()
                            .map(|q| Question {
                                id: q.question.clone(),
                                question: q.question,
                                header: q.header,
                                options: q.options,
                                multi_select: q.multi_select,
                                allow_free_text: true,
                                secret: false,
                            })
                            .collect(),
                        auto_resolution_ms: None,
                    };
                    card.decisions = vec![decision("deny", DecisionKind::Deny)];
                }
                "ExitPlanMode" => {
                    let input: Input = input.decode()?;
                    card.kind = RequestKind::PlanApproval { plan: input.plan };
                    card.decisions = vec![decision("allow", DecisionKind::AllowOnce)];
                    card.decisions.extend(remember);
                    card.decisions.push(decision("deny", DecisionKind::Deny));
                }
                _ => {
                    card.kind = RequestKind::ToolApproval {
                        tool: p.tool_name,
                        display_name: p.display_name,
                        title: p.title.map(|s| sanitize(&s)),
                        description: p.description.map(|s| sanitize(&s)),
                        input: Some(input.opaque()?),
                        blocked_path: p.blocked_path,
                        reason: p.decision_reason.map(|s| sanitize(&s)),
                        reason_type: p.decision_reason_type,
                        default_to_no: p.default_to_no,
                        requires_user_interaction: p.requires_user_interaction,
                        tool_use_id: p.tool_use_id,
                        agent_id: p.agent_id,
                        mcp_server: p.mcp_server.map(|v| v.opaque()).transpose()?,
                    };
                    card.decisions = if p.requires_user_interaction {
                        vec![]
                    } else {
                        let mut d = vec![decision("allow", DecisionKind::AllowOnce)];
                        d.extend(remember);
                        d
                    };
                    card.decisions.extend([
                        decision("deny", DecisionKind::Deny),
                        decision("denyAndStop", DecisionKind::Abort),
                    ]);
                }
            }
        }
        "elicitation" => {
            card.kind = RequestKind::Elicitation {
                server: p.mcp_server_name,
                message: p.message.or(p.title).unwrap_or_default(),
                mode: p.mode.unwrap_or("form".into()),
                url: p.url,
                schema: p.requested_schema.map(|v| v.opaque()).transpose()?,
                elicitation_id: p.elicitation_id,
            };
            card.decisions = vec![
                decision("accept", DecisionKind::AllowOnce),
                decision("decline", DecisionKind::Deny),
                decision("cancel", DecisionKind::Abort),
            ];
        }
        "request_user_dialog" => {
            card.kind = RequestKind::UserDialog {
                dialog_kind: p.dialog_kind,
                payload: p.payload.map(|v| v.opaque()).transpose()?,
            };
            card.decisions.clear();
        }
        _ => (),
    }
    Ok(card)
}
#[derive(Debug)]
pub struct Answer {
    pub reply: UserReply,
    pub denied: bool,
}
fn invalid(message: &str) -> ChatError {
    ChatError::new(ErrorKind::DecisionNotOffered, message)
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Allow<'a> {
    behavior: &'static str,
    updated_input: &'a RawJson,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_permissions: Option<Vec<&'a OpaqueJson>>,
}
#[derive(Serialize)]
struct Deny<'a> {
    behavior: &'static str,
    message: &'a str,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    interrupt: bool,
}
pub fn answer(request: &PendingRequest, response: &RequestResponse) -> Result<Answer> {
    let selected = |id: &str| {
        request
            .decisions
            .iter()
            .find(|d| d.id == id)
            .ok_or_else(|| invalid("decision was not offered"))
    };
    let result = |value, summary, denied| {
        Ok(Answer {
            reply: UserReply::Result { value, summary },
            denied,
        })
    };
    let deny = |message: Option<&str>, interrupt: bool| {
        RawJson::encode(&Deny {
            behavior: "deny",
            message: message.unwrap_or("The user declined this action."),
            interrupt,
        })
    };
    #[derive(Deserialize)]
    struct Frame {
        request: Option<RawJson>,
    }
    let input = request
        .raw
        .as_ref()
        .map(RawJson::encode)
        .transpose()?
        .and_then(|r| r.decode::<Frame>().ok())
        .and_then(|f| f.request)
        .and_then(|r| r.decode::<Params>().ok())
        .and_then(|p| p.input)
        .unwrap_or(RawJson::parse("{}")?);
    match &request.kind {
        RequestKind::ToolApproval { .. } | RequestKind::PlanApproval { .. } => {
            let RequestResponse::Decide {
                decision_id,
                message,
            } = response
            else {
                return Err(invalid("expected a decision"));
            };
            let d = selected(decision_id)?;
            if d.id == "allow" || d.id.starts_with("allowAlways:") {
                result(
                    RawJson::encode(&Allow {
                        behavior: "allow",
                        updated_input: &input,
                        updated_permissions: if d.id == "allow" {
                            None
                        } else {
                            Some(vec![
                                d.wire
                                    .as_ref()
                                    .ok_or_else(|| invalid("permission suggestion missing"))?,
                            ])
                        },
                    })?,
                    decision_id.clone(),
                    false,
                )
            } else {
                result(
                    deny(message.as_deref(), d.id == "denyAndStop")?,
                    decision_id.clone(),
                    true,
                )
            }
        }
        RequestKind::UserInput { questions, .. } => match response {
            RequestResponse::Answer { answers } => {
                if answers
                    .keys()
                    .any(|id| !questions.iter().any(|q| &q.id == id))
                {
                    return Err(invalid("unknown question ID"));
                }
                let mut fields = input.decode::<BTreeMap<String, RawJson>>()?;
                let answers: BTreeMap<_, _> = answers
                    .iter()
                    .map(|(id, values)| (id, values.join(", ")))
                    .collect();
                fields.insert("answers".into(), RawJson::encode(&answers)?);
                let input = RawJson::encode(&fields)?;
                result(
                    RawJson::encode(&Allow {
                        behavior: "allow",
                        updated_input: &input,
                        updated_permissions: None,
                    })?,
                    "answered".into(),
                    false,
                )
            }
            RequestResponse::Decide {
                decision_id,
                message,
            } => {
                selected(decision_id)?;
                result(deny(message.as_deref(), false)?, decision_id.clone(), true)
            }
            _ => Err(invalid("expected answers")),
        },
        RequestKind::Elicitation { .. } => {
            let (action, content) = match response {
                RequestResponse::Elicit { action, content } => (action.as_str(), content.as_ref()),
                RequestResponse::Decide { decision_id, .. } => {
                    (selected(decision_id)?.id.as_str(), None)
                }
                _ => return Err(invalid("expected an elicitation answer")),
            };
            if !matches!(action, "accept" | "decline" | "cancel") {
                return Err(invalid("invalid elicitation action"));
            }
            #[derive(Serialize)]
            struct Elicit<'a> {
                action: &'a str,
                #[serde(skip_serializing_if = "Option::is_none")]
                content: Option<&'a OpaqueObject>,
            }
            result(
                RawJson::encode(&Elicit { action, content })?,
                action.into(),
                action != "accept",
            )
        }
        RequestKind::UserDialog { .. } => {
            Err(invalid("undeclared dialog kinds must not be answered"))
        }
        RequestKind::Unknown { .. } => {
            if let RequestResponse::RawResult { result: raw } = response {
                let raw = RawJson::encode(raw)?;
                let _: BTreeMap<String, RawJson> = raw.decode()?;
                result(raw, "raw".into(), false)
            } else {
                Ok(Answer {
                    reply: UserReply::Error {
                        code: -32601,
                        message: match response {
                            RequestResponse::Reject { message } => message.clone(),
                            _ => "Rejected by user".into(),
                        },
                        summary: "rejected".into(),
                    },
                    denied: true,
                })
            }
        }
        _ => Err(invalid("unsupported Claude request kind")),
    }
}
pub fn sanitize(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            match chars.peek().copied() {
                Some('[') => {
                    chars.next();
                    for c in chars.by_ref() {
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    chars.next();
                    while let Some(c) = chars.next() {
                        if c == '\u{7}' {
                            break;
                        }
                        if c == '\u{1b}' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                Some('@'..='Z' | '\\'..='_') => {
                    chars.next();
                }
                _ => (),
            }
        } else if c == '\n' || c == '\t' || !c.is_control() {
            out.push(c);
        }
    }
    out
}
