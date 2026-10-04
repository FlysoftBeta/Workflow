package top.flysoftbeta.workflow.feature.workbench

import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.app.panel.PanelFrame
import top.flysoftbeta.workflow.app.panel.PanelPlacement
import top.flysoftbeta.workflow.core.layout.DropTarget
import top.flysoftbeta.workflow.core.layout.Edge
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.Paradigm
import top.flysoftbeta.workflow.core.layout.Placement
import top.flysoftbeta.workflow.core.layout.Region
import top.flysoftbeta.workflow.core.layout.StackId
import top.flysoftbeta.workflow.core.layout.Workbench
import top.flysoftbeta.workflow.core.session.Session
import top.flysoftbeta.workflow.ui.design.CompositeMenu
import top.flysoftbeta.workflow.ui.design.EmptyState
import top.flysoftbeta.workflow.ui.design.MenuGroup
import top.flysoftbeta.workflow.ui.design.RegionCard
import top.flysoftbeta.workflow.ui.design.RegionHeader
import top.flysoftbeta.workflow.ui.design.StackTabBar
import top.flysoftbeta.workflow.ui.design.StackTabDragDrop
import top.flysoftbeta.workflow.ui.design.SymbolIcon
import top.flysoftbeta.workflow.ui.design.TabItem
import top.flysoftbeta.workflow.ui.design.TextAction
import top.flysoftbeta.workflow.ui.design.ToolAction
import top.flysoftbeta.workflow.ui.design.dnd.AreaStyle
import top.flysoftbeta.workflow.ui.design.dnd.DropTargetKind
import top.flysoftbeta.workflow.ui.design.dnd.DropZone
import top.flysoftbeta.workflow.ui.design.dnd.FilesDragPayload
import top.flysoftbeta.workflow.ui.design.dnd.DragPayload
import top.flysoftbeta.workflow.ui.design.dnd.PanelDragPayload
import top.flysoftbeta.workflow.ui.design.dnd.dropTarget
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** How a stack's header row looks. */
internal enum class HeaderStyle {
    /** The StackTabBar (editor stacks, bottom, aux with several panels). */
    Tabs,
    /** One title (with the panel's switcher) instead of tabs: aux / Chat main with ≤ 1 panel. */
    Title,
}

/** A stack: its one header row on the frame, then its content card (docs/ux/README.md §3.2, §4.1). */
@Composable
internal fun StackView(
    env: WorkbenchEnv,
    session: Session,
    wb: Workbench,
    stackId: StackId,
    placement: PanelPlacement,
    frameFacts: FrameFacts,
    modifier: Modifier = Modifier,
    leading: (@Composable RowScope.() -> Unit)? = null,
    trailing: (@Composable RowScope.() -> Unit)? = null,
    headerStyle: HeaderStyle = HeaderStyle.Tabs,
    shellActions: List<ToolAction> = emptyList(),
) {
    BoxWithConstraints(modifier) {
        CompositionLocalProvider(LocalCompactWorkbenchHeader provides (maxWidth < 480.dp)) {
            Column(Modifier.fillMaxSize()) {
                StackHeader(env, session, wb, stackId, frameFacts, leading, trailing, headerStyle, shellActions)
                RegionCard(Modifier.fillMaxWidth().weight(1f)) {
                    StackBody(env, wb, stackId, placement, frameFacts)
                }
            }
        }
    }
}

/** Window facts every stack needs. */
internal data class FrameFacts(val imeVisible: Boolean, val resizing: Boolean)

