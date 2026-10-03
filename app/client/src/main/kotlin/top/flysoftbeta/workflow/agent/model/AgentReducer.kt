package top.flysoftbeta.workflow.agent.model

/**
 * Pure fold of [AgentEvent]s into [AgentState]. No IO, no clocks, no randomness: the same event
 * sequence always yields the same state, which is what the replay tests assert.
 *
 * Rules that matter for correctness:
 * - Deltas append to the item with the same id; a missing item is created as a placeholder.
 * - A completion replaces the item with the authoritative payload but keeps streamed content the
 *   payload omits (Codex `aggregatedOutput: null`, empty reasoning content).
 * - When a turn ends, every still in-progress item becomes INCOMPLETE and every open request of
 *   that turn EXPIRED (visible, disabled). Unknown agent-message phases resolve: last = FINAL.
 * - A process exit ends all running turns of that backend and expires its open requests.
 * - Requests are only ever closed by events; the reducer never answers anything.
 */
object AgentReducer {
    const val MAX_UNKNOWN = 200
    const val MAX_NOTICES = 100
    const val MAX_COMMAND_OUTPUT = 512 * 1024

    fun reduceAll(state: AgentState, events: Iterable<AgentEvent>): AgentState = events.fold(state, ::reduce)

    fun reduce(state: AgentState, event: AgentEvent): AgentState = when (event) {
        is AgentEvent.ProcessChanged -> processChanged(state, event)
        is AgentEvent.ServerInfo -> state.backend(event.backend) { it.copy(serverInfo = event.info) }
        is AgentEvent.AccountChanged -> state.backend(event.backend) { it.copy(account = accountRead(it.account, event.account)) }
        is AgentEvent.LoginChanged -> state.backend(event.backend) {
            val loginState = when (val flow = event.flow) {
                null -> it.account.state
                is LoginFlow.Completed -> if (flow.success) LoginState.LOGGED_IN else it.account.state.takeIf { s -> s != LoginState.LOGGING_IN } ?: LoginState.LOGGED_OUT
                else -> LoginState.LOGGING_IN
            }
            // A new attempt starts without the previous attempt's check failure.
            val checkError = if (event.flow.isPending) null else it.account.checkError
            it.copy(account = it.account.copy(login = event.flow, state = loginState, checkError = checkError))
        }
        is AgentEvent.AccountCheckFailed -> state.backend(event.backend) { it.copy(account = it.account.copy(checkError = event.message)) }
        is AgentEvent.RateLimitsChanged -> state.backend(event.backend) {
            val previous = it.rateLimits
            it.copy(rateLimits = if (event.merge && previous != null) mergeLimits(previous, event.limits) else event.limits)
        }
        is AgentEvent.ModelsChanged -> state.backend(event.backend) { it.copy(models = event.catalog) }
        is AgentEvent.McpServerChanged -> state.backend(event.backend) { it.copy(mcpServers = it.mcpServers + (event.status.name to event.status)) }
        is AgentEvent.BackendNotice -> state.backend(event.backend) { it.copy(notices = (it.notices + event.notice).takeLast(MAX_NOTICES)) }
        is AgentEvent.Unknown -> unknown(state, event)

        is AgentEvent.ThreadUpserted -> state.thread(event.backend, event.threadId) { t ->
            t.copy(
                title = event.title ?: t.title,
                preview = event.preview?.takeIf { it.isNotEmpty() } ?: t.preview,
                cwd = event.cwd ?: t.cwd,
                path = event.path ?: t.path,
                forkedFrom = event.forkedFrom ?: t.forkedFrom,
                ephemeral = event.ephemeral ?: t.ephemeral,
                runState = event.runState ?: t.runState,
                settings = event.settings?.let { mergeSettings(t.settings, it) } ?: t.settings,
                createdAtSec = event.createdAtSec ?: t.createdAtSec,
                updatedAtSec = event.updatedAtSec ?: t.updatedAtSec,
                raw = event.raw ?: t.raw,
            )
        }
        is AgentEvent.ThreadStatusChanged -> state.thread(event.backend, event.threadId) { it.copy(runState = event.runState) }
        is AgentEvent.ThreadRenamed -> state.thread(event.backend, event.threadId) { it.copy(title = event.title) }
        is AgentEvent.ThreadArchived -> state.thread(event.backend, event.threadId) { it.copy(archived = event.archived) }
        is AgentEvent.ThreadDeleted -> state.thread(event.backend, event.threadId) { it.copy(deleted = true, runState = RunState.CLOSED) }
        is AgentEvent.ThreadClosed -> state.thread(event.backend, event.threadId) { it.copy(runState = RunState.NOT_LOADED) }
        is AgentEvent.ThreadSettingsChanged -> state.thread(event.backend, event.threadId) { it.copy(settings = mergeSettings(it.settings, event.settings)) }
        is AgentEvent.TokenUsageChanged -> state.thread(event.backend, event.threadId) { t ->
            val withTurn = event.turnId?.let { id -> t.updateTurn(id, create = false) { it.copy(usage = event.usage) } } ?: t
            withTurn.copy(usage = event.usage)
        }
        is AgentEvent.ThreadNotice -> state.thread(event.backend, event.threadId) { it.copy(notices = (it.notices + event.notice).takeLast(MAX_NOTICES)) }
        is AgentEvent.HistoryLoaded -> state.thread(event.backend, event.threadId) { historyLoaded(it, event) }

        is AgentEvent.QueueUpdated -> state.thread(event.backend, event.threadId) { thread ->
            val incoming = event.messages.distinctBy { it.clientMessageId }
            val ids = incoming.map { it.clientMessageId }.toSet()
            val activeIds = thread.turns.filter { it.status != TurnStatus.QUEUED }.mapNotNull { it.clientMessageId }.toSet()
            val old = thread.queuedTurns.associateBy { it.clientMessageId }
            val kept = thread.turns.filterNot {
                it.status == TurnStatus.QUEUED && (it.clientMessageId in ids || it.clientMessageId in event.previouslyQueued)
            }
            val queued = incoming.filter { it.clientMessageId !in activeIds }.map { message ->
                val previous = old[message.clientMessageId]
                (previous ?: Turn(message.clientMessageId, clientMessageId = message.clientMessageId, status = TurnStatus.QUEUED, bound = false))
                    .copy(items = listOf(UserMessageItem(localId(message.clientMessageId), message.parts, message.clientMessageId, local = true)))
            }
            thread.copy(turns = kept + queued)
        }

        is AgentEvent.TurnSubmitted -> state.thread(event.backend, event.threadId) { t ->
            if (t.turns.any { it.clientMessageId == event.clientMessageId }) t
            else t.copy(turns = t.turns + Turn(
                id = event.clientMessageId,
                clientMessageId = event.clientMessageId,
                status = TurnStatus.QUEUED,
                items = listOf(UserMessageItem(localId(event.clientMessageId), event.parts, event.clientMessageId, local = true)),
                settings = event.settings,
                startedAtMs = event.atMs,
                bound = false,
            ))
        }
        is AgentEvent.TurnBound -> state.thread(event.backend, event.threadId) { bind(it, event.clientMessageId, event.turnId) }
        is AgentEvent.TurnStarted -> state.thread(event.backend, event.threadId) { t ->
            val bound = event.clientMessageId?.let { bind(t, it, event.turnId) } ?: t
            val started = bound.updateTurn(event.turnId) { turn ->
                turn.copy(
                    clientMessageId = turn.clientMessageId ?: event.clientMessageId,
                    status = if (turn.status.isFinal) turn.status else TurnStatus.RUNNING,
                    startedAtMs = turn.startedAtMs ?: event.atMs,
                    bound = true,
                )
            }
            if (started.runState in IDLE_LIKE) started.copy(runState = RunState.RUNNING) else started
        }
        is AgentEvent.TurnCancelled -> state.thread(event.backend, event.threadId) { t ->
            t.updateTurn(event.turnId, create = false) { if (it.status.isFinal) it else it.copy(status = TurnStatus.CANCELLED) }
        }
        is AgentEvent.TurnCompleted -> turnCompleted(state, event)
        is AgentEvent.TurnPlanUpdated -> state.thread(event.backend, event.threadId) { t -> t.updateTurn(event.turnId) { it.copy(plan = event.plan) } }
        is AgentEvent.TurnDiffUpdated -> state.thread(event.backend, event.threadId) { t -> t.updateTurn(event.turnId) { it.copy(diff = event.diff) } }
        is AgentEvent.TurnProgress -> state.thread(event.backend, event.threadId) { t -> t.updateTurn(event.turnId, create = false) { it.copy(thinkingTokens = event.thinkingTokens) } }
        is AgentEvent.TurnNotice -> state.thread(event.backend, event.threadId) { t ->
            val turnId = event.turnId
            if (turnId == null) t.copy(notices = (t.notices + event.notice).takeLast(MAX_NOTICES))
            else t.updateTurn(turnId) { turn ->
                val n = turn.items.count { it is NoticeItem }
                turn.copy(items = turn.items + NoticeItem("notice-$n", event.notice))
            }
        }

        is AgentEvent.ItemStarted -> state.thread(event.backend, event.threadId) { itemUpsert(it, event.turnId, event.item, completed = false) }
        is AgentEvent.ItemCompleted -> state.thread(event.backend, event.threadId) { itemUpsert(it, event.turnId, event.item, completed = true) }
        is AgentEvent.ItemUpdated -> state.thread(event.backend, event.threadId) { t ->
            t.updateTurn(event.turnId) { turn -> turn.updateItem(event.itemId, placeholder(event.itemId, event.delta)) { applyDelta(it, event.delta) } }
        }
        is AgentEvent.ItemDeclined -> state.thread(event.backend, event.threadId) { t ->
            t.copy(turns = t.turns.map { turn ->
                if (event.turnId != null && turn.id != event.turnId) turn
                else turn.copy(items = turn.items.map { if (it.id == event.itemId) it.withStatus(ItemStatus.DECLINED) else it })
            })
        }

        is AgentEvent.RequestOpened -> requestOpened(state, event.request)
        is AgentEvent.RequestClosed -> requestClosed(state, event)
    }

