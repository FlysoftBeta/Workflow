package top.flysoftbeta.workflow

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.*
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import top.flysoftbeta.workflow.app.AppGraph
import top.flysoftbeta.workflow.app.ShellViewModel
import top.flysoftbeta.workflow.app.Space
import top.flysoftbeta.workflow.app.WorkflowApp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.platform.connection.*
import top.flysoftbeta.workflow.platform.importer.ImportService

/** Local shell: explicit workspace connection precedes all Workbench/service construction. */
class MainActivity : ComponentActivity() {
    private var model: ShellViewModel? = null
    private var boundToken: String? = null
    private var maintenance: Job? = null
    private var overlayRestore: Job? = null
    private var savedSpace: String? = null
    private var savedConnection: String? = null
    private var foreground = false

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        savedSpace = savedInstanceState?.getString(SPACE)
        savedConnection = savedInstanceState?.getString(CONNECTION)
        ImportService.get(this).attach(this)
        val manager = WorkspaceConnectionManager.get(this)
        setContent {
            val status by manager.status.collectAsState()
            val connected = status as? ConnectionStatus.Connected
            if (connected == null) ConnectionScreen(manager, status)
            else key(connected.session.token) {
                val viewModel = remember(connected.session.token) { bind(connected.session) }
                DisposableEffect(connected.session.token) {
                    onDispose {
                        if (boundToken == connected.session.token && manager.sessionOrNull()?.token != boundToken) {
                            maintenance?.cancel(); overlayRestore?.cancel()
                            viewModelStore.clear()
                            model = null; boundToken = null
                            AppGraph.useConnection(null)
                        }
                    }
                }
                if (viewModel != null) WorkflowApp(viewModel) else ConnectionScreen(manager, status)
            }
        }
    }

    private fun bind(session: WorkspaceConnectionSession): ShellViewModel? {
        val manager = WorkspaceConnectionManager.get(this)
        if (manager.sessionOrNull()?.token != session.token) return null
        if (boundToken != session.token) {
            maintenance?.cancel(); overlayRestore?.cancel()
            viewModelStore.clear()
            model = null
            AppGraph.useConnection(session.token)
            boundToken = session.token
            model = try { ViewModelProvider(this)[ShellViewModel::class.java] }
            catch (error: Exception) {
                // The Engine may disconnect on its IO thread while this frame constructs services.
                if (manager.sessionOrNull()?.token == session.token) throw error
                viewModelStore.clear(); boundToken = null; AppGraph.useConnection(null)
                return null
            }
            val restoring = savedConnection == session.profile.id && savedSpace != null
            if (restoring) {
                savedSpace?.let { name -> Space.entries.firstOrNull { it.name == name } }?.let(model!!.shell::restoreSpace)
            }
            savedSpace = null; savedConnection = null
            if (intent.isHome() && !restoring) model!!.shell.goHome()
            openDestination(intent)
            if (foreground) startForegroundWork()
        }
        return checkNotNull(model)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        if (WorkspaceConnectionManager.get(this).sessionOrNull()?.token == boundToken) {
            model?.let { if (intent.isHome()) { it.dnd.cancel(); it.shell.goHome() } }
            if (model != null) openDestination(intent)
        }
    }

    private fun openDestination(intent: Intent?) {
        val current = model ?: return
        when (intent?.getStringExtra(EXTRA_DESTINATION)?.uppercase()) {
            "LAUNCHER", "APPS" -> current.shell.goHome()
            "WORKBENCH" -> current.shell.enterWorkbench()
            "PROXY" -> current.shell.openInCurrentSession(PanelTarget.Proxy())
            "SETTINGS" -> current.shell.openInCurrentSession(PanelTarget.Settings)
        }
        intent?.removeExtra(EXTRA_DESTINATION)
    }

    override fun onStart() { super.onStart(); foreground = true; startForegroundWork() }
    private fun startForegroundWork() {
        val current = model ?: return
        maintenance?.cancel(); maintenance = current.maintenanceLoop()
        overlayRestore?.cancel()
        overlayRestore = lifecycleScope.launch {
            val ready = current.store.awaitReady()
            if (ready.status == top.flysoftbeta.workflow.core.store.StoreStatus.READY &&
                WorkspaceConnectionManager.get(this@MainActivity).sessionOrNull()?.token == boundToken) {
                top.flysoftbeta.workflow.platform.WorkflowOverlayService.restoreIfEnabled(this@MainActivity)
            }
        }
    }
    override fun onStop() {
        foreground = false
        overlayRestore?.cancel(); overlayRestore = null
        maintenance?.cancel(); maintenance = null
        model?.flush()
        super.onStop()
    }
    override fun onSaveInstanceState(outState: Bundle) {
        model?.let { outState.putString(SPACE, it.shell.space.name) }
        WorkspaceConnectionManager.get(this).sessionOrNull()?.let { outState.putString(CONNECTION, it.profile.id) }
        super.onSaveInstanceState(outState)
    }
    private fun Intent?.isHome() = this?.hasCategory(Intent.CATEGORY_HOME) == true
    companion object {
        private const val SPACE = "workflow.space"
        private const val CONNECTION = "workflow.connection"
        const val EXTRA_DESTINATION = "destination"
    }
}
