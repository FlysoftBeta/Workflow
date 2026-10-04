package top.flysoftbeta.workflow.platform.connection

import android.content.Context
import android.util.Log
import java.io.File
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.util.UUID
import kotlinx.coroutines.*
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import top.flysoftbeta.workflow.core.connection.*
import top.flysoftbeta.workflow.client.sync.ClientConfigurationSync
import top.flysoftbeta.workflow.client.sync.WorkspaceIdentity
import top.flysoftbeta.workflow.core.config.AppConfig
import top.flysoftbeta.workflow.core.config.ConfigCodec
import top.flysoftbeta.workflow.core.config.ConfigParse
import top.flysoftbeta.workflow.core.json.Json
import top.flysoftbeta.workflow.core.store.StoreStatus
import top.flysoftbeta.workflow.platform.UnhandledFailures
import top.flysoftbeta.workflow.platform.service.LocalRuntimeService

/** The step a connection attempt is performing, shown while it connects (docs/app/connection.md). */
enum class ConnectPhase(val label: String) {
    ENGINE("正在启动工作区 Engine"),
    WORKSPACE("正在加载工作区"),
    CONFIGURATION("正在同步配置"),
}

sealed interface ConnectionStatus {
    data object Loading : ConnectionStatus
    data object Configure : ConnectionStatus
    data class Connecting(val name: String, val phase: ConnectPhase = ConnectPhase.ENGINE) : ConnectionStatus
    data class Connected(val session: WorkspaceConnectionSession) : ConnectionStatus
    /**
     * A connection that was online was lost and is retried automatically. [phase] is null while the attempt waits
     * until [retryAt] (epoch milliseconds); [detail] is the lost Engine's diagnostic output.
     */
    data class Reconnecting(
        val name: String,
        val attempt: Int,
        val cause: String,
        val phase: ConnectPhase?,
        val retryAt: Long?,
        val detail: String?,
    ) : ConnectionStatus
    /** [retryable] is false when retrying cannot help, such as an Engine of another version. */
    data class Failed(
        val message: String,
        val profile: WorkspaceConnectionConfig?,
        val detail: String? = null,
        val retryable: Boolean = true,
    ) : ConnectionStatus
}

data class WorkspaceConnectionSession(
    val token: String,
    val profile: WorkspaceConnectionConfig,
    val rpc: WorkspaceRpc,
    val store: RemoteWorkspaceStore,
    val scope: CoroutineScope,
    val configuration: ClientConfigurationSync? = null,
    /** The Engine's recent diagnostic output, for Details after a failure. */
    val diagnostic: () -> String = { "" },
)

