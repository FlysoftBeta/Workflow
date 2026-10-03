package top.flysoftbeta.workflow.agent.codex

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
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import top.flysoftbeta.workflow.agent.AgentBackend
import top.flysoftbeta.workflow.agent.AgentEventSink
import top.flysoftbeta.workflow.agent.SendMode
import top.flysoftbeta.workflow.agent.ThreadOptions
import top.flysoftbeta.workflow.agent.ThreadSummary
import top.flysoftbeta.workflow.agent.UnknownRequestPolicy
import top.flysoftbeta.workflow.agent.codex.CodexProtocol.ClientNotification as CN
import top.flysoftbeta.workflow.agent.codex.CodexProtocol.ClientRequest as C
import top.flysoftbeta.workflow.agent.codex.CodexProtocol.ServerNotification as N
import top.flysoftbeta.workflow.agent.json.arr
import top.flysoftbeta.workflow.agent.json.bool
import top.flysoftbeta.workflow.agent.json.get
import top.flysoftbeta.workflow.agent.json.long
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
import top.flysoftbeta.workflow.agent.model.RequestKey
import top.flysoftbeta.workflow.agent.model.RequestKind
import top.flysoftbeta.workflow.agent.model.RequestResponse
import top.flysoftbeta.workflow.agent.model.RequestStatus
import top.flysoftbeta.workflow.agent.model.TurnError
import top.flysoftbeta.workflow.agent.model.TurnSettings
import top.flysoftbeta.workflow.agent.model.TurnStatus
import top.flysoftbeta.workflow.agent.model.UserPart
import top.flysoftbeta.workflow.agent.process.AgentProcess
import top.flysoftbeta.workflow.agent.process.LaunchSpec
import top.flysoftbeta.workflow.agent.process.ProcessLauncher
import top.flysoftbeta.workflow.agent.transport.JsonLineChannel
import top.flysoftbeta.workflow.agent.transport.JsonRpcConnection

data class CodexConfig(
    /** argv[0]: the Codex binary as seen by the launcher (e.g. the bundled `libcodex.so` path). */
    val executable: String,
    /** App-owned CODEX_HOME so the user's desktop config.toml (reviewer, MCP servers, effort) never leaks in. */
    val codexHome: String,
    val cwd: String = "/workspace",
    /** Base environment (HOME, PATH, TMPDIR, LANG, SSL_CERT_FILE, proxy variables, …). Filtered, see [CodexLaunch]. */
    val env: Map<String, String> = emptyMap(),
    val args: List<String> = listOf("app-server"),
    val clientName: String = "workflow",
    val clientTitle: String = "Workflow",
    val clientVersion: String = "0",
    val defaultPermissions: PermissionPreset = PermissionPreset.ASK,
    val unknownRequests: UnknownRequestPolicy = UnknownRequestPolicy.REJECT,
    val optOutNotifications: List<String> = emptyList(),
    val historyPageSize: Int = 20,
)

object CodexLaunch {
    /** Never forwarded to the agent: host credentials and other agents' configuration. */
    private val DENY_PREFIXES = listOf("OPENAI_", "CODEX_", "ANTHROPIC_", "CLAUDE_", "LD_PRELOAD", "LD_LIBRARY_PATH")

    fun spec(config: CodexConfig): LaunchSpec {
        val env = LinkedHashMap<String, String>()
        config.env.forEach { (k, v) -> if (DENY_PREFIXES.none { k.startsWith(it) }) env[k] = v }
        env["CODEX_HOME"] = config.codexHome
        return LaunchSpec(listOf(config.executable) + config.args, env, config.cwd, "codex")
    }
}

/**
 * Codex App-Server adapter. One process serves many threads. All state goes to [sink]; the
 * backend keeps only what it needs to address requests (open server requests, the running turn
 * per thread, queued submission ids).
 */
