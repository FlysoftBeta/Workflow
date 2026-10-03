package top.flysoftbeta.workflow.core.layout

import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.core.resource.ResourceRef

typealias PanelId = String
typealias StackId = String
typealias SplitId = String

enum class PanelKind { FILE, IMAGE, DIFF, TERMINAL, CONVERSATION, PROXY, SETTINGS }

enum class ProxyPage { OVERVIEW, LOGS, CONNECTIONS }

/** What a panel shows. [key] identifies the resource: one panel per key in a session. */
sealed interface PanelTarget {
    val kind: PanelKind
    val key: String
    /** The Working Resource this panel edits or displays, if any. */
    val resource: ResourceRef? get() = null

    /** Text editor for a workspace file. Shares [key] with [Image]: a path is open at most once. */
    data class File(val path: String) : PanelTarget {
        override val kind get() = PanelKind.FILE
        override val key get() = "file:$path"
        override val resource get() = ResourceRef.File(path)
    }

    data class Image(val path: String) : PanelTarget {
        override val kind get() = PanelKind.IMAGE
        override val key get() = "file:$path"
        override val resource get() = ResourceRef.File(path)
    }

    /** Read-only comparison of a file's draft with its disk version ("对比"). */
    data class Diff(val path: String) : PanelTarget {
        override val kind get() = PanelKind.DIFF
        override val key get() = "diff:$path"
        override val resource get() = ResourceRef.File(path)
    }

    /** A terminal owned by the runtime; the id is assigned by the runtime and globally unique. */
    data class Terminal(val terminalId: String) : PanelTarget {
        override val kind get() = PanelKind.TERMINAL
        override val key get() = "terminal:$terminalId"
    }

    data class Conversation(val conversationId: String) : PanelTarget {
        override val kind get() = PanelKind.CONVERSATION
        override val key get() = "conversation:$conversationId"
        override val resource get() = ResourceRef.Conversation(conversationId)
    }

    data class Proxy(val page: ProxyPage = ProxyPage.OVERVIEW) : PanelTarget {
        override val kind get() = PanelKind.PROXY
        override val key get() = "proxy:${page.name.lowercase()}"
    }

    data object Settings : PanelTarget {
        override val kind get() = PanelKind.SETTINGS
        override val key get() = "settings"
    }

    companion object {
        private val IMAGE_EXTENSIONS = setOf("png", "jpg", "jpeg", "gif", "webp", "bmp", "heic", "heif", "ico")

        /** The panel for opening a workspace file: an image preview for image types, else the editor. */
        fun forFile(path: String): PanelTarget =
            if (path.substringAfterLast('/').substringAfterLast('.', "").lowercase() in IMAGE_EXTENSIONS) Image(path) else File(path)

        /** Paths must be normalized non-root workspace paths; ids must be non-blank. */
        fun isValid(target: PanelTarget): Boolean = when (target) {
            is File -> WorkspacePaths.isValidEntry(target.path)
            is Image -> WorkspacePaths.isValidEntry(target.path)
            is Diff -> WorkspacePaths.isValidEntry(target.path)
            is Terminal -> target.terminalId.isNotBlank()
            is Conversation -> target.conversationId.isNotBlank()
            is Proxy, Settings -> true
        }
    }
}

data class TextCursor(val line: Int, val column: Int, val anchorLine: Int = line, val anchorColumn: Int = column)

/**
 * Per-panel UI state that survives leaving the session: reading position and cursor. The UI
 * pushes it when scrolling settles, not per frame. [extras] holds kind-specific values
 * (image zoom, settings category, ...) without a core change.
 */
data class PanelView(
    /** Editor: first visible line; conversation: item id; null = top (or "follow" for streams). */
    val scrollAnchor: String? = null,
    val scrollOffset: Int = 0,
    val cursor: TextCursor? = null,
    val extras: Map<String, String> = emptyMap(),
)

data class Panel(val id: PanelId, val target: PanelTarget, val view: PanelView = PanelView())

/** Panels shown as tabs sharing one area. [active] is null exactly when the stack is empty. */
data class Stack(val id: StackId, val panels: List<PanelId> = emptyList(), val active: PanelId? = null)

/** ROW: children side by side (vertical dividers). COLUMN: children on top of each other. */
enum class Axis { ROW, COLUMN }

enum class Edge(val axis: Axis, val before: Boolean) {
    LEFT(Axis.ROW, true), RIGHT(Axis.ROW, false), TOP(Axis.COLUMN, true), BOTTOM(Axis.COLUMN, false)
}

