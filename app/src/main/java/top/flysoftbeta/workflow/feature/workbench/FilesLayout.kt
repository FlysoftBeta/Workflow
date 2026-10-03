package top.flysoftbeta.workflow.feature.workbench

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.key
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import top.flysoftbeta.workflow.core.layout.DropTarget
import top.flysoftbeta.workflow.core.layout.Edge
import top.flysoftbeta.workflow.ui.design.dnd.DropTargetKind
import top.flysoftbeta.workflow.ui.design.dnd.DropZone
import top.flysoftbeta.workflow.ui.design.dnd.PanelDragPayload
import top.flysoftbeta.workflow.ui.design.dnd.dropTarget
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.app.panel.ExplorerVariant
import top.flysoftbeta.workflow.app.panel.PanelPlacement
import top.flysoftbeta.workflow.core.layout.Axis
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.Region
import top.flysoftbeta.workflow.core.layout.SplitNode
import top.flysoftbeta.workflow.core.layout.StackId
import top.flysoftbeta.workflow.core.layout.Workbench
import top.flysoftbeta.workflow.core.session.Session
import top.flysoftbeta.workflow.ui.design.DockingDecision
import top.flysoftbeta.workflow.ui.design.DockingSpec
import top.flysoftbeta.workflow.ui.design.MenuGroup
import top.flysoftbeta.workflow.ui.design.RegionCard
import top.flysoftbeta.workflow.ui.design.RegionHeader
import top.flysoftbeta.workflow.ui.design.ToolAction
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.resolveDocking
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** Docking of Files for this window width (docs/ui.md §3.2): explorer left, conversation right. */
internal fun filesDocking(wb: Workbench, windowWidth: Float): DockingDecision = resolveDocking(
    windowWidth,
    sideOpen = !wb.files.explorer.collapsed,
    auxOpen = !wb.files.aux.collapsed,
    sideWidth = wb.files.explorer.size.toFloat(),
    auxWidth = wb.files.aux.size.toFloat(),
    spec = DockingSpec(auxDefault = Region.AUX.defaultSize.toFloat()),
)

