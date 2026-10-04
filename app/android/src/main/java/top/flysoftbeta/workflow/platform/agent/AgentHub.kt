package top.flysoftbeta.workflow.platform.agent

import android.content.Context
import java.io.ByteArrayOutputStream
import java.util.Base64
import java.util.IdentityHashMap
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.serialization.json.*
import top.flysoftbeta.workflow.agent.SendMode
import top.flysoftbeta.workflow.agent.model.*
import top.flysoftbeta.workflow.agent.rpc.*
import top.flysoftbeta.workflow.client.protocol.HelloResult
import top.flysoftbeta.workflow.core.connection.RemoteWorkspaceStore
import top.flysoftbeta.workflow.core.connection.WorkspaceRpc
import top.flysoftbeta.workflow.core.connection.WorkspaceClosedException
import top.flysoftbeta.workflow.core.connection.WorkspaceRpcException
import top.flysoftbeta.workflow.core.resource.ComposerDraft
import top.flysoftbeta.workflow.core.store.StateCodec
import top.flysoftbeta.workflow.core.store.WorkspaceStore
import top.flysoftbeta.workflow.core.json.Json as CoreJson

/** Connection-scoped, disposable Engine projection. No vendor process or workspace writer lives here. */
class AgentHub(
    @Suppress("UNUSED_PARAMETER") appContext: Context,
    private val store: WorkspaceStore,
    private val scope: CoroutineScope,
    @Suppress("UNUSED_PARAMETER") io: CoroutineDispatcher,
) {
    private val rpc: WorkspaceRpc get() = (store as? RemoteWorkspaceStore)?.rpc ?: throw WorkspaceClosedException("请连接工作区 Engine")
    private val mutable = MutableStateFlow(AgentState())
    val state = mutable.asStateFlow()
    private val metadata = MutableStateFlow(ChatMetadata())
    val configuration = metadata.asStateFlow()
    val conversations = metadata.map { it.conversations }.stateIn(scope, SharingStarted.Eagerly, emptyList())
    val available = metadata.map { it.available }.stateIn(scope, SharingStarted.Eagerly, emptySet())
    val attention = state.map { s -> s.requests.values.any { it.status.isOpen } }
        .distinctUntilChanged().stateIn(scope, SharingStarted.Eagerly, false)
    private val lock = Mutex()
    private var epoch = ""
    private var revision = -1L
    private var job: Job? = null
    private val requestEpochs = IdentityHashMap<PendingRequest, String>()

    @Synchronized fun start() {
        if (job != null) return
        job = scope.launch {
            store.awaitReady()
            while (isActive) {
                try {
                    snapshot()
                    while (isActive) {
                        val cursor = lock.withLock { epoch to revision }
                        val update = ChatWire.update(wire(rpc.request("chat.watch", mapOf("epoch" to cursor.first,
                            "afterRevision" to cursor.second, "timeoutMs" to 25_000), 35_000)))
                        if (update.resnapshot || update.epoch != cursor.first) break
                        lock.withLock {
                            if (appliesTo(update, cursor, epoch to revision)) {
                                publish(AgentReducer.reduceAll(mutable.value, update.events), update.metadata)
                                revision = update.revision
                            }
                        }
                    }
                } catch (cancelled: CancellationException) { throw cancelled }
                catch (_: Exception) {
                    // A failed watch never authorizes stale capabilities or approval cards.
                    lock.withLock { metadata.value = metadata.value.copy(available = emptySet()); requestEpochs.clear() }
                    if (rpc.failure.value != null) return@launch
                    delay(1000)
                }
            }
        }
    }

    companion object {
        /**
         * Whether a `chat.watch` [update] requested from [requested] (epoch, afterRevision) may be applied to the
         * projection now at [current]. Its events follow the requested revision, so they apply only while the
         * projection is still exactly there. A command's snapshot taken during the watch moves [current] to the
         * snapshot's revision, which already contains some or all of those events; reducing them again would
         * duplicate them. Such an update is dropped and the next watch resumes after the snapshot's revision,
         * so the Engine's journal delivers the remaining changes and nothing is lost.
         */
        internal fun appliesTo(update: ChatUpdate, requested: Pair<String, Long>, current: Pair<String, Long>): Boolean =
            update.epoch == requested.first && current == requested && update.revision > requested.second

        /** The Engine refused a chat call because the environment is not usable yet; it has already started preparing it. */
        internal fun preparingEnvironment(error: Throwable): Boolean =
            error is WorkspaceRpcException && error.kind == "environment_preparing"

        /**
         * Runs [call] until the Engine accepts it. The Engine's chat gate refuses before dispatch, so a refused
         * command had no effect and repeating it is safe; this waits at the watch loop's one-second cadence.
         * Any other failure, including a closed connection, ends the wait.
         */
        internal suspend fun <T> awaitingEnvironment(retryDelayMs: Long = 1000, call: suspend () -> T): T {
            while (true) {
                try { return call() } catch (error: WorkspaceRpcException) { if (!preparingEnvironment(error)) throw error }
                delay(retryDelayMs)
            }
        }
    }

    private suspend fun snapshot() = lock.withLock {
        var value = wire(rpc.request("chat.snapshot", timeoutMs = 60_000))
        if ((value as? JsonObject)?.containsKey("transferId") == true) {
            val bytes = ByteArrayOutputStream()
            var transfer: String? = null
            while (true) {
                val chunk = ChatWire.decode<ChatSnapshotChunk>(value)
                if (transfer == null) transfer = chunk.transferId
                check(chunk.transferId == transfer && chunk.offset == bytes.size()) { "聊天快照偏移无效" }
                val data = Base64.getDecoder().decode(chunk.data)
                check(chunk.nextOffset == chunk.offset + data.size && (data.isNotEmpty() || chunk.eof)) { "聊天快照没有进展" }
                check(bytes.size().toLong() + data.size <= 64L * 1024 * 1024) { "聊天快照过大" }
                bytes.write(data)
                if (chunk.eof) break
                value = wire(rpc.request("chat.snapshot", mapOf("transferId" to transfer, "offset" to chunk.nextOffset)))
            }
            value = ChatWire.json.parseToJsonElement(bytes.toString(Charsets.UTF_8.name()))
        }
        val next = ChatWire.snapshot(value)
        if (epoch != next.epoch) requestEpochs.clear()
        epoch = next.epoch; revision = next.revision
        publish(next.state, next.metadata)
    }

    private fun publish(next: AgentState, details: ChatMetadata) {
        val live = next.requests.values.toSet()
        requestEpochs.keys.removeAll { it !in live }
        next.requests.values.forEach { request -> details.processEpochs[request.key.backend]?.let { processEpoch ->
            if (!requestEpochs.containsKey(request)) requestEpochs[request] = processEpoch
        } }
        metadata.value = details
        mutable.value = next
    }

    private suspend fun command(name: String, vararg args: Pair<String, Any?>): JsonElement {
        // A user action waits while the environment prepares instead of failing; panels show the preparation.
        val value = wire(awaitingEnvironment { rpc.request("chat.command", mapOf("name" to name, "args" to mapOf(*args)), 120_000) })
        // The command is already committed. A projection read failure must not turn it into a failed
        // send or encourage a duplicate submission; the watch will recover the display.
        try { snapshot() } catch (cancelled: CancellationException) { throw cancelled } catch (_: Exception) { }
        return value
    }
    private fun wire(value: Any?): JsonElement = ChatWire.json.parseToJsonElement(CoreJson.stringify(value))
    private inline fun <reified T> encoded(value: T): Any? = CoreJson.parse(ChatWire.stringify(value))
    fun entry(id: String) = metadata.value.conversations.firstOrNull { it.id == id }
    fun threadKey(entry: ConversationEntry) = entry.backendThreadId?.let { ThreadKey(entry.backend, it) }
    /** Agent paths with the homes Engine reported in its handshake; before it, workspace paths only. */
    fun paths(@Suppress("UNUSED_PARAMETER") kind: BackendKind): AgentPaths {
        val hello = (store as? RemoteWorkspaceStore)?.rpc?.helloResult ?: return AgentPaths()
        agentPaths?.takeIf { it.first === hello }?.let { return it.second }
        val homes = HelloResult.decode(wire(hello)).agentHomes.associate { it.guest to it.path }
        return AgentPaths(homes = homes).also { agentPaths = hello to it }
    }
    @Volatile private var agentPaths: Pair<Map<String, Any?>, AgentPaths>? = null
    fun defaultBackend() = metadata.value.defaultBackend
    fun permissions() = metadata.value.permissions
    fun loginMethods(kind: BackendKind) = metadata.value.loginMethods[kind].orEmpty()
    fun warmUp(kind: BackendKind) { scope.launch { runCatching { command("warmUp", "kind" to kind.id) } } }
    suspend fun ensureConversation(id: String): ConversationEntry = ChatWire.decode(command("ensureConversation", "id" to id))
    suspend fun open(id: String) { command("open", "id" to id) }
    suspend fun loadEarlier(id: String) { command("loadEarlier", "id" to id) }
    suspend fun send(id: String, submitted: ComposerDraft, settings: TurnSettings, mode: SendMode, operationId: String): String =
        command("send", "id" to id, "text" to submitted.text,
            "attachments" to submitted.attachments.map { mapOf("path" to it.path, "mimeType" to it.mimeType) },
            "settings" to encoded(settings), "mode" to mode.name, "operationId" to operationId,
            "submitted" to CoreJson.parse(StateCodec.encodeComposer(submitted))).jsonPrimitive.content
    suspend fun interrupt(id: String) { command("interrupt", "id" to id) }
    suspend fun cancelQueued(id: String, clientMessageId: String) { command("cancelQueued", "id" to id, "clientMessageId" to clientMessageId) }
    suspend fun respond(request: PendingRequest, response: RequestResponse) {
        val processEpoch = lock.withLock { requestEpochs[request] } ?: error("请求已过期，请等待工作区刷新")
        command("respond", "key" to encoded(request.key), "response" to encoded(response), "processEpoch" to processEpoch)
    }
    suspend fun rename(id: String, title: String) { command("rename", "id" to id, "title" to title) }
    suspend fun archive(id: String, archived: Boolean) { command("archive", "id" to id, "archived" to archived) }
    suspend fun deleteConversation(id: String) { command("deleteConversation", "id" to id) }
    suspend fun compact(id: String) { command("compact", "id" to id) }
    suspend fun refreshUsage(kind: BackendKind) { command("refreshUsage", "kind" to kind.id) }
    suspend fun rawRequest(id: String, method: String, params: JsonElement?): JsonElement =
        command("rawRequest", "id" to id, "method" to method, "params" to params?.let { CoreJson.parse(it.toString()) })
    suspend fun fork(id: String, atTurnId: String?): String = command("fork", "id" to id, "atTurnId" to atTurnId).jsonPrimitive.content
    suspend fun setPermissions(id: String, preset: PermissionPreset) { command("setPermissions", "id" to id, "preset" to preset.name) }
    suspend fun rememberSelection(id: String, model: String?, effort: String?) { command("rememberSelection", "id" to id, "model" to model, "effort" to effort) }
    suspend fun setBackend(id: String, kind: BackendKind) { command("setBackend", "id" to id, "kind" to kind.id) }
    suspend fun login(kind: BackendKind, method: LoginMethod, secret: String? = null): LoginFlow =
        ChatWire.decode(command("login", "kind" to kind.id, "method" to method.name, "secret" to secret))
    suspend fun cancelLogin(kind: BackendKind, loginId: String) { command("cancelLogin", "kind" to kind.id, "loginId" to loginId) }
    suspend fun logout(kind: BackendKind) { command("logout", "kind" to kind.id) }
    suspend fun refreshAccount(kind: BackendKind) { command("refreshAccount", "kind" to kind.id) }
    suspend fun flush() { command("flush") }
}