@Composable
internal fun StackHeader(
    env: WorkbenchEnv,
    session: Session,
    wb: Workbench,
    stackId: StackId,
    frameFacts: FrameFacts,
    leading: (@Composable RowScope.() -> Unit)?,
    trailing: (@Composable RowScope.() -> Unit)?,
    headerStyle: HeaderStyle,
    shellActions: List<ToolAction>,
    modifier: Modifier = Modifier,
) {
    val panels = wb.panelsIn(stackId)
    val active = wb.activePanel(stackId)
    val activeController = active?.let { env.controller(it) }
    val focused = wb.focusedStack == stackId
    val titleOf = { id: String -> wb.panels[id]?.let { env.controller(it).tab.title } }
    val actions = shellActions + activeController?.actions.orEmpty()
    val more = moreMenu(actions, buildList {
        activeController?.resourceMenu?.takeIf { it.isNotEmpty() }?.let { add(MenuGroup("resource", it)) }
        add(layoutGroup(env, wb, stackId, active?.id, titleOf))
        add(sessionGroup(env, session, wb))
    })
    if (headerStyle == HeaderStyle.Title && panels.size <= 1) {
        TitleHeader(
            title = activeController?.tab?.title ?: "",
            switcher = activeController?.switcher,
            actions = actions,
            moreGroups = more,
            leading = leading,
            trailing = trailing,
            modifier = modifier,
        )
        return
    }
    val tabs = panels.map { panel ->
        val tab = env.controller(panel).tab
        TabItem(panel.id, tab.title, tab.icon, tab.dirty, tab.caption)
    }.let { items ->
        items.mapIndexed { i, item -> if (item.caption == null) item.copy(caption = autoCaption(items, i, panels[i].target)) else item }
    }
    val dnd = env.dnd
    StackTabBar(
        tabs = tabs,
        activeKey = active?.id,
        onSelect = { id ->
            env.layout(LayoutOp.Focus(id))
            wb.panels[id]?.let { env.controller(it).requestInputFocus() }
        },
        onClose = { id -> env.runtime.close(listOf(id)) },
        modifier = modifier,
        focused = focused,
        actions = actions,
        moreGroups = more,
        tabMenuGroups = { tab ->
            val panel = wb.panels[tab.key]
            buildList {
                panel?.let { env.controller(it).resourceMenu }?.takeIf { it.isNotEmpty() }?.let { add(MenuGroup("resource", it)) }
                add(layoutGroup(env, wb, stackId, tab.key, titleOf))
            }
        },
        leading = leading,
        trailing = trailing,
        dragDrop = StackTabDragDrop(dnd, stackId, onDrop = { payload, index ->
            if (payload is PanelDragPayload) env.layout(LayoutOp.Move(payload.panelId, DropTarget.Tab(stackId, index)))
        }),
    )
}

/** Equal file names in one stack get their parent directory as caption (docs/ux/README.md §4.1). */
private fun autoCaption(items: List<TabItem>, index: Int, target: PanelTarget): String? {
    val title = items[index].title
    if (items.count { it.title == title } < 2) return null
    val path = when (target) {
        is PanelTarget.File -> target.path
        is PanelTarget.Image -> target.path
        is PanelTarget.Diff -> target.path
        else -> return null
    }
    return path.substringBeforeLast('/', "").substringAfterLast('/').ifEmpty { "/" }
}

