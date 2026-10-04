//! Codex `ThreadItem`, `UserInput` and `Turn` bodies to and from the neutral model. Missing fields get
//! defaults, as in the retained adapter.
use super::wire;
use crate::{
    error::{ChatError, Result},
    model::*,
    wire::{Json, Strings, project},
};
use serde::Serialize;

pub fn parse(json: &OpaqueJson, completed: bool) -> Item {
    if !crate::wire::is_object(json) {
        return Item::Unknown(UnknownItem {
            id: "unknown".into(),
            type_name: "invalid".into(),
            raw: Some(json.clone()).filter(|v| !v.0.is_null()),
            ..Default::default()
        });
    }
    let o: wire::Item = project(json);
    let raw = Some(json.clone());
    let id = o.id.or("unknown");
    let done = if completed {
        ItemStatus::Completed
    } else {
        ItemStatus::InProgress
    };
    let kind = o.kind.owned();
    match kind.as_deref() {
        Some("userMessage") => Item::UserMessage(UserMessageItem {
            id,
            parts: o.content.elements().iter().map(user_part).collect(),
            client_message_id: o.client_id.owned(),
            status: ItemStatus::Completed,
            raw,
            ..Default::default()
        }),
        Some("hookPrompt") => Item::Marker(MarkerItem {
            id,
            kind: MarkerKind::HookPrompt,
            text: o.fragments.0.as_ref().map(|f| {
                f.iter()
                    .filter_map(|f| f.text.owned())
                    .collect::<Vec<_>>()
                    .join("\n")
            }),
            raw,
            ..Default::default()
        }),
        Some("agentMessage") => Item::AgentMessage(AgentMessageItem {
            id,
            text: o.text.or(""),
            phase: phase(o.phase.get()),
            status: done,
            raw,
            ..Default::default()
        }),
        Some("functionCallOutput") => Item::ToolCall(ToolCallItem {
            id,
            kind: ToolKind::FunctionOutput,
            tool: join_present(&[o.namespace.owned(), o.name.owned()], "."),
            result: o.output.value().cloned(),
            result_text: o.output.string(),
            status: ItemStatus::Completed,
            raw,
            ..Default::default()
        }),
        Some("plan") => Item::Plan(PlanItem {
            id,
            text: o.text.or(""),
            status: done,
            raw,
            ..Default::default()
        }),
        Some("reasoning") => Item::Reasoning(ReasoningItem {
            id,
            summary: o.summary.0.clone(),
            content: o
                .content
                .get()
                .map(|c| project::<Strings>(c).0)
                .unwrap_or_default(),
            status: done,
            raw,
            ..Default::default()
        }),
        Some("commandExecution") => Item::Command(CommandItem {
            id,
            command: o.command.or(""),
            cwd: o.cwd.owned(),
            output: o.aggregated_output.or(""),
            exit_code: o.exit_code.int(),
            duration_ms: o.duration_ms.get(),
            process_id: o.process_id.owned(),
            actions: o
                .command_actions
                .items()
                .iter()
                .map(|a| CommandAction {
                    kind: a.kind.or("unknown"),
                    command: a.command.or(""),
                    path: a.path.owned(),
                    query: a.query.owned(),
                })
                .collect(),
            status: status(o.status.get(), done),
            raw,
            ..Default::default()
        }),
        Some("fileChange") => Item::FileChange(FileChangeItem {
            id,
            changes: changes(o.changes.get()),
            status: status(o.status.get(), done),
            raw,
            ..Default::default()
        }),
        Some("mcpToolCall") => Item::ToolCall(ToolCallItem {
            id,
            kind: ToolKind::Mcp,
            tool: o.tool.or(""),
            server: o.server.owned(),
            arguments: o.arguments.value().cloned(),
            result: o.result.object().cloned(),
            error: o.error.get().and_then(|e| e.message.owned()),
            duration_ms: o.duration_ms.get(),
            status: status(o.status.get(), done),
            raw,
            ..Default::default()
        }),
        Some("dynamicToolCall") => Item::ToolCall(ToolCallItem {
            id,
            kind: ToolKind::Dynamic,
            tool: join_present(&[o.namespace.owned(), o.tool.owned()], "."),
            arguments: o.arguments.value().cloned(),
            result: o.content_items.array().cloned(),
            duration_ms: o.duration_ms.get(),
            status: if o.success.is_false() {
                ItemStatus::Failed
            } else {
                status(o.status.get(), done)
            },
            raw,
            ..Default::default()
        }),
        Some("collabAgentToolCall") => Item::SubAgent(SubAgentItem {
            id,
            tool: o.tool.or("collab"),
            prompt: o.prompt.owned(),
            model: o.model.owned(),
            thread_ids: o.receiver_thread_ids.0.clone(),
            status: status(o.status.get(), done),
            raw,
            ..Default::default()
        }),
        Some("subAgentActivity") => Item::SubAgent(SubAgentItem {
            id,
            tool: "subAgent".into(),
            description: o.agent_path.owned(),
            thread_ids: o.agent_thread_id.owned().into_iter().collect(),
            status: match o.activity.get() {
                Some("completed") => ItemStatus::Completed,
                Some("interrupted") => ItemStatus::Incomplete,
                _ => done,
            },
            raw,
            ..Default::default()
        }),
        Some("webSearch") => Item::WebSearch(WebSearchItem {
            id,
            query: o.query.or(""),
            action: o.action.get().and_then(|a| a.kind.owned()),
            url: o.action.get().and_then(|a| a.url.owned()),
            results: o.results.value().cloned(),
            status: done,
            raw,
            ..Default::default()
        }),
        Some("imageView") => Item::Image(ImageItem {
            id,
            kind: ImageKind::View,
            path: o.path.owned(),
            raw,
            ..Default::default()
        }),
        Some("imageGeneration") => Item::Image(ImageItem {
            id,
            kind: ImageKind::Generated,
            path: o.saved_path.owned(),
            prompt: o.revised_prompt.owned(),
            status: if o.failure.object().is_some() {
                ItemStatus::Failed
            } else if o.status.get() == Some("completed") || completed {
                ItemStatus::Completed
            } else {
                ItemStatus::InProgress
            },
            raw,
            ..Default::default()
        }),
        Some("sleep") => Item::Marker(MarkerItem {
            id,
            kind: MarkerKind::Sleep,
            text: o.duration_ms.get().map(|v| format!("{v}ms")),
            raw,
            ..Default::default()
        }),
        Some("enteredReviewMode") => Item::Marker(MarkerItem {
            id,
            kind: MarkerKind::ReviewEntered,
            text: o.review.owned(),
            raw,
            ..Default::default()
        }),
        Some("exitedReviewMode") => Item::Marker(MarkerItem {
            id,
            kind: MarkerKind::ReviewExited,
            text: o.review.owned(),
            raw,
            ..Default::default()
        }),
        Some("contextCompaction") => Item::Marker(MarkerItem {
            id,
            kind: MarkerKind::Compaction,
            text: None,
            status: done,
            raw,
            ..Default::default()
        }),
        other => Item::Unknown(UnknownItem {
            id,
            type_name: other.unwrap_or("unknown").into(),
            status: done,
            raw,
            ..Default::default()
        }),
    }
}

