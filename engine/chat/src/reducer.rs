//! Pure event reduction. This module has no authority to send a vendor response.
use crate::model::*;

pub const MAX_UNKNOWN: usize = 200;
pub const MAX_NOTICES: usize = 100;
pub const MAX_COMMAND_OUTPUT: usize = 512 * 1024;

fn bounded<T>(items: &mut Vec<T>, item: T, max: usize) {
    items.push(item);
    if items.len() > max {
        items.drain(..items.len() - max);
    }
}
fn backend(s: &mut AgentState, kind: BackendKind) -> &mut BackendStatus {
    s.backends.entry(kind).or_insert_with(|| BackendStatus {
        backend: kind,
        ..Default::default()
    })
}
fn thread<'a>(s: &'a mut AgentState, kind: BackendKind, id: &str) -> &'a mut ThreadState {
    let key = ThreadKey {
        backend: kind,
        id: id.into(),
    };
    if s.threads.get(&key).is_none() {
        s.threads.insert(key.clone(), ThreadState::new(key.clone()));
    }
    s.threads.get_mut(&key).unwrap()
}
fn turn<'a>(t: &'a mut ThreadState, id: &str) -> &'a mut Turn {
    let index = t.turns.iter().position(|v| v.id == id).unwrap_or_else(|| {
        t.turns.push(Turn {
            id: id.into(),
            ..Default::default()
        });
        t.turns.len() - 1
    });
    &mut t.turns[index]
}
fn merge_settings(old: &mut ThreadSettings, new: &ThreadSettings) {
    macro_rules! update { ($($f:ident),*) => { $(if new.$f.is_some() { old.$f = new.$f.clone(); })* } }
    update!(
        model,
        effort,
        approval_policy,
        sandbox,
        approvals_reviewer,
        raw
    );
}
fn idle_like(s: RunState) -> bool {
    matches!(s, RunState::Idle | RunState::NotLoaded | RunState::Error)
}
fn waiting(s: RunState) -> bool {
    matches!(s, RunState::WaitingApproval | RunState::WaitingInput)
}

/// Folds one answered account read (see the Codex login state machine in docs/engine/chat.md).
fn account_read(previous: &AccountState, read: &AccountState) -> AccountState {
    if read.state == LoginState::LoggedIn {
        read.clone()
    } else if LoginFlow::is_confirmed_success(previous.login.as_ref()) {
        AccountState {
            check_error: None,
            ..previous.clone()
        }
    } else if LoginFlow::is_pending(previous.login.as_ref()) {
        AccountState {
            state: LoginState::LoggingIn,
            login: previous.login.clone(),
            ..read.clone()
        }
    } else {
        let mut next = read.clone();
        if next.login.is_none() && !LoginFlow::is_confirmed_success(previous.login.as_ref()) {
            next.login = previous.login.clone();
        }
        next
    }
}

