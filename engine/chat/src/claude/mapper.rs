//! Stateful mapping of one Claude Code session's stdout (stream-json) to neutral events.
//!
//! Claude is content-block centric: `stream_event` carries raw Messages-API deltas, then one
//! `assistant` frame per completed block (same `message.id`, no block index), `user` frames carry
//! our echoed prompt and `tool_result`s, and `result` ends the turn. Item IDs are
//! `<message id>:<index>` for text and thinking, the `tool_use` ID for tools and the prompt UUID
//! for user messages. The turn ID is the client UUID of the prompt (also
//! `command_lifecycle.command_uuid` and `result.user_message_uuid`).
use super::wire::{self as cw, Frame};
use crate::{
    model::*,
    wire::{self, Arr, Json, Obj, project, project_opt},
};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

const B: BackendKind = BackendKind::Claude;

struct Block {
    index: i32,
    kind: String,
    item_id: String,
    completed: bool,
}
#[derive(Clone)]
struct Tool {
    id: String,
    name: String,
    input: Option<OpaqueJson>,
    turn_id: String,
    parent_id: Option<String>,
}

pub struct ClaudeMapper {
    session_id: Option<String>,
    current_turn: Option<String>,
    started: HashSet<String>,
    finished: HashSet<String>,
    blocks: HashMap<String, Vec<Block>>,
    streaming: HashMap<Option<String>, String>,
    tools: HashMap<String, Tool>,
    denied: HashSet<String>,
    /// Last assistant/user UUID per turn: the fork anchor for `--resume-session-at`.
    last_uuid: HashMap<String, String>,
    marker_seq: i64,
}

fn notice(level: NoticeLevel, message: impl Into<String>, code: Option<String>, retry: bool, raw: &OpaqueJson) -> Notice {
    Notice {
        level,
        message: message.into(),
        code,
        will_retry: retry,
        detail: None,
        raw: Some(raw.clone()),
    }
}
fn join(parts: &[Option<String>], separator: &str) -> String {
    parts.iter().flatten().cloned().collect::<Vec<_>>().join(separator)
}
fn enum_name<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string()
}
/// Kotlin `String.lines()`: splits on CRLF, LF or CR and always yields at least one line.
fn lines(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    loop {
        match rest.find(['\r', '\n']) {
            Some(i) => {
                out.push(&rest[..i]);
                let skip = if rest[i..].starts_with("\r\n") { 2 } else { 1 };
                rest = &rest[i + skip..];
            }
            None => {
                out.push(rest);
                return out;
            }
        }
    }
}
fn between<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = text.find(open)? + open.len();
    let end = text[start..].find(close)?;
    Some(&text[start..start + end])
}
/// The visible text of a local slash command echoed as a user frame.
fn local_command(text: &str) -> Option<String> {
    let mut from = 0;
    while let Some(i) = text[from..].find("<local-command-") {
        let at = from + i + "<local-command-".len();
        for kind in ["stdout", "stderr"] {
            let open = format!("{kind}>");
            if text[at..].starts_with(&open) {
                let body = at + open.len();
                if let Some(end) = text[body..].find(&format!("</local-command-{kind}>")) {
                    return Some(text[body..body + end].trim().to_string());
                }
            }
        }
        from = at;
    }
    let name = between(text, "<command-name>", "</command-name>")?;
    let args = between(text, "<command-args>", "</command-args>");
    Some(join(
        &[
            Some(name.trim().to_string()),
            args.map(|a| a.trim().to_string()).filter(|a| !a.is_empty()),
        ],
        " ",
    ))
}
fn result_text(content: Option<&OpaqueJson>) -> Option<String> {
    let content = content?;
    let json = Json(Some(content.clone()));
    if json.array().is_some() {
        let text = json
            .elements()
            .iter()
            .filter_map(|b| project::<cw::Block>(b).text.owned())
            .collect::<Vec<_>>()
            .join("\n");
        Some(text).filter(|t| !t.is_empty())
    } else if json.object().is_some() {
        Some(wire::text(content))
    } else {
        json.string()
    }
}
#[derive(Serialize)]
struct TextBlock<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    text: &'a OpaqueJson,
}

