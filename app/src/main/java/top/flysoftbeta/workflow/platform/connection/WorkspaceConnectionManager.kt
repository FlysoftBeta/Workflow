package top.flysoftbeta.workflow.platform.connection

import android.content.Context
import java.io.File
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.util.UUID
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import top.flysoftbeta.workflow.core.connection.*
import top.flysoftbeta.workflow.core.config.AppConfig
import top.flysoftbeta.workflow.core.config.ConfigCodec
import top.flysoftbeta.workflow.core.config.ConfigParse
import top.flysoftbeta.workflow.core.json.Json
import top.flysoftbeta.workflow.core.store.StoreStatus
import top.flysoftbeta.workflow.platform.service.LocalRuntimeService

sealed interface ConnectionStatus {
    data object Loading : ConnectionStatus
    data object Configure : ConnectionStatus
    data class Connecting(val name: String) : ConnectionStatus
    data class Connected(val session: WorkspaceConnectionSession) : ConnectionStatus
    data class Failed(val message: String, val profile: WorkspaceConnectionConfig?) : ConnectionStatus
}

data class WorkspaceConnectionSession(
    val token: String,
    val profile: WorkspaceConnectionConfig,
    val rpc: WorkspaceRpc,
    val store: RemoteWorkspaceStore,
    val scope: CoroutineScope,
)

/** The shell's only persistent files: connection profiles and Engine-delivered local configuration. */
class WorkspaceConnectionManager private constructor(context: Context) {
    private val app = context.applicationContext
    private val bootstrapper: WorkspaceBootstrapper = EmbeddedWorkspaceBootstrapper(app)
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val mutable = MutableStateFlow<ConnectionStatus>(ConnectionStatus.Loading)
    val status = mutable.asStateFlow()
    private val configured = MutableStateFlow<List<WorkspaceConnectionConfig>>(emptyList())
    val profiles = configured.asStateFlow()
    private val profilesFile = File(app.filesDir, "workspace-connections.json")
    private val localCache = File(app.filesDir, "workspace-local-config.json")
    private val cacheWriteLock = Mutex()
    private var connecting: Job? = null
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

    fun requireSession(): WorkspaceConnectionSession = sessionOrNull() ?: error("请先配置并连接工作区")
    fun sessionOrNull(): WorkspaceConnectionSession? = current?.takeIf {
        it.rpc.failure.value == null && it.store.state.value.status == StoreStatus.READY && it.scope.isActive
    }

    fun useEmbedded(name: String = "此设备的工作区") {
        val existing = configured.value.firstOrNull { it.endpoint is WorkspaceEndpoint.Embedded }
        val id = UUID.randomUUID().toString()
        connect(existing ?: WorkspaceConnectionConfig(id, name, WorkspaceEndpoint.Embedded(id)))
    }

    @Synchronized fun connect(profile: WorkspaceConnectionConfig) {
        if (connecting?.isActive == true) return
        val token = UUID.randomUUID().toString()
        activeToken = token
        connecting = scope.launch(start = CoroutineStart.LAZY) {
            val previous = current
            current = null
            previous?.let(::retire)
            mutable.value = ConnectionStatus.Connecting(profile.name)
            var rpc: WorkspaceRpc? = null
            var sessionScope: CoroutineScope? = null
            var store: RemoteWorkspaceStore? = null
            try {
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
                val transport = bootstrapper.connect(normalized)
                sessionScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
                rpc = WorkspaceRpc(transport, sessionScope)
                val connectedStore = RemoteWorkspaceStore(rpc, sessionScope, profile.id).also { it.start() }
                store = connectedStore
                val ready = withTimeout(45_000) { connectedStore.awaitReady() }
                check(ready.status == StoreStatus.READY) { ready.failure ?: (transport as? EmbeddedWorkspaceTransport)?.diagnostic()?.ifBlank { null } ?: "工作区未能启动" }
                val session = WorkspaceConnectionSession(token, normalized, rpc, connectedStore, sessionScope)
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
                scope.launch {
                    val message = session.store.state.first { it.status == StoreStatus.FAILED }.failure ?: "工作区连接已断开"
                    synchronized(this@WorkspaceConnectionManager) {
                        if (current?.token == session.token) {
                            current = null
                            mutable.value = ConnectionStatus.Failed(message, normalized)
                        }
                    }
                    retire(session)
                }
                session.scope.launch {
                    session.rpc.failure.filterNotNull().first().let(session.store::disconnect)
                }
            } catch (error: Exception) {
                store?.disconnect(error.message ?: "无法连接工作区")
                rpc?.close(); sessionScope?.cancel()
                LocalRuntimeService.releaseEnvironment(app, token)
                if (error is CancellationException && error !is TimeoutCancellationException) throw error
                synchronized(this@WorkspaceConnectionManager) {
                    if (activeToken == token) {
                        current = null
                        mutable.value = ConnectionStatus.Failed(error.message ?: "无法连接工作区", profile)
                    }
                }
            }
        }.also { it.start() }
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
        val result = WorkspaceWire.obj(session.rpc.request("client.config", mapOf("clientId" to session.profile.id)))
        val config = localDocument(result["config"])
        val decoded = decodeLocal(config)
        currentCoroutineContext().ensureActive()
        cacheWriteLock.withLock {
            if (activeToken != session.token) return@withLock
            val text = Json.stringify(mapOf("format" to 1, "connection" to session.profile.id,
                "revision" to WorkspaceWire.long(result, "revision"), "config" to config))
            mutableLocalConfig.value = decoded
            mutableCacheError.value = runCatching { atomic(localCache, text) }.exceptionOrNull()?.let { "本地显示配置无法缓存：${it.message}" }
        }
    }

    fun retry() { (status.value as? ConnectionStatus.Failed)?.profile?.let(::connect) ?: run { mutable.value = ConnectionStatus.Configure } }

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
        @Volatile private var instance: WorkspaceConnectionManager? = null
        fun get(context: Context): WorkspaceConnectionManager = instance ?: synchronized(this) {
            instance ?: WorkspaceConnectionManager(context).also { instance = it }
        }
    }
}
