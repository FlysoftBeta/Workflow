package top.flysoftbeta.workflow.feature.workbench

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.job
import kotlinx.coroutines.launch
import top.flysoftbeta.workflow.app.Shell
import top.flysoftbeta.workflow.app.panel.DecisionRequest
import top.flysoftbeta.workflow.app.panel.ExplorerContext
import top.flysoftbeta.workflow.app.panel.ExplorerController
import top.flysoftbeta.workflow.app.panel.NewPanelRequest
import top.flysoftbeta.workflow.app.panel.PanelContext
import top.flysoftbeta.workflow.app.panel.PanelController
import top.flysoftbeta.workflow.app.panel.PanelRegistry
import top.flysoftbeta.workflow.app.panel.RailContext
import top.flysoftbeta.workflow.app.panel.RailController
import top.flysoftbeta.workflow.app.panel.WorkbenchCommands
import top.flysoftbeta.workflow.app.panel.launchAction
import top.flysoftbeta.workflow.platform.UnhandledFailures
import top.flysoftbeta.workflow.core.layout.ExplorerView
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.Panel
import top.flysoftbeta.workflow.core.layout.PanelId
import top.flysoftbeta.workflow.core.layout.PanelKind
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.PanelView
import top.flysoftbeta.workflow.core.layout.Paradigm
import top.flysoftbeta.workflow.core.layout.Placement
import top.flysoftbeta.workflow.core.layout.Region
import top.flysoftbeta.workflow.core.layout.TextCursor
import top.flysoftbeta.workflow.core.layout.Workbench
import top.flysoftbeta.workflow.core.session.SessionId
import top.flysoftbeta.workflow.core.store.WorkspaceStore
import top.flysoftbeta.workflow.ui.design.dnd.FilesDragPayload

/**
 * Activity-retained owner of the live panel controllers (docs/app/connection.md). Only
 * the active session has a [SessionRuntime]; switching sessions disposes the previous one. Leaving to
 * the Launcher keeps it, so returning is instant and nothing reloads.
 */
class WorkbenchRuntime(
    private val appContext: Context,
    private val store: WorkspaceStore,
    private val registry: PanelRegistry,
    private val shell: Shell,
) {
    private var current: SessionRuntime? = null

    fun session(id: SessionId): SessionRuntime {
        current?.let { if (it.sessionId == id) return it; it.dispose() }
        return SessionRuntime(id, appContext, store, registry, shell).also { current = it }
    }

    fun dispose() {
        current?.dispose()
        current = null
    }
}

/**
 * Where the session's transient region state lives: overlay regions (not persisted, workspace.md §5)
 * and whether a region can currently dock (depends on the window width, known to the composition).
 */
class RegionPresence {
    /** Files explorer / Chat rail shown as an overlay drawer. */
    var sideOverlay by mutableStateOf(false)
    /** Files conversation region / Chat file side shown as an overlay sheet. */
    var auxOverlay by mutableStateOf(false)
    /** Last docking capability computed by the composition (window width dependent). */
    var sideDockable = false
    var auxDockable = false
}