/** The editor area: an n-ary split tree whose leaves are stacks. Weights are positive and sum to 1. */
sealed interface SplitNode {
    data class Leaf(val stackId: StackId) : SplitNode
    data class Split(val id: SplitId, val axis: Axis, val children: List<SplitNode>, val weights: List<Double>) : SplitNode
}

enum class Paradigm { FILES, CHAT, SOLO }

/**
 * Resizable, collapsible regions. Sizes are dp for side regions and a fraction of the column
 * height for [BOTTOM]. The UI clamps against the window at render time (ui.md §3.2).
 */
enum class Region(val defaultSize: Double, val minSize: Double, val maxSize: Double) {
    /** Files: file explorer (left). */
    EXPLORER(264.0, 200.0, 1600.0),
    /** Files: conversation panel (right). */
    AUX(360.0, 300.0, 1600.0),
    /** Files: terminal stack below the editor area. */
    BOTTOM(0.35, 0.1, 0.9),
    /** Chat: conversation list rail (left). */
    CHAT_RAIL(264.0, 200.0, 1600.0),
    /** Chat: reduced Files view (right). */
    CHAT_SIDE(400.0, 300.0, 1600.0),
    /** Chat: explorer column inside the side region. */
    CHAT_TREE(200.0, 160.0, 1600.0),
}

data class RegionState(val size: Double, val collapsed: Boolean = false)

data class FilesArrangement(
    val explorer: RegionState = RegionState(Region.EXPLORER.defaultSize),
    val aux: RegionState = RegionState(Region.AUX.defaultSize),
    val bottom: RegionState = RegionState(Region.BOTTOM.defaultSize, collapsed = true),
    /** Focused stack while in Files; restored when returning from Chat. */
    val focus: StackId,
    /** A stack temporarily filling the whole workbench (persisted). */
    val maximized: StackId? = null,
)

/** Where a promoted conversation panel came from, so returning to Files puts it back. */
data class PanelOrigin(val panelId: PanelId, val stackId: StackId, val index: Int, val auxActiveBefore: PanelId?)

data class ChatArrangement(
    val rail: RegionState = RegionState(Region.CHAT_RAIL.defaultSize),
    val side: RegionState = RegionState(Region.CHAT_SIDE.defaultSize),
    val tree: RegionState = RegionState(Region.CHAT_TREE.defaultSize),
    /** Stack shown in the side region: an editor stack or the bottom (terminal) stack. */
    val sideStack: StackId,
    /** [Workbench.AUX] (main column) or [sideStack]. */
    val focus: StackId = Workbench.AUX,
    val promotedFrom: PanelOrigin? = null,
)

data class SoloArrangement(val panelId: PanelId, val returnTo: Paradigm)

/** Explorer state shared by Files' explorer and Chat's tree column. */
data class ExplorerView(val expanded: List<String> = emptyList(), val selected: String? = null, val showHidden: Boolean = false)

/**
 * The complete arrangement of one session. Panels and stacks are shared by all paradigms; each
 * paradigm keeps its own region sizes, collapsed flags and focus, so switching paradigm (or
 * promoting a conversation) only changes what is rendered where.
 *
 * Invariants (see [violations]): every panel is in exactly one stack; target keys are unique;
 * [editor] references each editor stack once and contains at least one stack; [BOTTOM] and
 * [AUX] always exist and are not in [editor]; non-empty stacks have an active panel; focus,
 * maximized, side stack and solo references are valid.
 */
