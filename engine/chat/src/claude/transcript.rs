//! History hydration from a Claude transcript (`$CLAUDE_CONFIG_DIR/projects/<cwd>/<session>.jsonl`).
//! Tolerant: unknown entry types and malformed lines are skipped, and subagent side chains are not
//! part of the main history. Live mapping and the reducer produce the same items as streaming.
use super::{
    mapper::ClaudeMapper,
    wire::{Block, Frame, Message},
};
use crate::{
    model::*,
    reducer,
    wire::{self, project, project_opt},
};

pub struct History {
    pub turns: Vec<Turn>,
    pub title: Option<String>,
}

fn entry(line: &str) -> Option<(OpaqueJson, Frame)> {
    let raw = wire::parse(line).ok().filter(wire::is_object)?;
    let frame = project(&raw);
    Some((raw, frame))
}

pub fn events(session: &str, lines: &[String]) -> Vec<AgentEvent> {
    let mut mapper = ClaudeMapper::new(Some(session));
    let mut out = Vec::new();
    let mut turn: Option<String> = None;
    let mut interrupted = false;
    let close = |mapper: &mut ClaudeMapper,
                 out: &mut Vec<AgentEvent>,
                 turn: &mut Option<String>,
                 interrupted: &mut bool| {
        if let Some(t) = turn.take() {
            let status = if *interrupted {
                TurnStatus::Interrupted
            } else {
                TurnStatus::Completed
            };
            out.extend(mapper.end_turn(&t, status, None));
        }
        *interrupted = false;
    };
    for line in lines {
        let Some((raw, f)) = entry(line) else {
            continue;
        };
        if f.is_sidechain.is_true() {
            continue;
        }
        match f.kind.get() {
            Some("user") => {
                let m: Message = project_opt(f.message.object());
                let text = m.content.string();
                if !f.is_meta.is_true()
                    && text.as_deref().is_some_and(|t| {
                        t.contains("<command-name>") || t.contains("<local-command-")
                    })
                {
                    close(&mut mapper, &mut out, &mut turn, &mut interrupted);
                }
                if is_prompt(&f) {
                    close(&mut mapper, &mut out, &mut turn, &mut interrupted);
                    let Some(id) = f.uuid.owned() else {
                        continue;
                    };
                    out.extend(mapper.begin_turn(&id));
                    turn = Some(id);
                }
                let mapped = mapper.user(session, &raw, false);
                if mapped.iter().any(|e| {
                    matches!(
                        e,
                        AgentEvent::ItemCompleted {
                            item: Item::Marker(MarkerItem {
                                kind: MarkerKind::Interrupted,
                                ..
                            }),
                            ..
                        }
                    )
                }) {
                    interrupted = true;
                }
                out.extend(mapped);
            }
            Some("assistant") => {
                if f.aborted_mid_stream.is_true() {
                    interrupted = true;
                }
                out.extend(mapper.map(&raw));
            }
            Some("custom-title") => {
                if let Some(title) = f.custom_title.owned() {
                    out.push(AgentEvent::ThreadRenamed {
                        backend: BackendKind::Claude,
                        thread_id: session.into(),
                        title: Some(title),
                    });
                }
            }
            Some("system") => out.extend(
                mapper
                    .map(&raw)
                    .into_iter()
                    .filter(|e| !matches!(e, AgentEvent::Unknown { .. })),
            ),
            _ => (),
        }
    }
    close(&mut mapper, &mut out, &mut turn, &mut interrupted);
    out
}

pub fn history(session: &str, lines: &[String]) -> History {
    let mut state = AgentState::default();
    for event in events(session, lines) {
        reducer::reduce(&mut state, &event);
    }
    let thread = state.threads.get(&ThreadKey {
        backend: BackendKind::Claude,
        id: session.into(),
    });
    History {
        turns: thread.map(|t| t.turns.clone()).unwrap_or_default(),
        title: thread.and_then(|t| t.title.clone()),
    }
}

/// The last message UUID of `turn` (the fork anchor for `--resume-session-at`).
pub fn last_uuid(lines: &[String], turn: &str) -> Option<String> {
    let mut in_turn = false;
    let mut last = None;
    for line in lines {
        let Some((_, f)) = entry(line) else {
            continue;
        };
        if f.is_sidechain.is_true() {
            continue;
        }
        let kind = f.kind.get();
        if kind == Some("user") && is_prompt(&f) {
            if in_turn {
                return last;
            }
            in_turn = f.uuid.get() == Some(turn);
        }
        if in_turn && matches!(kind, Some("user" | "assistant")) {
            last = f.uuid.owned().or(last);
        }
    }
    if in_turn { last } else { None }
}

/// A real prompt: a user entry that is not meta, a tool result, a local command or an interrupt.
pub fn is_prompt(f: &Frame) -> bool {
    if f.is_meta.is_true() || f.parent_tool_use_id.get().is_some() {
        return false;
    }
    let m: Message = project_opt(f.message.object());
    if let Some(text) = m.content.string() {
        return !text.contains("<local-command-") && !text.contains("<command-name>");
    }
    if m.content.array().is_none() {
        return false;
    }
    let blocks: Vec<Block> = m.content.elements().iter().map(project).collect();
    if blocks.iter().any(|b| b.kind.get() == Some("tool_result")) {
        return false;
    }
    let text = blocks
        .iter()
        .filter_map(|b| b.text.owned())
        .collect::<Vec<_>>()
        .join("\n");
    !text.starts_with("[Request interrupted by user")
}