/** The shell's only persistent files: connection profiles and Engine-delivered local configuration. */
class WorkspaceConnectionManager private constructor(context: Context) {
    private val app = context.applicationContext
    private val bootstrapper: WorkspaceBootstrapper = EmbeddedWorkspaceBootstrapper(app)
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO + UnhandledFailures.handler("connection"))
    private val mutable = MutableStateFlow<ConnectionStatus>(ConnectionStatus.Loading)
    val status = mutable.asStateFlow()
    private val configured = MutableStateFlow<List<WorkspaceConnectionConfig>>(emptyList())
    val profiles = configured.asStateFlow()
    private val profilesFile = File(app.filesDir, "workspace-connections.json")
    private val localCache = File(app.filesDir, "workspace-local-config.json")
    private val cacheWriteLock = Mutex()
    private var connecting: Job? = null
    /** Ends a reconnect wait early: Retry now, or the app returning to the foreground. */
    private val wake = Channel<Unit>(Channel.CONFLATED)
    @Volatile private var activeToken: String? = null
    @Volatile private var current: WorkspaceConnectionSession? = null
    private val mutableLocalConfig = MutableStateFlow(AppConfig())
    /** Cached presentation preferences may style the connection screen, never substitute workspace state. */
    val localConfig = mutableLocalConfig.asStateFlow()
    private val mutableCacheError = MutableStateFlow<String?>(null)
    val cacheError = mutableCacheError.asStateFlow()

    init {
        scope.launch {
            try {
                if (!profilesFile.isFile) { mutable.value = ConnectionStatus.Configure; return@launch }
                val document = WorkspaceWire.obj(Json.parse(profilesFile.readText()))
                check((document["format"] as? Number)?.toInt() == 1) { "不支持此连接配置格式" }
                configured.value = (document["profiles"] as? List<*>).orEmpty().map { item ->
                    val profile = WorkspaceWire.obj(item)
                    val id = WorkspaceWire.string(profile, "id")
                    check(ID.matches(id)) { "连接标识无效" }
                    check(profile["transport"] == "embedded") { "此版本尚未实现远程连接" }
                    WorkspaceConnectionConfig(id, WorkspaceWire.string(profile, "name"), WorkspaceEndpoint.Embedded(id))
                }
                val selected = configured.value.firstOrNull { it.id == document["active"] }
                if (selected == null) mutable.value = ConnectionStatus.Configure else { loadLocalConfig(selected.id); connect(selected) }
            } catch (error: Exception) { mutable.value = ConnectionStatus.Failed(error.message ?: "连接配置无法读取", null) }
        }
    }

    fun requireSession(): WorkspaceConnectionSession = sessionOrNull() ?: throw WorkspaceClosedException("请先配置并连接工作区")
    fun sessionOrNull(): WorkspaceConnectionSession? = current?.takeIf {
        it.rpc.failure.value == null && it.store.state.value.status == StoreStatus.READY && it.scope.isActive
    }

    fun useEmbedded(name: String = "此设备的工作区") {
        val existing = configured.value.firstOrNull { it.endpoint is WorkspaceEndpoint.Embedded }
        val id = UUID.randomUUID().toString()
        connect(existing ?: WorkspaceConnectionConfig(id, name, WorkspaceEndpoint.Embedded(id)))
    }

    @Synchronized fun connect(profile: WorkspaceConnectionConfig) = start(profile, lost = null)

    /** Why an attempt or an online connection failed; [retryable] is false when retrying cannot help. */
    private class Loss(val message: String, val detail: String?, val retryable: Boolean)

    /**
     * Connects [profile]. With [lost], an online connection was lost: attempts repeat automatically after
     * [RECONNECT_DELAYS_MS] while the previous Workbench stays inert. A first connection reports its failure at once.
     */
    @Synchronized private fun start(profile: WorkspaceConnectionConfig, lost: Loss?) {
        if (connecting?.isActive == true) return
        val token = UUID.randomUUID().toString()
        activeToken = token
        while (wake.tryReceive().isSuccess) Unit
        connecting = scope.launch(start = CoroutineStart.LAZY) {
            val previous = current
            current = null
            previous?.let(::retire)
            var cause = lost
            var attempt = 0
            while (true) {
                attempt++
                val reconnecting = cause
                if (reconnecting != null) {
                    val wait = RECONNECT_DELAYS_MS[attempt - 1]
                    log("Reconnect attempt $attempt in $wait ms: ${reconnecting.message}")
                    publish(token, ConnectionStatus.Reconnecting(profile.name, attempt, reconnecting.message, null,
                        System.currentTimeMillis() + wait, reconnecting.detail))
                    withTimeoutOrNull(wait) { wake.receive() }
                }
                val failure = connectOnce(profile, token) { phase ->
                    publish(token, if (reconnecting == null) ConnectionStatus.Connecting(profile.name, phase)
                        else ConnectionStatus.Reconnecting(profile.name, attempt, reconnecting.message, phase, null, reconnecting.detail))
                } ?: return@launch
                log("Connection attempt $attempt failed: ${failure.message}")
                if (reconnecting == null || !failure.retryable || attempt >= RECONNECT_DELAYS_MS.size) {
                    publish(token, ConnectionStatus.Failed(failure.message, profile, failure.detail, failure.retryable))
                    return@launch
                }
                cause = failure
            }
        }.also { it.start() }
    }

    private fun log(message: String) { runCatching { Log.w(UnhandledFailures.TAG, message) } }

    /** Publishes [status] only while [token] is still the current attempt. */
    private fun publish(token: String, status: ConnectionStatus) = synchronized(this) {
        if (activeToken == token) mutable.value = status
    }

    /** One connection attempt; null when it connected. */
    private suspend fun connectOnce(profile: WorkspaceConnectionConfig, token: String, phase: (ConnectPhase) -> Unit): Loss? {
        var rpc: WorkspaceRpc? = null
        var sessionScope: CoroutineScope? = null
        var store: RemoteWorkspaceStore? = null
        var transport: WorkspaceTransport? = null
        try {
            phase(ConnectPhase.ENGINE)
            val endpoint = profile.endpoint as? WorkspaceEndpoint.Embedded
                ?: throw UnsupportedWorkspaceTransport("远程连接只提供扩展接口，此版本尚未实现")
            check(ID.matches(profile.id) && ID.matches(endpoint.workspaceId)) { "工作区标识无效" }
            // Use profile id for the local directory; no old workspace is scanned or migrated.
            val normalized = profile.copy(endpoint = WorkspaceEndpoint.Embedded(profile.id))
            val profiles = configured.value.filterNot { it.id == normalized.id } + normalized
            saveProfiles(normalized.id, profiles)
            configured.value = profiles
            loadLocalConfig(normalized.id)
            LocalRuntimeService.retainEnvironment(app, token).getOrThrow()
            val connected = bootstrapper.connect(normalized)
            transport = connected
            val diagnostic = { (connected as? EmbeddedWorkspaceTransport)?.diagnostic().orEmpty() }
            phase(ConnectPhase.WORKSPACE)
            sessionScope = CoroutineScope(SupervisorJob() + Dispatchers.Default + UnhandledFailures.handler("session"))
            rpc = WorkspaceRpc(connected, sessionScope)
            val connectedStore = RemoteWorkspaceStore(rpc, sessionScope, profile.id).also { it.start() }
            store = connectedStore
            val ready = withTimeout(45_000) { connectedStore.awaitReady() }
            check(ready.status == StoreStatus.READY) { ready.failure ?: diagnostic().ifBlank { null } ?: "工作区未能启动" }
            val identity = WorkspaceIdentity(normalized.id, WorkspaceWire.string(rpc.hello(normalized.id), "workspaceRoot"))
            val session = WorkspaceConnectionSession(token, normalized, rpc, connectedStore, sessionScope,
                ClientConfigurationSync(rpc, identity), diagnostic)
            phase(ConnectPhase.CONFIGURATION)
            refreshLocalConfig(session)
            check(session.rpc.failure.value == null && session.store.state.value.status == StoreStatus.READY) { "工作区连接已断开" }
            WorkspaceNetworkReporter.attach(app, session)
            val attemptContext = currentCoroutineContext()
            synchronized(this@WorkspaceConnectionManager) {
                attemptContext.ensureActive()
                check(activeToken == token) { "工作区连接已取消" }
                current = session
                mutable.value = ConnectionStatus.Connected(session)
            }
            session.scope.launch(Dispatchers.IO) {
                session.store.state.map { it.config }.distinctUntilChanged().collect {
                    try { refreshLocalConfig(session) }
                    catch (cancelled: CancellationException) { throw cancelled }
                    catch (error: Exception) { mutableCacheError.value = error.message ?: "本地显示配置无法缓存" }
                }
            }
            // Run cleanup in the manager scope so retiring the session cannot cancel its own cleanup.
            val attemptJob = currentCoroutineContext()[Job]
            scope.launch {
                val message = session.store.state.first { it.status == StoreStatus.FAILED }.failure ?: "工作区连接已断开"
                val lost = synchronized(this@WorkspaceConnectionManager) {
                    (current?.token == session.token).also { if (it) current = null }
                }
                val detail = session.diagnostic().ifBlank { null }
                retire(session)
                // The attempt that produced this session may still be finishing; reconnect after it ended.
                attemptJob?.join()
                if (lost) start(normalized, Loss(message, detail, retryable = true))
            }
            session.scope.launch {
                session.rpc.failure.filterNotNull().first().let(session.store::disconnect)
            }
            return null
        } catch (error: Exception) {
            store?.disconnect(error.message ?: "无法连接工作区")
            rpc?.close(); sessionScope?.cancel()
            LocalRuntimeService.releaseEnvironment(app, token)
            if (error is CancellationException && error !is TimeoutCancellationException) throw error
            val detail = (transport as? EmbeddedWorkspaceTransport)?.diagnostic()?.ifBlank { null }
            val retryable = error !is IncompatibleWorkspaceException && error !is UnsupportedWorkspaceTransport
            val message = if (error is TimeoutCancellationException) "工作区启动超时" else error.message ?: "无法连接工作区"
            return Loss(message, detail, retryable)
        }
    }

    /** Explicit disconnection invalidates old consumers before their callbacks can issue more work. */
    @Synchronized fun disconnect() {
        activeToken = null
        connecting?.cancel()
        connecting = null
        val previous = current
        current = null
        mutable.value = ConnectionStatus.Configure
        previous?.let { scope.launch { retire(it) } }
    }

    private fun retire(session: WorkspaceConnectionSession) {
        session.store.disconnect()
        session.scope.cancel()
        LocalRuntimeService.releaseEnvironment(app, session.token)
    }

    private fun localDocument(value: Any?): Map<String, Any?> = WorkspaceWire.obj(value).filterKeys { it in LOCAL_KEYS }
    private fun decodeLocal(value: Map<String, Any?>): AppConfig =
        (ConfigCodec.decode(Json.stringify(value + ("version" to ConfigCodec.VERSION))) as? ConfigParse.Ok)?.config
            ?: error("工作区下发的本地配置无效")

    private suspend fun loadLocalConfig(connection: String) = cacheWriteLock.withLock {
        mutableLocalConfig.value = AppConfig()
        if (!localCache.isFile) return@withLock
        runCatching {
            val cached = WorkspaceWire.obj(Json.parse(localCache.readText()))
            if (cached["format"] == 1L && cached["connection"] == connection) {
                mutableLocalConfig.value = decodeLocal(localDocument(cached["config"]))
            }
        }.onFailure { mutableCacheError.value = "本地显示配置无法读取，将从工作区重新获取" }
    }

    private suspend fun refreshLocalConfig(session: WorkspaceConnectionSession) {
        val result = checkNotNull(session.configuration).refresh()
        val config = localDocument(Json.parse(result.configuration.json.toString()))
        val decoded = decodeLocal(config)
        currentCoroutineContext().ensureActive()
        cacheWriteLock.withLock {
            if (activeToken != session.token) return@withLock
            val text = Json.stringify(mapOf("format" to 1, "connection" to session.profile.id,
                "workspaceRoot" to result.identity.workspaceRoot, "revision" to result.revision, "config" to config))
            mutableLocalConfig.value = decoded
            mutableCacheError.value = runCatching { atomic(localCache, text) }.exceptionOrNull()?.let { "本地显示配置无法缓存：${it.message}" }
        }
    }

    /** Retry now: ends a reconnect wait, or repeats a failed connection once. */
    fun retry() {
        when (val current = status.value) {
            is ConnectionStatus.Reconnecting -> wake.trySend(Unit)
            is ConnectionStatus.Failed -> current.profile?.let(::connect) ?: run { mutable.value = ConnectionStatus.Configure }
            else -> Unit
        }
    }

    /** The app returned to the foreground: a waiting reconnect attempt runs now. */
    fun onForeground() {
        if (status.value is ConnectionStatus.Reconnecting) wake.trySend(Unit)
    }

    private fun saveProfiles(active: String, profiles: List<WorkspaceConnectionConfig>) = atomic(profilesFile, Json.stringify(mapOf(
        "format" to 1, "active" to active,
        "profiles" to profiles.map { mapOf("id" to it.id, "name" to it.name, "transport" to "embedded") },
    ), pretty = true))

    private fun atomic(file: File, text: String) {
        file.parentFile!!.mkdirs()
        val temp = File(file.parentFile, ".${file.name}.${UUID.randomUUID()}.tmp")
        try { temp.outputStream().use { it.write(text.toByteArray()); it.fd.sync() }; Files.move(temp.toPath(), file.toPath(), StandardCopyOption.ATOMIC_MOVE, StandardCopyOption.REPLACE_EXISTING) }
        finally { temp.delete() }
    }

    companion object {
        private val LOCAL_KEYS = setOf("appearance", "overlay", "launcher", "terminal")
        private val ID = Regex("[A-Za-z0-9_-]{1,64}")
        /** Waits before each automatic reconnect attempt; after the last one the connection is reported offline. */
        val RECONNECT_DELAYS_MS = longArrayOf(1_000, 2_000, 5_000, 10_000, 30_000)
        @Volatile private var instance: WorkspaceConnectionManager? = null
        fun get(context: Context): WorkspaceConnectionManager = instance ?: synchronized(this) {
            instance ?: WorkspaceConnectionManager(context).also { instance = it }
        }
    }
}
