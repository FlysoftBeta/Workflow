//! Claude control requests as cards, the user's explicit answers, and stream-json builders.
//! Permission suggestions are data until the user selects one; nothing here grants consent.
use crate::{
    codex::requests::UserReply,
    error::{ChatError, ErrorKind, Result},
    model::*,
    wire::{self, Arr, Bool, Json, Obj, Str, Strings},
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Reads attachment bytes by guest path (`/workspace/...`); the Engine maps it to workspace files.
pub type AttachmentReader<'a> = dyn Fn(&str, usize) -> Result<Vec<u8>> + Send + Sync + 'a;

pub fn key(request_id: &str) -> RequestKey {
    RequestKey {
        backend: BackendKind::Claude,
        raw_id: wire::string_value(request_id),
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct Params {
    tool_name: Str,
    input: Json,
    tool_use_id: Str,
    permission_suggestions: Arr<Json>,
    suppress_always_allow_rule: Bool,
    display_name: Str,
    title: Str,
    description: Str,
    blocked_path: Str,
    decision_reason: Str,
    decision_reason_type: Str,
    default_to_no: Bool,
    requires_user_interaction: Bool,
    agent_id: Str,
    mcp_server: Json,
    mcp_server_name: Str,
    message: Str,
    mode: Str,
    url: Str,
    requested_schema: Json,
    elicitation_id: Str,
    dialog_kind: Str,
    payload: Json,
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct Input {
    questions: Arr<Obj<QuestionInput>>,
    plan: Str,
}
#[derive(Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct QuestionInput {
    question: Str,
    header: Str,
    options: Arr<Obj<OptionInput>>,
    multi_select: Bool,
}
#[derive(Clone, Default, Deserialize)]
#[serde(default)]
struct OptionInput {
    label: Str,
    description: Str,
    preview: Str,
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct Suggestion {
    destination: Str,
    #[serde(rename = "type")]
    kind: Str,
    behavior: Str,
    rules: Arr<Obj<Rule>>,
    mode: Str,
    directories: Strings,
}
#[derive(Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Rule {
    tool_name: Str,
    rule_content: Str,
}

fn decision(id: &str, kind: DecisionKind) -> Decision {
    Decision {
        id: id.into(),
        kind,
        ..Default::default()
    }
}

/// A human-readable summary of one permission suggestion.
pub fn describe_suggestion(raw: &OpaqueJson) -> String {
    let s: Suggestion = wire::project(raw);
    let destination = s.destination.or("session");
    match s.kind.get() {
        Some("addRules" | "replaceRules") => format!(
            "{} {} → {destination}",
            s.behavior.or("allow"),
            s.rules
                .items()
                .iter()
                .map(|r| {
                    let r = r.0.clone().unwrap_or_default();
                    format!(
                        "{}{}",
                        r.tool_name.or(""),
                        r.rule_content.get().map(|v| format!("({v})")).unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Some("setMode") => format!("mode {} → {destination}", s.mode.or("null")),
        Some("addDirectories") => format!("directories {} → {destination}", s.directories.0.join(", ")),
        Some(kind @ ("removeRules" | "removeDirectories")) => format!("{kind} → {destination}"),
        _ => wire::text(raw),
    }
}

/// `can_use_tool`, `elicitation` and `request_user_dialog` become cards; any other subtype becomes
/// a generic card whose only choice rejects it. Nothing is answered here.
pub fn pending(
    id: &str,
    subtype: &str,
    request: &OpaqueJson,
    raw: &OpaqueJson,
    thread: Option<&str>,
    turn: Option<&str>,
    now: i64,
) -> PendingRequest {
    let mut card = PendingRequest {
        key: key(id),
        method: subtype.into(),
        kind: RequestKind::Unknown {
            method: subtype.into(),
            params: Some(request.clone()),
        },
        decisions: vec![decision("reject", DecisionKind::Deny)],
        thread_id: thread.map(str::to_owned),
        turn_id: turn.map(str::to_owned),
        received_at_ms: Some(now),
        raw: Some(raw.clone()),
        ..Default::default()
    };
    if !wire::is_object(request) {
        return card;
    }
    let p: Params = wire::project(request);
    match subtype {
        "can_use_tool" => {
            let input = p.input.get().cloned().unwrap_or_else(wire::empty_object);
            let remember: Vec<Decision> = if p.suppress_always_allow_rule.is_true() {
                vec![]
            } else {
                p.permission_suggestions
                    .items()
                    .iter()
                    .enumerate()
                    .map(|(n, s)| {
                        let s = s.0.clone().unwrap_or_default();
                        let destination = wire::project::<Suggestion>(&s).destination;
                        Decision {
                            id: format!("allowAlways:{n}"),
                            kind: if matches!(destination.get(), Some("session" | "cliArg")) {
                                DecisionKind::AllowSession
                            } else {
                                DecisionKind::AllowPersistent
                            },
                            detail: Some(describe_suggestion(&s)),
                            wire: Some(s),
                        }
                    })
                    .collect()
            };
            card.item_id = p.tool_use_id.owned();
            let parsed: Input = wire::project(&input);
            match p.tool_name.get().unwrap_or("") {
                "AskUserQuestion" => {
                    card.kind = RequestKind::UserInput {
                        questions: parsed
                            .questions
                            .items()
                            .iter()
                            .map(|q| {
                                let q = q.0.clone().unwrap_or_default();
                                Question {
                                    id: q.question.or(""),
                                    question: q.question.or(""),
                                    header: q.header.owned(),
                                    options: q
                                        .options
                                        .items()
                                        .iter()
                                        .map(|o| {
                                            let o = o.0.clone().unwrap_or_default();
                                            QuestionOption {
                                                label: o.label.or(""),
                                                description: o.description.owned(),
                                                preview: o.preview.owned(),
                                            }
                                        })
                                        .collect(),
                                    multi_select: q.multi_select.is_true(),
                                    // Claude always offers "Other".
                                    allow_free_text: true,
                                    secret: false,
                                }
                            })
                            .collect(),
                        auto_resolution_ms: None,
                    };
                    card.decisions = vec![decision("deny", DecisionKind::Deny)];
                }
                "ExitPlanMode" => {
                    card.kind = RequestKind::PlanApproval {
                        plan: parsed.plan.or(""),
                    };
                    card.decisions = vec![decision("allow", DecisionKind::AllowOnce)];
                    card.decisions.extend(remember);
                    card.decisions.push(decision("deny", DecisionKind::Deny));
                }
                tool => {
                    let interactive = p.requires_user_interaction.is_true();
                    card.kind = RequestKind::ToolApproval {
                        tool: tool.into(),
                        display_name: p.display_name.owned(),
                        title: p.title.get().map(sanitize),
                        description: p.description.get().map(sanitize),
                        input: Some(input),
                        blocked_path: p.blocked_path.owned(),
                        reason: p.decision_reason.get().map(sanitize),
                        reason_type: p.decision_reason_type.owned(),
                        default_to_no: p.default_to_no.is_true(),
                        requires_user_interaction: interactive,
                        tool_use_id: p.tool_use_id.owned(),
                        agent_id: p.agent_id.owned(),
                        mcp_server: p.mcp_server.get().cloned(),
                    };
                    // A card that is itself the interaction surface cannot be answered with one tap.
                    card.decisions = if interactive {
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
                server: p.mcp_server_name.owned(),
                message: p.message.owned().or(p.title.owned()).unwrap_or_default(),
                mode: p.mode.or("form"),
                url: p.url.owned(),
                schema: p.requested_schema.get().cloned(),
                elicitation_id: p.elicitation_id.owned(),
            };
            card.decisions = vec![
                decision("accept", DecisionKind::AllowOnce),
                decision("decline", DecisionKind::Deny),
                decision("cancel", DecisionKind::Abort),
            ];
        }
        // We declare no supported dialog kinds, so the CLI fails closed; a stray dialog is shown
        // but is never answered.
        "request_user_dialog" => {
            card.kind = RequestKind::UserDialog {
                dialog_kind: p.dialog_kind.or(""),
                payload: p.payload.get().cloned(),
            };
            card.decisions.clear();
        }
        _ => (),
    }
    card
}

/// The record of a control request answered with an error (an SDK MCP message).
pub fn rejected(
    id: &str,
    subtype: &str,
    request: &OpaqueJson,
    raw: &OpaqueJson,
    thread: Option<&str>,
    now: i64,
) -> PendingRequest {
    PendingRequest {
        key: key(id),
        method: subtype.into(),
        kind: RequestKind::Unknown {
            method: subtype.into(),
            params: Some(request.clone()),
        },
        decisions: vec![],
        thread_id: thread.map(str::to_owned),
        status: RequestStatus::Rejected,
        received_at_ms: Some(now),
        raw: Some(raw.clone()),
        ..Default::default()
    }
}

#[derive(Debug)]
pub struct Answer {
    pub reply: UserReply,
    pub denied: bool,
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
#[serde(rename_all = "camelCase")]
struct Allow<'a> {
    behavior: &'static str,
    updated_input: &'a OpaqueJson,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_permissions: Option<[&'a OpaqueJson; 1]>,
}
#[derive(Serialize)]
struct Deny<'a> {
    behavior: &'static str,
    message: &'a str,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    interrupt: bool,
}
#[derive(Serialize)]
struct Elicit<'a> {
    action: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<&'a OpaqueObject>,
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct Frame {
    request: Json,
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct RequestInput {
    input: Json,
}

/// The `control_response` payload for the user's answer on `request`.
pub fn answer(request: &PendingRequest, response: &RequestResponse) -> Result<Answer> {
    let selected = |id: &str| {
        request
            .decisions
            .iter()
            .find(|d| d.id == id)
            .ok_or_else(|| not_offered(request, id))
    };
    let result = |value, summary: &str, denied| {
        Ok(Answer {
            reply: UserReply::Result {
                value,
                summary: summary.into(),
            },
            denied,
        })
    };
    let deny = |message: Option<&str>, interrupt: bool| {
        wire::encode(&Deny {
            behavior: "deny",
            message: message.unwrap_or("The user declined this action."),
            interrupt,
        })
    };
    let frame: Frame = wire::project_opt(request.raw.as_ref());
    let input = frame
        .request
        .object()
        .map(wire::project::<RequestInput>)
        .and_then(|r| r.input.get().cloned())
        .unwrap_or_else(wire::empty_object);
    match &request.kind {
        RequestKind::ToolApproval { .. } | RequestKind::PlanApproval { .. } => {
            let RequestResponse::Decide {
                decision_id,
                message,
            } = response
            else {
                return Err(ChatError::invalid("expected a decision"));
            };
            let d = selected(decision_id)?;
            if d.id == "allow" {
                result(
                    wire::encode(&Allow {
                        behavior: "allow",
                        updated_input: &input,
                        updated_permissions: None,
                    }),
                    &d.id,
                    false,
                )
            } else if d.id.starts_with("allowAlways:") {
                let null = OpaqueJson::default();
                result(
                    wire::encode(&Allow {
                        behavior: "allow",
                        updated_input: &input,
                        updated_permissions: Some([d.wire.as_ref().unwrap_or(&null)]),
                    }),
                    &d.id,
                    false,
                )
            } else {
                result(deny(message.as_deref(), d.id == "denyAndStop"), &d.id, true)
            }
        }
        RequestKind::UserInput { questions, .. } => match response {
            RequestResponse::Answer { answers } => {
                if let Some(unknown) = answers
                    .keys()
                    .find(|id| !questions.iter().any(|q| &q.id == *id))
                {
                    return Err(ChatError::invalid(format!("unknown question {unknown}")));
                }
                let mut fields: OpaqueObject = if wire::is_object(&input) {
                    wire::project(&input)
                } else {
                    OpaqueObject::new()
                };
                // Multi-select answers are comma-separated (AskUserQuestionOutput.answers).
                let joined: BTreeMap<&str, String> = answers
                    .iter()
                    .map(|(id, values)| (id.as_str(), values.join(", ")))
                    .collect();
                fields.insert("answers".into(), wire::encode(&joined));
                let input = wire::encode(&fields);
                result(
                    wire::encode(&Allow {
                        behavior: "allow",
                        updated_input: &input,
                        updated_permissions: None,
                    }),
                    "answered",
                    false,
                )
            }
            RequestResponse::Decide {
                decision_id,
                message,
            } => {
                selected(decision_id)?;
                result(deny(message.as_deref(), false), decision_id, true)
            }
            _ => Err(ChatError::invalid("expected answers")),
        },
        RequestKind::Elicitation { .. } => {
            let (action, content) = match response {
                RequestResponse::Elicit { action, content } => (action.as_str(), content.as_ref()),
                RequestResponse::Decide { decision_id, .. } => {
                    (selected(decision_id)?.id.as_str(), None)
                }
                _ => return Err(ChatError::invalid("expected an elicitation answer")),
            };
            if !matches!(action, "accept" | "decline" | "cancel") {
                return Err(ChatError::invalid(format!("invalid action {action}")));
            }
            result(
                wire::encode(&Elicit { action, content }),
                action,
                action != "accept",
            )
        }
        RequestKind::UserDialog { .. } => Err(ChatError::new(
            ErrorKind::State,
            "undeclared dialog kinds must not be answered",
        )),
        RequestKind::Unknown { .. } => match response {
            RequestResponse::RawResult { result: raw } => {
                if !wire::is_object(raw) {
                    return Err(ChatError::invalid("result must be an object"));
                }
                result(raw.clone(), "raw", false)
            }
            other => Ok(Answer {
                reply: UserReply::Error {
                    code: -32601,
                    message: match other {
                        RequestResponse::Reject { message } => message.clone(),
                        _ => "Rejected by user".into(),
                    },
                    summary: "rejected".into(),
                },
                denied: true,
            }),
        },
        _ => Err(ChatError::invalid("unsupported request kind for Claude")),
    }
}

/// An app-registered hook (answers `hook_callback`). It is never a user approval.
pub struct ClaudeHook {
    pub event: String,
    pub matcher: Option<String>,
    pub callback_id: String,
    pub handler: Box<dyn Fn(&OpaqueJson) -> std::result::Result<OpaqueJson, String> + Send + Sync>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HookMatcher<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    matcher: Option<&'a str>,
    hook_callback_ids: [&'a str; 1],
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Initialize<'a> {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    hooks: BTreeMap<&'a str, Vec<HookMatcher<'a>>>,
    prompt_suggestions: bool,
}
/// `initialize` request fields: our hooks, no prompt suggestions and no dialog kinds.
pub fn initialize(hooks: &[ClaudeHook]) -> OpaqueObject {
    let mut grouped: BTreeMap<&str, Vec<HookMatcher>> = BTreeMap::new();
    for h in hooks {
        grouped.entry(&h.event).or_default().push(HookMatcher {
            matcher: h.matcher.as_deref(),
            hook_callback_ids: [&h.callback_id],
        });
    }
    wire::project(&wire::encode(&Initialize {
        hooks: grouped,
        prompt_suggestions: false,
    }))
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Source<'a> {
    Base64 { media_type: &'a str, data: String },
    Url { url: &'a str },
}
#[derive(Serialize)]
struct Block<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    source: Source<'a>,
}
#[derive(Serialize)]
struct TextBlock<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    text: &'a str,
}
fn is_pdf(path: &str, mime: Option<&str>) -> bool {
    mime == Some("application/pdf") || path.to_ascii_lowercase().ends_with(".pdf")
}
pub fn mime_for(path: &str) -> Option<&'static str> {
    let ext = path.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("png") => Some("image/png"),
        Some("jpg" | "jpeg") => Some("image/jpeg"),
        Some("gif") => Some("image/gif"),
        Some("webp") => Some("image/webp"),
        Some("pdf") => Some("application/pdf"),
        _ => None,
    }
}

/// Neutral parts to Messages-API content blocks. Images and PDFs are inlined as base64; other
/// files are referenced as `@path` so the CLI loads them itself.
pub fn content(parts: &[UserPart], reader: &AttachmentReader, max: usize) -> Result<OpaqueJson> {
    let mentions = parts
        .iter()
        .filter_map(|p| match p {
            UserPart::File { path, mime_type } if !is_pdf(path, mime_type.as_deref()) => {
                Some(format!("@{path}"))
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ");
    let encode = |bytes: Vec<u8>| base64::engine::general_purpose::STANDARD.encode(bytes);
    let mut mention_added = mentions.is_empty();
    let mut out = Vec::new();
    for part in parts {
        match part {
            UserPart::Text { text } => {
                let text = if !mention_added {
                    mention_added = true;
                    format!("{text}\n\n{mentions}")
                } else {
                    text.clone()
                };
                out.push(wire::encode(&TextBlock { kind: "text", text: &text }));
            }
            UserPart::Image { path, mime_type } => out.push(wire::encode(&Block {
                kind: "image",
                source: Source::Base64 {
                    media_type: mime_type
                        .as_deref()
                        .or_else(|| mime_for(path))
                        .unwrap_or("image/png"),
                    data: encode(reader(path, max)?),
                },
            })),
            UserPart::File { path, mime_type } => {
                if is_pdf(path, mime_type.as_deref()) {
                    out.push(wire::encode(&Block {
                        kind: "document",
                        source: Source::Base64 {
                            media_type: "application/pdf",
                            data: encode(reader(path, max)?),
                        },
                    }));
                }
            }
            UserPart::InlineData {
                kind,
                media_type,
                base64,
            } => out.push(wire::encode(&Block {
                kind,
                source: Source::Base64 {
                    media_type: media_type.as_deref().unwrap_or("application/octet-stream"),
                    data: base64.clone(),
                },
            })),
            UserPart::ImageUrl { url } => out.push(wire::encode(&Block {
                kind: "image",
                source: Source::Url { url },
            })),
            UserPart::Reference { path, .. } => out.push(wire::encode(&TextBlock {
                kind: "text",
                text: &format!("@{path}"),
            })),
            UserPart::Unknown { raw } => out.push(raw.clone()),
        }
    }
    if !mention_added {
        out.push(wire::encode(&TextBlock { kind: "text", text: &mentions }));
    }
    Ok(wire::encode(&out))
}

#[derive(Serialize)]
struct UserMessage<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    uuid: &'a str,
    session_id: &'static str,
    parent_tool_use_id: Option<()>,
    message: MessageBody<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<&'a str>,
}
#[derive(Serialize)]
struct MessageBody<'a> {
    role: &'static str,
    content: &'a OpaqueJson,
}
/// A stream-json user prompt whose `uuid` is the client message ID (and later the turn ID).
pub fn user_message(client_message_id: &str, content: &OpaqueJson, priority: Option<&str>) -> OpaqueJson {
    wire::encode(&UserMessage {
        kind: "user",
        uuid: client_message_id,
        session_id: "",
        parent_tool_use_id: None,
        message: MessageBody {
            role: "user",
            content,
        },
        priority,
    })
}

/// Strips terminal control sequences from vendor text shown on a card.
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
