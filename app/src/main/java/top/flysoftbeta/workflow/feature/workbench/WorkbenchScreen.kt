package top.flysoftbeta.workflow.feature.workbench

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.isAltPressed
import androidx.compose.ui.input.key.isCtrlPressed
import androidx.compose.ui.input.key.isShiftPressed
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.app.Shell
import top.flysoftbeta.workflow.app.panel.PanelActionKeys
import top.flysoftbeta.workflow.app.panel.PanelFrame
import top.flysoftbeta.workflow.app.panel.PanelPlacement
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.Paradigm
import top.flysoftbeta.workflow.core.layout.Region
import top.flysoftbeta.workflow.core.layout.Workbench
import top.flysoftbeta.workflow.core.session.Session
import top.flysoftbeta.workflow.core.store.StoreStatus
import top.flysoftbeta.workflow.ui.design.MenuGroup
import top.flysoftbeta.workflow.ui.design.RegionCard
import top.flysoftbeta.workflow.ui.design.dnd.DragDropState
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme
import top.flysoftbeta.workflow.ui.design.theme.frame

/**
 * The Workbench space (product.md §2, §5; ui.md §3): renders the active session's workbench in its
 * paradigm. All state comes from the WorkspaceStore; every change is a store command or LayoutOp.
 * Back here (after transient UI) returns to the Launcher; tabs and paradigm are untouched.
 */
@Composable
fun WorkbenchScreen(shell: Shell, runtime: WorkbenchRuntime, dnd: DragDropState, modifier: Modifier = Modifier) {
    val state by shell.store.state.collectAsState()
    val attention by remember(shell) { shell.registry.attention }.collectAsState(false)
    val session = state.activeSession

    // Registered after the DragDropHost's own handler, so it must yield while a drag is active (Back cancels it).
    BackHandler(enabled = !dnd.isDragging) { shell.goHome() }

    if (session == null) {
        Box(modifier.fillMaxSize().background(WorkflowTheme.colors.frame))
        // Archived (manually or by maintenance) or not created yet: the most recent live one, else a new one.
        LaunchedEffect(state.status, state.activeSessionId) {
            if (state.status == StoreStatus.READY) shell.store.enterWorkbench()
        }
        return
    }
    key(session.id) {
        val sessionRuntime = remember(session.id) { runtime.session(session.id) }
        val env = remember(sessionRuntime) { WorkbenchEnv(shell, sessionRuntime, dnd) }
        SessionWorkbench(env, session, attention, modifier)
    }
}

@Composable
private fun SessionWorkbench(env: WorkbenchEnv, session: Session, attention: Boolean, modifier: Modifier) {
    val wb = session.workbench
    val density = LocalDensity.current
    val imeVisible = WindowInsets.ime.getBottom(density) > 0
    val facts = FrameFacts(imeVisible = imeVisible, resizing = env.runtime.resizing)

    SideEffect { env.prune(wb) }
    val focusedPanel = wb.focusedPanel?.id
    LaunchedEffect(focusedPanel) { env.runtime.setFocused(focusedPanel) }
    DisposableEffect(env) { onDispose { env.runtime.setFocused(null) } }

    BoxWithConstraints(
        modifier
            .fillMaxSize()
            .background(WorkflowTheme.colors.frame)
            .onPreviewKeyEvent { event -> event.type == KeyEventType.KeyDown && handleShortcut(env, session, wb, event) },
    ) {
        when (wb.paradigm) {
            Paradigm.FILES -> FilesWorkbench(env, session, wb, maxWidth, facts, attention)
            Paradigm.CHAT -> ChatWorkbench(env, session, wb, maxWidth, facts, attention)
            Paradigm.SOLO -> SoloWorkbench(env, session, wb, facts)
        }
    }
}

/**
 * Solo (docs/ui.md §3.5): one panel fills the window; header = ⌂ + session chip + panel name + surfaced
 * actions + ⋯ + ⚙ (⚙ hidden when the panel is Settings). Content max width 840dp, centred.
 */
@Composable
private fun SoloWorkbench(env: WorkbenchEnv, session: Session, wb: Workbench, facts: FrameFacts) {
    val panel = wb.focusedPanel ?: return
    val controller = env.controller(panel)
    val gap = WorkflowTheme.dimens.gap
    val settings = panel.target == PanelTarget.Settings
    Column(Modifier.fillMaxSize().padding(gap)) {
        TitleHeader(
            title = controller.tab.title,
            switcher = controller.switcher,
            actions = controller.actions,
            moreGroups = buildList {
                controller.resourceMenu.takeIf { it.isNotEmpty() }?.let { add(MenuGroup("resource", it)) }
                add(sessionGroup(env, session, wb))
            },
            leading = { LeftCluster(env, session, sideRegion = null, sideShown = false) },
            trailing = if (settings) null else ({ RightCluster(env, auxRegion = null, auxShown = false, attention = false) }),
        )
        RegionCard(Modifier.fillMaxWidth().weight(1f)) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.TopCenter) {
                env.content(panel)(
                    PanelFrame(focused = true, placement = PanelPlacement.SOLO, resizing = facts.resizing, imeVisible = facts.imeVisible),
                    Modifier.widthIn(max = 840.dp).fillMaxSize(),
                )
            }
        }
    }
}

/** Hardware keyboard shortcuts (docs/ui.md §7). */
private fun handleShortcut(env: WorkbenchEnv, session: Session, wb: Workbench, event: androidx.compose.ui.input.key.KeyEvent): Boolean {
    if (!event.isCtrlPressed) return false
    val focused = wb.focusedPanel
    val controller = focused?.let { env.runtime.existing(it.id) }
    fun action(key: String): Boolean {
        val action = controller?.actions?.firstOrNull { it.key == key && it.enabled } ?: return false
        action.onClick()
        return true
    }
    val shift = event.isShiftPressed
    return when (event.key) {
        Key.S -> action(PanelActionKeys.SAVE)
        Key.Z -> action(if (shift) PanelActionKeys.REDO else PanelActionKeys.UNDO)
        Key.F -> action(PanelActionKeys.FIND)
        Key.W -> { focused?.let { env.runtime.close(listOf(it.id)) }; focused != null }
        Key.Tab -> {
            val stack = wb.stacks[wb.focusedStack] ?: return false
            if (stack.panels.size < 2) return false
            val i = stack.panels.indexOf(stack.active)
            val next = stack.panels[(i + if (shift) stack.panels.size - 1 else 1) % stack.panels.size]
            env.layout(LayoutOp.Focus(next)); true
        }
        Key.B -> {
            val region = when {
                event.isAltPressed -> if (wb.paradigm == Paradigm.CHAT) Region.CHAT_SIDE else Region.AUX
                wb.paradigm == Paradigm.CHAT -> Region.CHAT_RAIL
                else -> Region.EXPLORER
            }
            env.runtime.toggleRegion(region); true
        }
        Key.J -> { env.layout(LayoutOp.ToggleRegion(Region.BOTTOM)); true }
        Key.N -> {
            if (shift) env.runtime.commands.newConversation() else env.runtime.beginCreateFile()
            true
        }
        else -> false
    }
}
