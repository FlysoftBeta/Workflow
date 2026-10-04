package top.flysoftbeta.workflow.app

import android.content.Context
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.SnackbarResult
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.plus
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import top.flysoftbeta.workflow.platform.UnhandledFailures
import top.flysoftbeta.workflow.core.config.Appearance
import top.flysoftbeta.workflow.app.panel.DecisionRequest
import top.flysoftbeta.workflow.app.panel.PanelRegistry
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.resource.ResourceRef
import top.flysoftbeta.workflow.core.session.ArchiveDecision
import top.flysoftbeta.workflow.core.session.SessionId
import top.flysoftbeta.workflow.core.store.ArchiveOutcome
import top.flysoftbeta.workflow.core.store.WorkspaceStore

/** The two spaces of the app (product.md §2), like a desktop and the app running on it. */
enum class Space { LAUNCHER, WORKBENCH }

/** A manual archive that needs the save / keep drafts / discard decision (ui.md §4.9). */
data class ArchiveRequest(val sessionId: SessionId, val title: String, val resources: List<String>, val problem: String? = null)

/** A pending [top.flysoftbeta.workflow.app.panel.WorkbenchCommands.decide] dialog. */
class PendingDecision(val request: DecisionRequest, val result: CompletableDeferred<String?>)

/**
 * The navigation contract shared by the features (architecture.md §1 rule 3): which space is shown,
 * the session view and its dialogs, snackbars and decision dialogs. Activity-retained (owned by
 * [ShellViewModel]); all durable state lives in the [store].
 */
@Stable
class Shell(
    val appContext: Context,
    val store: WorkspaceStore,
    val registry: PanelRegistry,
    baseScope: CoroutineScope,
) {
    /** Shell commands report a failure nothing handled instead of terminating the process. */
    private val scope = baseScope + UnhandledFailures.handler("shell") { showSnackbar(it.summary) }
    var space by mutableStateOf(Space.LAUNCHER)
        private set

    /** The session view (side sheet). */
    var sessionsOpen by mutableStateOf(false)

    /** Session whose name is being edited (命名 / 重命名). */
    var renaming by mutableStateOf<SessionId?>(null)

    var archiving by mutableStateOf<ArchiveRequest?>(null)
        private set

    var decision by mutableStateOf<PendingDecision?>(null)
        private set

    val snackbar = SnackbarHostState()

    /** Theme, density and font scale from config.json (the root theme follows it live). */
    val appearance: StateFlow<Appearance> = store.state.map { it.config.appearance }.distinctUntilChanged()
        .stateIn(scope, SharingStarted.Eagerly, store.state.value.config.appearance)

    // ---- Spaces ----

    /** ⌂ / system Home: back to the Launcher; the Workbench stays exactly as it is. */
    fun goHome() {
        sessionsOpen = false
        space = Space.LAUNCHER
    }

    /** The Workbench tile: the active session, else the most recent, else a new one. */
    fun enterWorkbench() = scope.launch {
        store.enterWorkbench()
        space = Space.WORKBENCH
    }

    /** Proxy / Settings tile: a panel in the current session (focused when already open). */
    fun openInCurrentSession(target: PanelTarget) = scope.launch {
        val id = store.enterWorkbench()
        store.applyLayout(id, LayoutOp.Open(target))
        space = Space.WORKBENCH
    }

    /** "在单独的会话中打开": a new temporary Solo session with only [target]. */
    fun openInSeparateSession(target: PanelTarget) = scope.launch {
        store.openInSeparateSession(target)
        space = Space.WORKBENCH
    }

    // ---- Sessions ----

    fun newSession() = scope.launch {
        store.createSession()
        sessionsOpen = false
        space = Space.WORKBENCH
    }

    fun switchTo(id: SessionId) = scope.launch {
        store.activateSession(id)
        sessionsOpen = false
        space = Space.WORKBENCH
    }

    fun rename(id: SessionId, name: String) = scope.launch {
        if (name.isNotBlank()) store.renameSession(id, name.trim())
    }

    /** Manual archive: directly when nothing is unsaved (with undo), else ask ([archiving]). */
    fun archive(id: SessionId, decision: ArchiveDecision? = null) = scope.launch {
        val session = store.state.value.session(id) ?: return@launch
        val wasActive = store.state.value.activeSessionId == id
        when (val outcome = store.archiveSession(id, decision)) {
            is ArchiveOutcome.Archived -> {
                archiving = null
                if (wasActive && space == Space.WORKBENCH) store.enterWorkbench()
                showSnackbar("已归档", "撤销") { scope.launch { store.restoreSession(id) } }
            }
            is ArchiveOutcome.NeedsDecision -> archiving = ArchiveRequest(
                id, sessionLabel(session, java.time.ZoneId.systemDefault()),
                outcome.resources.map { resourceTitle(it) }.sorted(),
            )
            is ArchiveOutcome.SaveConflict -> archiving = archiving?.copy(problem = "${outcome.paths.size} 个文件在磁盘上已更改，未保存")
            is ArchiveOutcome.Invalid -> archiving = archiving?.copy(problem = "${outcome.path.substringAfterLast('/')}：${outcome.message}")
            is ArchiveOutcome.Failed -> archiving = archiving?.copy(problem = outcome.message)
            ArchiveOutcome.NotFound -> archiving = null
        }
    }

    fun cancelArchive() { archiving = null }

    fun resourceTitle(ref: ResourceRef): String = registry.titleOf(ref) ?: when (ref) {
        is ResourceRef.File -> ref.path.substringAfterLast('/')
        is ResourceRef.Conversation -> "对话"
    }

    // ---- Feedback ----

    fun showSnackbar(message: String, actionLabel: String? = null, onAction: (() -> Unit)? = null) {
        scope.launch {
            snackbar.currentSnackbarData?.dismiss()
            val result = snackbar.showSnackbar(message, actionLabel, duration = if (actionLabel != null) SnackbarDuration.Long else SnackbarDuration.Short)
            if (result == SnackbarResult.ActionPerformed) onAction?.invoke()
        }
    }

    suspend fun decide(request: DecisionRequest): String? {
        decision?.result?.complete(null)
        val pending = PendingDecision(request, CompletableDeferred())
        decision = pending
        return try { pending.result.await() } finally { if (decision === pending) decision = null }
    }

    fun answer(key: String?) {
        decision?.result?.complete(key)
        decision = null
    }

    /** Restores the space after the activity was recreated. */
    fun restoreSpace(value: Space) { space = value }
}