/** Files paradigm: explorer ｜ editor split tree over the terminal stack ｜ conversation (§3.3). */
@Composable
internal fun FilesWorkbench(env: WorkbenchEnv, session: Session, wb: Workbench, windowWidth: Dp, facts: FrameFacts, attention: Boolean) {
    val presence = env.runtime.presence
    val gap = WorkflowTheme.dimens.gap
    val decision = filesDocking(wb, windowWidth.value)
    val maximized = wb.files.maximized?.takeIf { wb.isEditorStack(it) }
    presence.sideDockable = resolveDocking(windowWidth.value, true, false, wb.files.explorer.size.toFloat(), 0f).sideDocked
    presence.auxDockable = resolveDocking(
        windowWidth.value, decision.sideDocked, true, wb.files.explorer.size.toFloat(), wb.files.aux.size.toFloat(),
    ).auxDocked
    if (maximized != null) { presence.sideDockable = false; presence.auxDockable = false }
    val sideShown = (maximized == null && decision.sideDocked) || presence.sideOverlay
    val auxShown = (maximized == null && decision.auxDocked) || presence.auxOverlay

    val left: @Composable RowScope.() -> Unit = { LeftCluster(env, session, Region.EXPLORER, sideShown) }
    val right: @Composable RowScope.() -> Unit = { RightCluster(env, Region.AUX, auxShown, attention) }

    if (maximized != null) {
        Box(Modifier.fillMaxSize()) {
            StackView(
                env, session, wb, maximized, PanelPlacement.EDITOR, facts, Modifier.fillMaxSize().padding(gap),
                leading = left, trailing = right,
                shellActions = listOf(ToolAction("restore", Sym.CloseFullscreen, "还原") { env.layout(LayoutOp.SetMaximized(null)) }),
            )
            RegionOverlays(
                env = env, sideOpen = presence.sideOverlay, auxOpen = presence.auxOverlay,
                narrow = decision.narrow, sideWidth = decision.sideOverlayWidth.dp, auxWidth = decision.auxOverlayWidth.dp,
                side = { ExplorerRegion(env, session, wb, leading = null, Modifier.fillMaxSize()) },
                aux = { AuxRegion(env, session, wb, facts, trailing = null, Modifier.fillMaxSize()) },
            )
        }
        return
    }

    val editorHosts = if (decision.narrow) null else cornerHosts(wb.editor)
    val leftHost = if (decision.sideDocked) null else editorHosts?.first
    val rightHost = if (decision.auxDocked) null else editorHosts?.second

    Box(Modifier.fillMaxSize()) {
        RegionRow(
            sideWidth = if (decision.sideDocked) decision.sideWidth.dp else null,
            auxWidth = if (decision.auxDocked) decision.auxWidth.dp else null,
            sideLimits = Region.EXPLORER.minSize.toFloat()..(0.4f * windowWidth.value).coerceAtLeast(Region.EXPLORER.minSize.toFloat()),
            auxLimits = Region.AUX.minSize.toFloat()..(0.55f * windowWidth.value).coerceAtLeast(Region.AUX.minSize.toFloat()),
            onResize = { side, width -> env.layout(LayoutOp.ResizeRegion(if (side) Region.EXPLORER else Region.AUX, width.toDouble())) },
            onCollapse = { side -> env.layout(LayoutOp.SetRegionCollapsed(if (side) Region.EXPLORER else Region.AUX, true)) },
            onReset = { side ->
                val region = if (side) Region.EXPLORER else Region.AUX
                env.layout(LayoutOp.ResizeRegion(region, region.defaultSize))
            },
            onResizing = { env.runtime.resizing = it },
            modifier = Modifier.fillMaxSize().padding(gap),
            side = {
                if (decision.sideDocked) ExplorerRegion(env, session, wb, leading = left, Modifier.fillMaxSize())
            },
            center = {
                if (decision.narrow) {
                    NarrowCenter(env, session, wb, facts, left, right)
                } else {
                    CenterColumn(
                        bottomFraction = wb.files.bottom.size.toFloat(),
                        bottomCollapsed = wb.files.bottom.collapsed,
                        collapsedHeight = WorkflowTheme.dimens.bar,
                        editorMin = 120.dp,
                        imeFocus = imeFocus(wb, facts),
                        onResize = { env.layout(LayoutOp.ResizeRegion(Region.BOTTOM, it.toDouble())) },
                        onCollapse = { env.layout(LayoutOp.SetRegionCollapsed(Region.BOTTOM, it)) },
                        onReset = {
                            env.layout(LayoutOp.SetRegionCollapsed(Region.BOTTOM, false))
                            env.layout(LayoutOp.ResizeRegion(Region.BOTTOM, Region.BOTTOM.defaultSize))
                        },
                        onResizing = { env.runtime.resizing = it },
                        editor = {
                            EditorArea(env, session, wb, facts, leftHost to left, rightHost to right)
                        },
                        bottom = { BottomStack(env, session, wb, facts, Modifier.fillMaxSize()) },
                        modifier = Modifier.fillMaxSize(),
                    )
                }
            },
            aux = {
                if (decision.auxDocked) AuxRegion(env, session, wb, facts, trailing = right, Modifier.fillMaxSize())
            },
        )
        RegionOverlays(
            env = env,
            sideOpen = !decision.sideDocked && presence.sideOverlay,
            auxOpen = !decision.auxDocked && presence.auxOverlay,
            narrow = decision.narrow,
            sideWidth = decision.sideOverlayWidth.dp,
            auxWidth = decision.auxOverlayWidth.dp,
            side = { ExplorerRegion(env, session, wb, leading = null, Modifier.fillMaxSize()) },
            aux = { AuxRegion(env, session, wb, facts, trailing = null, Modifier.fillMaxSize()) },
        )
    }
}

/** Outer-edge placement wraps the complete tree, so its preview and the server operation agree. */
@Composable
private fun EditorArea(
    env: WorkbenchEnv,
    session: Session,
    wb: Workbench,
    facts: FrameFacts,
    leftHost: Pair<StackId?, @Composable RowScope.() -> Unit>,
    rightHost: Pair<StackId?, @Composable RowScope.() -> Unit>,
) {
    val band = with(LocalDensity.current) { 12.dp.toPx() }
    Box(Modifier.fillMaxSize().testTag("editor-area").dropTarget(
        env.dnd, key = "editor-edges", kind = DropTargetKind.EditorEdges(band), priority = 2,
        accepts = { it is PanelDragPayload },
        onDrop = { payload, result ->
            val edge = when (result.zone) {
                DropZone.Left -> Edge.LEFT
                DropZone.Top -> Edge.TOP
                DropZone.Right -> Edge.RIGHT
                DropZone.Bottom -> Edge.BOTTOM
                else -> null
            }
            if (payload is PanelDragPayload && edge != null) env.layout(LayoutOp.Move(payload.panelId, DropTarget.EditorEdge(edge)))
        },
    )) {
        EditorTree(env, session, wb, wb.editor, facts, leftHost, rightHost, Modifier.fillMaxSize())
    }
}