    // ------------------------------------------------------------------ helpers

    private val IDLE_LIKE = setOf(RunState.IDLE, RunState.NOT_LOADED, RunState.ERROR)
    private val WAITING = setOf(RunState.WAITING_APPROVAL, RunState.WAITING_INPUT)
    private val SERVER_CLOSED = setOf(RequestStatus.RESOLVED, RequestStatus.EXPIRED, RequestStatus.CANCELLED)

    fun localId(clientMessageId: String) = "local:$clientMessageId"

    private inline fun AgentState.backend(kind: BackendKind, f: (BackendStatus) -> BackendStatus): AgentState =
        copy(backends = backends + (kind to f(backend(kind))))

    private inline fun AgentState.thread(kind: BackendKind, id: String, f: (ThreadState) -> ThreadState): AgentState {
        val key = ThreadKey(kind, id)
        return copy(threads = threads + (key to f(threads[key] ?: ThreadState(key))))
    }

    private inline fun ThreadState.updateTurn(id: String, create: Boolean = true, f: (Turn) -> Turn): ThreadState {
        val index = turns.indexOfFirst { it.id == id }
        return when {
            index >= 0 -> copy(turns = turns.toMutableList().also { it[index] = f(it[index]) })
            create -> copy(turns = turns + f(Turn(id)))
            else -> this
        }
    }

