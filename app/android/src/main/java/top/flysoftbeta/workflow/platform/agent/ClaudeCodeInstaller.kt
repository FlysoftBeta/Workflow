package top.flysoftbeta.workflow.platform.agent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*
import top.flysoftbeta.workflow.core.connection.WorkspaceWire
import top.flysoftbeta.workflow.platform.engine.EngineController

/** Disposable projection of the optional Engine tool; the client knows no executable or release. */
class ClaudeCodeInstaller(private val controller: EngineController, private val scope: CoroutineScope) {
    sealed interface State {
        data object NotInstalled : State
        data class Installing(val progress: Float?) : State
        data object Installed : State
        data class Failed(val message: String) : State
    }
    private val mutable = MutableStateFlow<State>(State.NotInstalled)
    val state = mutable.asStateFlow()
    private var installation: Job? = null

    init {
        scope.launch {
            while (isActive) {
                try { refresh() }
                catch (cancelled: CancellationException) { throw cancelled }
                catch (_: Exception) { mutable.value = State.Failed("无法读取工作区工具状态") }
                delay(1000)
            }
        }
    }

    suspend fun refresh(): State {
        val response = WorkspaceWire.obj(controller.rpc.request("environment.tools.status"))
        val tool = (response["tools"] as? List<*>)?.map { WorkspaceWire.obj(it) }?.firstOrNull { it["id"] == "claude" }
        mutable.value = when (tool?.get("phase")) {
            "ready" -> State.Installed
            "installing", "verifying" -> State.Installing((tool["progress"] as? Number)?.toFloat()?.takeIf { it.isFinite() }?.coerceIn(0f, 1f))
            "failed" -> State.Failed(tool["error"] as? String ?: "Claude Code 安装失败")
            "needs_restart" -> State.Failed("工具已准备，请重启环境后使用")
            "not_installed", null -> State.NotInstalled
            else -> State.Failed("工作区返回了未知工具状态")
        }
        return state.value
    }

    @Synchronized fun install() {
        if (installation?.isActive == true) return
        val retry = state.value is State.Failed
        installation = scope.launch {
            try {
                controller.rpc.request("environment.tools.install", mapOf("toolId" to "claude", "retry" to retry), 660_000)
                refresh()
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { mutable.value = State.Failed(error.message ?: "Claude Code 安装失败") }
        }
    }
}