pub fn reduce(s: &mut AgentState, event: &AgentEvent) {
    use AgentEvent::*;
    match event {
        ProcessChanged {
            backend: kind,
            state,
        } => {
            let b = backend(s, *kind);
            b.process = state.clone();
            // A confirmed completion belongs to the process that reported it; a new process reads afresh.
            if matches!(state, ProcessState::Starting {})
                && LoginFlow::is_confirmed_success(b.account.login.as_ref())
            {
                b.account.login = None;
            }
            let message = match state {
                ProcessState::Exited { exit_code, .. } => format!(
                    "Backend process exited{}",
                    exit_code.map(|v| format!(" ({v})")).unwrap_or_default()
                ),
                ProcessState::Failed { message, .. } => message.clone(),
                _ => return,
            };
            for t in s.threads.values_mut().filter(|t| t.key.backend == *kind) {
                if !t.deleted {
                    t.run_state = RunState::NotLoaded;
                }
                for turn in &mut t.turns {
                    match turn.status {
                        TurnStatus::Running => {
                            turn.status = TurnStatus::Failed;
                            if turn.error.is_none() {
                                turn.error = Some(TurnError {
                                    message: message.clone(),
                                    code: Some("processExited".into()),
                                    ..Default::default()
                                });
                            }
                            finish_items(turn);
                        }
                        TurnStatus::Queued => turn.status = TurnStatus::Cancelled,
                        _ => (),
                    }
                }
            }
            for request in s
                .requests
                .values_mut()
                .filter(|r| r.key.backend == *kind && r.status == RequestStatus::Pending)
            {
                request.status = RequestStatus::Expired;
            }
        }
        ServerInfo {
            backend: kind,
            info,
        } => backend(s, *kind).server_info = Some(info.clone()),
        AccountChanged {
            backend: kind,
            account,
        } => {
            let b = backend(s, *kind);
            b.account = account_read(&b.account, account);
        }
        AccountCheckFailed {
            backend: kind,
            message,
        } => backend(s, *kind).account.check_error = Some(message.clone()),
        LoginChanged {
            backend: kind,
            flow,
        } => {
            let a = &mut backend(s, *kind).account;
            a.state = match flow {
                None => a.state,
                Some(LoginFlow::Completed { success: true, .. }) => LoginState::LoggedIn,
                Some(LoginFlow::Completed { success: false, .. }) => {
                    if a.state == LoginState::LoggingIn {
                        LoginState::LoggedOut
                    } else {
                        a.state
                    }
                }
                _ => LoginState::LoggingIn,
            };
            // A new attempt starts without the previous attempt's check failure.
            if LoginFlow::is_pending(flow.as_ref()) {
                a.check_error = None;
            }
            a.login = flow.clone();
        }
        RateLimitsChanged {
            backend: kind,
            limits,
            merge,
        } => {
            let b = backend(s, *kind);
            if *merge && b.rate_limits.is_some() {
                let old = b.rate_limits.as_mut().unwrap();
                for l in &limits.limits {
                    if let Some(v) = old.limits.iter_mut().find(|v| v.id == l.id) {
                        *v = l.clone();
                    } else {
                        old.limits.push(l.clone());
                    }
                }
                if limits.ordinary_usage_allowed.is_some() {
                    old.ordinary_usage_allowed = limits.ordinary_usage_allowed;
                }
                if limits.upsell.is_some() {
                    old.upsell = limits.upsell.clone();
                }
            } else {
                b.rate_limits = Some(limits.clone());
            }
        }
        ModelsChanged {
            backend: kind,
            catalog,
        } => backend(s, *kind).models = Some(catalog.clone()),
        McpServerChanged {
            backend: kind,
            status,
        } => {
            backend(s, *kind)
                .mcp_servers
                .insert(status.name.clone(), status.clone());
        }
        BackendNotice {
            backend: kind,
            notice,
        } => bounded(&mut backend(s, *kind).notices, notice.clone(), MAX_NOTICES),
        Unknown {
            backend: kind,
            kind: name,
            thread_id,
            raw,
        } => {
            let record = UnknownRecord {
                kind: name.clone(),
                thread_id: thread_id.clone(),
                raw: raw.clone(),
            };
            bounded(&mut backend(s, *kind).unknown, record.clone(), MAX_UNKNOWN);
            if let Some(id) = thread_id
                && let Some(t) = s.threads.get_mut(&ThreadKey {
                    backend: *kind,
                    id: id.clone(),
                })
            {
                bounded(&mut t.unknown, record, MAX_UNKNOWN);
            }
        }
        ThreadUpserted {
            backend: kind,
            thread_id,
            title,
            preview,
            cwd,
            path,
            forked_from,
            ephemeral,
            run_state,
            settings,
            created_at_sec,
            updated_at_sec,
            raw,
        } => {
            let t = thread(s, *kind, thread_id);
            macro_rules! update { ($($f:ident),*) => { $(if $f.is_some() { t.$f = $f.clone(); })* } }
            update!(
                title,
                cwd,
                path,
                forked_from,
                created_at_sec,
                updated_at_sec,
                raw
            );
            if preview.as_ref().is_some_and(|p| !p.is_empty()) {
                t.preview = preview.clone();
            }
            if let Some(v) = ephemeral {
                t.ephemeral = *v;
            }
            if let Some(v) = run_state {
                t.run_state = *v;
            }
            if let Some(v) = settings {
                merge_settings(&mut t.settings, v);
            }
        }
        ThreadStatusChanged {
            backend: kind,
            thread_id,
            run_state,
        } => thread(s, *kind, thread_id).run_state = *run_state,
        ThreadRenamed {
            backend: kind,
            thread_id,
            title,
        } => thread(s, *kind, thread_id).title = title.clone(),
        ThreadArchived {
            backend: kind,
            thread_id,
            archived,
        } => thread(s, *kind, thread_id).archived = *archived,
        ThreadDeleted {
            backend: kind,
            thread_id,
        } => {
            let t = thread(s, *kind, thread_id);
            t.deleted = true;
            t.run_state = RunState::Closed;
        }
        ThreadClosed {
            backend: kind,
            thread_id,
        } => thread(s, *kind, thread_id).run_state = RunState::NotLoaded,
        ThreadSettingsChanged {
            backend: kind,
            thread_id,
            settings,
        } => merge_settings(&mut thread(s, *kind, thread_id).settings, settings),
        TokenUsageChanged {
            backend: kind,
            thread_id,
            turn_id,
            usage,
        } => {
            let t = thread(s, *kind, thread_id);
            if let Some(id) = turn_id
                && let Some(turn) = t.turns.iter_mut().find(|v| &v.id == id)
            {
                turn.usage = Some(usage.clone());
            }
            t.usage = Some(usage.clone());
        }
        ThreadNotice {
            backend: kind,
            thread_id,
            notice,
        } => bounded(
            &mut thread(s, *kind, thread_id).notices,
            notice.clone(),
            MAX_NOTICES,
        ),
        HistoryLoaded {
            backend: kind,
            thread_id,
            turns,
            prepend,
            cursor,
        } => {
            let t = thread(s, *kind, thread_id);
            let mut next = Vec::new();
            if *prepend {
                next.extend(
                    turns
                        .iter()
                        .filter(|v| !t.turns.iter().any(|old| old.id == v.id))
                        .cloned(),
                );
                next.append(&mut t.turns);
            } else {
                next.extend(turns.iter().map(|loaded| {
                    t.turns
                        .iter()
                        .find(|old| {
                            old.id == loaded.id
                                && (!old.status.is_final() || old.items.len() >= loaded.items.len())
                        })
                        .unwrap_or(loaded)
                        .clone()
                }));
                next.extend(
                    t.turns
                        .iter()
                        .filter(|old| {
                            !turns.iter().any(|v| v.id == old.id)
                                && (!old.status.is_final() || !old.bound)
                        })
                        .cloned(),
                );
            }
            t.turns = next;
            if cursor.is_some() {
                t.history_cursor = cursor.clone();
            }
        }
        QueueUpdated {
            backend: kind,
            thread_id,
            messages,
            previously_queued,
        } => {
            let t = thread(s, *kind, thread_id);
            let mut incoming = Vec::<&QueuedMessage>::new();
            for message in messages {
                if !incoming
                    .iter()
                    .any(|m| m.client_message_id == message.client_message_id)
                {
                    incoming.push(message);
                }
            }
            let active: Vec<_> = t
                .turns
                .iter()
                .filter(|v| v.status != TurnStatus::Queued)
                .filter_map(|v| v.client_message_id.clone())
                .collect();
            let old = t.turns.clone();
            t.turns.retain(|v| {
                v.status != TurnStatus::Queued
                    || !v.client_message_id.as_ref().is_some_and(|id| {
                        previously_queued.contains(id)
                            || incoming.iter().any(|m| &m.client_message_id == id)
                    })
            });
            for message in incoming
                .iter()
                .filter(|m| !active.contains(&m.client_message_id))
            {
                let mut queued = old
                    .iter()
                    .find(|v| {
                        v.status == TurnStatus::Queued
                            && v.client_message_id.as_ref() == Some(&message.client_message_id)
                    })
                    .cloned()
                    .unwrap_or_else(|| Turn {
                        id: message.client_message_id.clone(),
                        client_message_id: Some(message.client_message_id.clone()),
                        status: TurnStatus::Queued,
                        bound: false,
                        ..Default::default()
                    });
                queued.items = vec![local_user(&message.client_message_id, &message.parts)];
                t.turns.push(queued);
            }
        }
        TurnSubmitted {
            backend: kind,
            thread_id,
            client_message_id,
            parts,
            settings,
            at_ms,
        } => {
            let t = thread(s, *kind, thread_id);
            if !t
                .turns
                .iter()
                .any(|v| v.client_message_id.as_ref() == Some(client_message_id))
            {
                t.turns.push(Turn {
                    id: client_message_id.clone(),
                    client_message_id: Some(client_message_id.clone()),
                    status: TurnStatus::Queued,
                    items: vec![local_user(client_message_id, parts)],
                    settings: settings.clone(),
                    started_at_ms: *at_ms,
                    bound: false,
                    ..Default::default()
                });
            }
        }
        TurnBound {
            backend: kind,
            thread_id,
            client_message_id,
            turn_id,
        } => bind(thread(s, *kind, thread_id), client_message_id, turn_id),
        TurnStarted {
            backend: kind,
            thread_id,
            turn_id,
            client_message_id,
            at_ms,
        } => {
            let t = thread(s, *kind, thread_id);
            if let Some(id) = client_message_id {
                bind(t, id, turn_id);
            }
            let v = turn(t, turn_id);
            if v.client_message_id.is_none() {
                v.client_message_id = client_message_id.clone();
            }
            if !v.status.is_final() {
                v.status = TurnStatus::Running;
            }
            if v.started_at_ms.is_none() {
                v.started_at_ms = *at_ms;
            }
            v.bound = true;
            if idle_like(t.run_state) {
                t.run_state = RunState::Running;
            }
        }
        TurnCancelled {
            backend: kind,
            thread_id,
            turn_id,
        } => {
            if let Some(t) = thread(s, *kind, thread_id)
                .turns
                .iter_mut()
                .find(|v| &v.id == turn_id)
                && !t.status.is_final()
            {
                t.status = TurnStatus::Cancelled;
            }
        }
        TurnCompleted {
            backend: kind,
            thread_id,
            turn_id,
            status,
            error,
            items,
            duration_ms,
            usage,
            at_ms,
        } => {
            let t = thread(s, *kind, thread_id);
            for item in items {
                item_upsert(t, turn_id, item, true);
            }
            let v = turn(t, turn_id);
            v.status = *status;
            if error.is_some() {
                v.error = error.clone();
            }
            if duration_ms.is_some() {
                v.duration_ms = *duration_ms;
            }
            if usage.is_some() {
                v.usage = usage.clone();
            }
            if at_ms.is_some() {
                v.completed_at_ms = *at_ms;
            }
            v.bound = true;
            finish_items(v);
            if !t.turns.iter().any(|v| v.status == TurnStatus::Running)
                && (t.run_state == RunState::Running || waiting(t.run_state))
            {
                t.run_state = RunState::Idle;
            }
            for r in s.requests.values_mut().filter(|r| {
                r.key.backend == *kind
                    && r.status == RequestStatus::Pending
                    && r.thread_id.as_ref() == Some(thread_id)
                    && r.turn_id.as_ref() == Some(turn_id)
            }) {
                r.status = RequestStatus::Expired;
            }
        }
        TurnPlanUpdated {
            backend: kind,
            thread_id,
            turn_id,
            plan,
        } => turn(thread(s, *kind, thread_id), turn_id).plan = Some(plan.clone()),
        TurnDiffUpdated {
            backend: kind,
            thread_id,
            turn_id,
            diff,
        } => turn(thread(s, *kind, thread_id), turn_id).diff = Some(diff.clone()),
        TurnProgress {
            backend: kind,
            thread_id,
            turn_id,
            thinking_tokens,
        } => {
            if let Some(v) = thread(s, *kind, thread_id)
                .turns
                .iter_mut()
                .find(|v| &v.id == turn_id)
            {
                v.thinking_tokens = *thinking_tokens;
            }
        }
        TurnNotice {
            backend: kind,
            thread_id,
            turn_id,
            notice,
        } => {
            let t = thread(s, *kind, thread_id);
            if let Some(id) = turn_id {
                let v = turn(t, id);
                let n = v
                    .items
                    .iter()
                    .filter(|i| matches!(i, Item::Notice(_)))
                    .count();
                v.items.push(Item::Notice(NoticeItem {
                    id: format!("notice-{n}"),
                    notice: notice.clone(),
                    raw: notice.raw.clone(),
                    ..Default::default()
                }));
            } else {
                bounded(&mut t.notices, notice.clone(), MAX_NOTICES);
            }
        }
        ItemStarted {
            backend: kind,
            thread_id,
            turn_id,
            item,
        } => item_upsert(thread(s, *kind, thread_id), turn_id, item, false),
        ItemCompleted {
            backend: kind,
            thread_id,
            turn_id,
            item,
        } => item_upsert(thread(s, *kind, thread_id), turn_id, item, true),
        ItemUpdated {
            backend: kind,
            thread_id,
            turn_id,
            item_id,
            delta,
        } => {
            let v = turn(thread(s, *kind, thread_id), turn_id);
            let index = v
                .items
                .iter()
                .position(|i| i.id() == item_id)
                .unwrap_or_else(|| {
                    v.items.push(placeholder(item_id, delta));
                    v.items.len() - 1
                });
            apply_delta(&mut v.items[index], delta);
        }
        ItemDeclined {
            backend: kind,
            thread_id,
            turn_id,
            item_id,
        } => {
            for v in thread(s, *kind, thread_id)
                .turns
                .iter_mut()
                .filter(|v| turn_id.as_ref().is_none_or(|id| id == &v.id))
            {
                for i in v.items.iter_mut().filter(|i| i.id() == item_id) {
                    i.set_status(ItemStatus::Declined);
                }
            }
        }
        RequestOpened { request } => {
            let mut next = request.clone();
            if let Some(old) = s.requests.get(&request.key) {
                if old.status != RequestStatus::Pending {
                    s.requests.remove(&request.key);
                } else {
                    next.received_at_ms = old.received_at_ms.or(request.received_at_ms);
                }
            }
            s.requests.insert(request.key.clone(), next);
            if let Some(id) = request
                .thread_id
                .as_ref()
                .filter(|_| request.status == RequestStatus::Pending)
            {
                let t = thread(s, request.key.backend, id);
                if matches!(t.run_state, RunState::Running | RunState::Idle) {
                    t.run_state = if matches!(
                        request.kind,
                        RequestKind::UserInput { .. } | RequestKind::Elicitation { .. }
                    ) {
                        RunState::WaitingInput
                    } else {
                        RunState::WaitingApproval
                    };
                }
            }
        }
        RequestClosed {
            key,
            status,
            answer,
        } => {
            let Some(r) = s.requests.get_mut(key) else {
                return;
            };
            let late = *status == RequestStatus::Answered
                && matches!(
                    r.status,
                    RequestStatus::Resolved | RequestStatus::Expired | RequestStatus::Cancelled
                );
            if r.status != RequestStatus::Pending && !late {
                return;
            }
            r.status = *status;
            if answer.is_some() {
                r.answer = answer.clone();
            }
            let Some(id) = r.thread_id.clone() else {
                return;
            };
            if s.requests.values().any(|v| {
                v.status == RequestStatus::Pending
                    && v.key.backend == key.backend
                    && v.thread_id.as_ref() == Some(&id)
            }) {
                return;
            }
            let t = thread(s, key.backend, &id);
            if waiting(t.run_state) {
                t.run_state = if t.turns.iter().any(|v| v.status == TurnStatus::Running) {
                    RunState::Running
                } else {
                    RunState::Idle
                };
            }
        }
    }
}