    private inline fun Turn.updateItem(id: String, placeholder: Item?, f: (Item) -> Item): Turn {
        val index = items.indexOfFirst { it.id == id }
        return when {
            index >= 0 -> copy(items = items.toMutableList().also { it[index] = f(it[index]) })
            placeholder != null -> copy(items = items + f(placeholder))
            else -> this
        }
    }

    private fun mergeSettings(old: ThreadSettings, new: ThreadSettings) = ThreadSettings(
        model = new.model ?: old.model,
        effort = new.effort ?: old.effort,
        approvalPolicy = new.approvalPolicy ?: old.approvalPolicy,
        sandbox = new.sandbox ?: old.sandbox,
        approvalsReviewer = new.approvalsReviewer ?: old.approvalsReviewer,
        raw = new.raw ?: old.raw,
    )

    private fun mergeLimits(old: RateLimitState, new: RateLimitState): RateLimitState {
        val merged = LinkedHashMap<String, RateLimit>()
        old.limits.forEach { merged[it.id] = it }
        new.limits.forEach { merged[it.id] = it }
        return old.copy(
            limits = merged.values.toList(),
            ordinaryUsageAllowed = new.ordinaryUsageAllowed ?: old.ordinaryUsageAllowed,
            upsell = new.upsell ?: old.upsell,
        )
    }