class CodexBackend(
    private val launcher: ProcessLauncher,
    private val config: CodexConfig,
    private val sink: AgentEventSink,
    parentScope: CoroutineScope,
    private val clock: () -> Long = System::currentTimeMillis,
    private val newId: () -> String = { UUID.randomUUID().toString() },
) : AgentBackend {
    override val kind = BackendKind.CODEX
    override val loginMethods = listOf(LoginMethod.CODEX_DEVICE_CODE, LoginMethod.CODEX_BROWSER, LoginMethod.CODEX_API_KEY)

    private val scope = CoroutineScope(parentScope.coroutineContext + SupervisorJob(parentScope.coroutineContext[Job]))
    private val lifecycle = Mutex()
    @Volatile private var rpc: JsonRpcConnection? = null
    @Volatile private var process: AgentProcess? = null
    private var sessionScope: CoroutineScope? = null

    private val openRequests = ConcurrentHashMap<RequestKey, PendingRequest>()
    private val activeTurn = ConcurrentHashMap<String, String>()
    private val queued = ConcurrentHashMap<String, String>() // clientMessageId -> queuedSubmissionId
    private val queuedThread = ConcurrentHashMap<String, String>() // clientMessageId -> threadId
    private val queueLocks = ConcurrentHashMap<String, Mutex>()
    private val reviewer = ConcurrentHashMap<String, String>()

    private fun emit(event: AgentEvent) = sink.emit(event)

    private fun connection(): JsonRpcConnection = rpc ?: throw IllegalStateException("Codex is not running")

    override suspend fun start() = lifecycle.withLock {
        if (rpc != null && process?.isAlive == true) return@withLock
        emit(AgentEvent.ProcessChanged(kind, ProcessState.Starting))
        val spec = CodexLaunch.spec(config)
        val proc = try {
            launcher.launch(spec)
        } catch (e: Exception) {
            emit(AgentEvent.ProcessChanged(kind, ProcessState.Failed(e.message ?: "launch failed")))
            throw e
        }
        val session = CoroutineScope(scope.coroutineContext + SupervisorJob(scope.coroutineContext[Job]))
        val lines = JsonLineChannel(proc, session)
        val connection = JsonRpcConnection(lines, session)
        process = proc; rpc = connection; sessionScope = session
        session.launch { for (message in connection.inbound) handle(connection, message) }
        session.launch {
            val code = proc.awaitExit()
            lines.join()
            if (rpc === connection) { rpc = null; process = null }
            openRequests.clear(); activeTurn.clear(); queued.clear(); queuedThread.clear()
            emit(AgentEvent.ProcessChanged(kind, ProcessState.Exited(code, lines.stderr.takeLast(4000))))
        }
        try {
            val info = connection.request(C.INITIALIZE, CodexParams.initialize(config.clientName, config.clientTitle, config.clientVersion, config.optOutNotifications))
            emit(AgentEvent.ServerInfo(kind, info))
            connection.notify(CN.INITIALIZED)
            emit(AgentEvent.ProcessChanged(kind, ProcessState.Ready))
        } catch (e: Exception) {
            emit(AgentEvent.ProcessChanged(kind, ProcessState.Failed(e.message ?: "initialize failed", lines.stderr.takeLast(4000))))
            proc.kill(force = true)
            throw e
        }
        session.launch {
            runCatching { refreshAccount() }
            runCatching { refreshModels() }
            runCatching { refreshRateLimits() }
        }
        Unit
    }

    override suspend fun stop() = lifecycle.withLock {
        val proc = process ?: return@withLock
        proc.kill()
        sessionScope?.cancel()
        rpc = null; process = null
    }

    // ------------------------------------------------------------------ inbound

    private suspend fun handle(connection: JsonRpcConnection, message: JsonRpcConnection.Inbound) {
        when (message) {
            is JsonRpcConnection.Inbound.Notification -> {
                val events = CodexEvents.notification(message.method, message.params, message.raw)
                track(message.method, message.params, events)
                events.forEach(::emit)
                if (message.method == N.ACCOUNT_UPDATED || message.method == N.ACCOUNT_LOGIN_COMPLETED) scope.launch { runCatching { refreshAccount() } }
                if (message.method == N.THREAD_QUEUE_CHANGED) message.params["threadId"].str?.let { threadId ->
                    scope.launch {
                        runCatching { refreshQueue(threadId, connection) }.onFailure {
                            emit(AgentEvent.ThreadNotice(kind, threadId, Notice(NoticeLevel.WARNING, "队列更新失败", code = "queueRefreshFailed")))
                        }
                    }
                }
            }
            is JsonRpcConnection.Inbound.Request -> onServerRequest(connection, message)
            is JsonRpcConnection.Inbound.Stray -> emit(AgentEvent.Unknown(kind, "stray: ${message.reason}", null, message.raw))
            is JsonRpcConnection.Inbound.Malformed -> emit(AgentEvent.BackendNotice(kind, Notice(NoticeLevel.WARNING, "Malformed frame: ${message.reason}", code = "malformed", detail = message.preview)))
        }
    }

    private fun track(method: String, params: JsonElement?, events: List<AgentEvent>) {
        for (event in events) when (event) {
            is AgentEvent.TurnStarted -> activeTurn[event.threadId] = event.turnId
            is AgentEvent.TurnCompleted -> {
                activeTurn.remove(event.threadId, event.turnId)
                openRequests.entries.removeIf { it.value.threadId == event.threadId && it.value.turnId == event.turnId }
            }
            is AgentEvent.RequestClosed -> {
                openRequests.remove(event.key)
                rpc?.forget(event.key.rawId)
            }
            is AgentEvent.ThreadSettingsChanged -> event.settings.approvalsReviewer?.let { checkReviewer(event.threadId, it) }
            is AgentEvent.ItemStarted, is AgentEvent.ItemCompleted -> {
                // A queued submission started: its user message carries our clientMessageId.
                val item = (event as? AgentEvent.ItemStarted)?.item ?: (event as AgentEvent.ItemCompleted).item
                (item as? top.flysoftbeta.workflow.agent.model.UserMessageItem)?.clientMessageId?.let { queued.remove(it); queuedThread.remove(it) }
            }
            else -> Unit
        }
    }

    private fun checkReviewer(threadId: String, value: String) {
        reviewer[threadId] = value
        if (value != CodexParams.REVIEWER_USER) {
            emit(AgentEvent.ThreadNotice(kind, threadId, Notice(
                NoticeLevel.ERROR, "Approvals for this thread are not routed to you (approvalsReviewer=$value). Sending is blocked.",
                code = "approvalsReviewerNotUser",
            )))
        }
    }

    private suspend fun onServerRequest(connection: JsonRpcConnection, message: JsonRpcConnection.Inbound.Request) {
        var disposition = CodexEvents.request(message.id, message.method, message.params, message.raw, clock())
        if (disposition is CodexDisposition.AutoError && config.unknownRequests == UnknownRequestPolicy.ASK_USER &&
            disposition.record.kind is RequestKind.Unknown && message.method !in CodexProtocol.ServerRequest.ALL
        ) {
            disposition = CodexDisposition.Ask(disposition.record.copy(
                status = RequestStatus.PENDING,
                decisions = listOf(Decision("reject", DecisionKind.DENY)),
            ))
        }
        when (disposition) {
            is CodexDisposition.Ask -> {
                openRequests[disposition.request.key] = disposition.request
                emit(AgentEvent.RequestOpened(disposition.request))
            }
            is CodexDisposition.AutoResult -> {
                emit(AgentEvent.RequestOpened(disposition.record))
                connection.respond(message.id, disposition.result)
            }
            is CodexDisposition.AutoError -> {
                emit(AgentEvent.RequestOpened(disposition.record))
                emit(AgentEvent.Unknown(kind, "serverRequest:${message.method}", disposition.record.threadId, message.raw))
                connection.respondError(message.id, disposition.code, disposition.message)
            }
        }
    }

    // ------------------------------------------------------------------ account

    override suspend fun refreshAccount() {
        val result = connection().request(C.ACCOUNT_READ, buildJsonObject { put("refreshToken", false) })
        emit(AgentEvent.AccountChanged(kind, CodexEvents.account(result)))
    }

    override suspend fun refreshRateLimits() {
        val result = connection().request(C.ACCOUNT_RATE_LIMITS_READ, null)
        emit(AgentEvent.RateLimitsChanged(kind, CodexEvents.rateLimits(result)))
    }

    override suspend fun refreshModels(): ModelCatalog {
        val pages = ArrayList<JsonElement>()
        var cursor: String? = null
        do {
            val page = connection().request(C.MODEL_LIST, buildJsonObject {
                put("includeHidden", true)
                if (cursor != null) put("cursor", cursor)
            })
            pages += page
            cursor = page["nextCursor"].str
        } while (cursor != null && pages.size < 20)
        val catalog = CodexEvents.models(pages)
        emit(AgentEvent.ModelsChanged(kind, catalog))
        return catalog
    }

    override suspend fun login(method: LoginMethod, secret: String?): LoginFlow {
        val result = connection().request(C.ACCOUNT_LOGIN_START, CodexParams.login(method, secret))
        val flow = CodexEvents.loginFlow(result) ?: LoginFlow.Progress(result["loginId"].str, emptyList())
        emit(AgentEvent.LoginChanged(kind, flow))
        if (flow is LoginFlow.Completed) runCatching { refreshAccount() }
        return flow
    }

    override suspend fun cancelLogin(loginId: String) {
        connection().request(C.ACCOUNT_LOGIN_CANCEL, buildJsonObject { put("loginId", loginId) })
        emit(AgentEvent.LoginChanged(kind, LoginFlow.Completed(loginId, false, "cancelled")))
    }

    override suspend fun logout() {
        connection().request(C.ACCOUNT_LOGOUT, null)
        emit(AgentEvent.LoginChanged(kind, null))
        refreshAccount()
    }

    // ------------------------------------------------------------------ threads

    private fun threadResult(result: JsonElement): String {
        val thread = result["thread"].obj ?: throw IllegalStateException("response has no thread")
        val event = CodexEvents.threadUpserted(thread, result.obj)
        emit(event)
        result["approvalsReviewer"].str?.let { checkReviewer(event.threadId, it) }
        return event.threadId
    }

    override suspend fun startThread(options: ThreadOptions): String {
        val id = threadResult(connection().request(C.THREAD_START, CodexParams.threadStart(options.cwd, options.settings, config.defaultPermissions, options.ephemeral)))
        options.title?.let { rename(id, it) }
        return id
    }

    override suspend fun resumeThread(threadId: String, options: ThreadOptions): String {
        val result = connection().request(C.THREAD_RESUME, CodexParams.threadResume(threadId, options.cwd, options.settings, config.defaultPermissions))
        val id = threadResult(result)
        loadHistory(id)
        return id
    }

    override suspend fun forkThread(threadId: String, atTurnId: String?, options: ThreadOptions): String {
        val result = connection().request(C.THREAD_FORK, CodexParams.threadFork(threadId, atTurnId, options.cwd, options.settings, config.defaultPermissions, options.ephemeral))
        val id = threadResult(result)
        if (!options.ephemeral) runCatching { loadHistory(id) }
        return id
    }

    override suspend fun loadHistory(threadId: String, olderThan: String?) {
        val result = connection().request(C.THREAD_TURNS_LIST, buildJsonObject {
            put("threadId", threadId)
            put("limit", config.historyPageSize)
            put("itemsView", "full")
            if (olderThan != null) put("cursor", olderThan)
        })
        // The server pages newest-first; the model is oldest-first.
        val turns = result["data"].arr?.map(CodexItems::turn)?.reversed() ?: emptyList()
        emit(AgentEvent.HistoryLoaded(kind, threadId, turns, prepend = olderThan != null, cursor = result["nextCursor"].str))
    }

    override suspend fun listThreads(cwd: String?): List<ThreadSummary> {
        val out = ArrayList<ThreadSummary>()
        for (archived in listOf(false, true)) {
            var cursor: String? = null
            var pages = 0
            do {
                val page = connection().request(C.THREAD_LIST, buildJsonObject {
                    put("limit", 100)
                    put("archived", archived)
                    if (cwd != null) put("cwd", cwd)
                    if (cursor != null) put("cursor", cursor)
                })
                page["data"].arr?.forEach { t ->
                    out += ThreadSummary(t["id"].str ?: return@forEach, t["name"].str, t["preview"].str, t["cwd"].str, t["createdAt"].long, t["updatedAt"].long, archived)
                }
                cursor = page["nextCursor"].str
            } while (cursor != null && ++pages < 50)
        }
        return out
    }

    override suspend fun rename(threadId: String, title: String) {
        connection().request(C.THREAD_NAME_SET, buildJsonObject { put("threadId", threadId); put("name", title) })
    }

    override suspend fun archive(threadId: String, archived: Boolean) {
        connection().request(if (archived) C.THREAD_ARCHIVE else C.THREAD_UNARCHIVE, buildJsonObject { put("threadId", threadId) })
    }

    override suspend fun delete(threadId: String) {
        connection().request(C.THREAD_DELETE, buildJsonObject { put("threadId", threadId) })
    }

    override suspend fun compact(threadId: String) {
        connection().request(C.THREAD_COMPACT_START, buildJsonObject { put("threadId", threadId) })
    }

    // ------------------------------------------------------------------ turns

    override suspend fun send(threadId: String, parts: List<UserPart>, settings: TurnSettings?, mode: SendMode): String {
        reviewer[threadId]?.let { check(it == CodexParams.REVIEWER_USER) { "approvalsReviewer is $it; refusing to send" } }
        val clientMessageId = newId()
        val running = activeTurn[threadId]
        val effective = when (mode) {
            SendMode.AUTO -> if (running != null) SendMode.QUEUE else SendMode.START
            SendMode.STEER -> if (running != null) SendMode.STEER else SendMode.START
            else -> mode
        }
        emit(AgentEvent.TurnSubmitted(kind, threadId, clientMessageId, parts, settings, clock()))
        try {
            when (effective) {
                SendMode.START -> {
                    val result = connection().request(C.TURN_START, CodexParams.turnStart(threadId, parts, settings, clientMessageId))
                    result["turn"]["id"].str?.let { emit(AgentEvent.TurnBound(kind, threadId, clientMessageId, it)) }
                }
                SendMode.STEER -> {
                    val result = connection().request(C.TURN_STEER, CodexParams.turnSteer(threadId, running!!, parts, clientMessageId))
                    emit(AgentEvent.TurnBound(kind, threadId, clientMessageId, result["turnId"].str ?: running))
                }
                SendMode.QUEUE -> queueLock(threadId).withLock {
                    val result = connection().request(C.THREAD_QUEUE_ADD, CodexParams.queueAdd(threadId, parts, clientMessageId))
                    result["queuedSubmission"]["id"].str?.let { queued[clientMessageId] = it; queuedThread[clientMessageId] = threadId }
                }
                SendMode.AUTO -> error("unreachable")
            }
        } catch (e: Exception) {
            emit(AgentEvent.TurnCompleted(kind, threadId, clientMessageId, TurnStatus.FAILED, TurnError(e.message ?: "send failed", code = "sendFailed")))
            throw e
        }
        return clientMessageId
    }

    override suspend fun cancelQueued(threadId: String, clientMessageId: String) {
        queueLock(threadId).withLock {
        val submission = queued[clientMessageId] ?: throw IllegalStateException("$clientMessageId is not queued")
        check(queuedThread[clientMessageId] == threadId) { "queued message belongs to a different thread" }
        connection().request(C.THREAD_QUEUE_DELETE, buildJsonObject { put("threadId", threadId); put("queuedSubmissionId", submission) })
        queued.remove(clientMessageId); queuedThread.remove(clientMessageId)
        emit(AgentEvent.TurnCancelled(kind, threadId, clientMessageId))
        }
    }

    private fun queueLock(threadId: String): Mutex = queueLocks.getOrPut(threadId) { Mutex() }

    private suspend fun refreshQueue(threadId: String, connection: JsonRpcConnection) = queueLock(threadId).withLock {
        val previous = queuedThread.filterValues { it == threadId }.keys.toSet()
        val messages = mutableListOf<top.flysoftbeta.workflow.agent.model.QueuedMessage>()
        val submissions = linkedMapOf<String, String>()
        var cursor: String? = null
        val seen = mutableSetOf<String>()
        do {
            val page = connection.request(C.THREAD_QUEUE_LIST, buildJsonObject {
                put("threadId", threadId)
                cursor?.let { put("cursor", it) }
            })
            for (item in page["data"].arr.orEmpty()) {
                val client = item["clientUserMessageId"].str ?: continue
                val submission = item["id"].str ?: continue
                submissions[client] = submission
                messages += top.flysoftbeta.workflow.agent.model.QueuedMessage(client, item["input"].arr.orEmpty().map(CodexItems::userPart))
            }
            cursor = page["nextCursor"].str
            check(cursor == null || (seen.add(cursor) && seen.size < 50)) { "queue pagination did not finish" }
        } while (cursor != null)
        if (rpc !== connection) return@withLock
        previous.forEach { queued.remove(it); queuedThread.remove(it) }
        submissions.forEach { (client, submission) -> queued[client] = submission; queuedThread[client] = threadId }
        emit(AgentEvent.QueueUpdated(kind, threadId, messages, previous))
    }

    override suspend fun interrupt(threadId: String, cancelQueued: Boolean) {
        if (cancelQueued) {
            queuedThread.filterValues { it == threadId }.keys.forEach { runCatching { cancelQueued(threadId, it) } }
        }
        val turn = activeTurn[threadId] ?: return
        connection().request(C.TURN_INTERRUPT, buildJsonObject { put("threadId", threadId); put("turnId", turn) })
    }

    override suspend fun setPermissions(threadId: String, preset: PermissionPreset) {
        val policy = CodexParams.policy(preset)
        connection().request(C.THREAD_SETTINGS_UPDATE, buildJsonObject {
            put("threadId", threadId)
            put("approvalPolicy", policy.approvalPolicy)
            put("sandboxPolicy", policy.sandboxPolicy)
            put("approvalsReviewer", CodexParams.REVIEWER_USER)
        })
    }

    // ------------------------------------------------------------------ requests

    override suspend fun respond(key: RequestKey, response: RequestResponse) {
        require(key.backend == kind)
        val request = openRequests[key] ?: throw IllegalStateException("request $key is not open")
        val connection = connection()
        val summary: String
        when (response) {
            is RequestResponse.Decide -> {
                val decision = request.decisions.firstOrNull { it.id == response.decisionId }
                    ?: throw IllegalArgumentException("decision ${response.decisionId} was not offered for $key (offered: ${request.decisions.map { it.id }})")
                if (request.kind is RequestKind.Unknown) {
                    connection.respondError(key.rawId, JsonRpcConnection.METHOD_NOT_FOUND, response.message ?: "Rejected by user")
                } else {
                    connection.respond(key.rawId, CodexEvents.decisionResult(request, decision, response.message))
                }
                summary = decision.id
            }
            is RequestResponse.Answer -> {
                val kindInput = request.kind as? RequestKind.UserInput ?: throw IllegalArgumentException("$key is not a question")
                val known = kindInput.questions.map { it.id }.toSet()
                require(response.answers.keys.all { it in known }) { "unknown question ids ${response.answers.keys - known}" }
                connection.respond(key.rawId, buildJsonObject {
                    put("answers", buildJsonObject {
                        response.answers.forEach { (id, values) -> put(id, buildJsonObject { put("answers", kotlinx.serialization.json.JsonArray(values.map(::JsonPrimitive))) }) }
                    })
                })
                summary = "answered"
            }
            is RequestResponse.Elicit -> {
                require(request.kind is RequestKind.Elicitation) { "$key is not an elicitation" }
                require(response.action in setOf("accept", "decline", "cancel")) { "invalid action ${response.action}" }
                connection.respond(key.rawId, buildJsonObject {
                    put("action", response.action)
                    put("content", response.content ?: JsonNull)
                })
                summary = response.action
            }
            is RequestResponse.RawResult -> {
                require(request.kind is RequestKind.Unknown) { "raw results are only allowed for unknown requests" }
                connection.respond(key.rawId, response.result)
                summary = "raw"
            }
            is RequestResponse.Reject -> {
                connection.respondError(key.rawId, -32000, response.message)
                summary = "rejected"
            }
        }
        openRequests.remove(key)
        emit(AgentEvent.RequestClosed(key, RequestStatus.ANSWERED, summary))
    }

    override suspend fun rawRequest(method: String, params: JsonElement?, threadId: String?): JsonElement =
        connection().request(method, CodexParams.enforceReviewer(method, params))

    /** Test hook: currently open (unanswered) server requests. */
    internal fun openRequestKeys(): Set<RequestKey> = openRequests.keys.toSet()
}

@Suppress("unused")
private val EMPTY_OBJECT = JsonObject(emptyMap())