fn join_present(parts: &[Option<String>], separator: &str) -> String {
    parts
        .iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>()
        .join(separator)
}

pub fn phase(value: Option<&str>) -> MessagePhase {
    match value {
        Some("commentary") => MessagePhase::Commentary,
        Some("final_answer") => MessagePhase::Final,
        None => MessagePhase::Unknown,
        // Unknown phases are treated as final.
        Some(_) => MessagePhase::Final,
    }
}

fn status(value: Option<&str>, default: ItemStatus) -> ItemStatus {
    match value {
        Some("inProgress") => ItemStatus::InProgress,
        Some("completed") => ItemStatus::Completed,
        Some("failed") => ItemStatus::Failed,
        Some("declined") => ItemStatus::Declined,
        Some("interrupted") => ItemStatus::Incomplete,
        _ => default,
    }
}

pub fn changes(json: Option<&OpaqueJson>) -> Vec<FileDelta> {
    let list: crate::wire::Arr<crate::wire::Obj<wire::Change>> =
        json.map(project).unwrap_or_default();
    list.items()
        .iter()
        .map(|c| {
            let c = c.0.clone().unwrap_or_default();
            let kind = c.kind.or_default();
            let move_path = kind.move_path.owned();
            FileDelta {
                path: c.path.or(""),
                kind: match kind.kind.get() {
                    Some("add") => FileChangeKind::Add,
                    Some("delete") => FileChangeKind::Delete,
                    _ if move_path.is_some() => FileChangeKind::Move,
                    _ => FileChangeKind::Update,
                },
                diff: c.diff.owned(),
                move_path,
            }
        })
        .collect()
}