    private fun unknown(state: AgentState, event: AgentEvent.Unknown): AgentState {
        val record = UnknownRecord(event.kind, event.threadId, event.raw)
        val withBackend = state.backend(event.backend) { it.copy(unknown = (it.unknown + record).takeLast(MAX_UNKNOWN)) }
        val threadId = event.threadId ?: return withBackend
        val key = ThreadKey(event.backend, threadId)
        val thread = withBackend.threads[key] ?: return withBackend
        return withBackend.copy(threads = withBackend.threads + (key to thread.copy(unknown = (thread.unknown + record).takeLast(MAX_UNKNOWN))))
    }

    /**
     * Folds one answered account read. Login rules (docs/engine/chat.md "Codex login state machine"):
     * - An authenticated read always wins and ends any login flow.
     * - After a successful completion of this process, a read that is not authenticated changes nothing:
     *   a null or lagging read never overrides a confirmed completion.
     * - While an attempt is pending, a logged-out read keeps LOGGING_IN and the flow.
     * - Otherwise the read replaces the account and keeps a failure or terminal flow visible.
     */
    private fun accountRead(previous: AccountState, read: AccountState): AccountState = when {
        read.state == LoginState.LOGGED_IN -> read
        previous.login.isConfirmedSuccess -> previous.copy(checkError = null)
        previous.login.isPending -> read.copy(state = LoginState.LOGGING_IN, login = previous.login)
        else -> read.copy(login = read.login ?: previous.login.takeIf { !it.isConfirmedSuccess })
    }

    private fun processChanged(state: AgentState, event: AgentEvent.ProcessChanged): AgentState {
        val updated = state.backend(event.backend) {
            // A confirmed completion belongs to the process that reported it; a new process reads afresh.
            val account = if (event.state is ProcessState.Starting && it.account.login.isConfirmedSuccess) it.account.copy(login = null) else it.account
            it.copy(process = event.state, account = account)
        }
        val message = when (val s = event.state) {
            is ProcessState.Exited -> "Backend process exited" + (s.exitCode?.let { " ($it)" } ?: "")
            is ProcessState.Failed -> s.message
            else -> return updated
        }
        val threads = updated.threads.mapValues { (key, thread) ->
            if (key.backend != event.backend) return@mapValues thread
            thread.copy(
                runState = if (thread.deleted) thread.runState else RunState.NOT_LOADED,
                turns = thread.turns.map { turn ->
                    when (turn.status) {
                        TurnStatus.RUNNING -> finishItems(turn.copy(status = TurnStatus.FAILED, error = turn.error ?: TurnError(message, code = "processExited")))
                        TurnStatus.QUEUED -> turn.copy(status = TurnStatus.CANCELLED)
                        else -> turn
                    }
                },
            )
        }
        val requests = updated.requests.mapValues { (key, request) ->
            if (key.backend == event.backend && request.status.isOpen) request.copy(status = RequestStatus.EXPIRED) else request
        }
        return updated.copy(threads = threads, requests = requests)
    }

