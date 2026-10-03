package top.flysoftbeta.workflow.app

import android.content.Context
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import top.flysoftbeta.workflow.app.panel.PanelRegistry
import top.flysoftbeta.workflow.app.panel.PanelWiring
import top.flysoftbeta.workflow.core.store.WorkspaceStore
import top.flysoftbeta.workflow.platform.agent.AgentHub
import top.flysoftbeta.workflow.platform.agent.AgentEnvironmentWiring
import top.flysoftbeta.workflow.platform.agent.ClaudeCodeInstaller
import top.flysoftbeta.workflow.platform.engine.EngineController
import java.io.File
import java.time.ZoneId

/**
 * Process-scoped owners (docs/architecture.md §1). ViewModels read from here and hold no
 * durable state. Workspace data is supplied exclusively by the selected Engine connection.
 */
object AppGraph {
    @Volatile private var connectionToken: String? = null
    @Synchronized fun useConnection(token: String?) {
        if (connectionToken == token) return
        connectionToken = token
        panels = null; hub = null; engine = null; claude = null
    }
    private fun workspaceScope(context: Context) =
        top.flysoftbeta.workflow.platform.connection.WorkspaceConnectionManager.get(context).requireSession().scope

    /** Lives as long as the process; failures in one child do not cancel the others. */
    val processScope: CoroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    @Volatile private var panels: PanelRegistry? = null
    @Volatile private var hub: AgentHub? = null
    @Volatile private var engine: EngineController? = null
    @Volatile private var claude: ClaudeCodeInstaller? = null

    fun workspaceRoot(context: Context): File = top.flysoftbeta.workflow.platform.connection.WorkspaceConnectionManager.get(context).requireSession().root

    /** The zone used for timeline buckets ("今天 / 昨天 / 本周 / 更早"). */
    fun zone(): ZoneId = ZoneId.systemDefault()

    /**
     * The workspace store, started on first use. Call `flush()` from `onStop` so pending drafts
     * are written before the process may be killed.
     */
    fun workspaceStore(context: Context): WorkspaceStore =
        top.flysoftbeta.workflow.platform.connection.WorkspaceConnectionManager.get(context).requireSession().store

    /** Panel providers and region slots (docs/implementation/android-client.md), built once. */
    fun panelRegistry(context: Context): PanelRegistry = panels ?: synchronized(this) {
        panels ?: PanelWiring.create(context.applicationContext).also { panels = it }
    }

    /**
     * The agent host (docs/agents.md §2): backends, their shared state and the conversation index.
     * Both backends run inside the environment once it is usable ([AgentEnvironmentWiring]).
     */
    fun agentHub(context: Context): AgentHub = hub ?: synchronized(this) {
        hub ?: run {
            val app = context.applicationContext
            AgentHub(app, workspaceStore(app), workspaceScope(app), Dispatchers.IO).also { created ->
                created.start()
                hub = created
                workspaceScope(app).launch {
                    AgentEnvironmentWiring.run(created, engine(app), claudeInstaller(app), workspaceStore(app))
                }
            }
        }
    }

    /** The hub if something already used it (lifecycle flushes must not start it). */
    fun agentHubIfStarted(): AgentHub? = hub

    /**
     * The workspace environment (docs/environment.md, platform.engine): install, reconcile, instances.
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
     * The Mihomo proxy (docs/proxy.md): one kernel per process, started only by an explicit user action.
     * Creating it touches neither root nor the network.
     */
    fun proxy(context: Context): top.flysoftbeta.workflow.proxy.runtime.ProxyApi =
        top.flysoftbeta.workflow.platform.proxy.ProxyService.get(context)

    /** Claude Code inside the environment, installed on demand. */
    fun claudeInstaller(context: Context): ClaudeCodeInstaller = claude ?: synchronized(this) {
        claude ?: ClaudeCodeInstaller(engine(context), workspaceScope(context)).also { claude = it }
    }

}
