package top.flysoftbeta.workflow.app.panel

import android.content.Context
import androidx.annotation.DrawableRes
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.Stable
import androidx.compose.ui.Modifier
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.StateFlow
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.Panel
import top.flysoftbeta.workflow.core.layout.PanelId
import top.flysoftbeta.workflow.core.layout.PanelKind
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.PanelView
import top.flysoftbeta.workflow.core.layout.TextCursor
import top.flysoftbeta.workflow.core.resource.ResourceRef
import top.flysoftbeta.workflow.core.session.SessionId
import top.flysoftbeta.workflow.core.store.WorkspaceStore
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.MenuGroup
import top.flysoftbeta.workflow.ui.design.ToolAction
import top.flysoftbeta.workflow.ui.design.dnd.AreaStyle
import top.flysoftbeta.workflow.ui.design.dnd.DragPayload

/*
 * The panel contract between the Workbench shell (feature.workbench) and the feature workstreams
 * (files/editor, terminal, chat, proxy, settings). See docs/report/initial/w5b-shell.md §2.
 *
 * The shell owns layout, chrome and routing: tab rows, the More menu's layout (②) and session (③)
 * groups, region docking, drag and drop between stacks, the corner clusters. A feature owns what one
 * panel shows and does: its tab label, its surfaced actions, its resource menu group (①), its content,
 * what it accepts when something is dropped on it, and whether it may close now.
 */

/** Label of a panel's tab (or of the single-title header that replaces tabs in aux / Chat / Solo). */
@Immutable
data class PanelTab(
    val title: String,
    @param:DrawableRes val icon: Int,
    /** Unsaved content (a Working Resource) exists: the tab shows the dirty dot instead of ×. */
    val dirty: Boolean = false,
    /**
     * Disambiguation after the title (for example the parent directory). Leave null for files: the shell
     * adds the parent directory itself when two tabs of one stack share a title.
     */
    val caption: String? = null,
)

/** Where a panel is currently rendered; lets content adapt its padding or density. */
enum class PanelPlacement {
    /** An editor stack in Files, or the Chat side region. */
    EDITOR,
    /** The bottom (terminal) stack. */
    BOTTOM,
    /** Files' conversation region (docked or overlay). */
    AUX,
    /** Chat's main column (content max width 720dp, centred by the panel). */
    CHAT_MAIN,
    /** The only panel of a Solo session (content max width 840dp, centred by the shell). */
    SOLO,
}

/** Per-frame facts the shell passes to [PanelController.Content]. */
@Immutable
data class PanelFrame(
    /** This panel is the active panel of the focused stack (keyboard/toolbar focus). */
    val focused: Boolean,
    val placement: PanelPlacement,
    /**
     * A seam or region is being dragged. WebView- and sora-backed content should keep its old size and
     * relayout ~100ms after this turns false (docs/ui.md §3.2).
     */
    val resizing: Boolean,
    /** The soft keyboard is visible (the focused stack then fills its column, §2.3). */
    val imeVisible: Boolean,
)

/** How content signals it accepts a non-panel payload dropped on it (files, external content). */
@Immutable
data class DropAffordance(
    /** [AreaStyle.Outline] = "添加为附件" (composer); [AreaStyle.Scrim] = "粘贴路径" (terminal). */
    val style: AreaStyle,
    val label: String,
)

/**
 * Standard keys of surfaced actions. The shell maps hardware shortcuts (docs/ui.md §7) to the focused
 * panel's action with the same key: Ctrl+S → [SAVE], Ctrl+Z → [UNDO], Ctrl+Shift+Z → [REDO],
 * Ctrl+F → [FIND]. Use these keys when a panel has such an action.
 */
object PanelActionKeys {
    const val SAVE = "save"
    const val UNDO = "undo"
    const val REDO = "redo"
    const val FIND = "find"
    const val ATTACH = "attach"
    const val NEW_TERMINAL = "newTerminal"
}

/**
 * One live panel. Created by [PanelProvider.create] when the panel appears in the active session and
 * disposed ([dispose]) when it is closed, its target changes (Retarget/RenamePath create a new
 * controller), or the session is left. It survives moves between stacks, splits, paradigm switches and
 * region collapse; its [Content] is a movable composition, so it moves with the panel instead of being
 * recreated while it stays visible.
 *
 * [Content] leaves composition when the tab becomes inactive or its region collapses: keep expensive
 * views (WebView, CodeEditor) and in-memory state (undo history) in the controller and re-attach them.
 * Durable state goes to the store (drafts via `editFile`, reading position via [PanelContext.updateView]).
 *
 * Properties are read during composition: back them with snapshot state (`mutableStateOf`,
 * `collectAsState` in [Content] is not enough for [tab] and [actions], which are read outside it).
 * All methods are called on the main thread.
 */