    private fun historyLoaded(thread: ThreadState, event: AgentEvent.HistoryLoaded): ThreadState {
        val loadedIds = event.turns.map { it.id }.toSet()
        val turns = if (event.prepend) {
            event.turns.filter { loaded -> thread.turns.none { it.id == loaded.id } } + thread.turns
        } else {
            val keep = thread.turns.filter { it.id !in loadedIds && (!it.status.isFinal || !it.bound) }
            // A live turn is kept when it is still running or richer than the (possibly summarised) history copy.
            event.turns.map { loaded -> thread.turn(loaded.id)?.takeIf { !it.status.isFinal || it.items.size >= loaded.items.size } ?: loaded } + keep
        }
        return thread.copy(turns = turns, historyCursor = event.cursor ?: thread.historyCursor)
    }

    /** Merges the optimistic local turn for [clientMessageId] into backend turn [turnId]. */
    private fun bind(thread: ThreadState, clientMessageId: String, turnId: String): ThreadState {
        val localIndex = thread.turns.indexOfFirst { it.clientMessageId == clientMessageId && it.id != turnId }
        val targetIndex = thread.turns.indexOfFirst { it.id == turnId }
        if (localIndex < 0) {
            if (targetIndex < 0) return thread
            return thread.updateTurn(turnId) { it.copy(clientMessageId = it.clientMessageId ?: clientMessageId, bound = true) }
        }
        val local = thread.turns[localIndex]
        val list = thread.turns.toMutableList()
        if (targetIndex < 0) {
            list[localIndex] = local.copy(id = turnId, bound = true)
        } else {
            val target = list[targetIndex]
            val localUsers = local.items.filterIsInstance<UserMessageItem>()
            // An echo without clientId (server did not report it) is taken to be ours.
            val echoed = target.items.any { it is UserMessageItem && !it.local && (it.clientMessageId == clientMessageId || it.clientMessageId == null) }
            val hasUser = target.items.any { it is UserMessageItem }
            val merged = target.copy(
                clientMessageId = target.clientMessageId ?: clientMessageId,
                // First message of a turn goes in front; a steer message joins the running turn at its end.
                items = when {
                    echoed -> target.items
                    !hasUser -> localUsers + target.items
                    else -> target.items + localUsers
                },
                settings = target.settings ?: local.settings,
                startedAtMs = target.startedAtMs ?: local.startedAtMs,
                bound = true,
            )
            // Keep the earlier of the two positions so submission order is preserved.
            val keep = minOf(localIndex, targetIndex)
            val drop = maxOf(localIndex, targetIndex)
            list[keep] = merged
            list.removeAt(drop)
        }
        return thread.copy(turns = list)
    }

    private fun itemUpsert(thread: ThreadState, turnId: String, item: Item, completed: Boolean): ThreadState {
        var t = thread
        if (item is UserMessageItem && !item.local && item.clientMessageId != null) t = bind(t, item.clientMessageId, turnId)
        return t.updateTurn(turnId) { turn ->
            var working = turn
            if (item is UserMessageItem && !item.local) {
                val replaced = dropLocalEcho(working, item)
                // The optimistic copy was replaced in place by the echo (keeping the composed parts): done.
                if (replaced !== working && replaced.items.any { it.id == item.id } && working.items.none { it.id == item.id }) {
                    return@updateTurn replaced.copy(status = if (replaced.status == TurnStatus.QUEUED) TurnStatus.RUNNING else replaced.status)
                }
                working = replaced
            }
            if (!completed && item.parentId == null && item.isToolLike()) working = markCommentary(working)
            val existing = working.item(item.id)
            val next = when {
                existing == null -> item
                completed -> mergeCompleted(existing, item)
                else -> mergeStarted(existing, item)
            }
            val items = if (existing == null) working.items + next else working.items.map { if (it.id == item.id) next else it }
            working.copy(items = items, status = if (working.status == TurnStatus.QUEUED) TurnStatus.RUNNING else working.status)
        }
    }

