package top.flysoftbeta.workflow.platform.engine

import android.content.Context
import java.io.File
import java.io.IOException
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import top.flysoftbeta.workflow.core.connection.RemoteWorkspaceStore
import top.flysoftbeta.workflow.core.connection.WorkspaceRpc
import top.flysoftbeta.workflow.core.connection.WorkspaceWire
import top.flysoftbeta.workflow.core.environment.ActivationReason
import top.flysoftbeta.workflow.core.environment.EnvironmentStatus
import top.flysoftbeta.workflow.core.environment.Stage
import top.flysoftbeta.workflow.core.store.WorkspaceStore

sealed interface EnvironmentHealth {
    val usable: Boolean get() = false
    data object NotInstalled : EnvironmentHealth
    data class Unavailable(val reason: String) : EnvironmentHealth
    data class Installing(val stage: Stage, val step: Int, val steps: Int, val progress: Float?) : EnvironmentHealth
    data class Provisioning(val step: String, val index: Int, val steps: Int, val progress: Float?) : EnvironmentHealth
    data class Ready(val applying: EnvironmentStatus.Applying? = null) : EnvironmentHealth { override val usable get() = true }
    data class NeedsRestart(val reason: ActivationReason) : EnvironmentHealth { override val usable get() = true }
    data class Failed(val stage: String, val message: String, val log: File?, val environmentAvailable: Boolean) : EnvironmentHealth {
        override val usable get() = environmentAvailable
    }
}
class EnvironmentUnavailableException(val health: EnvironmentHealth, message: String) : IOException(message)
/** Read-only health projection and commands; generation/lifecycle/IO are entirely server-owned. */
class EngineController(context: Context, val scope: CoroutineScope, private val store: () -> WorkspaceStore) {
    val appContext = context.applicationContext
    val rpc: WorkspaceRpc get() = (store() as? RemoteWorkspaceStore)?.rpc ?: throw IOException("请连接工作区 Engine")
    private val mutable = MutableStateFlow<EnvironmentHealth>(EnvironmentHealth.NotInstalled)
    val health = mutable.asStateFlow()
    private val count = MutableStateFlow(0)
    val runningProcesses = count.asStateFlow()
    @Volatile var architecture: String? = null
        private set
    private var job: Job? = null
    private var lastLog = ""
    private val restartLock = Mutex()

    @Synchronized fun start() {
        if (job != null) return
        job = scope.launch {
            try {
                store().awaitReady()
                refresh()
                if (health.value == EnvironmentHealth.NotInstalled) rpc.request("environment.reconcile")
                while (isActive) { refresh(); delay(750) }
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { mutable.value = EnvironmentHealth.Unavailable(error.message ?: "环境不可用") }
        }
    }
    private suspend fun refresh() {
        val value = WorkspaceWire.obj(rpc.request("environment.status"))
        architecture = value["architecture"] as? String
        count.value = (value["runningProcesses"] as? Number)?.toInt() ?: 0
        lastLog = value["logTail"] as? String ?: value["error"] as? String ?: ""
        val progress = (value["progress"] as? Number)?.toFloat()?.takeIf { it.isFinite() }?.coerceIn(0f, 1f)
        val step = (value["step"] as? Number)?.toInt()?.coerceAtLeast(0) ?: 0
        val steps = (value["steps"] as? Number)?.toInt()?.coerceAtLeast(1) ?: 1
        val usable = value["usable"] == true
        mutable.value = when (value["phase"]) {
            "not_installed" -> EnvironmentHealth.NotInstalled
            "installing" -> if (usable) EnvironmentHealth.Ready(EnvironmentStatus.Applying(Stage.INSTALL, step, steps))
                else EnvironmentHealth.Installing(Stage.INSTALL, step, steps, progress)
            "building" -> if (usable) EnvironmentHealth.Ready(EnvironmentStatus.Applying(Stage.VERIFY, step, steps))
                else EnvironmentHealth.Provisioning("正在准备环境", step, steps, progress)
            "ready" -> if (usable) EnvironmentHealth.Ready() else EnvironmentHealth.Unavailable("环境尚未就绪")
            "needs_restart" -> EnvironmentHealth.NeedsRestart(ActivationReason.CONFIG)
            "failed" -> EnvironmentHealth.Failed(value["stage"] as? String ?: "build", value["error"] as? String ?: "环境构建失败", null, usable)
            else -> EnvironmentHealth.Unavailable("工作区返回了未知环境状态")
        }
    }
    suspend fun awaitUsable() {
        start()
        val state = health.first { it.usable || it is EnvironmentHealth.Unavailable || it is EnvironmentHealth.Failed }
        if (!state.usable) throw EnvironmentUnavailableException(state, describe(state))
    }
    fun retry() { scope.launch {
        try { rpc.request("environment.reconcile", mapOf("retry" to true)); refresh() }
        catch (e: Exception) { if (e is CancellationException) throw e; mutable.value = EnvironmentHealth.Failed("retry", e.message ?: "重试失败", null, health.value.usable) }
    } }
    suspend fun restartEnvironment() = restartLock.withLock {
        try {
            rpc.request("environment.restart", timeoutMs = 60_000)
        } finally {
            // Engine decides whether activation and resource restoration are necessary.
            if (currentCoroutineContext().isActive) {
                refresh()
            }
        }
        Unit
    }
    fun logTail(file: File?, lines: Int = 30): String = lastLog.lineSequence().takeLastLines(lines)
    private fun Sequence<String>.takeLastLines(lines: Int) = toList().takeLast(lines).joinToString("\n")
    fun describe(state: EnvironmentHealth): String = when (state) {
        EnvironmentHealth.NotInstalled -> "正在准备环境"
        is EnvironmentHealth.Unavailable -> state.reason
        is EnvironmentHealth.Installing, is EnvironmentHealth.Provisioning -> "正在准备环境"
        is EnvironmentHealth.Ready -> if (state.applying == null) "环境已就绪" else "正在应用环境配置"
        is EnvironmentHealth.NeedsRestart -> "环境配置已更改"
        is EnvironmentHealth.Failed -> state.message
    }
}