impl ClaudeMapper {
    pub fn new(session_id: Option<&str>) -> Self {
        Self {
            session_id: session_id.map(str::to_owned),
            current_turn: None,
            started: HashSet::new(),
            finished: HashSet::new(),
            blocks: HashMap::new(),
            streaming: HashMap::new(),
            tools: HashMap::new(),
            denied: HashSet::new(),
            last_uuid: HashMap::new(),
            marker_seq: 0,
        }
    }
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }
    pub fn last_message_uuid(&self, turn: &str) -> Option<String> {
        self.last_uuid.get(turn).cloned()
    }
    pub fn running_turn(&self) -> Option<String> {
        self.current_turn.clone()
    }
    pub fn is_started(&self, turn: &str) -> bool {
        self.started.contains(turn)
    }
    pub fn is_finished(&self, turn: &str) -> bool {
        self.finished.contains(turn)
    }
    fn next_marker(&mut self, uuid: Option<String>) -> String {
        match uuid {
            Some(u) => format!("marker-{u}"),
            None => {
                let n = self.marker_seq;
                self.marker_seq += 1;
                format!("marker-{n}")
            }
        }
    }

    /// Transcript mode: a prompt entry opens a turn (transcripts have no `command_lifecycle`).
    pub fn begin_turn(&mut self, turn: &str) -> Vec<AgentEvent> {
        let Some(t) = self.session_id.clone() else {
            return vec![];
        };
        self.current_turn = Some(turn.into());
        self.started.insert(turn.into());
        vec![AgentEvent::TurnStarted {
            backend: B,
            thread_id: t,
            turn_id: turn.into(),
            client_message_id: Some(turn.into()),
            at_ms: None,
        }]
    }

    /// Transcript mode and process exit: closes `turn` unless a `result` already did.
    pub fn end_turn(&mut self, turn: &str, status: TurnStatus, error: Option<TurnError>) -> Vec<AgentEvent> {
        let Some(t) = self.session_id.clone() else {
            return vec![];
        };
        if !self.finished.insert(turn.into()) {
            return vec![];
        }
        if self.current_turn.as_deref() == Some(turn) {
            self.current_turn = None;
        }
        vec![if self.started.contains(turn) {
            completed(t, turn.into(), status, error)
        } else {
            AgentEvent::TurnCancelled {
                backend: B,
                thread_id: t,
                turn_id: turn.into(),
            }
        }]
    }

    /// An app-registered hook ran (`hook_callback`): a marker in the running turn.
    pub fn hook_marker(&mut self, callback_id: &str, input: &OpaqueJson) -> Vec<AgentEvent> {
        let Some(t) = self.session_id.clone() else {
            return vec![];
        };
        let i: cw::HookInput = project(input);
        let text = join(
            &[i.hook_event_name.owned(), i.tool_name.owned(), Some(callback_id.into())],
            " · ",
        );
        let Some(turn) = self.current_turn.clone() else {
            return vec![AgentEvent::ThreadNotice {
                backend: B,
                thread_id: t,
                notice: notice(NoticeLevel::Info, format!("Hook: {text}"), Some("hook_callback".into()), false, input),
            }];
        };
        let id = format!("hook-{}", self.marker_seq);
        self.marker_seq += 1;
        vec![AgentEvent::ItemCompleted {
            backend: B,
            thread_id: t,
            turn_id: turn,
            item: Item::Marker(MarkerItem {
                id,
                kind: MarkerKind::Hook,
                text: Some(text),
                raw: Some(input.clone()),
                ..Default::default()
            }),
        }]
    }

    /// The user denied `tool_use_id`, so its error result renders as declined.
    pub fn mark_denied(&mut self, tool_use_id: &str) {
        self.denied.insert(tool_use_id.into());
    }

    fn turn_of(&self, f: &Frame) -> Option<String> {
        f.user_message_uuid.owned().or_else(|| self.current_turn.clone())
    }

    pub fn map(&mut self, raw: &OpaqueJson) -> Vec<AgentEvent> {
        let f: Frame = project(raw);
        if let Some(s) = f.session_id.get().filter(|s| !s.is_empty()) {
            if self.session_id.is_none() {
                self.session_id = Some(s.into());
            }
        }
        let unknown = |kind: &str, thread: Option<String>| AgentEvent::Unknown {
            backend: B,
            kind: kind.into(),
            thread_id: thread,
            raw: raw.clone(),
        };
        let Some(t) = self.session_id.clone() else {
            return vec![unknown(f.kind.get().unwrap_or("?"), None)];
        };
        match f.kind.get() {
            Some("system") => self.system(t, &f, raw),
            Some("command_lifecycle") => self.lifecycle(t, &f, raw),
            Some("stream_event") => self.stream_event(t, &f, raw),
            Some("assistant") => self.assistant(t, &f, raw),
            Some("user") => self.user(&t, raw, true),
            Some("result") => self.result(t, &f, raw),
            Some("rate_limit_event") => vec![AgentEvent::RateLimitsChanged {
                backend: B,
                limits: RateLimitState {
                    limits: rate_limit(f.rate_limit_info.get()).into_iter().collect(),
                    ..Default::default()
                },
                merge: true,
            }],
            Some("auth_status") => vec![AgentEvent::LoginChanged {
                backend: B,
                flow: Some(if f.is_authenticating.is_true() || f.error.get().is_some() {
                    LoginFlow::Progress {
                        login_id: None,
                        output: project_opt::<wire::Strings>(f.output.get()).0,
                        error: f.error.owned(),
                    }
                } else {
                    LoginFlow::Completed {
                        login_id: None,
                        success: true,
                        error: None,
                    }
                }),
            }],
            Some("tool_progress") => match f.tool_use_id.get().and_then(|id| self.tools.get(id)) {
                None => vec![unknown("tool_progress", Some(t))],
                Some(tool) => vec![AgentEvent::ItemUpdated {
                    backend: B,
                    thread_id: t,
                    turn_id: tool.turn_id.clone(),
                    item_id: tool.id.clone(),
                    delta: ItemDelta::ToolProgress {
                        message: format!("{}s", f.elapsed_time_seconds.0.map(|s| s as i64).unwrap_or(0)),
                    },
                }],
            },
            other => vec![unknown(other.unwrap_or("?"), Some(t))],
        }
    }

    // ---------------------------------------------------------------- system

    fn system(&mut self, t: String, f: &Frame, raw: &OpaqueJson) -> Vec<AgentEvent> {
        let subtype = f.subtype.owned();
        let turn = self.turn_of(f);
        let note = |level: NoticeLevel, message: String, code: Option<String>, retry: bool| match &turn {
            Some(u) => AgentEvent::TurnNotice {
                backend: B,
                thread_id: t.clone(),
                turn_id: Some(u.clone()),
                notice: notice(level, message, code, retry, raw),
            },
            None => AgentEvent::ThreadNotice {
                backend: B,
                thread_id: t.clone(),
                notice: notice(level, message, code, retry, raw),
            },
        };
        let info = |message: String| note(NoticeLevel::Info, message, subtype.clone(), false);
        let marker = |me: &mut Self, kind: MarkerKind, text: Option<String>| match &turn {
            Some(u) => AgentEvent::ItemCompleted {
                backend: B,
                thread_id: t.clone(),
                turn_id: u.clone(),
                item: Item::Marker(MarkerItem {
                    id: me.next_marker(f.uuid.owned()),
                    kind,
                    text,
                    raw: Some(raw.clone()),
                    ..Default::default()
                }),
            },
            None => AgentEvent::ThreadNotice {
                backend: B,
                thread_id: t.clone(),
                notice: notice(
                    NoticeLevel::Info,
                    text.unwrap_or_else(|| enum_name(&kind)),
                    subtype.clone(),
                    false,
                    raw,
                ),
            },
        };
        let message = f.message.string();
        match subtype.as_deref() {
            Some("init") => {
                self.session_id = f.session_id.owned().or(self.session_id.clone());
                vec![AgentEvent::ThreadUpserted {
                    backend: B,
                    thread_id: self.session_id.clone().unwrap_or(t.clone()),
                    title: None,
                    preview: None,
                    cwd: f.cwd.owned(),
                    path: None,
                    forked_from: None,
                    ephemeral: None,
                    run_state: None,
                    settings: Some(ThreadSettings {
                        model: f.model.owned(),
                        approval_policy: f.permission_mode.owned(),
                        raw: Some(raw.clone()),
                        ..Default::default()
                    }),
                    created_at_sec: None,
                    updated_at_sec: None,
                    raw: Some(raw.clone()),
                }]
            }
            Some("status") => {
                let mut out = Vec::new();
                if let Some(mode) = f.permission_mode.owned() {
                    out.push(AgentEvent::ThreadSettingsChanged {
                        backend: B,
                        thread_id: t.clone(),
                        settings: ThreadSettings {
                            approval_policy: Some(mode),
                            ..Default::default()
                        },
                    });
                }
                if f.status.get() == Some("compacting") {
                    out.push(info("Compacting context".into()));
                }
                out
            }
            Some("session_state_changed") => vec![AgentEvent::ThreadStatusChanged {
                backend: B,
                thread_id: t.clone(),
                run_state: match f.state.get() {
                    Some("running") => RunState::Running,
                    Some("requires_action") => RunState::WaitingApproval,
                    _ => RunState::Idle,
                },
            }],
            Some("thinking_tokens") => match &turn {
                Some(u) => vec![AgentEvent::TurnProgress {
                    backend: B,
                    thread_id: t.clone(),
                    turn_id: u.clone(),
                    thinking_tokens: f.estimated_tokens.get(),
                }],
                None => vec![],
            },
            Some("api_retry") => vec![note(
                NoticeLevel::Warning,
                format!(
                    "Retrying request ({}/{})",
                    f.attempt.get().map_or("?".into(), |v| v.to_string()),
                    f.max_retries.get().map_or("?".into(), |v| v.to_string())
                ),
                f.error.owned().or_else(|| f.error_status.get().map(|v| v.to_string())),
                true,
            )],
            Some("compact_boundary") => {
                let m = f.compact_metadata.or_default();
                let text = join(
                    &[
                        m.trigger.owned(),
                        m.pre_tokens.get().map(|v| v.to_string()),
                        m.post_tokens.get().map(|v| format!("→ {v}")),
                    ],
                    " ",
                );
                vec![marker(self, MarkerKind::Compaction, Some(text))]
            }
            Some(s @ ("hook_started" | "hook_progress" | "hook_response")) => {
                let text = join(
                    &[
                        f.hook_event.owned().or(f.hook_event_name.owned()),
                        f.hook_name.owned(),
                        f.outcome.owned(),
                    ],
                    " · ",
                );
                let text = if text.is_empty() { s.to_string() } else { text };
                vec![marker(self, MarkerKind::Hook, Some(text))]
            }
            Some("local_command_output") => {
                let text = f.content.string().or(f.output.string());
                vec![marker(self, MarkerKind::LocalCommand, text)]
            }
            Some("permission_denied") => {
                let mut out = Vec::new();
                if let Some(id) = f.tool_use_id.owned() {
                    self.denied.insert(id.clone());
                    out.push(AgentEvent::ItemDeclined {
                        backend: B,
                        thread_id: t.clone(),
                        turn_id: self.tools.get(&id).map(|tool| tool.turn_id.clone()),
                        item_id: id,
                    });
                }
                out.push(note(
                    NoticeLevel::Warning,
                    message.unwrap_or_else(|| "Permission denied".into()),
                    subtype.clone(),
                    false,
                ));
                out
            }
            Some(s @ ("notification" | "informational")) => vec![info(
                message
                    .or(f.text.owned())
                    .or(f.title.owned())
                    .unwrap_or_else(|| s.into()),
            )],
            Some(s @ ("model_refusal_fallback" | "model_refusal_no_fallback")) => vec![note(
                NoticeLevel::Warning,
                message.unwrap_or_else(|| {
                    format!(
                        "Model refused; {}",
                        if s == "model_refusal_fallback" { "switched model" } else { "no fallback" }
                    )
                }),
                subtype.clone(),
                false,
            )],
            Some(s @ ("task_started" | "task_progress" | "task_updated" | "task_notification")) => {
                let tool = f.tool_use_id.get().and_then(|id| self.tools.get(id));
                let text = join(&[f.description.owned(), f.summary.owned(), f.status.owned()], " · ");
                let text = if text.is_empty() { s.to_string() } else { text };
                match tool {
                    Some(tool) => vec![AgentEvent::ItemUpdated {
                        backend: B,
                        thread_id: t.clone(),
                        turn_id: tool.turn_id.clone(),
                        item_id: tool.id.clone(),
                        delta: ItemDelta::ToolProgress { message: text },
                    }],
                    None => vec![AgentEvent::ThreadNotice {
                        backend: B,
                        thread_id: t.clone(),
                        notice: notice(NoticeLevel::Info, format!("Background task: {text}"), subtype.clone(), false, raw),
                    }],
                }
            }
            other => vec![AgentEvent::Unknown {
                backend: B,
                kind: format!("system/{}", other.unwrap_or("null")),
                thread_id: Some(t.clone()),
                raw: raw.clone(),
            }],
        }
    }

    // ---------------------------------------------------------------- turns

    fn lifecycle(&mut self, t: String, f: &Frame, raw: &OpaqueJson) -> Vec<AgentEvent> {
        let Some(id) = f.command_uuid.owned() else {
            return vec![AgentEvent::Unknown {
                backend: B,
                kind: "command_lifecycle".into(),
                thread_id: Some(t),
                raw: raw.clone(),
            }];
        };
        match f.state.get() {
            Some("started") => {
                self.current_turn = Some(id.clone());
                self.started.insert(id.clone());
                vec![AgentEvent::TurnStarted {
                    backend: B,
                    thread_id: t,
                    turn_id: id.clone(),
                    client_message_id: Some(id),
                    at_ms: None,
                }]
            }
            Some("completed") => {
                if !self.finished.insert(id.clone()) {
                    return vec![];
                }
                vec![completed(t, id, TurnStatus::Completed, None)]
            }
            Some("cancelled") => {
                if !self.finished.insert(id.clone()) {
                    return vec![];
                }
                if self.current_turn.as_ref() == Some(&id) {
                    self.current_turn = None;
                }
                vec![if self.started.contains(&id) {
                    completed(t, id, TurnStatus::Interrupted, None)
                } else {
                    AgentEvent::TurnCancelled {
                        backend: B,
                        thread_id: t,
                        turn_id: id,
                    }
                }]
            }
            // queued: the local TurnSubmitted already shows it.
            _ => vec![],
        }
    }

    fn result(&mut self, t: String, f: &Frame, raw: &OpaqueJson) -> Vec<AgentEvent> {
        let Some(turn) = self.turn_of(f) else {
            return vec![AgentEvent::Unknown {
                backend: B,
                kind: "result".into(),
                thread_id: Some(t),
                raw: raw.clone(),
            }];
        };
        self.finished.insert(turn.clone());
        if self.current_turn.as_ref() == Some(&turn) {
            self.current_turn = None;
        }
        let terminal = f.terminal_reason.owned();
        let status = if terminal.as_deref().is_some_and(|r| r.starts_with("aborted")) {
            TurnStatus::Interrupted
        } else if f.is_error.is_true() || f.subtype.get() != Some("success") {
            TurnStatus::Failed
        } else {
            TurnStatus::Completed
        };
        let error = (status == TurnStatus::Failed).then(|| TurnError {
            message: f.result.owned().unwrap_or_else(|| {
                let joined = f.errors.0.join("\n");
                if joined.is_empty() {
                    f.subtype.or("error")
                } else {
                    joined
                }
            }),
            code: terminal.clone().or(f.subtype.owned()),
            detail: f.api_error_status.value().map(wire::text),
            raw: Some(raw.clone()),
        });
        let u: cw::Usage = project_opt(f.usage.object());
        let mut out = Vec::new();
        for d in f.permission_denials.items() {
            if let Some(id) = d.get().and_then(|d| d.tool_use_id.owned()) {
                self.denied.insert(id.clone());
                out.push(AgentEvent::ItemDeclined {
                    backend: B,
                    thread_id: t.clone(),
                    turn_id: Some(turn.clone()),
                    item_id: id,
                });
            }
        }
        let input = u.input_tokens.get().unwrap_or(0);
        let output = u.output_tokens.get().unwrap_or(0);
        out.push(AgentEvent::TurnCompleted {
            backend: B,
            thread_id: t,
            turn_id: turn,
            status,
            error,
            items: vec![],
            duration_ms: f.duration_ms.get(),
            usage: Some(TokenUsage {
                input_tokens: input,
                cached_input_tokens: u.cache_read_input_tokens.get().unwrap_or(0),
                output_tokens: output,
                reasoning_tokens: u
                    .output_tokens_details
                    .get()
                    .and_then(|d| d.thinking_tokens.get())
                    .unwrap_or(0),
                total_tokens: input + output,
                context_window: None,
                cost_usd: f.total_cost_usd.0,
                raw: f.usage.get().cloned(),
            }),
            at_ms: None,
        });
        out
    }

    // ---------------------------------------------------------------- assistant content

    fn stream_event(&mut self, t: String, f: &Frame, raw: &OpaqueJson) -> Vec<AgentEvent> {
        let Some(event) = f.event.object() else {
            return vec![];
        };
        let parent = f.parent_tool_use_id.owned();
        let Some(turn) = self.turn_of(f) else {
            return vec![AgentEvent::Unknown {
                backend: B,
                kind: "stream_event".into(),
                thread_id: Some(t),
                raw: raw.clone(),
            }];
        };
        let e: cw::StreamEvent = project(event);
        match e.kind.get() {
            Some("message_start") => {
                if let Some(id) = e.message.get().and_then(|m| m.id.owned()) {
                    self.streaming.insert(parent, id.clone());
                    self.blocks.entry(id).or_default();
                }
                vec![]
            }
            Some("content_block_start") => {
                let (Some(message), Some(index), Some(block)) = (
                    self.streaming.get(&parent).cloned(),
                    e.index.int(),
                    e.content_block.object(),
                ) else {
                    return vec![];
                };
                self.start_block(&t, &turn, &message, index, block, parent)
            }
            Some("content_block_delta") => {
                let (Some(message), Some(index)) = (self.streaming.get(&parent).cloned(), e.index.int()) else {
                    return vec![];
                };
                let Some(item) = self
                    .blocks
                    .get(&message)
                    .and_then(|l| l.iter().find(|b| b.index == index))
                    .map(|b| b.item_id.clone())
                else {
                    return vec![];
                };
                let d = e.delta.or_default();
                let delta = match d.kind.get() {
                    Some("text_delta") => ItemDelta::AgentText { text: d.text.or("") },
                    Some("thinking_delta") => ItemDelta::ReasoningText {
                        index: 0,
                        text: d.thinking.or(""),
                    },
                    Some("input_json_delta") => ItemDelta::ToolArguments {
                        partial_json: d.partial_json.or(""),
                    },
                    // signature_delta, citations_delta: not rendered.
                    _ => return vec![],
                };
                vec![AgentEvent::ItemUpdated {
                    backend: B,
                    thread_id: t,
                    turn_id: turn,
                    item_id: item,
                    delta,
                }]
            }
            // content_block_stop, message_delta, message_stop: completion comes from
            // `assistant` and `result`.
            _ => vec![],
        }
    }

    fn start_block(
        &mut self,
        t: &str,
        turn: &str,
        message: &str,
        index: i32,
        block: &OpaqueJson,
        parent: Option<String>,
    ) -> Vec<AgentEvent> {
        let b: cw::Block = project(block);
        let kind = b.kind.or("unknown");
        if self
            .blocks
            .get(message)
            .is_some_and(|l| l.iter().any(|x| x.index == index))
        {
            return vec![];
        }
        let block_id = format!("{message}:{index}");
        let item = match kind.as_str() {
            "text" => Item::AgentMessage(AgentMessageItem {
                id: block_id,
                phase: MessagePhase::Unknown,
                parent_id: parent,
                ..Default::default()
            }),
            "thinking" => Item::Reasoning(ReasoningItem {
                id: block_id,
                parent_id: parent,
                ..Default::default()
            }),
            "redacted_thinking" => Item::Reasoning(ReasoningItem {
                id: block_id,
                redacted: true,
                parent_id: parent,
                ..Default::default()
            }),
            "tool_use" | "server_tool_use" | "mcp_tool_use" => {
                let id = b.id.owned().unwrap_or(block_id);
                let name = b.name.or("");
                self.tools.insert(
                    id.clone(),
                    Tool {
                        id: id.clone(),
                        name: name.clone(),
                        input: b.input.get().cloned(),
                        turn_id: turn.into(),
                        parent_id: parent.clone(),
                    },
                );
                tool_item(&id, &name, None, parent, ItemStatus::InProgress, Some(block.clone()))
            }
            _ => Item::Unknown(UnknownItem {
                id: block_id,
                type_name: kind.clone(),
                status: ItemStatus::InProgress,
                parent_id: parent,
                raw: Some(block.clone()),
            }),
        };
        self.blocks.entry(message.into()).or_default().push(Block {
            index,
            kind,
            item_id: item.id().into(),
            completed: false,
        });
        vec![AgentEvent::ItemStarted {
            backend: B,
            thread_id: t.into(),
            turn_id: turn.into(),
            item,
        }]
    }

    fn assistant(&mut self, t: String, f: &Frame, raw: &OpaqueJson) -> Vec<AgentEvent> {
        let unknown = || {
            vec![AgentEvent::Unknown {
                backend: B,
                kind: "assistant".into(),
                thread_id: Some(t.clone()),
                raw: raw.clone(),
            }]
        };
        let Some(message) = f.message.object() else {
            return unknown();
        };
        let Some(turn) = self.turn_of(f) else {
            return unknown();
        };
        let parent = f.parent_tool_use_id.owned();
        if let Some(u) = f.uuid.owned() {
            self.last_uuid.insert(turn.clone(), u);
        }
        let m: cw::Message = project(message);
        let contents = m.content.elements();
        let text = m.content.array().map(|_| {
            contents
                .iter()
                .filter_map(|c| project::<cw::Block>(c).text.owned())
                .collect::<Vec<_>>()
                .join("\n")
        });
        if let (Some(error), true) = (f.error.owned(), f.is_api_error_message.is_true()) {
            let mut out = vec![AgentEvent::TurnNotice {
                backend: B,
                thread_id: t.clone(),
                turn_id: Some(turn),
                notice: notice(NoticeLevel::Error, text.unwrap_or(error.clone()), Some(error.clone()), false, raw),
            }];
            if error == "authentication_failed" {
                out.push(AgentEvent::AccountChanged {
                    backend: B,
                    account: AccountState {
                        state: LoginState::LoggedOut,
                        raw: Some(raw.clone()),
                        ..Default::default()
                    },
                });
            }
            return out;
        }
        let message_id = m.id.owned().or(f.uuid.owned()).unwrap_or_else(|| "msg".into());
        let aborted = f.aborted.is_true() || f.aborted_mid_stream.is_true();
        let mut out = Vec::new();
        self.blocks.entry(message_id.clone()).or_default();
        for content in &contents {
            if !wire::is_object(content) {
                continue;
            }
            let b: cw::Block = project(content);
            let kind = b.kind.or("unknown");
            let wanted = category(&kind);
            let open = self.blocks[&message_id]
                .iter()
                .position(|x| !x.completed && category(&x.kind) == wanted);
            let position = match open {
                Some(p) => p,
                None => {
                    let list = &self.blocks[&message_id];
                    let index = f
                        .api_block_index
                        .int()
                        .filter(|i| list.iter().all(|x| x.index != *i))
                        .unwrap_or_else(|| list.iter().map(|x| x.index).max().unwrap_or(-1) + 1);
                    out.extend(self.start_block(&t, &turn, &message_id, index, content, parent.clone()));
                    self.blocks[&message_id]
                        .iter()
                        .position(|x| x.index == index)
                        .expect("started block")
                }
            };
            let entry = &mut self.blocks.get_mut(&message_id).unwrap()[position];
            entry.completed = true;
            let item_id = entry.item_id.clone();
            let status = if aborted { ItemStatus::Incomplete } else { ItemStatus::Completed };
            let done = |item: Item| AgentEvent::ItemCompleted {
                backend: B,
                thread_id: t.clone(),
                turn_id: turn.clone(),
                item,
            };
            out.push(match kind.as_str() {
                "text" => done(Item::AgentMessage(AgentMessageItem {
                    id: item_id,
                    text: b.text.or(""),
                    phase: MessagePhase::Unknown,
                    status,
                    parent_id: parent.clone(),
                    raw: Some(content.clone()),
                })),
                "thinking" => done(Item::Reasoning(ReasoningItem {
                    id: item_id,
                    content: vec![b.thinking.or("")],
                    status,
                    parent_id: parent.clone(),
                    raw: Some(content.clone()),
                    ..Default::default()
                })),
                "redacted_thinking" => done(Item::Reasoning(ReasoningItem {
                    id: item_id,
                    redacted: true,
                    status,
                    parent_id: parent.clone(),
                    raw: Some(content.clone()),
                    ..Default::default()
                })),
                "tool_use" | "server_tool_use" | "mcp_tool_use" => {
                    let id = b.id.owned().unwrap_or(item_id);
                    let name = b.name.or("");
                    let tool = self.tools.entry(id.clone()).or_insert(Tool {
                        id: id.clone(),
                        name: name.clone(),
                        input: None,
                        turn_id: turn.clone(),
                        parent_id: parent.clone(),
                    });
                    tool.input = b.input.get().cloned();
                    // Arguments are final; the tool itself finishes with its tool_result.
                    AgentEvent::ItemStarted {
                        backend: B,
                        thread_id: t.clone(),
                        turn_id: turn.clone(),
                        item: tool_item(&id, &name, tool.input.as_ref(), parent.clone(), ItemStatus::InProgress, Some(content.clone())),
                    }
                }
                "web_search_tool_result" | "web_fetch_tool_result" => {
                    match b.tool_use_id.get().and_then(|id| self.tools.get(id)).cloned() {
                        Some(tool) => AgentEvent::ItemCompleted {
                            backend: B,
                            thread_id: t.clone(),
                            turn_id: tool.turn_id.clone(),
                            item: finished_tool(&tool, b.content.get(), None, false, None),
                        },
                        None => done(Item::Unknown(UnknownItem {
                            id: item_id,
                            type_name: kind.clone(),
                            status: ItemStatus::Completed,
                            parent_id: parent.clone(),
                            raw: Some(content.clone()),
                        })),
                    }
                }
                _ => done(Item::Unknown(UnknownItem {
                    id: item_id,
                    type_name: kind.clone(),
                    status: ItemStatus::Completed,
                    parent_id: parent.clone(),
                    raw: Some(content.clone()),
                })),
            });
        }
        out
    }

    // ---------------------------------------------------------------- user frames

    /// `live` means a stdout frame rather than a transcript entry.
    pub fn user(&mut self, t: &str, raw: &OpaqueJson, live: bool) -> Vec<AgentEvent> {
        let f: Frame = project(raw);
        let m: cw::Message = project_opt(f.message.object());
        let content = m.content;
        let uuid = f.uuid.owned();
        let parent = f.parent_tool_use_id.owned();
        let turn = self.turn_of(&f);
        // Meta entries are hidden context (image source paths, local-command caveats).
        if f.is_meta.is_true() {
            return vec![];
        }
        let primitive = content
            .get()
            .filter(|c| !wire::is_object(c) && !wire::is_array(c))
            .cloned();
        if let Some(value) = &primitive {
            let text = Json(Some(value.clone())).string().unwrap_or_default();
            let replayed = live
                && f.is_replay.is_true()
                && !uuid.as_ref().is_some_and(|u| self.started.contains(u));
            let stripped = local_command(&text).or_else(|| replayed.then_some(text.clone()));
            if let Some(stripped) = stripped {
                return vec![match &turn {
                    Some(u) if !self.finished.contains(u) => AgentEvent::ItemCompleted {
                        backend: B,
                        thread_id: t.into(),
                        turn_id: u.clone(),
                        item: Item::Marker(MarkerItem {
                            id: self.next_marker(uuid.clone()),
                            kind: MarkerKind::LocalCommand,
                            text: Some(stripped),
                            raw: Some(raw.clone()),
                            ..Default::default()
                        }),
                    },
                    _ => AgentEvent::ThreadNotice {
                        backend: B,
                        thread_id: t.into(),
                        notice: notice(NoticeLevel::Info, stripped, Some("local_command".into()), false, raw),
                    },
                }];
            }
        }
        let blocks: Vec<OpaqueJson> = if content.array().is_some() {
            content.elements()
        } else if let Some(value) = &primitive {
            vec![wire::encode(&TextBlock { kind: "text", text: value })]
        } else {
            return vec![];
        };
        let results: Vec<cw::Block> = blocks
            .iter()
            .map(project::<cw::Block>)
            .filter(|b| b.kind.get() == Some("tool_result"))
            .collect();
        if !results.is_empty() {
            let structured = f.tool_use_result.get().or(f.tool_use_result_camel.get()).cloned();
            if let (Some(u), Some(turn)) = (&uuid, &turn) {
                self.last_uuid.insert(turn.clone(), u.clone());
            }
            return results
                .iter()
                .filter_map(|r| {
                    let id = r.tool_use_id.owned()?;
                    let Some(tool) = self.tools.get(&id).cloned() else {
                        return Some(AgentEvent::Unknown {
                            backend: B,
                            kind: "tool_result".into(),
                            thread_id: Some(t.into()),
                            raw: raw.clone(),
                        });
                    };
                    let refused = self.denied.contains(&id)
                        || f.tool_denial_kind.get().is_some()
                        || f.tool_result_meta.items().iter().any(|m| {
                            m.get().is_some_and(|m| {
                                m.id.get() == Some(id.as_str()) && m.non_execution_kind.get().is_some()
                            })
                        });
                    Some(AgentEvent::ItemCompleted {
                        backend: B,
                        thread_id: t.into(),
                        turn_id: tool.turn_id.clone(),
                        item: finished_tool(
                            &tool,
                            r.content.get(),
                            structured.as_ref(),
                            r.is_error.is_true(),
                            refused.then_some(ItemStatus::Declined),
                        ),
                    })
                })
                .collect();
        }
        let parts: Vec<UserPart> = blocks.iter().filter_map(user_part).collect();
        let only_text = parts
            .iter()
            .filter_map(|p| match p {
                UserPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        if (parts.len() == 1 && only_text == "[Request interrupted by user]")
            || only_text == "[Request interrupted by user for tool use]"
        {
            let Some(target) = turn else {
                return vec![];
            };
            return vec![AgentEvent::ItemCompleted {
                backend: B,
                thread_id: t.into(),
                turn_id: target,
                item: Item::Marker(MarkerItem {
                    id: self.next_marker(uuid),
                    kind: MarkerKind::Interrupted,
                    text: Some(only_text),
                    raw: Some(raw.clone()),
                    ..Default::default()
                }),
            }];
        }
        let Some(id) = uuid else {
            return vec![AgentEvent::Unknown {
                backend: B,
                kind: "user".into(),
                thread_id: Some(t.into()),
                raw: raw.clone(),
            }];
        };
        let own = self.started.contains(&id) || (live && f.is_replay.is_true());
        let target = if own { id.clone() } else { turn.unwrap_or(id.clone()) };
        vec![AgentEvent::ItemCompleted {
            backend: B,
            thread_id: t.into(),
            turn_id: target,
            item: Item::UserMessage(UserMessageItem {
                id: id.clone(),
                parts,
                client_message_id: own.then_some(id),
                status: ItemStatus::Completed,
                parent_id: parent,
                raw: Some(raw.clone()),
                ..Default::default()
            }),
        }]
    }
}

fn completed(t: String, turn: String, status: TurnStatus, error: Option<TurnError>) -> AgentEvent {
    AgentEvent::TurnCompleted {
        backend: B,
        thread_id: t,
        turn_id: turn,
        status,
        error,
        items: vec![],
        duration_ms: None,
        usage: None,
        at_ms: None,
    }
}
fn category(kind: &str) -> &str {
    match kind {
        "tool_use" | "server_tool_use" | "mcp_tool_use" => "tool",
        "thinking" | "redacted_thinking" => "thinking",
        other => other,
    }
}

/// A user content block as a neutral part.
pub fn user_part(block: &OpaqueJson) -> Option<UserPart> {
    let b: cw::Block = project(block);
    match b.kind.get()? {
        "text" => Some(UserPart::Text { text: b.text.or("") }),
        kind @ ("image" | "document") => {
            let source = b.source.or_default();
            Some(match source.kind.get() {
                Some("base64") => UserPart::InlineData {
                    kind: kind.into(),
                    media_type: source.media_type.owned(),
                    base64: source.data.or(""),
                },
                Some("url") => UserPart::ImageUrl { url: source.url.or("") },
                _ => UserPart::Unknown { raw: block.clone() },
            })
        }
        _ => Some(UserPart::Unknown { raw: block.clone() }),
    }
}

fn tool_item(
    id: &str,
    name: &str,
    input: Option<&OpaqueJson>,
    parent: Option<String>,
    status: ItemStatus,
    raw: Option<OpaqueJson>,
) -> Item {
    let i: cw::ToolInput = project_opt(input.filter(|v| wire::is_object(v)));
    let file = |path: Option<String>, diff: Option<String>| -> Vec<FileDelta> {
        path.map(|p| FileDelta {
            path: p,
            kind: FileChangeKind::Update,
            diff,
            move_path: None,
        })
        .into_iter()
        .collect()
    };
    match name {
        "Bash" => Item::Command(CommandItem {
            id: id.into(),
            command: i.command.or(""),
            description: i.description.owned(),
            status,
            parent_id: parent,
            raw,
            ..Default::default()
        }),
        "Write" | "Edit" | "MultiEdit" | "NotebookEdit" => {
            let changes = match name {
                "Write" => file(i.file_path.owned(), None),
                "NotebookEdit" => file(i.notebook_path.owned(), None),
                _ => file(i.file_path.owned(), edit_diff(&i)),
            };
            Item::FileChange(FileChangeItem {
                id: id.into(),
                changes,
                tool: Some(name.into()),
                status,
                parent_id: parent,
                raw,
                ..Default::default()
            })
        }
        "TodoWrite" => Item::Plan(PlanItem {
            id: id.into(),
            steps: i
                .todos
                .items()
                .iter()
                .map(|t| {
                    let t = t.0.clone().unwrap_or_default();
                    PlanStep {
                        text: t.content.or(""),
                        status: match t.status.get() {
                            Some("completed") => PlanStepStatus::Completed,
                            Some("in_progress") => PlanStepStatus::InProgress,
                            _ => PlanStepStatus::Pending,
                        },
                    }
                })
                .collect(),
            status,
            parent_id: parent,
            raw,
            ..Default::default()
        }),
        "Task" | "Agent" => Item::SubAgent(SubAgentItem {
            id: id.into(),
            tool: name.into(),
            description: i.description.owned(),
            prompt: i.prompt.owned(),
            agent_type: i.subagent_type.owned(),
            model: i.model.owned(),
            status,
            parent_id: parent,
            raw,
            ..Default::default()
        }),
        "WebSearch" | "web_search" => Item::WebSearch(WebSearchItem {
            id: id.into(),
            query: i.query.or(""),
            status,
            parent_id: parent,
            raw,
            ..Default::default()
        }),
        _ if name.starts_with("mcp__") => {
            let rest = &name["mcp__".len()..];
            let (server, tool) = rest.split_once("__").unwrap_or((rest, rest));
            Item::ToolCall(ToolCallItem {
                id: id.into(),
                kind: ToolKind::Mcp,
                tool: tool.into(),
                server: Some(server.into()),
                arguments: input.cloned(),
                status,
                parent_id: parent,
                raw,
                ..Default::default()
            })
        }
        _ => Item::ToolCall(ToolCallItem {
            id: id.into(),
            kind: ToolKind::Builtin,
            tool: name.into(),
            arguments: input.cloned(),
            status,
            parent_id: parent,
            raw,
            ..Default::default()
        }),
    }
}

fn finished_tool(
    tool: &Tool,
    content: Option<&OpaqueJson>,
    structured: Option<&OpaqueJson>,
    is_error: bool,
    forced: Option<ItemStatus>,
) -> Item {
    let status = forced.unwrap_or(if is_error { ItemStatus::Failed } else { ItemStatus::Completed });
    let text = result_text(content);
    let s: cw::Structured = project_opt(structured.filter(|v| wire::is_object(v)));
    let raw = structured.or(content).cloned();
    match tool_item(&tool.id, &tool.name, tool.input.as_ref(), tool.parent_id.clone(), status, raw) {
        Item::Command(mut c) => {
            let (stdout, stderr) = (s.stdout.owned(), s.stderr.owned());
            c.output = if stdout.is_some() || stderr.is_some() {
                join(&[stdout, stderr.filter(|e| !e.is_empty())], "\n")
            } else {
                text.unwrap_or_default()
            };
            c.exit_code = None;
            Item::Command(c)
        }
        Item::FileChange(mut f) => {
            let path = s.file_path.owned();
            let kind = if s.kind.get() == Some("create") {
                FileChangeKind::Add
            } else {
                FileChangeKind::Update
            };
            let first = f.changes.first().cloned();
            let target = path.clone().or(first.as_ref().map(|c| c.path.clone()));
            let diff = structured_patch(target.as_deref(), s.structured_patch.get())
                .or(first.and_then(|c| c.diff));
            if let Some(target) = target {
                f.changes = vec![FileDelta {
                    path: target,
                    kind,
                    diff,
                    move_path: None,
                }];
            }
            f.output = text.unwrap_or_default();
            Item::FileChange(f)
        }
        Item::SubAgent(mut a) => {
            a.result = text;
            Item::SubAgent(a)
        }
        Item::WebSearch(mut w) => {
            w.results = content.cloned();
            Item::WebSearch(w)
        }
        Item::ToolCall(mut c) => {
            c.result = content.cloned();
            c.error = if is_error { text.clone() } else { None };
            c.result_text = text;
            Item::ToolCall(c)
        }
        other => other,
    }
}

/// A minimal unified diff from Edit's old and new strings (display only).
fn edit_diff(i: &cw::ToolInput) -> Option<String> {
    let edits: Vec<(Option<String>, Option<String>)> = if i.edits.is_present() {
        i.edits
            .items()
            .iter()
            .map(|e| {
                let e = e.0.clone().unwrap_or_default();
                (e.old_string.owned(), e.new_string.owned())
            })
            .collect()
    } else {
        vec![(i.old_string.owned(), i.new_string.owned())]
    };
    if edits.iter().all(|(old, new)| old.is_none() && new.is_none()) {
        return None;
    }
    let side = |text: &Option<String>, sign: char| {
        text.as_deref()
            .map(|t| lines(t).iter().map(|l| format!("{sign}{l}")).collect::<Vec<_>>().join("\n"))
            .unwrap_or_default()
    };
    Some(
        edits
            .iter()
            .map(|(old, new)| format!("{}\n{}", side(old, '-'), side(new, '+')))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn structured_patch(path: Option<&str>, patch: Option<&OpaqueJson>) -> Option<String> {
    let hunks: Arr<Obj<cw::Hunk>> = project_opt(patch);
    if hunks.items().is_empty() {
        return None;
    }
    let mut out = String::new();
    if let Some(p) = path {
        out.push_str(&format!("--- {p}\n+++ {p}\n"));
    }
    for h in hunks.items() {
        let h = h.0.clone().unwrap_or_default();
        out.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            h.old_start.get().unwrap_or(0),
            h.old_lines.get().unwrap_or(0),
            h.new_start.get().unwrap_or(0),
            h.new_lines.get().unwrap_or(0)
        ));
        for line in &h.lines.0 {
            out.push_str(line);
            out.push('\n');
        }
    }
    Some(out)
}

/// `rate_limit_info` of a `rate_limit_event`.
pub fn rate_limit(info: Option<&OpaqueJson>) -> Option<RateLimit> {
    let info = info.filter(|v| wire::is_object(v))?;
    let o: cw::RateLimitInfo = project(info);
    let status = o.status.owned();
    Some(RateLimit {
        id: o.rate_limit_type.or("claude"),
        name: None,
        primary: Some(RateLimitWindow {
            used_percent: o.utilization.0.map(|u| if u <= 1.0 { u * 100.0 } else { u }),
            window_minutes: None,
            resets_at_epoch_sec: o.resets_at.get(),
        }),
        secondary: None,
        reached: status.as_deref() == Some("rejected"),
        status,
        raw: Some(info.clone()),
    })
}

/// `list_models` / `initialize.models` as a catalog. Claude accepts text, images and PDFs on
/// every model.
pub fn models(models: Option<&OpaqueJson>) -> ModelCatalog {
    let list = Json(models.cloned()).elements();
    ModelCatalog {
        backend: B,
        models: list
            .iter()
            .enumerate()
            .map(|(i, raw)| {
                let m: cw::Model = project(raw);
                let efforts = m.supports_effort.is_true();
                ModelOption {
                    id: m.value.or(""),
                    display_name: m.display_name.owned().or(m.value.owned()).unwrap_or_default(),
                    description: m.description.owned(),
                    efforts: if efforts {
                        m.supported_effort_levels
                            .0
                            .iter()
                            .map(|id| EffortOption {
                                id: id.clone(),
                                description: None,
                            })
                            .collect()
                    } else {
                        vec![]
                    },
                    default_effort: (efforts && m.supported_effort_levels.0.iter().any(|e| e == "medium"))
                        .then(|| "medium".into()),
                    is_default: i == 0 && m.value.get() == Some("default"),
                    input_modalities: vec!["text".into(), "image".into(), "pdf".into()],
                    resolved_model: m.resolved_model.owned(),
                    raw: Some(raw.clone()),
                    ..Default::default()
                }
            })
            .collect(),
    }
}

/// The `initialize` response `account` as account state (it carries no secrets).
pub fn account(response: &OpaqueJson) -> AccountState {
    let r: cw::Initialized = project(response);
    let a: cw::Account = project_opt(r.account.object());
    let token = a.token_source.owned();
    let api = a.api_key_source.owned();
    let present = |v: &Option<String>| v.as_deref().is_some_and(|v| v != "none");
    AccountState {
        state: if present(&token) || present(&api) {
            LoginState::LoggedIn
        } else {
            LoginState::LoggedOut
        },
        method: token.filter(|t| t != "none").or(api),
        email: a.email.owned(),
        plan: a.subscription_type.owned(),
        organization: a.organization.owned(),
        raw: r.account.get().cloned(),
        ..Default::default()
    }
}