    /** Replaces the optimistic local copy of a user message in place with the backend's echo. */
    private fun dropLocalEcho(turn: Turn, echo: UserMessageItem): Turn {
        val locals = turn.items.filterIsInstance<UserMessageItem>().filter { it.local }
        if (locals.isEmpty()) return turn
        val match = locals.firstOrNull { it.clientMessageId != null && it.clientMessageId == echo.clientMessageId } ?: locals.first()
        if (turn.items.any { it.id == echo.id }) return turn.copy(items = turn.items.filter { it !== match })
        val replacement = echo.copy(
            clientMessageId = echo.clientMessageId ?: match.clientMessageId,
            // The composed parts (workspace paths) are richer than the backend's transformed echo (base64, @path).
            parts = match.parts.ifEmpty { echo.parts },
        )
        return turn.copy(items = turn.items.map { if (it === match) replacement else it })
    }

    private fun Item.isToolLike() = this is CommandItem || this is FileChangeItem || this is ToolCallItem || this is SubAgentItem || this is WebSearchItem

    private fun markCommentary(turn: Turn): Turn = turn.copy(items = turn.items.map {
        if (it is AgentMessageItem && it.parentId == null && it.phase == MessagePhase.UNKNOWN) it.copy(phase = MessagePhase.COMMENTARY) else it
    })

    private fun mergeStarted(existing: Item, incoming: Item): Item {
        val merged = mergeContent(existing, incoming)
        return if (existing.status.isFinal) merged.withStatus(existing.status) else merged
    }

    private fun mergeCompleted(existing: Item, incoming: Item): Item {
        val merged = mergeContent(existing, incoming)
        // A declined decision recorded earlier wins over a generic "failed" completion.
        return if (existing.status == ItemStatus.DECLINED && incoming.status == ItemStatus.FAILED) merged.withStatus(ItemStatus.DECLINED) else merged
    }

    /** [incoming] wins except where it lacks streamed content that [existing] accumulated. */
    private fun mergeContent(existing: Item, incoming: Item): Item {
        val parent = incoming.parentId ?: existing.parentId
        return when {
            existing is AgentMessageItem && incoming is AgentMessageItem -> incoming.copy(
                text = if (incoming.text.isEmpty) existing.text else incoming.text,
                phase = if (incoming.phase == MessagePhase.UNKNOWN) existing.phase else incoming.phase,
                parentId = parent,
            )
            existing is ReasoningItem && incoming is ReasoningItem -> incoming.copy(
                summary = if (incoming.summary.all { it.isEmpty }) existing.summary else incoming.summary,
                content = if (incoming.content.all { it.isEmpty }) existing.content else incoming.content,
                parentId = parent,
            )
            existing is CommandItem && incoming is CommandItem -> incoming.copy(
                output = if (incoming.output.isEmpty) existing.output else incoming.output,
                outputTruncated = if (incoming.output.isEmpty) existing.outputTruncated else incoming.outputTruncated,
                command = incoming.command.ifEmpty { existing.command },
                cwd = incoming.cwd ?: existing.cwd,
                description = incoming.description ?: existing.description,
                parentId = parent,
            )
            existing is FileChangeItem && incoming is FileChangeItem -> incoming.copy(
                output = if (incoming.output.isEmpty) existing.output else incoming.output,
                changes = incoming.changes.ifEmpty { existing.changes },
                parentId = parent,
            )
            existing is PlanItem && incoming is PlanItem -> incoming.copy(
                text = if (incoming.text.isEmpty) existing.text else incoming.text,
                steps = incoming.steps.ifEmpty { existing.steps },
                parentId = parent,
            )
            existing is ToolCallItem && incoming is ToolCallItem -> incoming.copy(
                argumentsText = if (incoming.argumentsText.isEmpty) existing.argumentsText else incoming.argumentsText,
                arguments = incoming.arguments ?: existing.arguments,
                progress = incoming.progress.ifEmpty { existing.progress },
                parentId = parent,
            )
            existing is SubAgentItem && incoming is SubAgentItem -> incoming.copy(
                progress = incoming.progress.ifEmpty { existing.progress },
                description = incoming.description ?: existing.description,
                prompt = incoming.prompt ?: existing.prompt,
                parentId = parent,
            )
            existing is UserMessageItem && incoming is UserMessageItem -> incoming.copy(
                parts = incoming.parts.ifEmpty { existing.parts },
                clientMessageId = incoming.clientMessageId ?: existing.clientMessageId,
                parentId = parent,
            )
            else -> incoming
        }
    }

