package top.flysoftbeta.workflow.platform.engine

import android.content.Context
import java.io.File
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import top.flysoftbeta.workflow.core.connection.CapabilityBlockedException
import top.flysoftbeta.workflow.core.connection.RemoteWorkspaceStore
import top.flysoftbeta.workflow.core.connection.WorkspaceClosedException
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
    /** Status could not be read for a while; the last known usability still applies. */
    data class Unknown(val wasUsable: Boolean) : EnvironmentHealth { override val usable get() = wasUsable }
    data class Installing(val stage: Stage, val step: Int, val steps: Int, val progress: Float?) : EnvironmentHealth
    data class Provisioning(val step: String, val index: Int, val steps: Int, val progress: Float?) : EnvironmentHealth
    data class Ready(val applying: EnvironmentStatus.Applying? = null) : EnvironmentHealth { override val usable get() = true }
    data class NeedsRestart(val reason: ActivationReason) : EnvironmentHealth { override val usable get() = true }
    data class Failed(val stage: String, val message: String, val log: File?, val environmentAvailable: Boolean) : EnvironmentHealth {
        override val usable get() = environmentAvailable
    }
}
class EnvironmentUnavailableException(val health: EnvironmentHealth, message: String) : CapabilityBlockedException(message)
/** Read-only health projection and commands; generation/lifecycle/IO are entirely server-owned. */
class EngineController(context: Context, val scope: CoroutineScope, private val store: () -> WorkspaceStore) {
    val appContext = context.applicationContext
    val rpc: WorkspaceRpc get() = (store() as? RemoteWorkspaceStore)?.rpc ?: throw WorkspaceClosedException("请连接工作区 Engine")
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
            try { store().awaitReady() }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { mutable.value = EnvironmentHealth.Unavailable(error.message ?: "环境不可用"); return@launch }
            // A slow or failed status read never freezes the projection: keep polling with backoff until the
            // connection retires, and say the state is unknown only after repeated failures.
            var failures = 0
            while (isActive) {
                try { refresh(); failures = 0; delay(750) }
                catch (cancelled: CancellationException) { throw cancelled }
                catch (closed: WorkspaceClosedException) { mutable.value = EnvironmentHealth.Unavailable(closed.message ?: "工作区连接已断开"); return@launch }
                catch (_: Exception) {
                    failures++
                    if (failures >= UNKNOWN_AFTER) mutable.value = EnvironmentHealth.Unknown(mutable.value.usable)
                    delay(pollBackoff(failures))
                }
            }
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
    /**
     * Waits until the environment is usable. [waitThroughFailure] keeps waiting while it is failed or unavailable,
     * so the caller resumes after the user retries the environment instead of failing.
     */
    suspend fun awaitUsable(waitThroughFailure: Boolean = false) {
        start()
        // A terminal/user action enrolls an unused environment; attaching to a ready one is read-only.
        store().awaitReady()
        refresh()
        if (health.value == EnvironmentHealth.NotInstalled) {
            rpc.request("environment.reconcile")
            refresh()
        }
        val state = health.first {
            it.usable || (!waitThroughFailure && (it is EnvironmentHealth.Unavailable || it is EnvironmentHealth.Failed))
        }
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
    companion object {
        /** Consecutive failed status reads before the environment is shown as being checked. */
        const val UNKNOWN_AFTER = 2
        /** Delay before the next status read after [failures] consecutive failures. */
        fun pollBackoff(failures: Int): Long = (750L shl (failures - 1).coerceIn(0, 4)).coerceAtMost(10_000)
    }

    fun logTail(file: File?, lines: Int = 30): String = lastLog.lineSequence().takeLastLines(lines)
    private fun Sequence<String>.takeLastLines(lines: Int) = toList().takeLast(lines).joinToString("\n")
    fun describe(state: EnvironmentHealth): String = when (state) {
        EnvironmentHealth.NotInstalled -> "正在准备环境"
        is EnvironmentHealth.Unavailable -> state.reason
        is EnvironmentHealth.Unknown -> "正在检查环境"
        is EnvironmentHealth.Installing, is EnvironmentHealth.Provisioning -> "正在准备环境"
        is EnvironmentHealth.Ready -> if (state.applying == null) "环境已就绪" else "正在应用环境配置"
        is EnvironmentHealth.NeedsRestart -> "环境配置已更改"
        is EnvironmentHealth.Failed -> state.message
    }
}
