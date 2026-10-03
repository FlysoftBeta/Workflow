package top.flysoftbeta.workflow.core.layout

/** Where [LayoutOp.Open] puts a target that is not open yet. */
sealed interface Placement {
    /**
     * By kind and paradigm: files, images, diffs, proxy and settings go to the focused editor stack
     * (or the last focused one); terminals go to the bottom stack; conversations go to the aux stack.
     */
    data object Auto : Placement

    /**
     * Into [stackId] at [index] (default: after the active panel). [replaceActive] replaces the
     * active panel when it has the same kind (the conversation switcher). [moveExisting] moves the
     * target here when it is already open elsewhere, instead of only focusing it.
     */
    data class InStack(
        val stackId: StackId,
        val index: Int? = null,
        val replaceActive: Boolean = false,
        val moveExisting: Boolean = false,
    ) : Placement

    /** Into a new stack split off [stackId] at [edge] ("在右侧拆分中打开"). */
    data class SplitEdge(val stackId: StackId, val edge: Edge) : Placement
}

/** Drop targets of a panel drag. */
sealed interface DropTarget {
    /** The centre of a stack: append there. */
    data class Center(val stackId: StackId) : DropTarget

    /**
     * The tab row of a stack, before the tab at [index]. The index counts the tabs as currently
     * displayed (including the dragged tab when it is in the same stack), 0..size.
     */
    data class Tab(val stackId: StackId, val index: Int) : DropTarget

    /** An edge of a stack: split there. Bottom and aux cannot be split; there it acts as [Center]. */
    data class Edge(val stackId: StackId, val edge: top.flysoftbeta.workflow.core.layout.Edge) : DropTarget

    /** An outer edge of the whole editor area: a new full-height or full-width stack. */
    data class EditorEdge(val edge: top.flysoftbeta.workflow.core.layout.Edge) : DropTarget
}

/**
 * Pure, total layout transactions. [Workbench.apply] never throws: an op that references
 * something missing or invalid returns the workbench unchanged. A drag commits exactly one op
 * on release. Every result is normalized (see [LayoutInvariants]).
 */
sealed interface LayoutOp {
    /** Open [target], or focus it if it is already open in this session (resources open once). */
    data class Open(val target: PanelTarget, val placement: Placement = Placement.Auto, val focus: Boolean = true) : LayoutOp

    /** Activate a panel in its stack, focus the stack and make it visible (expands its region). */
    data class Focus(val panelId: PanelId) : LayoutOp

    data class FocusStack(val stackId: StackId) : LayoutOp

    /** Close panels. Unsaved content is never touched: it is a Working Resource, not layout. */
    data class Close(val panelIds: List<PanelId>) : LayoutOp {
        constructor(panelId: PanelId) : this(listOf(panelId))
    }

    data class Move(val panelId: PanelId, val to: DropTarget) : LayoutOp

    /** "向右拆分/向下拆分": moves the active panel into a new stack at [edge]. Needs two or more panels. */
    data class SplitStack(val stackId: StackId, val edge: Edge) : LayoutOp

    /** New weights for a split's children (same count, positive); normalized and clamped. */
    data class ResizeSplit(val splitId: SplitId, val weights: List<Double>) : LayoutOp

    /** Double-tap on a divider: equal weights. */
    data class ResetSplit(val splitId: SplitId) : LayoutOp

    data class ResizeRegion(val region: Region, val size: Double) : LayoutOp

    data class SetRegionCollapsed(val region: Region, val collapsed: Boolean) : LayoutOp

    data class ToggleRegion(val region: Region) : LayoutOp

    /** Maximize a stack in Files, or restore with null. */
    data class SetMaximized(val stackId: StackId?) : LayoutOp

    /** FILES or CHAT. Entering Chat shows Files' last focused editor stack in the side region. */
    data class SwitchParadigm(val paradigm: Paradigm) : LayoutOp

    /** "设为主要内容": show this conversation panel as Chat's main content. */
    data class PromoteConversation(val panelId: PanelId) : LayoutOp

    /** Back to Files with its previous arrangement; a promoted panel returns to where it came from. */
    data object ReturnToFiles : LayoutOp

    /** Show only [target] (opened if needed). Opening or focusing anything else leaves Solo. */
    data class EnterSolo(val target: PanelTarget) : LayoutOp

    /** Chat side region's stack switcher (`[1/3 ▾]`): an editor stack or the bottom stack. */
    data class SetChatSideStack(val stackId: StackId) : LayoutOp

    /**
     * Show another target in the same panel (conversation switcher). If [target] is already open
     * elsewhere, that panel is focused instead and this one is unchanged.
     */
    data class Retarget(val panelId: PanelId, val target: PanelTarget) : LayoutOp

    data class UpdateView(val panelId: PanelId, val view: PanelView) : LayoutOp

    data class UpdateExplorer(val explorer: ExplorerView) : LayoutOp

    /** A file or directory was renamed or moved: panel targets and explorer paths follow it. */
    data class RenamePath(val from: String, val to: String) : LayoutOp

    companion object {
        /** Show a conversation in the aux stack (Files' conversation panel, Chat's main column). */
        fun showConversation(conversationId: String): LayoutOp = Open(
            PanelTarget.Conversation(conversationId),
            Placement.InStack(Workbench.AUX, replaceActive = true, moveExisting = true),
        )
    }
}