@Stable
interface PanelController {
    /** Tab title, icon and dirty flag. */
    val tab: PanelTab

    /**
     * Surfaced actions of this panel for its stack's tool row, highest priority first (docs/ui.md
     * §2.2 table). They collapse into the top of More when the tab area gets narrow. Use
     * [PanelActionKeys] for standard actions.
     */
    val actions: List<ToolAction> get() = emptyList()

    /**
     * The More menu's resource group ① (copy path, reveal in tree, attach, open with, ...). Destructive
     * entries last. The shell appends ② panel/layout and ③ session groups and uses ① + ② for the
     * long-press tab menu.
     */
    val resourceMenu: List<MenuEntry> get() = emptyList()

    /**
     * When the panel is shown under a single-title header (aux conversation region, Chat main column,
     * Solo), the title becomes a dropdown with these groups (conversation switcher: 新对话 · 最近 8 个 ·
     * 全部对话…). Null: a plain title.
     */
    val switcher: List<MenuGroup>? get() = null

    /** A request waits for the user (approval, question): ◨ and the Workbench tile get a dot. */
    val needsAttention: Boolean get() = false

    @Composable
    fun Content(frame: PanelFrame, modifier: Modifier)

    /**
     * Whether a non-panel payload dropped on this panel's content is accepted, and how the target looks.
     * Panel payloads (tabs) are handled by the shell and never reach here. Return null to reject.
     * Typical: conversation accepts [top.flysoftbeta.workflow.ui.design.dnd.FilesDragPayload] and
     * [top.flysoftbeta.workflow.ui.design.dnd.ExternalDragPayload] ("添加为附件"); terminal accepts files
     * ("粘贴路径").
     */
    fun dropAffordance(payload: DragPayload): DropAffordance? = null

    /**
     * The single commit of an accepted drop. The shell also routes the cross-feature commands
     * [WorkbenchCommands.attachToConversation] / [WorkbenchCommands.pasteIntoTerminal] here as a
     * FilesDragPayload, so one code path handles drop and menu.
     */
    fun onDrop(payload: DragPayload) {}

    /**
     * Called before the panel closes (× / dirty dot / 关闭其他 …). Return false to keep it open. May ask the
     * user through [WorkbenchCommands.decide]. Closing never discards drafts: a Working Resource outlives
     * its panel, so file editors normally return true without asking.
     */
    suspend fun prepareClose(): Boolean = true

    /** The panel became (or stopped being) the focused panel of the session. */
    fun onFocusChanged(focused: Boolean) {}

    /** The shell asks for keyboard focus inside the content (a tab was tapped, a terminal was created). */
    fun requestInputFocus() {}

    /**
     * Move the cursor / scroll to [cursor] (`path:line:col` links through [WorkbenchCommands.openFile]).
     * The shell has already stored it in the panel's view state.
     */
    fun navigate(cursor: TextCursor) {}

    fun dispose() {}
}

/** What the shell gives a controller. */
interface PanelContext {
    val appContext: Context
    val store: WorkspaceStore
    val sessionId: SessionId
    val panelId: PanelId
    val target: PanelTarget
    /** Reading position stored with the panel when it was created (restore it once). */
    val initialView: PanelView
    /** Main-dispatcher scope, cancelled on [PanelController.dispose]. */
    val scope: CoroutineScope
    val commands: WorkbenchCommands

    /** A layout transaction on this panel's session. */
    fun layout(op: LayoutOp)

    /** Persists the reading position; call when scrolling settles, not per frame. */
    fun updateView(view: PanelView) = layout(LayoutOp.UpdateView(panelId, view))
}

/** Request for [PanelProvider.newTarget] (shell actions 新建终端 / 新对话 / 在终端中打开). */
@Immutable
data class NewPanelRequest(
    /** Terminal: start directory, workspace-relative ("" = root). */
    val directory: String? = null,
)

/**
 * The feature side of one or more [PanelKind]s. One instance per process, registered in
 * [PanelRegistry]; it may hold process-wide collaborators (runtime, agent hub) obtained from AppGraph.
 */
interface PanelProvider {
    /** Creates the controller of [panel]. Must not block: load in [PanelContext.scope]. */
    fun create(panel: Panel, context: PanelContext): PanelController

    /**
     * Creates a new owner-assigned target (terminal id, conversation id) for the shell's "new" actions.
     * Null when this kind cannot be created that way or creation failed (the provider shows why).
     */
    suspend fun newTarget(request: NewPanelRequest): PanelTarget? = null

    /** Display title of a resource this provider owns (conversation titles for the session view/search). */
    fun resourceTitle(ref: ResourceRef): String? = null

    /** Process-wide "a request waits for the user" (badges the Workbench tile and ◨). */
    val attention: StateFlow<Boolean>? get() = null
}