/** The IME rule (docs/ui.md §2.3): which part of the centre column keeps its height. */
internal fun imeFocus(wb: Workbench, facts: FrameFacts): ImeFocus? = when {
    !facts.imeVisible -> null
    wb.focusedStack == Workbench.BOTTOM -> ImeFocus.BOTTOM
    wb.isEditorStack(wb.focusedStack) -> ImeFocus.EDITOR
    else -> null
}

/** Stacks at the editor area's top-left and top-right corners (where the corner clusters go). */
internal fun cornerHosts(node: SplitNode): Pair<StackId, StackId> {
    fun topLeft(n: SplitNode): StackId = when (n) {
        is SplitNode.Leaf -> n.stackId
        is SplitNode.Split -> topLeft(n.children.first())
    }
    fun topRight(n: SplitNode): StackId = when (n) {
        is SplitNode.Leaf -> n.stackId
        is SplitNode.Split -> topRight(if (n.axis == Axis.ROW) n.children.last() else n.children.first())
    }
    return topLeft(node) to topRight(node)
}

private fun containsStack(node: SplitNode, stackId: StackId): Boolean = when (node) {
    is SplitNode.Leaf -> node.stackId == stackId
    is SplitNode.Split -> node.children.any { containsStack(it, stackId) }
}

/** The editor split tree (workspace.md §3), recursively. */
@Composable
internal fun EditorTree(
    env: WorkbenchEnv,
    session: Session,
    wb: Workbench,
    node: SplitNode,
    facts: FrameFacts,
    leftHost: Pair<StackId?, @Composable RowScope.() -> Unit>,
    rightHost: Pair<StackId?, @Composable RowScope.() -> Unit>,
    modifier: Modifier = Modifier,
    placement: PanelPlacement = PanelPlacement.EDITOR,
) {
    when (node) {
        is SplitNode.Leaf -> StackView(
            env, session, wb, node.stackId, placement, facts, modifier,
            leading = leftHost.second.takeIf { leftHost.first == node.stackId },
            trailing = rightHost.second.takeIf { rightHost.first == node.stackId },
        )
        is SplitNode.Split -> {
            val horizontal = node.axis == Axis.ROW
            val focusedChild = node.children.indexOfFirst { containsStack(it, wb.focusedStack) }
            val bar = WorkflowTheme.dimens.bar
            val fixed = if (!horizontal && facts.imeVisible && focusedChild >= 0) {
                node.children.indices.filter { it != focusedChild }.associateWith { bar }
            } else emptyMap()
            WeightedSplit(
                horizontal = horizontal,
                weights = node.weights,
                fixed = fixed,
                minSize = if (horizontal) 240.dp else 120.dp,
                onCommit = { env.layout(LayoutOp.ResizeSplit(node.id, it)) },
                onReset = { env.layout(LayoutOp.ResetSplit(node.id)) },
                onResizing = { env.runtime.resizing = it },
                children = node.children.map { child ->
                    @Composable {
                        key(nodeKey(child)) { EditorTree(env, session, wb, child, facts, leftHost, rightHost, Modifier.fillMaxSize(), placement) }
                    }
                },
                modifier = modifier,
            )
        }
    }
}

private fun nodeKey(node: SplitNode): String = when (node) {
    is SplitNode.Leaf -> "leaf:${node.stackId}"
    is SplitNode.Split -> "split:${node.id}"
}