    private fun placeholder(id: String, delta: ItemDelta): Item? = when (delta) {
        is ItemDelta.AgentText -> AgentMessageItem(id, StreamText.EMPTY)
        is ItemDelta.ReasoningSummaryPart, is ItemDelta.ReasoningSummary, is ItemDelta.ReasoningText -> ReasoningItem(id)
        is ItemDelta.PlanText -> PlanItem(id)
        is ItemDelta.CommandOutput, is ItemDelta.TerminalInput -> CommandItem(id, command = "")
        is ItemDelta.FileChangeOutput, is ItemDelta.FileChangePatch -> FileChangeItem(id, emptyList())
        is ItemDelta.ToolArguments, is ItemDelta.ToolProgress -> ToolCallItem(id, ToolKind.BUILTIN, tool = "")
    }

    private fun List<StreamText>.appendAt(index: Int, text: String): List<StreamText> {
        val list = toMutableList()
        while (list.size <= index) list.add(StreamText.EMPTY)
        list[index] = list[index].append(text)
        return list
    }

    private fun appendOutput(item: CommandItem, text: String): CommandItem {
        val appended = item.output.append(text)
        return if (appended.length > MAX_COMMAND_OUTPUT) item.copy(output = appended.takeLast(MAX_COMMAND_OUTPUT), outputTruncated = true)
        else item.copy(output = appended)
    }

    private fun applyDelta(item: Item, delta: ItemDelta): Item = when (delta) {
        is ItemDelta.AgentText -> (item as? AgentMessageItem)?.copy(text = item.text.append(delta.text)) ?: item
        is ItemDelta.ReasoningSummaryPart -> (item as? ReasoningItem)?.copy(summary = item.summary.appendAt(delta.index, "")) ?: item
        is ItemDelta.ReasoningSummary -> (item as? ReasoningItem)?.copy(summary = item.summary.appendAt(delta.index, delta.text)) ?: item
        is ItemDelta.ReasoningText -> (item as? ReasoningItem)?.copy(content = item.content.appendAt(delta.index, delta.text)) ?: item
        is ItemDelta.PlanText -> (item as? PlanItem)?.copy(text = item.text.append(delta.text)) ?: item
        is ItemDelta.CommandOutput -> (item as? CommandItem)?.let { appendOutput(it, delta.text) } ?: item
        is ItemDelta.TerminalInput -> (item as? CommandItem)?.let { appendOutput(it, delta.text) } ?: item
        is ItemDelta.FileChangeOutput -> (item as? FileChangeItem)?.copy(output = item.output.append(delta.text)) ?: item
        is ItemDelta.FileChangePatch -> (item as? FileChangeItem)?.copy(changes = delta.changes) ?: item
        is ItemDelta.ToolArguments -> (item as? ToolCallItem)?.copy(argumentsText = item.argumentsText.append(delta.partialJson)) ?: item
        is ItemDelta.ToolProgress -> when (item) {
            is ToolCallItem -> item.copy(progress = (item.progress + delta.message).takeLast(50))
            is SubAgentItem -> item.copy(progress = (item.progress + delta.message).takeLast(50))
            else -> item
        }
    }