/** Single-title header row (aux conversation, Chat main column, Solo), same height as a tab row. */
@Composable
internal fun TitleHeader(
    title: String,
    switcher: List<MenuGroup>?,
    actions: List<ToolAction>,
    moreGroups: List<MenuGroup>,
    leading: (@Composable RowScope.() -> Unit)?,
    trailing: (@Composable RowScope.() -> Unit)?,
    modifier: Modifier = Modifier,
    style: TextStyle = WorkflowTheme.text.titleSm,
) {
    var open by remember { mutableStateOf(false) }
    RegionHeader(
        modifier = modifier,
        leading = leading,
        title = {
            Box {
                Row(
                    Modifier
                        .clip(WorkflowShapes.sm)
                        .then(if (switcher != null) Modifier.clickable { open = true }.semantics { role = Role.DropdownList } else Modifier)
                        .padding(horizontal = 4.dp, vertical = 4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(title, Modifier.widthIn(max = 480.dp), style = style, color = WorkflowTheme.colors.onSurface, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    if (switcher != null) SymbolIcon(Sym.ArrowDropDown, null, size = 18.dp, tint = WorkflowTheme.colors.onSurfaceVariant)
                }
                if (switcher != null) CompositeMenu(expanded = open, onDismissRequest = { open = false }, groups = switcher)
            }
        },
        actions = actions,
        moreGroups = moreGroups,
        trailing = trailing,
    )
}

/**
 * The content card of a stack: the active panel's movable content, the empty state, the placement
 * zones for moved tabs (centre / 4 edges) and dropped files, and the active panel's own drop area.
 * A touch anywhere focuses the stack.
 */
@Composable
internal fun StackBody(
    env: WorkbenchEnv,
    wb: Workbench,
    stackId: StackId,
    placement: PanelPlacement,
    frameFacts: FrameFacts,
    modifier: Modifier = Modifier,
) {
    val dnd = env.dnd
    val active = wb.activePanel(stackId)
    val controller = active?.let { env.controller(it) }
    val focused = wb.focusedStack == stackId
    val currentFocused by rememberUpdatedState(focused)
    val editor = wb.isEditorStack(stackId)
    Box(
        modifier
            .fillMaxSize()
            .testTag("stack:$stackId")
            .pointerInput(stackId) {
                awaitEachGesture {
                    awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
                    if (!currentFocused) env.layout(LayoutOp.FocusStack(stackId))
                }
            }
            .dropTarget(
                dnd, key = "zones:$stackId",
                kind = DropTargetKind.Zones(allowEdges = editor),
                accepts = { it is PanelDragPayload || (editor && it is FilesDragPayload) },
                onDrop = { payload, result ->
                    val zone = result.zone ?: DropZone.Center
                    when (payload) {
                        is PanelDragPayload -> env.layout(LayoutOp.Move(payload.panelId, zone.toDropTarget(stackId)))
                        is FilesDragPayload -> payload.paths.forEach { path ->
                            val target = PanelTarget.forFile(path)
                            val edge = zone.toEdge()
                            env.layout(LayoutOp.Open(target, if (edge == null) Placement.InStack(stackId) else Placement.SplitEdge(stackId, edge)))
                        }
                    }
                },
            ),
    ) {
        if (active != null) {
            env.content(active)(
                PanelFrame(focused = focused, placement = placement, resizing = frameFacts.resizing, imeVisible = frameFacts.imeVisible),
                Modifier.fillMaxSize(),
            )
        } else {
            EmptyStack(env, wb, stackId)
        }
        // The active panel's own drop area (composer "添加为附件", terminal "粘贴路径"): above the zones.
        val hovering = dnd.session?.payload?.let { resourceDropPayload(wb, it) }
        val affordance = hovering?.let { controller?.dropAffordance(it) }
        Box(
            Modifier
                .matchParentSize()
                .dropTarget(
                    dnd, key = "area:$stackId",
                    kind = DropTargetKind.Area(affordance?.style ?: AreaStyle.Outline, affordance?.label),
                    priority = 1,
                    accepts = { payload -> resourceDropPayload(wb, payload)?.let { controller?.dropAffordance(it) } != null },
                    onDrop = { payload, _ -> resourceDropPayload(wb, payload)?.let { controller?.onDrop(it) } },
                ),
        )
    }
}

/** An editor tab dropped on a composer/terminal is a file; tab rows still move the panel itself. */
internal fun resourceDropPayload(wb: Workbench, payload: DragPayload): DragPayload? {
    if (payload !is PanelDragPayload) return payload
    val path = when (val target = wb.panels[payload.panelId]?.target) {
        is PanelTarget.File -> target.path
        is PanelTarget.Image -> target.path
        is PanelTarget.Diff -> target.path
        else -> return null
    }
    return FilesDragPayload(listOf(path), payload.label, payload.icon)
}

private fun DropZone.toDropTarget(stackId: StackId): DropTarget = when (this) {
    DropZone.Center -> DropTarget.Center(stackId)
    else -> DropTarget.Edge(stackId, toEdge()!!)
}

private fun DropZone.toEdge(): Edge? = when (this) {
    DropZone.Center -> null
    DropZone.Left -> Edge.LEFT
    DropZone.Right -> Edge.RIGHT
    DropZone.Top -> Edge.TOP
    DropZone.Bottom -> Edge.BOTTOM
}

/** Empty states (docs/ux/README.md §6): editor "打开文件 · 新建文件 · 新建终端", terminals, conversations. */
@Composable
private fun EmptyStack(env: WorkbenchEnv, wb: Workbench, stackId: StackId) {
    val commands = env.runtime.commands
    val actions = when (stackId) {
        Workbench.BOTTOM -> listOf(TextAction("新建终端") { commands.newTerminal() })
        Workbench.AUX -> listOf(TextAction("新对话") { commands.newConversation() })
        else -> buildList {
            add(TextAction("打开文件") {
                if (wb.paradigm == Paradigm.CHAT) env.layout(LayoutOp.SetRegionCollapsed(Region.CHAT_TREE, false))
                else env.runtime.showRegion(Region.EXPLORER)
            })
            add(TextAction("新建文件") {
                env.runtime.beginCreateFile()
            })
            add(TextAction("新建终端") { commands.newTerminal() })
        }
    }
    EmptyState(actions, Modifier.fillMaxSize())
}