/** The bottom (terminal) stack, with ⌄/⌃ (collapse/expand) and "新建终端" (docs/ui.md §2.2). */
@Composable
internal fun BottomStack(env: WorkbenchEnv, session: Session, wb: Workbench, facts: FrameFacts, modifier: Modifier = Modifier) {
    val collapsed = wb.files.bottom.collapsed
    val empty = wb.stacks[Workbench.BOTTOM]?.panels.isNullOrEmpty()
    val shellActions = buildList {
        if (empty) add(ToolAction("newTerminal", Sym.Add, "新建终端") { env.runtime.commands.newTerminal() })
        add(
            if (collapsed) ToolAction("expand", Sym.KeyboardArrowUp, "展开") { env.layout(LayoutOp.SetRegionCollapsed(Region.BOTTOM, false)) }
            else ToolAction("collapse", Sym.KeyboardArrowDown, "收起") { env.layout(LayoutOp.SetRegionCollapsed(Region.BOTTOM, true)) },
        )
    }
    StackView(env, session, wb, Workbench.BOTTOM, PanelPlacement.BOTTOM, facts, modifier, shellActions = shellActions)
}

/** Files' conversation region: the aux stack under a single title (tabs when it holds several panels). */
@Composable
internal fun AuxRegion(
    env: WorkbenchEnv,
    session: Session,
    wb: Workbench,
    facts: FrameFacts,
    trailing: (@Composable RowScope.() -> Unit)?,
    modifier: Modifier = Modifier,
) {
    val active = wb.activePanel(Workbench.AUX)
    val promote = if (active?.target is PanelTarget.Conversation) {
        listOf(ToolAction("promote", Sym.OpenInFull, "设为主要内容") { env.layout(LayoutOp.PromoteConversation(active.id)) })
    } else emptyList()
    StackView(
        env, session, wb, Workbench.AUX, PanelPlacement.AUX, facts, modifier,
        trailing = trailing, headerStyle = HeaderStyle.Title, shellActions = promote,
    )
}

/** Files' explorer region: header `[corner cluster]─[＋][⋯]` and the explorer content (docs/ui.md §3.3). */
@Composable
internal fun ExplorerRegion(
    env: WorkbenchEnv,
    session: Session,
    wb: Workbench,
    leading: (@Composable RowScope.() -> Unit)?,
    modifier: Modifier = Modifier,
) {
    val explorer = env.runtime.explorerController()
    Column(modifier) {
        RegionHeader(
            leading = leading,
            actions = explorer.actions,
            moreGroups = listOf(
                MenuGroup("resource", explorer.resourceMenu),
                MenuGroup("layout", listOf(
                    top.flysoftbeta.workflow.ui.design.MenuEntry.Action("collapseSide", "收起侧栏", Sym.LeftPanelClose) { hideRegion(env, Region.EXPLORER) },
                )),
                sessionGroup(env, session, wb),
            ),
        )
        RegionCard(Modifier.fillMaxWidth().weight(1f)) {
            env.explorerContent(ExplorerVariant.SIDEBAR, Modifier.fillMaxSize())
        }
    }
}

/**
 * Below 600dp (docs/ui.md §3.5): only the focused editor stack with a `[1/3 ▾]` switcher, the terminal
 * stack reduced to its tab row unless focused.
 */
@Composable
private fun NarrowCenter(
    env: WorkbenchEnv,
    session: Session,
    wb: Workbench,
    facts: FrameFacts,
    left: @Composable RowScope.() -> Unit,
    right: @Composable RowScope.() -> Unit,
) {
    val current = wb.focusedStack.takeIf { wb.isEditorStack(it) } ?: wb.lastEditorStack
    CenterColumn(
        bottomFraction = wb.files.bottom.size.toFloat(),
        bottomCollapsed = wb.files.bottom.collapsed || wb.focusedStack != Workbench.BOTTOM,
        collapsedHeight = WorkflowTheme.dimens.bar,
        editorMin = 120.dp,
        imeFocus = imeFocus(wb, facts),
        onResize = { env.layout(LayoutOp.ResizeRegion(Region.BOTTOM, it.toDouble())) },
        onCollapse = { env.layout(LayoutOp.SetRegionCollapsed(Region.BOTTOM, it)) },
        onReset = { env.layout(LayoutOp.ResizeRegion(Region.BOTTOM, Region.BOTTOM.defaultSize)) },
        onResizing = { env.runtime.resizing = it },
        editor = {
            StackView(
                env, session, wb, current, PanelPlacement.EDITOR, facts, Modifier.fillMaxSize(),
                leading = { left(); StackSwitcher(env, wb, current, wb.editorStacks) },
                trailing = right,
            )
        },
        bottom = { BottomStack(env, session, wb, facts, Modifier.fillMaxSize()) },
        modifier = Modifier.fillMaxSize(),
    )
}