data class Workbench(
    val paradigm: Paradigm,
    val panels: Map<PanelId, Panel>,
    val stacks: Map<StackId, Stack>,
    val editor: SplitNode,
    val files: FilesArrangement,
    val chat: ChatArrangement,
    val solo: SoloArrangement? = null,
    /** Most recently focused editor stack: default target for files; shown in Chat's side. */
    val lastEditorStack: StackId,
    val explorer: ExplorerView = ExplorerView(),
    /** Panel ids, most recently focused first. */
    val mru: List<PanelId> = emptyList(),
    val nextSeq: Long = 1,
) {
    // ---- Queries for the UI ----

    val editorStacks: List<StackId> get() = leaves(editor)

    fun isEditorStack(id: StackId): Boolean = id != BOTTOM && id != AUX && id in stacks

    /** Only editor stacks can be split; bottom and aux are single stacks. */
    fun canSplit(id: StackId): Boolean = isEditorStack(id)

    fun stackOf(panelId: PanelId): StackId? = stacks.values.firstOrNull { panelId in it.panels }?.id

    fun panelFor(target: PanelTarget): Panel? = panels.values.firstOrNull { it.target.key == target.key }

    fun panelsIn(stackId: StackId): List<Panel> = stacks[stackId]?.panels.orEmpty().mapNotNull { panels[it] }

    fun activePanel(stackId: StackId): Panel? = stacks[stackId]?.active?.let { panels[it] }

    /** The stack with keyboard and toolbar focus in the current paradigm. */
    val focusedStack: StackId get() = when (paradigm) {
        Paradigm.FILES -> files.focus
        Paradigm.CHAT -> chat.focus
        Paradigm.SOLO -> solo?.let { stackOf(it.panelId) } ?: files.focus
    }

    val focusedPanel: Panel? get() = when (paradigm) {
        Paradigm.SOLO -> solo?.let { panels[it.panelId] }
        else -> activePanel(focusedStack)
    }

    /** Resources referenced by this workbench (for archive protection and search). */
    val resources: Set<ResourceRef> get() = panels.values.mapNotNull { it.target.resource }.toSet()

    /** Files and conversations ordered by recent focus; the session view shows the first few. */
    val primaryResources: List<ResourceRef> get() {
        val ordered = mru.mapNotNull { panels[it] } + stackOrder().mapNotNull { panels[it] }
        return ordered.mapNotNull { it.target.resource }.distinct()
    }

    /** Panel ids in stack order: editor stacks (tree order), then bottom, then aux. */
    fun stackOrder(): List<PanelId> = (editorStacks + BOTTOM + AUX).flatMap { stacks[it]?.panels.orEmpty() }

    fun regionState(region: Region): RegionState = when (region) {
        Region.EXPLORER -> files.explorer
        Region.AUX -> files.aux
        Region.BOTTOM -> files.bottom
        Region.CHAT_RAIL -> chat.rail
        Region.CHAT_SIDE -> chat.side
        Region.CHAT_TREE -> chat.tree
    }

    fun split(id: SplitId): SplitNode.Split? = findSplit(editor, id)

    /** Panels to the right of [panelId] in its stack ("关闭右侧"). */
    fun panelsAfter(panelId: PanelId): List<PanelId> =
        stackOf(panelId)?.let { stacks.getValue(it).panels }?.let { it.drop(it.indexOf(panelId) + 1) }.orEmpty()

    /** All other panels of [panelId]'s stack ("关闭其他"). */
    fun otherPanels(panelId: PanelId): List<PanelId> =
        stackOf(panelId)?.let { stacks.getValue(it).panels.filter { id -> id != panelId } }.orEmpty()

    fun apply(op: LayoutOp): Workbench = LayoutReducer.apply(this, op)

    fun violations(): List<String> = LayoutInvariants.violations(this)

    companion object {
        const val BOTTOM: StackId = "bottom"
        const val AUX: StackId = "aux"
        const val FIRST_EDITOR: StackId = "s0"

        /** A fresh Files workbench: one empty editor stack, empty bottom and aux stacks. */
        fun empty(): Workbench = Workbench(
            paradigm = Paradigm.FILES,
            panels = emptyMap(),
            stacks = linkedMapOf(FIRST_EDITOR to Stack(FIRST_EDITOR), BOTTOM to Stack(BOTTOM), AUX to Stack(AUX)),
            editor = SplitNode.Leaf(FIRST_EDITOR),
            files = FilesArrangement(focus = FIRST_EDITOR),
            chat = ChatArrangement(sideStack = FIRST_EDITOR),
            lastEditorStack = FIRST_EDITOR,
        )

        /** A workbench showing only [target] ("在单独的会话中打开"). Opening anything else turns it into Files. */
        fun solo(target: PanelTarget): Workbench = empty().apply(LayoutOp.EnterSolo(target))

        internal fun leaves(node: SplitNode): List<StackId> = when (node) {
            is SplitNode.Leaf -> listOf(node.stackId)
            is SplitNode.Split -> node.children.flatMap { leaves(it) }
        }

        internal fun findSplit(node: SplitNode, id: SplitId): SplitNode.Split? = when (node) {
            is SplitNode.Leaf -> null
            is SplitNode.Split -> if (node.id == id) node else node.children.firstNotNullOfOrNull { findSplit(it, id) }
        }

        internal fun splitIds(node: SplitNode): List<SplitId> = when (node) {
            is SplitNode.Leaf -> emptyList()
            is SplitNode.Split -> listOf(node.id) + node.children.flatMap { splitIds(it) }
        }
    }
}