/** Live state of one session: controllers of its panels, its explorer and rail, the commands. */
class SessionRuntime(
    val sessionId: SessionId,
    private val appContext: Context,
    private val store: WorkspaceStore,
    private val registry: PanelRegistry,
    private val shell: Shell,
) {
    /** Reports a failure nothing handled instead of letting it terminate the process (docs/app/workbench.md). */
    private val unhandled = UnhandledFailures.handler("workbench") { shell.showSnackbar(it.summary) }
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate + unhandled)
    val presence = RegionPresence()

    /** A seam or region edge is being dragged (PanelFrame.resizing). */
    var resizing by mutableStateOf(false)

    private class Entry(val key: String, val controller: PanelController, val scope: CoroutineScope)

    private val entries = HashMap<PanelId, Entry>()
    private var focused: PanelId? = null

    val commands: WorkbenchCommands = Commands()

    fun workbench(): Workbench? = store.state.value.session(sessionId)?.workbench

    fun layout(op: LayoutOp) = store.layout(sessionId, op)

    /** The controller of [panel], created on first use (and recreated when its target changed). */
    fun controller(panel: Panel): PanelController {
        entries[panel.id]?.let { entry ->
            if (entry.key == panel.target.key) return entry.controller
            disposeEntry(panel.id)
        }
        val childScope = CoroutineScope(SupervisorJob(scope.coroutineContext.job) + Dispatchers.Main.immediate + unhandled)
        val context = object : PanelContext {
            override val appContext = this@SessionRuntime.appContext
            override val store = this@SessionRuntime.store
            override val sessionId = this@SessionRuntime.sessionId
            override val panelId = panel.id
            override val target = panel.target
            override val initialView: PanelView = panel.view
            override val scope = childScope
            override val commands = this@SessionRuntime.commands
            override fun layout(op: LayoutOp) = this@SessionRuntime.layout(op)
        }
        val controller = registry.provider(panel.target.kind).create(panel, context)
        entries[panel.id] = Entry(panel.target.key, controller, childScope)
        return controller
    }

    fun existing(panelId: PanelId): PanelController? = entries[panelId]?.controller

    /** Disposes controllers of panels no longer in [workbench]. */
    fun prune(workbench: Workbench) {
        val stale = entries.filter { (id, entry) -> workbench.panels[id]?.target?.key != entry.key }.keys.toList()
        stale.forEach(::disposeEntry)
    }

    /** Focus hook: tells controllers when they gain or lose the session's focus. */
    fun setFocused(panelId: PanelId?) {
        if (focused == panelId) return
        focused?.let { entries[it]?.controller?.onFocusChanged(false) }
        focused = panelId
        panelId?.let { entries[it]?.controller?.onFocusChanged(true) }
    }

    /** Closes panels after each controller agreed ([PanelController.prepareClose]). */
    fun close(ids: List<PanelId>) {
        if (ids.isEmpty()) return
        scope.launch {
            val wb = workbench() ?: return@launch
            val allowed = ids.filter { id ->
                val panel = wb.panels[id] ?: return@filter false
                controller(panel).prepareClose()
            }
            if (allowed.isNotEmpty()) layout(LayoutOp.Close(allowed))
        }
    }

    // ---- Regions ----

    private var explorer: ExplorerController? = null
    private var rail: RailController? = null

    fun explorerController(): ExplorerController = explorer ?: createExplorer().also { explorer = it }

    fun railController(): RailController = rail ?: createRail().also { rail = it }

    private fun createExplorer(): ExplorerController {
        val explorerScope = CoroutineScope(SupervisorJob(scope.coroutineContext.job) + Dispatchers.Main.immediate + unhandled)
        val sessionFlow = store.state.map { it.session(sessionId)?.workbench }
        return registry.explorer.create(object : ExplorerContext {
            override val appContext = this@SessionRuntime.appContext
            override val store = this@SessionRuntime.store
            override val sessionId = this@SessionRuntime.sessionId
            override val scope = explorerScope
            override val commands = this@SessionRuntime.commands
            override fun layout(op: LayoutOp) = this@SessionRuntime.layout(op)
            override val view: StateFlow<ExplorerView> = sessionFlow.map { it?.explorer ?: ExplorerView() }.distinctUntilChanged()
                .stateIn(explorerScope, SharingStarted.Eagerly, workbench()?.explorer ?: ExplorerView())
            override val activeFile: StateFlow<String?> = sessionFlow.map { it?.let(::activeFilePath) }.distinctUntilChanged()
                .stateIn(explorerScope, SharingStarted.Eagerly, workbench()?.let(::activeFilePath))
        })
    }

    private fun createRail(): RailController {
        val railScope = CoroutineScope(SupervisorJob(scope.coroutineContext.job) + Dispatchers.Main.immediate + unhandled)
        return registry.rail.create(object : RailContext {
            override val appContext = this@SessionRuntime.appContext
            override val store = this@SessionRuntime.store
            override val sessionId = this@SessionRuntime.sessionId
            override val scope = railScope
            override val commands = this@SessionRuntime.commands
            override fun layout(op: LayoutOp) = this@SessionRuntime.layout(op)
            override val activeConversation: StateFlow<String?> = store.state
                .map { s -> (s.session(sessionId)?.workbench?.activePanel(Workbench.AUX)?.target as? PanelTarget.Conversation)?.conversationId }
                .distinctUntilChanged()
                .stateIn(railScope, SharingStarted.Eagerly, null)
        })
    }

    /**
     * Makes a region visible: expands it when it can dock, else opens it as an overlay. Used by
     * commands (reveal in tree, attach to conversation) and the corner toggles.
     */
    fun showRegion(region: Region) {
        val side = region == Region.EXPLORER || region == Region.CHAT_RAIL
        val dockable = if (side) presence.sideDockable else presence.auxDockable
        if (dockable) {
            if (side) presence.sideOverlay = false else presence.auxOverlay = false
            layout(LayoutOp.SetRegionCollapsed(region, false))
        }
        else if (side) presence.sideOverlay = true else presence.auxOverlay = true
    }

    fun beginCreateFile() {
        if (workbench()?.paradigm == Paradigm.CHAT) {
            layout(LayoutOp.SetRegionCollapsed(Region.CHAT_TREE, false))
            showRegion(Region.CHAT_SIDE)
        } else {
            if (workbench()?.paradigm == Paradigm.SOLO) layout(LayoutOp.SwitchParadigm(Paradigm.FILES))
            showRegion(Region.EXPLORER)
        }
        explorerController().beginCreate(false)
    }

    /** ◧ / ◨: toggles a docked region (persisted) or its overlay (transient). */
    fun toggleRegion(region: Region) {
        val side = region == Region.EXPLORER || region == Region.CHAT_RAIL
        val dockable = if (side) presence.sideDockable else presence.auxDockable
        if (dockable) {
            if (side) presence.sideOverlay = false else presence.auxOverlay = false
            layout(LayoutOp.ToggleRegion(region))
        } else if (side) {
            presence.sideOverlay = !presence.sideOverlay
        } else {
            presence.auxOverlay = !presence.auxOverlay
        }
    }

    fun dispose() {
        entries.keys.toList().forEach(::disposeEntry)
        explorer?.dispose()
        rail?.dispose()
        scope.cancel()
    }

    private fun disposeEntry(id: PanelId) {
        val entry = entries.remove(id) ?: return
        if (focused == id) focused = null
        runCatching { entry.controller.dispose() }
        entry.scope.cancel()
    }

    private inner class Commands : WorkbenchCommands {
        override fun open(target: PanelTarget, placement: Placement) {
            if (target is PanelTarget.File || target is PanelTarget.Image) revealEditorSide()
            layout(LayoutOp.Open(target, placement))
        }

        override fun openFile(path: String, cursor: TextCursor?) {
            revealEditorSide()
            val target = PanelTarget.forFile(path)
            launchReporting {
                val wb = store.applyLayout(sessionId, LayoutOp.Open(target)) ?: return@launchReporting
                val panel = wb.panelFor(target) ?: return@launchReporting
                if (cursor != null) {
                    layout(LayoutOp.UpdateView(panel.id, panel.view.copy(cursor = cursor)))
                    controller(panel).navigate(cursor)
                }
            }
        }

        override fun attachToConversation(paths: List<String>) = deliver(PanelKind.CONVERSATION, FilesDragPayload(paths))

        override fun pasteIntoTerminal(paths: List<String>) = deliver(PanelKind.TERMINAL, FilesDragPayload(paths))

        override fun newTerminal(directory: String?) {
            presence.sideOverlay = false
            launchReporting {
                val target = registry.provider(PanelKind.TERMINAL).newTarget(NewPanelRequest(directory = directory)) ?: return@launchReporting
                val wb = store.applyLayout(sessionId, LayoutOp.Open(target)) ?: return@launchReporting
                wb.panelFor(target)?.let { controller(it).requestInputFocus() }
            }
        }

        override fun newConversation() {
            launchReporting {
                val target = registry.provider(PanelKind.CONVERSATION).newTarget(NewPanelRequest()) ?: return@launchReporting
                showConversationRegion()
                store.applyLayout(sessionId, LayoutOp.showConversation((target as PanelTarget.Conversation).conversationId))
            }
        }

        override fun revealInExplorer(path: String) {
            val wb = workbench() ?: return
            if (wb.paradigm == Paradigm.CHAT) {
                layout(LayoutOp.SetRegionCollapsed(Region.CHAT_TREE, false))
                showRegion(Region.CHAT_SIDE)
            } else {
                if (wb.paradigm == Paradigm.SOLO) layout(LayoutOp.SwitchParadigm(Paradigm.FILES))
                showRegion(Region.EXPLORER)
            }
            explorerController().reveal(path)
        }

        override fun snackbar(message: String, actionLabel: String?, onAction: (() -> Unit)?) = shell.showSnackbar(message, actionLabel, onAction)

        override suspend fun decide(request: DecisionRequest): String? = shell.decide(request)

        /** A shell command has no panel to hold an error row, so its failure is reported in the Snackbar. */
        private fun launchReporting(block: suspend CoroutineScope.() -> Unit) {
            scope.launchAction({ shell.showSnackbar(it.summary) }, block)
        }

        /** In Chat, files open in the side region: make sure it is visible. */
        private fun revealEditorSide() {
            // A modal explorer must reveal the file the user just chose.
            presence.sideOverlay = false
            if (workbench()?.paradigm == Paradigm.CHAT) showRegion(Region.CHAT_SIDE)
        }

        private fun showConversationRegion() {
            when (workbench()?.paradigm) {
                Paradigm.FILES -> showRegion(Region.AUX)
                Paradigm.SOLO -> layout(LayoutOp.SwitchParadigm(Paradigm.FILES)).also { showRegion(Region.AUX) }
                else -> Unit
            }
        }

        /** Routes a payload to the most relevant panel of [kind], opening a new one when none exists. */
        private fun deliver(kind: PanelKind, payload: FilesDragPayload) {
            presence.sideOverlay = false
            val wb = workbench() ?: return
            val candidates = wb.panels.values.filter { it.target.kind == kind }
            val preferred = candidates.firstOrNull { it.id == wb.focusedPanel?.id }
                ?: candidates.firstOrNull { kind == PanelKind.CONVERSATION && wb.stacks[Workbench.AUX]?.active == it.id }
                ?: wb.mru.firstNotNullOfOrNull { id -> candidates.firstOrNull { it.id == id } }
                ?: candidates.firstOrNull()
            if (kind == PanelKind.CONVERSATION) showConversationRegion()
            if (preferred != null) {
                layout(LayoutOp.Focus(preferred.id))
                controller(preferred).onDrop(payload)
                return
            }
            launchReporting {
                val target = registry.provider(kind).newTarget(NewPanelRequest()) ?: return@launchReporting
                val op = if (target is PanelTarget.Conversation) LayoutOp.showConversation(target.conversationId) else LayoutOp.Open(target)
                val next = store.applyLayout(sessionId, op) ?: return@launchReporting
                next.panelFor(target)?.let { controller(it).onDrop(payload) }
            }
        }
    }

    companion object {
        /** The file shown in the most recently focused editor stack (the explorer follows it). */
        fun activeFilePath(wb: Workbench): String? = when (val t = wb.activePanel(wb.lastEditorStack)?.target) {
            is PanelTarget.File -> t.path
            is PanelTarget.Image -> t.path
            else -> null
        }
    }
}