    /** In-progress items become INCOMPLETE; unknown phases resolve (last top-level message = FINAL). */
    private fun finishItems(turn: Turn): Turn {
        val lastMessage = turn.items.indexOfLast { it is AgentMessageItem && it.parentId == null }
        val items = turn.items.mapIndexed { index, item ->
            var next = if (item.status == ItemStatus.IN_PROGRESS) item.withStatus(ItemStatus.INCOMPLETE) else item
            if (next is AgentMessageItem && next.phase == MessagePhase.UNKNOWN) {
                next = next.copy(phase = if (index == lastMessage) MessagePhase.FINAL else MessagePhase.COMMENTARY)
            }
            next
        }
        return turn.copy(items = items)
    }

    private fun turnCompleted(state: AgentState, event: AgentEvent.TurnCompleted): AgentState {
        val withThread = state.thread(event.backend, event.threadId) { thread ->
            var t = thread
            for (item in event.items) t = itemUpsert(t, event.turnId, item, completed = true)
            t = t.updateTurn(event.turnId) { turn ->
                finishItems(turn.copy(
                    status = event.status,
                    error = event.error ?: turn.error,
                    durationMs = event.durationMs ?: turn.durationMs,
                    usage = event.usage ?: turn.usage,
                    completedAtMs = event.atMs ?: turn.completedAtMs,
                    bound = true,
                ))
            }
            val stillRunning = t.turns.any { it.status == TurnStatus.RUNNING }
            if (!stillRunning && (t.runState == RunState.RUNNING || t.runState in WAITING)) t.copy(runState = RunState.IDLE) else t
        }
        val requests = withThread.requests.mapValues { (key, request) ->
            if (key.backend == event.backend && request.status.isOpen && request.threadId == event.threadId && request.turnId == event.turnId)
                request.copy(status = RequestStatus.EXPIRED)
            else request
        }
        return withThread.copy(requests = requests)
    }

    private fun requestOpened(state: AgentState, request: PendingRequest): AgentState {
        val existing = state.requests[request.key]
        val requests = if (existing != null && !existing.status.isOpen) (state.requests - request.key) + (request.key to request)
        else state.requests + (request.key to (if (existing != null) request.copy(receivedAtMs = existing.receivedAtMs ?: request.receivedAtMs) else request))
        val next = state.copy(requests = requests)
        val threadId = request.threadId ?: return next
        if (!request.status.isOpen) return next
        val waiting = if (request.kind is RequestKind.UserInput || request.kind is RequestKind.Elicitation) RunState.WAITING_INPUT else RunState.WAITING_APPROVAL
        return next.thread(request.key.backend, threadId) { t ->
            if (t.runState == RunState.RUNNING || t.runState == RunState.IDLE) t.copy(runState = waiting) else t
        }
    }

    private fun requestClosed(state: AgentState, event: AgentEvent.RequestClosed): AgentState {
        val existing = state.requests[event.key] ?: return state
        // The user's answer is written before the backend's reaction can be observed, so a server-side
        // closure (resolved / turn ended) may be reduced first; the recorded answer still wins.
        val lateAnswer = event.status == RequestStatus.ANSWERED && existing.status in SERVER_CLOSED
        if (!existing.status.isOpen && !lateAnswer) return state
        val closed = existing.copy(status = event.status, answer = event.answer ?: existing.answer)
        val next = state.copy(requests = state.requests + (event.key to closed))
        val threadId = existing.threadId ?: return next
        val stillWaiting = next.requests.values.any { it.status.isOpen && it.key.backend == event.key.backend && it.threadId == threadId }
        if (stillWaiting) return next
        return next.thread(event.key.backend, threadId) { t ->
            if (t.runState in WAITING) t.copy(runState = if (t.turns.any { it.status == TurnStatus.RUNNING }) RunState.RUNNING else RunState.IDLE) else t
        }
    }
}
