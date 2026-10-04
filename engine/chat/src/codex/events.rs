//! Stateless mapping of Codex notifications and results to neutral events.
use super::{items, wire};
use crate::{
    model::*,
    wire::{Json, Obj, project},
};

const B: BackendKind = BackendKind::Codex;

/// Notifications mapped to neutral events (the `model` rows of the coverage table).
pub const MODELED_NOTIFICATIONS: &[&str] = &[
    "error",
    "thread/started",
    "thread/status/changed",
    "thread/archived",
    "thread/deleted",
    "thread/unarchived",
    "thread/closed",
    "thread/reverted",
    "thread/name/updated",
    "thread/settings/updated",
    "thread/tokenUsage/updated",
    "turn/started",
    "hook/started",
    "turn/completed",
    "hook/completed",
    "turn/diff/updated",
    "turn/plan/updated",
    "item/started",
    "item/autoApprovalReview/started",
    "item/autoApprovalReview/completed",
    "autoApprovalReview/strictReviewRequired",
    "item/completed",
    "item/agentMessage/delta",
    "item/plan/delta",
    "item/commandExecution/outputDelta",
    "item/commandExecution/terminalInteraction",
    "item/fileChange/outputDelta",
    "item/fileChange/patchUpdated",
    "serverRequest/resolved",
    "item/mcpToolCall/progress",
    "mcpServer/oauthLogin/completed",
    "mcpServer/startupStatus/updated",
    "account/rateLimits/updated",
    "item/reasoning/summaryTextDelta",
    "item/reasoning/summaryPartAdded",
    "item/reasoning/textDelta",
    "thread/compacted",
    "model/rerouted",
    "model/verification",
    "modelProvider/authRecovery/started",
    "modelProvider/authRecovery/completed",
    "model/safetyBuffering/updated",
    "warning",
    "guardianWarning",
    "deprecationNotice",
    "configWarning",
    "account/login/completed",
];
/// Notifications the backend reacts to by re-reading state.
pub const REFRESH_NOTIFICATIONS: &[&str] = &["account/updated", "thread/queue/changed"];

fn notice(level: NoticeLevel, message: impl Into<String>) -> Notice {
    Notice {
        level,
        message: message.into(),
        ..Default::default()
    }
}

