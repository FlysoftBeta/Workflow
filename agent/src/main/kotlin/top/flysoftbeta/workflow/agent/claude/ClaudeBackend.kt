package top.flysoftbeta.workflow.agent.claude

import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import top.flysoftbeta.workflow.agent.AgentBackend
import top.flysoftbeta.workflow.agent.AgentEventSink
import top.flysoftbeta.workflow.agent.SendMode
import top.flysoftbeta.workflow.agent.ThreadOptions
import top.flysoftbeta.workflow.agent.ThreadSummary
import top.flysoftbeta.workflow.agent.UnknownRequestPolicy
import top.flysoftbeta.workflow.agent.json.get
import top.flysoftbeta.workflow.agent.json.obj
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.model.AgentEvent
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.Decision
import top.flysoftbeta.workflow.agent.model.DecisionKind
import top.flysoftbeta.workflow.agent.model.LoginFlow
import top.flysoftbeta.workflow.agent.model.LoginMethod
import top.flysoftbeta.workflow.agent.model.ModelCatalog
import top.flysoftbeta.workflow.agent.model.Notice
import top.flysoftbeta.workflow.agent.model.NoticeLevel
import top.flysoftbeta.workflow.agent.model.PendingRequest
import top.flysoftbeta.workflow.agent.model.PermissionPreset
import top.flysoftbeta.workflow.agent.model.ProcessState
import top.flysoftbeta.workflow.agent.model.RateLimit
import top.flysoftbeta.workflow.agent.model.RateLimitState
import top.flysoftbeta.workflow.agent.model.RateLimitWindow
import top.flysoftbeta.workflow.agent.model.RequestKey
import top.flysoftbeta.workflow.agent.model.RequestKind
import top.flysoftbeta.workflow.agent.model.RequestResponse
import top.flysoftbeta.workflow.agent.model.RequestStatus
import top.flysoftbeta.workflow.agent.model.RunState
import top.flysoftbeta.workflow.agent.model.ThreadSettings
import top.flysoftbeta.workflow.agent.model.TurnError
import top.flysoftbeta.workflow.agent.model.TurnSettings
import top.flysoftbeta.workflow.agent.model.TurnStatus
import top.flysoftbeta.workflow.agent.model.UserPart
import top.flysoftbeta.workflow.agent.process.AgentProcess
import top.flysoftbeta.workflow.agent.process.ProcessLauncher
import top.flysoftbeta.workflow.agent.transport.ControlConnection
import top.flysoftbeta.workflow.agent.transport.JsonLineChannel

/**
 * Claude Code adapter: one CLI process per session (`-p` stream-json + control protocol). A
 * spare process started by [start] answers account/model queries and becomes the next new
 * thread. List/archive/delete are app-level (Claude sessions are transcript files).
 */
