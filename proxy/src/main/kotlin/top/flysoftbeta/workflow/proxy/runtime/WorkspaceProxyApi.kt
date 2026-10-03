package top.flysoftbeta.workflow.proxy.runtime

import java.io.File
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import top.flysoftbeta.workflow.proxy.config.LocalProxyConfig
import top.flysoftbeta.workflow.proxy.config.MihomoConfigTemplate
import top.flysoftbeta.workflow.proxy.config.MihomoConfigAssets
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.util.UUID
import top.flysoftbeta.workflow.proxy.controller.ProxySnapshot

/** The Engine supplies canonical configuration and acknowledges measured service state. */
interface ProxyWorkspace {
    data class Config(val text: String?, val revision: Long)
    suspend fun readConfig(): Config
    suspend fun writeConfig(text: String, expectedRevision: Long)
    suspend fun report(measured: ProxyState)
    suspend fun writeLog(text: String)
    suspend fun readAsset(path: String): ByteArray
}

/**
 * Android's executor has only a disposable cache. Nothing may start from an old cached configuration;
 * each operation fetches the Engine document and every UI state is published after its Engine receipt.
 * The local [ProxyRuntime] retains process ownership and all root/TUN safety checks.
 */
class WorkspaceProxyApi(
    private val runtime: ProxyApi,
    private val workspace: ProxyWorkspace,
    scope: CoroutineScope,
) : ProxyApi by runtime {
    private val published = MutableStateFlow(ProxyState(phase = ProxyPhase.STARTING, progress = "正在连接代理服务"))
    override val state = published.asStateFlow()
    private val configuration = Mutex()
    private val publication = Mutex()
    private var stagedDigest: String? = null
    private var stagedText: String? = null
    @Volatile private var configurationAvailable = false
    @Volatile private var configurationFailure: String? = null

    init {
        scope.launch {
            runtime.state.collect {
                try { publish() }
                catch (cancelled: CancellationException) { throw cancelled }
                catch (_: Exception) { unavailable() }
            }
        }
    }

    private fun unavailable() {
        // A connection failure is not a measured stopped state. Preserve last acknowledged capabilities
        // but disable start/controller UI, and explicitly mark a possibly running kernel unconfirmed.
        published.value = published.value.copy(phase = ProxyPhase.ERROR, progress = null,
            error = "工作区未确认代理状态，请重新连接", controllerReachable = false,
            config = ProxyConfigState(), stopUnconfirmed = runtime.state.value.pid != null || runtime.state.value.busy || runtime.state.value.stopUnconfirmed)
    }

    private suspend fun publish(): ProxyState = publication.withLock {
        val local = runtime.state.value
        val measured = if (configurationAvailable) local else local.copy(config = ProxyConfigState(),
            error = local.error ?: configurationFailure,
            groups = if (local.running) local.groups else ProxySnapshot.EMPTY,
            mode = if (local.running) local.mode else null)
        workspace.report(measured)
        published.value = measured
        measured
    }

    private suspend fun syncConfig(create: Boolean = false): Boolean {
        var document = try { workspace.readConfig() } catch (error: Exception) {
            configurationAvailable = false
            configurationFailure = "无法读取工作区代理配置"
            throw error
        }
        if (document.text == null && create) {
            workspace.writeConfig(MihomoConfigTemplate.render(MihomoConfigTemplate.newSecret()), document.revision)
            document = workspace.readConfig() // Stage only bytes returned by the Engine.
        }
        val text = document.text ?: run {
            configurationAvailable = false
            configurationFailure = "还没有代理配置"
            return false
        }
        val bytes = text.toByteArray(Charsets.UTF_8)
        require(bytes.size <= LocalProxyConfig.MAX_CONFIG_BYTES) { "代理配置超过 16 MiB" }
        val digest = ProxyFiles.digest(bytes)
        if (stagedDigest != digest) {
            runtime.importConfig(bytes).getOrThrow()
            stagedDigest = digest
        }
        stagedText = text
        configurationAvailable = true
        configurationFailure = null
        return true
    }

    private suspend fun stageAssets() {
        var total = 0L
        for (path in MihomoConfigAssets.localProviders(checkNotNull(stagedText))) {
            val bytes = workspace.readAsset(path) // No cache fallback for missing/failed canonical assets.
            require(bytes.size <= LocalProxyConfig.MAX_CONFIG_BYTES) { "File provider 超过 16 MiB" }
            total += bytes.size
            require(total <= 64L * 1024 * 1024) { "File provider 总计超过 64 MiB" }
            withContext(Dispatchers.IO) {
                val root = runtime.configFile.parentFile.canonicalFile
                val target = File(root, path)
                check(target.canonicalPath.startsWith(root.path + File.separator)) { "File provider 超出临时目录" }
                check(target.parentFile.mkdirs() || target.parentFile.isDirectory) { "无法创建 File provider 临时目录" }
                val temporary = File(target.parentFile, ".provider-${UUID.randomUUID()}")
                try {
                    temporary.outputStream().use { it.write(bytes); it.fd.sync() }
                    check(temporary.renameTo(target)) { "无法暂存 File provider" }
                } finally { temporary.delete() }
            }
        }
    }

    private suspend fun <T> operation(block: suspend () -> T): Result<T> = withContext(Dispatchers.IO) {
        try { Result.success(configuration.withLock { block() }) }
        catch (cancelled: CancellationException) { throw cancelled }
        catch (error: Exception) {
            try { publish() } catch (cancelled: CancellationException) { throw cancelled } catch (_: Exception) { unavailable() }
            Result.failure(error)
        }
    }

    override suspend fun refresh(): Result<ProxyState> = operation {
        syncConfig()
        runtime.refresh().getOrThrow()
        publish()
    }

    override suspend fun ensureConfig(): Result<File> = operation {
        check(syncConfig(create = true)) { "工作区没有返回代理配置" }
        runtime.refresh().getOrThrow()
        publish()
        runtime.configFile // A staging location, never a canonical workspace path.
    }

    override suspend fun importConfig(bytes: ByteArray): Result<Unit> = operation {
        require(bytes.isNotEmpty() && bytes.size <= LocalProxyConfig.MAX_CONFIG_BYTES) { "代理配置须为 1 字节至 16 MiB" }
        // Reject lossy UTF-8 decoding rather than saving altered credentials or YAML.
        val text = Charsets.UTF_8.newDecoder().decode(java.nio.ByteBuffer.wrap(bytes)).toString()
        val previous = workspace.readConfig()
        workspace.writeConfig(text, previous.revision)
        check(syncConfig()) { "工作区没有返回代理配置" }
        runtime.refresh().getOrThrow()
        publish()
    }

    override suspend fun checkConfig(): Result<ProxyConfigCheck> = operation {
        check(syncConfig()) { "还没有代理配置" }
        stageAssets()
        runtime.checkConfig().getOrThrow()
    }

    override suspend fun start(): Result<Unit> = operation {
        check(syncConfig()) { "还没有代理配置" }
        stageAssets()
        publish() // An unavailable Engine must fail before any root prompt or process launch.
        val result = runtime.start()
        publish()
        result.getOrThrow()
    }

    override suspend fun stop(): Result<Unit> = operation {
        // Stopping an owned process remains possible even if the Engine connection has just failed.
        val result = runtime.stop()
        publish()
        result.getOrThrow()
    }

    override suspend fun refreshProviders() = operation {
        check(syncConfig()) { "还没有代理配置" }
        stageAssets()
        val result = runtime.refreshProviders().getOrThrow()
        publish()
        result
    }

    override suspend fun logTail(maxBytes: Int): String = withContext(Dispatchers.IO) {
        val text = runtime.logTail(maxBytes)
        workspace.writeLog(text)
        text
    }
}