pub fn user_part(json: &OpaqueJson) -> UserPart {
    let p: wire::Input = project(json);
    match p.kind.get() {
        Some("text") => UserPart::Text { text: p.text.or("") },
        Some("localImage") => UserPart::Image {
            path: p.path.or(""),
            mime_type: None,
        },
        Some("image") => match p.url.owned() {
            Some(url) => UserPart::ImageUrl { url },
            None => UserPart::Unknown { raw: json.clone() },
        },
        Some(kind @ ("skill" | "mention")) => UserPart::Reference {
            kind: kind.into(),
            name: p.name.or(""),
            path: p.path.or(""),
        },
        _ => UserPart::Unknown { raw: json.clone() },
    }
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Input<'a> {
    Text {
        text: &'a str,
        text_elements: [(); 0],
    },
    LocalImage {
        path: &'a str,
    },
    Image {
        url: &'a str,
    },
}
#[derive(Serialize)]
struct Reference<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    name: &'a str,
    path: &'a str,
}

/// Neutral parts to Codex `UserInput[]`. Images become `localImage`, read by the server in its own
/// environment; other files are referenced by path in text, since Codex has no generic file input.
pub fn user_input(parts: &[UserPart]) -> Result<OpaqueJson> {
    let mut out = Vec::new();
    for part in parts {
        out.push(match part {
            UserPart::Text { text } => crate::wire::encode(&Input::Text {
                text,
                text_elements: [],
            }),
            UserPart::Image { path, .. } => crate::wire::encode(&Input::LocalImage { path }),
            UserPart::ImageUrl { url } => crate::wire::encode(&Input::Image { url }),
            UserPart::File { path, .. } => crate::wire::encode(&Input::Text {
                text: &format!("Attached file: {path}"),
                text_elements: [],
            }),
            UserPart::Reference { kind, name, path } => {
                crate::wire::encode(&Reference { kind, name, path })
            }
            UserPart::InlineData { kind, .. } => {
                return Err(ChatError::new(
                    crate::error::ErrorKind::Vendor,
                    format!(
                        "Codex input does not accept inline {kind} data; save it to the workspace first"
                    ),
                ));
            }
            UserPart::Unknown { raw } => raw.clone(),
        });
    }
    Ok(crate::wire::encode(&out))
}

pub fn turn_status(value: Option<&str>) -> TurnStatus {
    match value {
        Some("completed") => TurnStatus::Completed,
        Some("interrupted") => TurnStatus::Interrupted,
        Some("failed") => TurnStatus::Failed,
        Some("inProgress") => TurnStatus::Running,
        _ => TurnStatus::Completed,
    }
}

pub fn turn_error(json: Option<&OpaqueJson>) -> Option<TurnError> {
    let json = json.filter(|v| crate::wire::is_object(v))?;
    let o: wire::TurnError = project(json);
    let code = o
        .codex_error_info
        .string()
        .or_else(|| o.codex_error_info.keys().into_iter().next());
    Some(TurnError {
        message: o.message.or("error"),
        code,
        detail: o.additional_details.owned(),
        raw: Some(json.clone()),
    })
}

pub fn plan_steps(json: Option<&OpaqueJson>) -> Vec<PlanStep> {
    let list: crate::wire::Arr<crate::wire::Obj<wire::PlanStep>> =
        json.map(project).unwrap_or_default();
    list.items()
        .iter()
        .map(|s| {
            let s = s.0.clone().unwrap_or_default();
            PlanStep {
                text: s.step.or(""),
                status: match s.status.get() {
                    Some("completed") => PlanStepStatus::Completed,
                    Some("inProgress") => PlanStepStatus::InProgress,
                    _ => PlanStepStatus::Pending,
                },
            }
        })
        .collect()
}

/// A history turn (`thread/turns/list`) as a finished neutral turn.
pub fn turn(json: &OpaqueJson) -> Turn {
    let o: wire::Turn = project(json);
    let status = turn_status(o.status.get());
    let items: Vec<Item> = o.items.elements().iter().map(|i| parse(i, true)).collect();
    let client_message_id = items.iter().find_map(|i| match i {
        Item::UserMessage(u) => Some(u.client_message_id.clone()),
        _ => None,
    });
    let items = if status == TurnStatus::Running {
        items
    } else {
        items
            .into_iter()
            .map(|mut i| {
                if i.status() == ItemStatus::InProgress {
                    i.set_status(ItemStatus::Incomplete);
                }
                i
            })
            .collect()
    };
    Turn {
        id: o.id.or("unknown"),
        client_message_id: client_message_id.flatten(),
        status,
        items,
        error: turn_error(o.error.get()),
        started_at_ms: o.started_at.get().map(|v| v * 1000),
        completed_at_ms: o.completed_at.get().map(|v| v * 1000),
        duration_ms: o.duration_ms.get(),
        bound: true,
        ..Default::default()
    }
}

/// Stable text for an approval policy: the string enum, or the encoded granular object.
pub fn policy_text(json: &Json) -> Option<String> {
    let value = json.get()?;
    if crate::wire::is_object(value) || crate::wire::is_array(value) {
        Some(crate::wire::text(value))
    } else {
        json.string()
    }
}