fn local_user(id: &str, parts: &[UserPart]) -> Item {
    Item::UserMessage(UserMessageItem {
        id: format!("local:{id}"),
        parts: parts.to_vec(),
        client_message_id: Some(id.into()),
        local: true,
        ..Default::default()
    })
}
fn bind(t: &mut ThreadState, client: &str, id: &str) {
    let local = t
        .turns
        .iter()
        .position(|v| v.client_message_id.as_deref() == Some(client) && v.id != id);
    let target = t.turns.iter().position(|v| v.id == id);
    let Some(local) = local else {
        if let Some(i) = target {
            let v = &mut t.turns[i];
            if v.client_message_id.is_none() {
                v.client_message_id = Some(client.into());
            }
            v.bound = true;
        }
        return;
    };
    let Some(target) = target else {
        t.turns[local].id = id.into();
        t.turns[local].bound = true;
        return;
    };
    let old = t.turns[local].clone();
    let mut next = t.turns[target].clone();
    let users: Vec<_> = old
        .items
        .into_iter()
        .filter(|i| matches!(i, Item::UserMessage(_)))
        .collect();
    let echoed=next.items.iter().any(|i| matches!(i, Item::UserMessage(u) if !u.local && u.client_message_id.as_deref().is_none_or(|v| v==client)));
    if !echoed {
        if next.items.iter().any(|i| matches!(i, Item::UserMessage(_))) {
            next.items.extend(users);
        } else {
            let mut items = users;
            items.append(&mut next.items);
            next.items = items;
        }
    }
    if next.client_message_id.is_none() {
        next.client_message_id = Some(client.into());
    }
    if next.settings.is_none() {
        next.settings = old.settings;
    }
    if next.started_at_ms.is_none() {
        next.started_at_ms = old.started_at_ms;
    }
    next.bound = true;
    t.turns[local.min(target)] = next;
    t.turns.remove(local.max(target));
}
fn item_upsert(t: &mut ThreadState, id: &str, item: &Item, completed: bool) {
    if let Item::UserMessage(u) = item
        && !u.local
        && let Some(client) = &u.client_message_id
    {
        bind(t, client, id);
    }
    let v = turn(t, id);
    if let Item::UserMessage(echo) = item
        && !echo.local
    {
        let locals: Vec<_> = v
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| match item {
                Item::UserMessage(u) if u.local => Some((i, u)),
                _ => None,
            })
            .collect();
        let found = locals
            .iter()
            .find(|(_, u)| {
                u.client_message_id.is_some() && u.client_message_id == echo.client_message_id
            })
            .or(locals.first())
            .map(|(i, _)| *i);
        if let Some(i) = found {
            if v.items.iter().any(|v| v.id() == echo.id) {
                v.items.remove(i);
            } else {
                let Item::UserMessage(local) = &v.items[i] else {
                    unreachable!()
                };
                let mut replacement = echo.clone();
                if replacement.client_message_id.is_none() {
                    replacement.client_message_id = local.client_message_id.clone();
                }
                if !local.parts.is_empty() {
                    replacement.parts = local.parts.clone();
                }
                v.items[i] = Item::UserMessage(replacement);
                if v.status == TurnStatus::Queued {
                    v.status = TurnStatus::Running;
                }
                return;
            }
        }
    }
    if !completed
        && item.parent_id().is_none()
        && matches!(
            item,
            Item::Command(_)
                | Item::FileChange(_)
                | Item::ToolCall(_)
                | Item::SubAgent(_)
                | Item::WebSearch(_)
        )
    {
        for i in &mut v.items {
            if let Item::AgentMessage(m) = i
                && m.parent_id.is_none()
                && m.phase == MessagePhase::Unknown
            {
                m.phase = MessagePhase::Commentary;
            }
        }
    }
    if let Some(i) = v.items.iter().position(|v| v.id() == item.id()) {
        let old = &v.items[i];
        let mut next = merge_content(old, item);
        if (!completed && old.status().is_final())
            || (completed
                && old.status() == ItemStatus::Declined
                && item.status() == ItemStatus::Failed)
        {
            next.set_status(old.status());
        }
        v.items[i] = next;
    } else {
        v.items.push(item.clone());
    }
    if v.status == TurnStatus::Queued {
        v.status = TurnStatus::Running;
    }
}
fn merge_content(old: &Item, incoming: &Item) -> Item {
    let mut next = incoming.clone();
    macro_rules! absent { ($a:ident,$b:ident,$($f:ident),*) => { $(if $b.$f.is_none() { $b.$f=$a.$f.clone(); })* } }
    macro_rules! empty { ($a:ident,$b:ident,$($f:ident),*) => { $(if $b.$f.is_empty() { $b.$f=$a.$f.clone(); })* } }
    match (old, &mut next) {
        (Item::AgentMessage(a), Item::AgentMessage(b)) => {
            empty!(a, b, text);
            if b.phase == MessagePhase::Unknown {
                b.phase = a.phase;
            }
            absent!(a, b, parent_id);
        }
        (Item::Reasoning(a), Item::Reasoning(b)) => {
            if b.summary.iter().all(String::is_empty) {
                b.summary = a.summary.clone();
            }
            if b.content.iter().all(String::is_empty) {
                b.content = a.content.clone();
            }
            absent!(a, b, parent_id);
        }
        (Item::Command(a), Item::Command(b)) => {
            if b.output.is_empty() {
                b.output_truncated = a.output_truncated;
            }
            empty!(a, b, output, command);
            absent!(a, b, cwd, description, parent_id);
        }
        (Item::FileChange(a), Item::FileChange(b)) => {
            empty!(a, b, output, changes);
            absent!(a, b, parent_id);
        }
        (Item::Plan(a), Item::Plan(b)) => {
            empty!(a, b, text, steps);
            absent!(a, b, parent_id);
        }
        (Item::ToolCall(a), Item::ToolCall(b)) => {
            empty!(a, b, arguments_text, progress);
            absent!(a, b, arguments, parent_id);
        }
        (Item::SubAgent(a), Item::SubAgent(b)) => {
            empty!(a, b, progress);
            absent!(a, b, description, prompt, parent_id);
        }
        (Item::UserMessage(a), Item::UserMessage(b)) => {
            empty!(a, b, parts);
            absent!(a, b, client_message_id, parent_id);
        }
        _ => (),
    }
    next
}
fn placeholder(id: &str, delta: &ItemDelta) -> Item {
    let id = id.to_owned();
    match delta {
        ItemDelta::AgentText { .. } => Item::AgentMessage(AgentMessageItem {
            id,
            ..Default::default()
        }),
        ItemDelta::ReasoningSummaryPart { .. }
        | ItemDelta::ReasoningSummary { .. }
        | ItemDelta::ReasoningText { .. } => Item::Reasoning(ReasoningItem {
            id,
            ..Default::default()
        }),
        ItemDelta::PlanText { .. } => Item::Plan(PlanItem {
            id,
            ..Default::default()
        }),
        ItemDelta::CommandOutput { .. } | ItemDelta::TerminalInput { .. } => {
            Item::Command(CommandItem {
                id,
                ..Default::default()
            })
        }
        ItemDelta::FileChangeOutput { .. } | ItemDelta::FileChangePatch { .. } => {
            Item::FileChange(FileChangeItem {
                id,
                ..Default::default()
            })
        }
        ItemDelta::ToolArguments { .. } | ItemDelta::ToolProgress { .. } => {
            Item::ToolCall(ToolCallItem {
                id,
                kind: ToolKind::Builtin,
                ..Default::default()
            })
        }
    }
}
fn append_at(list: &mut Vec<String>, index: i32, text: &str) {
    // Malformed vendor indices must not allocate unbounded memory or panic.
    let Ok(i) = usize::try_from(index) else {
        return;
    };
    if i > 65_536 {
        return;
    }
    list.resize_with(list.len().max(i + 1), String::new);
    list[i].push_str(text);
}
fn apply_delta(item: &mut Item, delta: &ItemDelta) {
    use ItemDelta::*;
    match (item, delta) {
        (Item::AgentMessage(v), AgentText { text }) => v.text.push_str(text),
        (Item::Reasoning(v), ReasoningSummaryPart { index }) => {
            append_at(&mut v.summary, *index, "")
        }
        (Item::Reasoning(v), ReasoningSummary { index, text }) => {
            append_at(&mut v.summary, *index, text)
        }
        (Item::Reasoning(v), ReasoningText { index, text }) => {
            append_at(&mut v.content, *index, text)
        }
        (Item::Plan(v), PlanText { text }) => v.text.push_str(text),
        (Item::Command(v), CommandOutput { text } | TerminalInput { text }) => {
            v.output.push_str(text);
            let units = v.output.encode_utf16().count();
            if units > MAX_COMMAND_OUTPUT {
                let mut remove = units - MAX_COMMAND_OUTPUT;
                let mut at = 0;
                for (i, c) in v.output.char_indices() {
                    if remove == 0 {
                        at = i;
                        break;
                    }
                    remove = remove.saturating_sub(c.len_utf16());
                    at = i + c.len_utf8();
                }
                v.output.drain(..at);
                v.output_truncated = true;
            }
        }
        (Item::FileChange(v), FileChangeOutput { text }) => v.output.push_str(text),
        (Item::FileChange(v), FileChangePatch { changes }) => v.changes = changes.clone(),
        (Item::ToolCall(v), ToolArguments { partial_json }) => {
            v.arguments_text.push_str(partial_json)
        }
        (Item::ToolCall(v), ToolProgress { message }) => {
            bounded(&mut v.progress, message.clone(), 50)
        }
        (Item::SubAgent(v), ToolProgress { message }) => {
            bounded(&mut v.progress, message.clone(), 50)
        }
        _ => (),
    }
}
fn finish_items(t: &mut Turn) {
    let last = t
        .items
        .iter()
        .rposition(|i| matches!(i,Item::AgentMessage(v) if v.parent_id.is_none()));
    for (n, item) in t.items.iter_mut().enumerate() {
        if item.status() == ItemStatus::InProgress {
            item.set_status(ItemStatus::Incomplete);
        }
        if let Item::AgentMessage(v) = item
            && v.phase == MessagePhase::Unknown
        {
            v.phase = if Some(n) == last {
                MessagePhase::Final
            } else {
                MessagePhase::Commentary
            };
        }
    }
}