class ClaudeBackend(
    private val launcher: ProcessLauncher,
    private val config: ClaudeConfig,
    private val sink: AgentEventSink,
    parentScope: CoroutineScope,
    private val attachments: AttachmentReader,
    private val transcripts: ClaudeTranscriptSource? = null,
    private val clock: () -> Long = System::currentTimeMillis,
    private val newId: () -> String = { UUID.randomUUID().toString() },
) : AgentBackend {
    override val kind = BackendKind.CLAUDE
    override val loginMethods = listOf(LoginMethod.CLAUDE_TERMINAL_LOGIN, LoginMethod.CLAUDE_SETUP_TOKEN, LoginMethod.CLAUDE_API_KEY)

    private val scope = CoroutineScope(parentScope.coroutineContext + SupervisorJob(parentScope.coroutineContext[Job]))
    private val lifecycle = Mutex()

    private inner class Session(
        val id: String,
        val cwd: String,
        val process: AgentProcess,
        val lines: JsonLineChannel,
        val connection: ControlConnection,
        val mapper: ClaudeMapper,
        val scope: CoroutineScope,
        var model: String?,
        var effort: String?,
        var mode: String,
    ) {
        val submitted = LinkedHashSet<String>()
        var initialized = false
    }

    private val sessions = ConcurrentHashMap<String, Session>()
    @Volatile private var spare: Session? = null
    private val openRequests = ConcurrentHashMap<RequestKey, Pair<String, PendingRequest>>()
    @Volatile private var apiKeyOverride: String? = null

    private fun emit(event: AgentEvent) = sink.emit(event)

    // ------------------------------------------------------------------ process lifecycle

    private suspend fun launch(session: ClaudeSession, cwd: String, settings: TurnSettings): Session {
        val id = when (session) {
            is ClaudeSession.New -> session.sessionId
            is ClaudeSession.Resume -> session.sessionId
            is ClaudeSession.Fork -> session.newSessionId
        }
        val effectiveConfig = config.copy(cwd = cwd, credentials = config.credentials + (apiKeyOverride?.let { mapOf("ANTHROPIC_API_KEY" to it) } ?: emptyMap()))
        val spec = ClaudeLaunch.spec(effectiveConfig, session, settings.model, settings.effort, settings.permissions)
        val proc = launcher.launch(spec)
        val child = CoroutineScope(scope.coroutineContext + SupervisorJob(scope.coroutineContext[Job]))
        val lines = JsonLineChannel(proc, child)
        val connection = ControlConnection(lines, child)
        val s = Session(id, cwd, proc, lines, connection, ClaudeMapper(id), child, settings.model, settings.effort,
            ClaudeLaunch.permissionMode(settings.permissions ?: config.defaultPermissions))
        sessions[id] = s
        child.launch { for (message in connection.inbound) handle(s, message) }
        child.launch {
            val code = proc.awaitExit()
            lines.join()
            onExit(s, code)
        }
        try {
            val response = connection.control("initialize", ClaudeRequests.initialize(config.hooks))
            s.initialized = true
            emit(AgentEvent.ServerInfo(kind, response))
            emit(AgentEvent.AccountChanged(kind, ClaudeMapper.account(response)))
            response["models"]?.let { emit(AgentEvent.ModelsChanged(kind, ClaudeMapper.models(it))) }
            emit(AgentEvent.ProcessChanged(kind, ProcessState.Ready))
            // Re-arm prompts a previous worker left pending (CLI ≥ 2.1.268).
            (response["pending_permission_requests"] as? kotlinx.serialization.json.JsonArray)?.forEach { pending ->
                val frame = pending.obj ?: return@forEach
                val requestId = frame["request_id"].str ?: return@forEach
                val request = frame["request"].obj ?: return@forEach
                ClaudeRequests.pending(requestId, request["subtype"].str ?: "", request, frame, id, null, clock())?.let { open(s, it) }
            }
        } catch (e: Exception) {
            emit(AgentEvent.BackendNotice(kind, Notice(NoticeLevel.ERROR, "Claude Code failed to start: ${e.message}", "startFailed", detail = lines.stderr.takeLast(4000))))
            proc.kill(force = true)
            sessions.remove(id)
            throw e
        }
        return s
    }

    private fun onExit(s: Session, code: Int) {
        sessions.remove(s.id, s)
        if (spare === s) spare = null
        for (turn in s.submitted) {
            s.mapper.endTurn(turn, TurnStatus.FAILED, TurnError("Claude Code exited ($code)", "processExited")).forEach(::emit)
        }
        openRequests.entries.filter { it.value.first == s.id }.forEach { (key, _) ->
            openRequests.remove(key)
            emit(AgentEvent.RequestClosed(key, RequestStatus.EXPIRED))
        }
        emit(AgentEvent.ThreadStatusChanged(kind, s.id, RunState.NOT_LOADED))
        if (code != 0 && code != 143 && code != 137) {
            emit(AgentEvent.ThreadNotice(kind, s.id, Notice(NoticeLevel.WARNING, "Claude Code exited with status $code", "processExited", detail = s.lines.stderr.takeLast(4000))))
        }
        s.scope.cancel()
    }

    override suspend fun start() = lifecycle.withLock {
        if (spare != null || sessions.isNotEmpty()) return@withLock
        emit(AgentEvent.ProcessChanged(kind, ProcessState.Starting))
        try {
            spare = launch(ClaudeSession.New(newId()), config.cwd, TurnSettings())
        } catch (e: Exception) {
            emit(AgentEvent.ProcessChanged(kind, ProcessState.Failed(e.message ?: "launch failed")))
            throw e
        }
    }

    override suspend fun stop() = lifecycle.withLock {
        sessions.values.forEach { it.process.kill() }
        spare = null
        emit(AgentEvent.ProcessChanged(kind, ProcessState.Stopped))
    }

    private fun anySession(): Session = spare ?: sessions.values.firstOrNull { it.initialized } ?: throw IllegalStateException("Claude Code is not running")

    private fun session(threadId: String): Session = sessions[threadId] ?: throw IllegalStateException("session $threadId is not running; resume it first")

    // ------------------------------------------------------------------ inbound

    private suspend fun handle(s: Session, message: ControlConnection.Inbound) {
        when (message) {
            is ControlConnection.Inbound.Message -> s.mapper.map(message.raw).forEach(::emit)
            is ControlConnection.Inbound.ControlRequest -> onControlRequest(s, message)
            is ControlConnection.Inbound.ControlCancel -> {
                val key = ClaudeRequests.key(message.requestId)
                openRequests.remove(key)
                emit(AgentEvent.RequestClosed(key, RequestStatus.CANCELLED))
            }
            is ControlConnection.Inbound.Malformed -> emit(AgentEvent.ThreadNotice(kind, s.id, Notice(NoticeLevel.WARNING, "Malformed frame: ${message.reason}", "malformed", detail = message.preview)))
        }
    }

    private fun open(s: Session, request: PendingRequest) {
        openRequests[request.key] = s.id to request
        emit(AgentEvent.RequestOpened(request))
    }

    private suspend fun onControlRequest(s: Session, m: ControlConnection.Inbound.ControlRequest) {
        val turn = s.mapper.runningTurn
        when (m.subtype) {
            "can_use_tool", "elicitation", "request_user_dialog" -> {
                val request = ClaudeRequests.pending(m.requestId, m.subtype, m.request, m.raw, s.id, turn, clock()) ?: return
                if (m.subtype == "request_user_dialog") s.connection.forget(m.requestId) // must never be answered by us
                open(s, request)
            }
            "hook_callback" -> {
                val callbackId = m.request["callback_id"].str
                val hook = config.hooks.firstOrNull { it.callbackId == callbackId }
                if (hook == null) {
                    s.connection.respondError(m.requestId, "no hook registered for ${callbackId ?: "?"}")
                    emit(AgentEvent.Unknown(kind, "control_request/hook_callback", s.id, m.raw))
                } else {
                    val input = m.request["input"].obj ?: JsonObject(emptyMap())
                    s.mapper.hookMarker(callbackId!!, input).forEach(::emit)
                    val output = runCatching { hook.handler.handle(input) }
                    output.fold(
                        onSuccess = { s.connection.respond(m.requestId, it) },
                        onFailure = { s.connection.respondError(m.requestId, it.message ?: "hook failed") },
                    )
                }
            }
            else -> {
                if (config.unknownRequests == UnknownRequestPolicy.ASK_USER && m.subtype != "mcp_message") {
                    open(s, ClaudeRequests.rejected(m.requestId, m.subtype, m.request, m.raw, s.id, clock())
                        .copy(status = RequestStatus.PENDING, turnId = turn, decisions = listOf(Decision("reject", DecisionKind.DENY))))
                } else {
                    emit(AgentEvent.RequestOpened(ClaudeRequests.rejected(m.requestId, m.subtype, m.request, m.raw, s.id, clock())))
                    emit(AgentEvent.Unknown(kind, "control_request/${m.subtype}", s.id, m.raw))
                    s.connection.respondError(m.requestId, "unsupported control request: ${m.subtype}")
                }
            }
        }
    }

    // ------------------------------------------------------------------ account

    override suspend fun refreshAccount() {
        // Account state is only reported by `initialize`; restart an idle spare so new credentials are read.
        lifecycle.withLock {
            spare?.let { if (it.submitted.isEmpty()) { it.process.kill(); spare = null } }
            spare = launch(ClaudeSession.New(newId()), config.cwd, TurnSettings())
        }
    }

    override suspend fun refreshRateLimits() {
        val usage = anySession().connection.control("get_usage")
        val limits = usage["rate_limits"].obj?.mapNotNull { (id, window) ->
            val w = window.obj ?: return@mapNotNull null
            RateLimit(id, primary = RateLimitWindow((w["utilization"] as? kotlinx.serialization.json.JsonPrimitive)?.content?.toDoubleOrNull(), null, null), raw = w)
        } ?: emptyList()
        emit(AgentEvent.RateLimitsChanged(kind, RateLimitState(limits, raw = usage)))
    }

    override suspend fun refreshModels(): ModelCatalog {
        val response = anySession().connection.control("list_models")
        val catalog = ClaudeMapper.models(response["models"])
        emit(AgentEvent.ModelsChanged(kind, catalog))
        return catalog
    }

    override suspend fun login(method: LoginMethod, secret: String?): LoginFlow {
        val flow = when (method) {
            LoginMethod.CLAUDE_TERMINAL_LOGIN -> LoginFlow.Terminal(ClaudeLaunch.loginArgv(config), ClaudeLaunch.env(config))
            LoginMethod.CLAUDE_SETUP_TOKEN -> LoginFlow.Terminal(ClaudeLaunch.setupTokenArgv(config), ClaudeLaunch.env(config))
            LoginMethod.CLAUDE_API_KEY -> {
                // Held in memory only; persisting it (Android-protected storage) is the app's job.
                apiKeyOverride = requireNotNull(secret?.takeIf { it.isNotBlank() }) { "API key required" }
                refreshAccount()
                LoginFlow.Completed(null, true, null)
            }
            else -> throw IllegalArgumentException("$method is not a Claude login method")
        }
        emit(AgentEvent.LoginChanged(kind, flow))
        return flow
    }

    override suspend fun cancelLogin(loginId: String) = emit(AgentEvent.LoginChanged(kind, null))

    override suspend fun logout() {
        apiKeyOverride = null
        emit(AgentEvent.LoginChanged(kind, LoginFlow.Terminal(ClaudeLaunch.logoutArgv(config), ClaudeLaunch.env(config))))
    }

    // ------------------------------------------------------------------ threads

    override suspend fun startThread(options: ThreadOptions): String {
        val reuse = lifecycle.withLock { spare?.takeIf { it.cwd == options.cwd && options.settings == TurnSettings() }?.also { spare = null } }
        val s = reuse ?: launch(ClaudeSession.New(newId()), options.cwd, options.settings)
        emit(AgentEvent.ThreadUpserted(kind, s.id, cwd = options.cwd, runState = RunState.IDLE, path = ClaudeLaunch.transcriptPath(config.configDir, options.cwd, s.id),
            settings = ThreadSettings(model = s.model, effort = s.effort, approvalPolicy = s.mode)))
        options.title?.let { rename(s.id, it) }
        return s.id
    }

    override suspend fun resumeThread(threadId: String, options: ThreadOptions): String {
        sessions[threadId]?.let { return threadId }
        val s = launch(ClaudeSession.Resume(threadId), options.cwd, options.settings)
        emit(AgentEvent.ThreadUpserted(kind, threadId, cwd = options.cwd, runState = RunState.IDLE, path = ClaudeLaunch.transcriptPath(config.configDir, options.cwd, threadId),
            settings = ThreadSettings(model = s.model, effort = s.effort, approvalPolicy = s.mode)))
        runCatching { loadHistory(threadId) }
        return threadId
    }

    override suspend fun forkThread(threadId: String, atTurnId: String?, options: ThreadOptions): String {
        val anchor = atTurnId?.let { turn ->
            sessions[threadId]?.mapper?.lastMessageUuid(turn)
                ?: transcripts?.read(ClaudeLaunch.transcriptPath(config.configDir, options.cwd, threadId))?.let { ClaudeTranscript.lastUuid(it, turn) }
                ?: throw IllegalArgumentException("turn $atTurnId not found in $threadId")
        }
        val newSession = newId()
        launch(ClaudeSession.Fork(threadId, newSession, anchor), options.cwd, options.settings)
        emit(AgentEvent.ThreadUpserted(kind, newSession, cwd = options.cwd, forkedFrom = threadId, runState = RunState.IDLE,
            path = ClaudeLaunch.transcriptPath(config.configDir, options.cwd, newSession)))
        runCatching { loadHistory(newSession) }
        return newSession
    }

    override suspend fun loadHistory(threadId: String, olderThan: String?) {
        if (olderThan != null) return // transcripts are loaded whole
        val cwd = sessions[threadId]?.cwd ?: config.cwd
        val lines = transcripts?.read(ClaudeLaunch.transcriptPath(config.configDir, cwd, threadId)) ?: return
        val history = ClaudeTranscript.history(threadId, lines)
        emit(AgentEvent.HistoryLoaded(kind, threadId, history.turns))
        history.title?.let { emit(AgentEvent.ThreadRenamed(kind, threadId, it)) }
    }

    override suspend fun listThreads(cwd: String?): List<ThreadSummary>? = null

    override suspend fun rename(threadId: String, title: String) {
        sessions[threadId]?.connection?.control("rename_session", buildJsonObject { put("title", title); put("source", "host") })
        emit(AgentEvent.ThreadRenamed(kind, threadId, title))
    }

    override suspend fun archive(threadId: String, archived: Boolean) {
        if (archived) sessions[threadId]?.process?.kill()
        emit(AgentEvent.ThreadArchived(kind, threadId, archived))
    }

    override suspend fun delete(threadId: String) {
        sessions[threadId]?.process?.kill()
        emit(AgentEvent.ThreadDeleted(kind, threadId))
    }

    override suspend fun compact(threadId: String) {
        send(threadId, listOf(UserPart.Text("/compact")), null, SendMode.START)
    }

    // ------------------------------------------------------------------ turns

    private suspend fun applySettings(s: Session, settings: TurnSettings?) {
        if (settings == null) return
        settings.model?.takeIf { it != s.model }?.let {
            s.connection.control("set_model", buildJsonObject { put("model", it) }); s.model = it
            emit(AgentEvent.ThreadSettingsChanged(kind, s.id, ThreadSettings(model = it)))
        }
        settings.effort?.takeIf { it != s.effort }?.let {
            s.connection.control("apply_flag_settings", buildJsonObject { put("settings", buildJsonObject { put("effortLevel", it) }) }); s.effort = it
            emit(AgentEvent.ThreadSettingsChanged(kind, s.id, ThreadSettings(effort = it)))
        }
        settings.permissions?.let { setMode(s, ClaudeLaunch.permissionMode(it)) }
    }

    private suspend fun setMode(s: Session, mode: String) {
        check(mode in ClaudeLaunch.ALLOWED_MODES) { "permission mode $mode is not allowed" }
        if (mode == s.mode) return
        s.connection.control("set_permission_mode", buildJsonObject { put("mode", mode) })
        s.mode = mode
    }

    override suspend fun send(threadId: String, parts: List<UserPart>, settings: TurnSettings?, mode: SendMode): String {
        val s = sessions[threadId] ?: session(resumeThread(threadId, ThreadOptions(config.cwd)))
        applySettings(s, settings)
        val clientMessageId = newId()
        emit(AgentEvent.TurnSubmitted(kind, threadId, clientMessageId, parts, settings, clock()))
        try {
            val content = ClaudeRequests.content(parts, attachments, config.maxInlineBytes)
            s.submitted += clientMessageId
            s.connection.send(ClaudeRequests.userMessage(clientMessageId, content, if (mode == SendMode.STEER) "now" else null))
        } catch (e: Exception) {
            s.submitted -= clientMessageId
            emit(AgentEvent.TurnCompleted(kind, threadId, clientMessageId, TurnStatus.FAILED, TurnError(e.message ?: "send failed", "sendFailed")))
            throw e
        }
        return clientMessageId
    }

    override suspend fun cancelQueued(threadId: String, clientMessageId: String) {
        val s = session(threadId)
        s.connection.control("cancel_async_message", buildJsonObject { put("message_uuid", clientMessageId) })
        s.mapper.endTurn(clientMessageId, TurnStatus.CANCELLED).forEach(::emit)
    }

    override suspend fun interrupt(threadId: String, cancelQueued: Boolean) {
        session(threadId).connection.control("interrupt", buildJsonObject { if (cancelQueued) put("cancel_queued", true) })
    }

    override suspend fun setPermissions(threadId: String, preset: PermissionPreset) {
        val s = session(threadId)
        setMode(s, ClaudeLaunch.permissionMode(preset))
    }

    // ------------------------------------------------------------------ requests

    override suspend fun respond(key: RequestKey, response: RequestResponse) {
        require(key.backend == kind)
        val (sessionId, request) = openRequests[key] ?: throw IllegalStateException("request $key is not open")
        val s = session(sessionId)
        val requestId = key.rawId.str ?: throw IllegalStateException("bad key")
        val answer = ClaudeRequests.answer(request, response)
        if (answer.error != null) s.connection.respondError(requestId, answer.error) else s.connection.respond(requestId, answer.payload)
        openRequests.remove(key)
        if (answer.denied && request.kind is RequestKind.ToolApproval) {
            request.itemId?.let { s.mapper.markDenied(it); emit(AgentEvent.ItemDeclined(kind, sessionId, request.turnId, it)) }
        }
        emit(AgentEvent.RequestClosed(key, RequestStatus.ANSWERED, answer.summary))
    }

    override suspend fun rawRequest(method: String, params: JsonElement?, threadId: String?): JsonElement {
        require(method != "initialize") { "initialize is sent once per process" }
        val fields = params as? JsonObject ?: JsonObject(emptyMap())
        if (method == "set_permission_mode") check(fields["mode"].str in ClaudeLaunch.ALLOWED_MODES) { "permission mode not allowed" }
        val session = if (threadId == null) anySession() else sessions[threadId]
            ?: throw IllegalStateException("session is not open")
        return session.connection.control(method, fields)
    }
}
