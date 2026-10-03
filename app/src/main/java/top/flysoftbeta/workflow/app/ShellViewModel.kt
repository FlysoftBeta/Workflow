package top.flysoftbeta.workflow.app

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import top.flysoftbeta.workflow.feature.workbench.WorkbenchRuntime
import top.flysoftbeta.workflow.ui.design.dnd.DragDropState

/**
 * Activity-retained owner of the shell: navigation state, the live panel controllers and the DnD
 * state. Durable state is only in the process-wide WorkspaceStore (AppGraph).
 */
class ShellViewModel(application: Application) : AndroidViewModel(application) {
    val store = AppGraph.workspaceStore(application)
    val registry = AppGraph.panelRegistry(application)
    val shell = Shell(application, store, registry, viewModelScope)
    val workbench = WorkbenchRuntime(application, store, registry, shell)
    val dnd = DragDropState()

    /** Auto-maintenance while the app is in the foreground: on resume and hourly (workspace.md §5). */
    fun maintenanceLoop() = viewModelScope.launch {
        while (isActive) {
            runCatching { store.runMaintenance() }
            delay(60 * 60 * 1000L)
        }
    }

    /** Flush pending drafts and layout when the app leaves the foreground. */
    fun flush() {
        AppGraph.processScope.launch { runCatching { store.flush() } }
        AppGraph.agentHubIfStarted()?.let { hub -> AppGraph.processScope.launch { runCatching { hub.flush() } } }
    }

    override fun onCleared() {
        workbench.dispose()
    }
}
