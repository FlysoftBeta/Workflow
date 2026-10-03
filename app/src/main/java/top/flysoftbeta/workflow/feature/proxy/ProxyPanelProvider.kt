package top.flysoftbeta.workflow.feature.proxy

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.launch
import top.flysoftbeta.workflow.app.AppGraph
import top.flysoftbeta.workflow.app.panel.DecisionOption
import top.flysoftbeta.workflow.app.panel.DecisionRequest
import top.flysoftbeta.workflow.app.panel.DecisionStyle
import top.flysoftbeta.workflow.app.panel.PanelContext
import top.flysoftbeta.workflow.app.panel.PanelController
import top.flysoftbeta.workflow.app.panel.PanelProvider
import top.flysoftbeta.workflow.core.layout.Panel
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.Placement
import top.flysoftbeta.workflow.core.layout.ProxyPage
import top.flysoftbeta.workflow.platform.proxy.ProxyService
import top.flysoftbeta.workflow.proxy.network.ProxyConflict
import top.flysoftbeta.workflow.proxy.runtime.ProxyApi
import top.flysoftbeta.workflow.proxy.runtime.ProxyConflictException
import top.flysoftbeta.workflow.proxy.runtime.ProxyState
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.icons.Sym

/**
 * The proxy app (docs/ui.md §4.12, docs/proxy.md): overview, logs and connections are three panels of
 * one kind. All of them talk to the process-wide [ProxyApi]; nothing here holds proxy state of its own.
 */
class ProxyPanelProvider internal constructor(private val apiFactory: () -> ProxyApi) : PanelProvider {
    constructor(context: Context) : this({ AppGraph.proxy(context.applicationContext) })

    override fun create(panel: Panel, context: PanelContext): PanelController {
        val api = apiFactory()
        return when ((panel.target as PanelTarget.Proxy).page) {
            ProxyPage.OVERVIEW -> ProxyOverviewController(api, context)
            ProxyPage.LOGS -> ProxyLogsController(api, context)
            ProxyPage.CONNECTIONS -> ProxyConnectionsController(api, context)
        }
    }
}

/** Actions every proxy panel shares (start/stop, edit config, open a sibling page). Main thread. */
internal abstract class ProxyPanelBase(protected val api: ProxyApi, protected val context: PanelContext) : PanelController {
    /** Snapshot copy of [ProxyApi.state] for composition and the tool row. */
    protected var state by mutableStateOf(api.state.value)
        private set

    init {
        context.scope.launch { api.state.collect { state = it } }
    }

    /** ⏻: start when stopped, stop when running (or when a stop is still unconfirmed). */
    fun toggle() {
        val current = api.state.value
        if (current.busy) return
        context.scope.launch {
            if (current.running || current.stopUnconfirmed) api.stop() else start()
        }
    }

    protected suspend fun start() {
        val error = api.start().exceptionOrNull()
        if (error is ProxyConflictException) explainConflict(error.conflict)
        // Other failures are in state.error, shown where the toggle is.
    }

    /** Refuse to take over an existing VPN/TUN. */
    suspend fun explainConflict(conflict: ProxyConflict = api.state.value.conflict) {
        val message = conflict.describe().ifEmpty { "检测到其他 VPN 或 TUN" } +
            "。Workflow 不会接管其他代理。请先停止其他 VPN/TUN，或在配置中关闭 TUN；网卡名或路由规则冲突时请调整配置。"
        val title = if (conflict.vpnActive || conflict.foreignInterfaces.isNotEmpty()) "另一个 VPN 正在运行" else "TUN 设置冲突"
        val options = listOf(DecisionOption("edit", "编辑配置", DecisionStyle.Text))
        if (context.commands.decide(DecisionRequest(title, message, conflict.foreignInterfaces + conflict.ruleCollisions, options)) == "edit") editConfig()
    }

    /** Opens config.yaml in the editor, creating the safe template on first use. */
    fun editConfig() {
        context.scope.launch {
            api.ensureConfig()
                .onSuccess { context.commands.openFile(ProxyService.CONFIG_PATH) }
                .onFailure { context.commands.snackbar(it.message ?: "无法创建代理配置") }
        }
    }

    /** Logs and connections open as panels of the same stack (docs/ui.md §4.12). */
    fun openPage(page: ProxyPage) {
        val stack = context.store.state.value.session(context.sessionId)?.workbench?.stackOf(context.panelId)
        context.commands.open(PanelTarget.Proxy(page), stack?.let { Placement.InStack(it) } ?: Placement.Auto)
    }

    protected fun report(result: Result<*>) {
        result.exceptionOrNull()?.let { context.commands.snackbar(it.message ?: "代理操作失败") }
    }

    protected fun pageEntries(except: ProxyPage): List<MenuEntry> = buildList {
        if (except != ProxyPage.OVERVIEW) add(MenuEntry.Action("overview", "代理", Sym.Shield) { openPage(ProxyPage.OVERVIEW) })
        if (except != ProxyPage.LOGS) add(MenuEntry.Action("logs", "日志", Sym.ReceiptLong) { openPage(ProxyPage.LOGS) })
        if (except != ProxyPage.CONNECTIONS) add(MenuEntry.Action("connections", "连接", Sym.Lan) { openPage(ProxyPage.CONNECTIONS) })
    }

    protected val ProxyState.toggleEnabled: Boolean
        get() = !busy && (running || stopUnconfirmed || canStart)
}