pub fn notification(
    method: &str,
    params: Option<&OpaqueJson>,
    raw: &OpaqueJson,
) -> Vec<AgentEvent> {
    let p: wire::Notification =
        crate::wire::project_opt(params.filter(|v| crate::wire::is_object(v)));
    let raw_params = params
        .filter(|v| crate::wire::is_object(v))
        .cloned()
        .unwrap_or_else(crate::wire::empty_object);
    let thread = p.thread_id.owned();
    let turn = p.turn_id.owned();
    let unknown = || {
        vec![AgentEvent::Unknown {
            backend: B,
            kind: method.into(),
            thread_id: thread.clone(),
            raw: raw.clone(),
        }]
    };
    let with_raw = |mut n: Notice| {
        n.raw = Some(raw_params.clone());
        n
    };
    macro_rules! need_thread {
        ($t:ident => $body:expr) => {
            match thread.clone() {
                Some($t) => $body,
                None => unknown(),
            }
        };
    }
    macro_rules! need_turn {
        ($t:ident, $u:ident => $body:expr) => {
            match (thread.clone(), turn.clone()) {
                (Some($t), Some($u)) => $body,
                _ => unknown(),
            }
        };
    }
    let delta = |d: ItemDelta| -> Vec<AgentEvent> {
        match (thread.clone(), turn.clone(), p.item_id.owned()) {
            (Some(t), Some(u), Some(item)) => vec![AgentEvent::ItemUpdated {
                backend: B,
                thread_id: t,
                turn_id: u,
                item_id: item,
                delta: d,
            }],
            _ => unknown(),
        }
    };
    match method {
        "error" => need_thread!(t => {
            let error = items::turn_error(p.error.get());
            let will_retry = p.will_retry.is_true();
            vec![AgentEvent::TurnNotice {
                backend: B,
                thread_id: t,
                turn_id: turn.clone(),
                notice: Notice {
                    level: if will_retry { NoticeLevel::Warning } else { NoticeLevel::Error },
                    message: error.as_ref().map(|e| e.message.clone()).unwrap_or_else(|| "error".into()),
                    code: error.as_ref().and_then(|e| e.code.clone()),
                    will_retry,
                    detail: error.as_ref().and_then(|e| e.detail.clone()),
                    raw: Some(raw_params.clone()),
                },
            }]
        }),
        "thread/started" => match p.thread.object() {
            Some(t) => vec![thread_upserted(t, None)],
            None => unknown(),
        },
        "thread/status/changed" => need_thread!(t => vec![AgentEvent::ThreadStatusChanged {
            backend: B,
            thread_id: t,
            run_state: run_state(&p.status),
        }]),
        "thread/archived" | "thread/unarchived" => {
            need_thread!(t => vec![AgentEvent::ThreadArchived {
                backend: B,
                thread_id: t,
                archived: method == "thread/archived",
            }])
        }
        "thread/deleted" => {
            need_thread!(t => vec![AgentEvent::ThreadDeleted { backend: B, thread_id: t }])
        }
        "thread/closed" => {
            need_thread!(t => vec![AgentEvent::ThreadClosed { backend: B, thread_id: t }])
        }
        "thread/reverted" => need_thread!(t => vec![AgentEvent::ThreadNotice {
            backend: B,
            thread_id: t,
            notice: with_raw(Notice { code: Some("reverted".into()), ..notice(NoticeLevel::Info, "Thread reverted") }),
        }]),
        "thread/name/updated" => need_thread!(t => vec![AgentEvent::ThreadRenamed {
            backend: B,
            thread_id: t,
            title: p.thread_name.owned(),
        }]),
        "thread/settings/updated" => need_thread!(t => vec![AgentEvent::ThreadSettingsChanged {
            backend: B,
            thread_id: t,
            settings: settings(&p.thread_settings),
        }]),
        "thread/tokenUsage/updated" => need_thread!(t => vec![AgentEvent::TokenUsageChanged {
            backend: B,
            thread_id: t,
            turn_id: turn.clone(),
            usage: token_usage(&p.token_usage),
        }]),
        "turn/started" => need_thread!(t => {
            let turn_json: wire::Turn = crate::wire::project_opt(p.turn.object());
            match turn_json.id.owned() {
                Some(id) => vec![AgentEvent::TurnStarted {
                    backend: B,
                    thread_id: t,
                    turn_id: id,
                    client_message_id: None,
                    at_ms: turn_json.started_at.get().map(|v| v * 1000),
                }],
                None => unknown(),
            }
        }),
        "turn/completed" => need_thread!(t => {
            let turn_json: wire::Turn = crate::wire::project_opt(p.turn.object());
            match turn_json.id.owned() {
                Some(id) => vec![AgentEvent::TurnCompleted {
                    backend: B,
                    thread_id: t,
                    turn_id: id,
                    status: items::turn_status(turn_json.status.get()),
                    error: items::turn_error(turn_json.error.get()),
                    items: turn_json.items.elements().iter().map(|i| items::parse(i, true)).collect(),
                    duration_ms: turn_json.duration_ms.get(),
                    usage: None,
                    at_ms: turn_json.completed_at.get().map(|v| v * 1000),
                }],
                None => unknown(),
            }
        }),
        "hook/started" | "hook/completed" => need_thread!(t => {
            let run = p.run.or_default();
            let text = [run.event_name.owned(), run.status.owned(), run.status_message.owned()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" · ");
            match turn.clone() {
                Some(u) => {
                    let item = Item::Marker(MarkerItem {
                        id: format!("hook:{}", run.id.or("?")),
                        kind: MarkerKind::Hook,
                        text: Some(text),
                        raw: Some(raw_params.clone()),
                        ..Default::default()
                    });
                    vec![if method == "hook/started" {
                        AgentEvent::ItemStarted { backend: B, thread_id: t, turn_id: u, item }
                    } else {
                        AgentEvent::ItemCompleted { backend: B, thread_id: t, turn_id: u, item }
                    }]
                }
                None => vec![AgentEvent::ThreadNotice {
                    backend: B,
                    thread_id: t,
                    notice: with_raw(Notice { code: Some("hook".into()), ..notice(NoticeLevel::Info, format!("Hook: {text}")) }),
                }],
            }
        }),
        "turn/diff/updated" => need_turn!(t, u => vec![AgentEvent::TurnDiffUpdated {
            backend: B,
            thread_id: t,
            turn_id: u,
            diff: p.diff.or(""),
        }]),
        "turn/plan/updated" => need_turn!(t, u => vec![AgentEvent::TurnPlanUpdated {
            backend: B,
            thread_id: t,
            turn_id: u,
            plan: TurnPlan { steps: items::plan_steps(p.plan.get()), explanation: p.explanation.owned() },
        }]),
        "item/started" | "item/completed" => need_turn!(t, u => match p.item.get() {
            Some(item) => {
                let completed = method == "item/completed";
                let item = items::parse(item, completed);
                vec![if completed {
                    AgentEvent::ItemCompleted { backend: B, thread_id: t, turn_id: u, item }
                } else {
                    AgentEvent::ItemStarted { backend: B, thread_id: t, turn_id: u, item }
                }]
            }
            None => unknown(),
        }),
        "item/autoApprovalReview/started"
        | "item/autoApprovalReview/completed"
        | "autoApprovalReview/strictReviewRequired" => {
            // Must not happen with approvalsReviewer=user; surfaced loudly if it does.
            let n = with_raw(Notice {
                code: Some(method.into()),
                ..notice(
                    NoticeLevel::Error,
                    "Approval was routed to automatic review",
                )
            });
            vec![match thread.clone() {
                Some(t) => AgentEvent::TurnNotice {
                    backend: B,
                    thread_id: t,
                    turn_id: turn.clone(),
                    notice: n,
                },
                None => AgentEvent::BackendNotice {
                    backend: B,
                    notice: n,
                },
            }]
        }
        "item/agentMessage/delta" => delta(ItemDelta::AgentText {
            text: p.delta.or(""),
        }),
        "item/plan/delta" => delta(ItemDelta::PlanText {
            text: p.delta.or(""),
        }),
        "item/commandExecution/outputDelta" => delta(ItemDelta::CommandOutput {
            text: p.delta.or(""),
        }),
        "item/commandExecution/terminalInteraction" => delta(ItemDelta::TerminalInput {
            text: p.stdin.or(""),
        }),
        "item/fileChange/outputDelta" => delta(ItemDelta::FileChangeOutput {
            text: p.delta.or(""),
        }),
        "item/fileChange/patchUpdated" => delta(ItemDelta::FileChangePatch {
            changes: items::changes(p.changes.get()),
        }),
        "item/mcpToolCall/progress" => delta(ItemDelta::ToolProgress {
            message: p.message.or(""),
        }),
        "item/reasoning/summaryTextDelta" => delta(ItemDelta::ReasoningSummary {
            index: p.summary_index.int().unwrap_or(0),
            text: p.delta.or(""),
        }),
        "item/reasoning/summaryPartAdded" => delta(ItemDelta::ReasoningSummaryPart {
            index: p.summary_index.int().unwrap_or(0),
        }),
        "item/reasoning/textDelta" => delta(ItemDelta::ReasoningText {
            index: p.content_index.int().unwrap_or(0),
            text: p.delta.or(""),
        }),
        "serverRequest/resolved" => match p.request_id.value() {
            Some(id) => vec![AgentEvent::RequestClosed {
                key: RequestKey {
                    backend: B,
                    raw_id: id.clone(),
                },
                status: RequestStatus::Resolved,
                answer: None,
            }],
            None => unknown(),
        },
        "mcpServer/oauthLogin/completed" => {
            let failed = p.success.is_false();
            vec![AgentEvent::BackendNotice {
                backend: B,
                notice: with_raw(Notice {
                    code: Some(method.into()),
                    detail: p.error.string(),
                    ..notice(
                        if failed {
                            NoticeLevel::Warning
                        } else {
                            NoticeLevel::Info
                        },
                        format!(
                            "MCP login {}: {}",
                            if failed { "failed" } else { "completed" },
                            p.name.or("")
                        )
                        .trim()
                        .to_string(),
                    )
                }),
            }]
        }
        "mcpServer/startupStatus/updated" => vec![AgentEvent::McpServerChanged {
            backend: B,
            status: McpServerStatus {
                name: p.name.or(""),
                status: p.status.string().unwrap_or_default(),
                error: p.error.string(),
            },
        }],
        "account/rateLimits/updated" => vec![AgentEvent::RateLimitsChanged {
            backend: B,
            limits: RateLimitState {
                limits: rate_limit(p.rate_limits.get()).into_iter().collect(),
                ..Default::default()
            },
            merge: true,
        }],
        "thread/compacted" => need_thread!(t => vec![AgentEvent::ThreadNotice {
            backend: B,
            thread_id: t,
            notice: with_raw(Notice { code: Some("compacted".into()), ..notice(NoticeLevel::Info, "Context compacted") }),
        }]),
        "model/rerouted" => need_thread!(t => vec![AgentEvent::TurnNotice {
            backend: B,
            thread_id: t,
            turn_id: turn.clone(),
            notice: with_raw(Notice {
                code: p.reason.display_text(),
                ..notice(
                    NoticeLevel::Warning,
                    format!(
                        "Model rerouted: {} → {}",
                        p.from_model.or("null"),
                        p.to_model.or("null")
                    ),
                )
            }),
        }]),
        "model/verification" => need_thread!(t => vec![AgentEvent::TurnNotice {
            backend: B,
            thread_id: t,
            turn_id: turn.clone(),
            notice: with_raw(Notice { code: Some(method.into()), ..notice(NoticeLevel::Info, "Model verification") }),
        }]),
        "modelProvider/authRecovery/started" | "modelProvider/authRecovery/completed" => {
            vec![AgentEvent::BackendNotice {
                backend: B,
                notice: with_raw(Notice {
                    code: Some(method.into()),
                    ..notice(
                        NoticeLevel::Info,
                        if method == "modelProvider/authRecovery/started" {
                            "Refreshing sign-in"
                        } else {
                            "Sign-in refreshed"
                        },
                    )
                }),
            }]
        }
        "model/safetyBuffering/updated" => need_thread!(t => if p.show_buffering_ui.is_true() {
            vec![AgentEvent::TurnNotice {
                backend: B,
                thread_id: t,
                turn_id: turn.clone(),
                notice: with_raw(Notice {
                    code: Some(method.into()),
                    detail: Some(p.reasons.0.join(", ")),
                    ..notice(NoticeLevel::Info, "Safety check in progress")
                }),
            }]
        } else {
            vec![]
        }),
        "warning" => {
            let n = with_raw(Notice {
                code: Some(method.into()),
                ..notice(NoticeLevel::Warning, p.message.or(""))
            });
            vec![match thread.clone() {
                Some(t) => AgentEvent::ThreadNotice {
                    backend: B,
                    thread_id: t,
                    notice: n,
                },
                None => AgentEvent::BackendNotice {
                    backend: B,
                    notice: n,
                },
            }]
        }
        "guardianWarning" => need_thread!(t => vec![AgentEvent::ThreadNotice {
            backend: B,
            thread_id: t,
            notice: with_raw(Notice { code: Some(method.into()), ..notice(NoticeLevel::Warning, p.message.or("")) }),
        }]),
        "deprecationNotice" => vec![AgentEvent::BackendNotice {
            backend: B,
            notice: with_raw(Notice {
                code: Some(method.into()),
                detail: p.details.owned(),
                ..notice(NoticeLevel::Info, p.summary.or(""))
            }),
        }],
        "configWarning" => {
            let detail = [p.path.owned(), p.details.owned()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join("\n");
            vec![AgentEvent::BackendNotice {
                backend: B,
                notice: with_raw(Notice {
                    code: Some(method.into()),
                    detail: Some(detail).filter(|d| !d.is_empty()),
                    ..notice(NoticeLevel::Warning, p.summary.or(""))
                }),
            }]
        }
        "account/login/completed" => vec![AgentEvent::LoginChanged {
            backend: B,
            flow: Some(LoginFlow::Completed {
                login_id: p.login_id.owned(),
                success: p.success.is_true(),
                error: p.error.string(),
            }),
        }],
        // The backend re-reads the account or the queue.
        m if REFRESH_NOTIFICATIONS.contains(&m) => vec![],
        _ => unknown(),
    }
}

pub fn run_state(status: &Json) -> RunState {
    let s: wire::RunStatus = crate::wire::project_opt(status.object());
    match s.kind.get() {
        Some("idle") => RunState::Idle,
        Some("notLoaded") => RunState::NotLoaded,
        Some("systemError") => RunState::Error,
        Some("active") => {
            if s.active_flags.0.iter().any(|f| f == "waitingOnApproval") {
                RunState::WaitingApproval
            } else if s.active_flags.0.iter().any(|f| f == "waitingOnUserInput") {
                RunState::WaitingInput
            } else {
                RunState::Running
            }
        }
        _ => RunState::Idle,
    }
}

pub fn settings(json: &Json) -> ThreadSettings {
    let s: wire::Settings = crate::wire::project_opt(json.object());
    // `sandboxPolicy ?: sandbox`: an explicit null policy still wins over the mode.
    let sandbox = if s.sandbox_policy.get().is_some() {
        &s.sandbox_policy
    } else {
        &s.sandbox
    };
    ThreadSettings {
        model: s.model.owned(),
        effort: s.effort.owned().or(s.reasoning_effort.owned()),
        approval_policy: items::policy_text(&s.approval_policy),
        sandbox: sandbox
            .object()
            .and_then(|o| project::<wire::Typed>(o).kind.owned())
            .or_else(|| sandbox.string()),
        approvals_reviewer: s.approvals_reviewer.owned(),
        raw: json.value().cloned(),
    }
}

/// A `Thread` (thread/started, start/resume/fork results) as an upsert.
pub fn thread_upserted(thread: &OpaqueJson, response: Option<&OpaqueJson>) -> AgentEvent {
    let t: wire::Thread = project(thread);
    AgentEvent::ThreadUpserted {
        backend: B,
        thread_id: t.id.or(""),
        title: t.name.owned(),
        preview: t.preview.owned(),
        cwd: t.cwd.owned(),
        path: t.path.owned(),
        forked_from: t.forked_from_id.owned(),
        ephemeral: t.ephemeral.get(),
        run_state: t.status.get().map(|_| run_state(&t.status)),
        settings: Some(match response {
            Some(r) => settings(&Json(Some(r.clone()))),
            None => ThreadSettings {
                model: t.model.owned(),
                effort: t.reasoning_effort.owned(),
                ..Default::default()
            },
        }),
        created_at_sec: t.created_at.get(),
        updated_at_sec: t.updated_at.get(),
        raw: Some(thread.clone()),
    }
}

pub fn token_usage(json: &Json) -> TokenUsage {
    let u: wire::TokenUsage = crate::wire::project_opt(json.object());
    let total = u.total.or_default();
    TokenUsage {
        input_tokens: total.input_tokens.get().unwrap_or(0),
        cached_input_tokens: total.cached_input_tokens.get().unwrap_or(0),
        output_tokens: total.output_tokens.get().unwrap_or(0),
        reasoning_tokens: total.reasoning_output_tokens.get().unwrap_or(0),
        total_tokens: total.total_tokens.get().unwrap_or(0),
        context_window: u.model_context_window.get(),
        raw: json.value().cloned(),
        ..Default::default()
    }
}

fn window(w: &Obj<wire::RateLimitWindow>) -> Option<RateLimitWindow> {
    w.get().map(|w| RateLimitWindow {
        used_percent: w.used_percent.0,
        window_minutes: w.window_duration_mins.get(),
        resets_at_epoch_sec: w.resets_at.get(),
    })
}

pub fn rate_limit(json: Option<&OpaqueJson>) -> Option<RateLimit> {
    let json = json.filter(|v| crate::wire::is_object(v))?;
    let o: wire::RateLimit = project(json);
    let primary = window(&o.primary);
    Some(RateLimit {
        id: o.limit_id.or("codex"),
        name: o.limit_name.owned(),
        status: o.rate_limit_reached_type.string(),
        reached: !o.rate_limit_reached_type.is_nullish()
            || primary.as_ref().and_then(|w| w.used_percent).unwrap_or(0.0) >= 100.0,
        primary,
        secondary: window(&o.secondary),
        raw: Some(json.clone()),
    })
}

/// `account/rateLimits/read` result.
pub fn rate_limits(result: &OpaqueJson) -> RateLimitState {
    let r: wire::RateLimits = project(result);
    let limits = match r.rate_limits_by_limit_id.0 {
        Some(members) => members
            .iter()
            .filter_map(|(_, v)| rate_limit(Some(v)))
            .collect(),
        None => rate_limit(r.rate_limits.get()).into_iter().collect(),
    };
    RateLimitState {
        limits,
        ordinary_usage_allowed: r.ordinary_usage_allowed.get(),
        upsell: r.rate_limit_upsell.value().cloned(),
        raw: Some(result.clone()),
    }
}

/// `account/read` result.
pub fn account(result: &OpaqueJson) -> AccountState {
    let r: wire::AccountRead = project(result);
    let a: wire::Account = crate::wire::project_opt(r.account.object());
    AccountState {
        state: if r.account.is_nullish() {
            LoginState::LoggedOut
        } else {
            LoginState::LoggedIn
        },
        method: a.kind.owned(),
        email: a.email.owned(),
        plan: a.plan_type.owned(),
        requires_auth: r.requires_openai_auth.get(),
        raw: Some(result.clone()),
        ..Default::default()
    }
}

/// `account/login/start` result.
pub fn login_flow(result: &OpaqueJson) -> Option<LoginFlow> {
    let r: wire::LoginStart = project(result);
    match r.kind.get() {
        Some("chatgptDeviceCode") => Some(LoginFlow::DeviceCode {
            login_id: r.login_id.owned(),
            verification_url: r.verification_url.or(""),
            user_code: r.user_code.or(""),
        }),
        Some("chatgpt") => Some(LoginFlow::Browser {
            login_id: r.login_id.owned(),
            auth_url: r.auth_url.or(""),
        }),
        Some("apiKey" | "chatgptAuthTokens" | "amazonBedrock") => Some(LoginFlow::Completed {
            login_id: None,
            success: true,
            error: None,
        }),
        _ => None,
    }
}
pub fn login_id(result: &OpaqueJson) -> Option<String> {
    project::<wire::LoginStart>(result).login_id.owned()
}

/// `model/list` results, all pages concatenated by the caller.
pub fn models(pages: &[OpaqueJson]) -> ModelCatalog {
    let mut models = Vec::new();
    for page in pages {
        let page: wire::Page = project(page);
        for m in page.data.items() {
            let raw = m.get().cloned().unwrap_or_default();
            let o: wire::Model = project(&raw);
            let mut modalities = Vec::new();
            for v in &o.input_modalities.0 {
                if !modalities.contains(v) {
                    modalities.push(v.clone());
                }
            }
            models.push(ModelOption {
                id: o.id.owned().or(o.model.owned()).unwrap_or_default(),
                display_name: o.display_name.owned().or(o.id.owned()).unwrap_or_default(),
                description: o.description.owned(),
                efforts: o
                    .supported_reasoning_efforts
                    .items()
                    .iter()
                    .map(|e| EffortOption {
                        id: e.reasoning_effort.or(""),
                        description: e.description.owned(),
                    })
                    .collect(),
                default_effort: o.default_reasoning_effort.owned(),
                is_default: o.is_default.is_true(),
                hidden: o.hidden.is_true(),
                input_modalities: modalities,
                upgrade_to: o.upgrade.owned(),
                upgrade_message: o
                    .upgrade_info
                    .get()
                    .and_then(|u| u.migration_markdown.owned()),
                raw: Some(raw),
                ..Default::default()
            });
        }
    }
    ModelCatalog { backend: B, models }
}
pub fn next_cursor(page: &OpaqueJson) -> Option<String> {
    project::<wire::Page>(page).next_cursor.owned()
}
pub fn page_items(page: &OpaqueJson) -> Vec<OpaqueJson> {
    project::<wire::Page>(page)
        .data
        .items()
        .iter()
        .map(|m| m.0.clone().unwrap_or_default())
        .collect()
}
