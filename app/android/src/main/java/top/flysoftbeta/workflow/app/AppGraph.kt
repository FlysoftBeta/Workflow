package top.flysoftbeta.workflow.app

import android.content.Context
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import top.flysoftbeta.workflow.app.panel.PanelRegistry
import top.flysoftbeta.workflow.app.panel.PanelWiring
import top.flysoftbeta.workflow.core.store.WorkspaceStore
import top.flysoftbeta.workflow.platform.UnhandledFailures
import top.flysoftbeta.workflow.platform.agent.AgentHub
import top.flysoftbeta.workflow.platform.agent.ClaudeCodeInstaller
import top.flysoftbeta.workflow.platform.connection.WorkspaceConnectionManager
import top.flysoftbeta.workflow.platform.connection.WorkspaceConnectionSession
import top.flysoftbeta.workflow.platform.engine.EngineController
import java.time.ZoneId

/**
 * Connection-scoped service projections and Android adapters. ViewModels read from here and hold no
 * durable state. Workspace data is supplied exclusively by the selected Engine connection.
 */
object AppGraph {
    @Volatile private var bound: WorkspaceConnectionSession? = null
    /**
     * Binds the services below to [session]. A lost session stays bound while its Workbench is kept inert during
     * reconnection, so accessors never throw mid-transition; its services then fail as a closed connection.
     */
    @Synchronized fun useConnection(session: WorkspaceConnectionSession?) {
        if (bound?.token == session?.token) return
        bound = session
        panels = null; hub = null; engine = null; claude = null
    }
    /**
     * The bound session. A service running without the activity, such as the overlay, binds the live session on
     * first use; with no connection this is a closed-connection failure.
     */
    fun session(context: Context): WorkspaceConnectionSession = bound
        ?: WorkspaceConnectionManager.get(context).requireSession().also(::useConnection)
    private fun workspaceScope(context: Context) = session(context).scope

    /** Lives as long as the process; failures in one child do not cancel the others. */
    val processScope: CoroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.Default + UnhandledFailures.handler("process"))

    @Volatile private var panels: PanelRegistry? = null
    @Volatile private var hub: AgentHub? = null
    @Volatile private var engine: EngineController? = null
    @Volatile private var claude: ClaudeCodeInstaller? = null

    /** The zone used for timeline buckets ("今天 / 昨天 / 本周 / 更早"). */
    fun zone(): ZoneId = ZoneId.systemDefault()

    /**
     * The workspace store, started on first use. Call `flush()` from `onStop` so pending drafts
     * are written before the process may be killed.
     */
    fun workspaceStore(context: Context): WorkspaceStore = session(context).store

    /** Panel providers and region slots (docs/app/connection.md), built once. */
    fun panelRegistry(context: Context): PanelRegistry = panels ?: synchronized(this) {
        panels ?: PanelWiring.create(context.applicationContext).also { panels = it }
    }

    /**
     * The chat RPC client: disposable transcript and metadata projections.
     * The Engine owns backend processes and conversation lifecycle.
     */
    fun agentHub(context: Context): AgentHub = hub ?: synchronized(this) {
        hub ?: run {
            val app = context.applicationContext
            AgentHub(app, workspaceStore(app), workspaceScope(app), Dispatchers.IO).also { created ->
                created.start()
                hub = created
            }
        }
    }

    /** The hub if something already used it (lifecycle flushes must not start it). */
    fun agentHubIfStarted(): AgentHub? = hub

    /**
     * The workspace environment (docs/engine/environment.md, platform.engine): install, reconcile, instances.
     * Started on first use; MainActivity touches it so a fresh install begins right away.
     */
    fun engine(context: Context): EngineController = engine ?: synchronized(this) {
        engine ?: run {
            val app = context.applicationContext
            EngineController(app, workspaceScope(app)) { workspaceStore(app) }.also {
                engine = it
                it.start()
            }
        }
    }

    /**
     * The Mihomo proxy (docs/app/proxy.md): one kernel per process, started only by an explicit user action.
     * Creating it touches neither root nor the network.
     */
    fun proxy(context: Context): top.flysoftbeta.workflow.proxy.runtime.ProxyApi =
        top.flysoftbeta.workflow.platform.proxy.ProxyService.get(context)

    /** Engine-managed Claude tool status and install commands. */
    fun claudeInstaller(context: Context): ClaudeCodeInstaller = claude ?: synchronized(this) {
        claude ?: ClaudeCodeInstaller(engine(context), workspaceScope(context)).also { claude = it }
    }

}
