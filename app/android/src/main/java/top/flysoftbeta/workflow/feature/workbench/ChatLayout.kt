package top.flysoftbeta.workflow.feature.workbench

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.app.panel.ExplorerVariant
import top.flysoftbeta.workflow.app.panel.PanelPlacement
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.Region
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

private val ChatSpec = DockingSpec(auxDefault = Region.CHAT_SIDE.defaultSize.toFloat())

internal fun chatDocking(wb: Workbench, windowWidth: Float): DockingDecision = resolveDocking(
    windowWidth,
    sideOpen = !wb.chat.rail.collapsed,
    auxOpen = !wb.chat.side.collapsed,
    sideWidth = wb.chat.rail.size.toFloat(),
    auxWidth = wb.chat.side.size.toFloat(),
    spec = ChatSpec,
)

/**
 * Chat paradigm (docs/ux/README.md §3.4): conversation list ｜ conversation column (the aux stack) ｜ reduced
 * Files view (the focused editor stack itself plus a tree column). Only region weights change when
 * switching paradigm; panels keep their controllers and movable content.
 */
@Composable
internal fun ChatWorkbench(env: WorkbenchEnv, session: Session, wb: Workbench, windowWidth: Dp, facts: FrameFacts, attention: Boolean) {
    val presence = env.runtime.presence
    val gap = WorkflowTheme.dimens.gap
    val decision = chatDocking(wb, windowWidth.value)
    presence.sideDockable = resolveDocking(windowWidth.value, true, false, wb.chat.rail.size.toFloat(), 0f, ChatSpec).sideDocked
    presence.auxDockable = resolveDocking(
        windowWidth.value, decision.sideDocked, true, wb.chat.rail.size.toFloat(), wb.chat.side.size.toFloat(), ChatSpec,
    ).auxDocked
    val railShown = decision.sideDocked || presence.sideOverlay
    val sideShown = decision.auxDocked || presence.auxOverlay
    val left: @Composable RowScope.() -> Unit = { LeftCluster(env, session, Region.CHAT_RAIL, railShown) }
    val right: @Composable RowScope.() -> Unit = { RightCluster(env, Region.CHAT_SIDE, sideShown, attention) }

    Box(Modifier.fillMaxSize()) {
        RegionRow(
            sideWidth = if (decision.sideDocked) decision.sideWidth.dp else null,
            auxWidth = if (decision.auxDocked) decision.auxWidth.dp else null,
            sideLimits = Region.CHAT_RAIL.minSize.toFloat()..(0.4f * windowWidth.value).coerceAtLeast(Region.CHAT_RAIL.minSize.toFloat()),
            auxLimits = Region.CHAT_SIDE.minSize.toFloat()..(0.55f * windowWidth.value).coerceAtLeast(Region.CHAT_SIDE.minSize.toFloat()),
            onResize = { side, width -> env.layout(LayoutOp.ResizeRegion(if (side) Region.CHAT_RAIL else Region.CHAT_SIDE, width.toDouble())) },
            onCollapse = { side -> env.layout(LayoutOp.SetRegionCollapsed(if (side) Region.CHAT_RAIL else Region.CHAT_SIDE, true)) },
            onReset = { side ->
                val region = if (side) Region.CHAT_RAIL else Region.CHAT_SIDE
                env.layout(LayoutOp.ResizeRegion(region, region.defaultSize))
            },
            onResizing = { env.runtime.resizing = it },
            modifier = Modifier.fillMaxSize().padding(gap),
            side = { if (decision.sideDocked) RailRegion(env, session, wb, leading = left, Modifier.fillMaxSize()) },
            center = {
                StackView(
                    env, session, wb, Workbench.AUX, PanelPlacement.CHAT_MAIN, facts, Modifier.fillMaxSize(),
                    leading = if (decision.sideDocked) null else left,
                    trailing = if (decision.auxDocked) null else right,
                    headerStyle = HeaderStyle.Title,
                )
            },
            aux = { if (decision.auxDocked) ChatSide(env, session, wb, facts, trailing = right, Modifier.fillMaxSize()) },
        )
        RegionOverlays(
            env = env,
            sideOpen = !decision.sideDocked && presence.sideOverlay,
            auxOpen = !decision.auxDocked && presence.auxOverlay,
            narrow = decision.narrow,
            sideWidth = decision.sideOverlayWidth.dp,
            auxWidth = if (decision.narrow) decision.auxOverlayWidth.dp else minOf(decision.auxOverlayWidth, wb.chat.side.size.toFloat().coerceAtLeast(300f)).dp,
            side = { RailRegion(env, session, wb, leading = null, Modifier.fillMaxSize()) },
            aux = { ChatSide(env, session, wb, facts, trailing = null, Modifier.fillMaxSize()) },
        )
    }
}

/** The conversation list: header `[corner cluster]─[⌕][✎]` and the rail content. */
@Composable
private fun RailRegion(env: WorkbenchEnv, session: Session, wb: Workbench, leading: (@Composable RowScope.() -> Unit)?, modifier: Modifier) {
    val rail = env.runtime.railController()
    Column(modifier) {
        RegionHeader(
            leading = leading,
            actions = rail.actions,
            moreGroups = listOf(
                MenuGroup("resource", rail.resourceMenu),
                MenuGroup("layout", listOf(
                    top.flysoftbeta.workflow.ui.design.MenuEntry.Action("collapseRail", "收起侧栏", Sym.LeftPanelClose) { hideRegion(env, Region.CHAT_RAIL) },
                )),
                sessionGroup(env, session, wb),
            ),
        )
        RegionCard(Modifier.fillMaxWidth().weight(1f)) { env.railContent(Modifier.fillMaxSize()) }
    }
}

/**
 * The reduced Files view (docs/ux/README.md §3.4): the side stack (Files' focused editor stack itself, `[1/3 ▾]`
 * switches), 🗀 toggles the 200dp tree column on its right, ⤢ returns to Files. With no open file the
 * tree fills the region.
 */
@Composable
private fun ChatSide(env: WorkbenchEnv, session: Session, wb: Workbench, facts: FrameFacts, trailing: (@Composable RowScope.() -> Unit)?, modifier: Modifier) {
    val sideStack = wb.chat.sideStack
    val empty = wb.stacks[sideStack]?.panels.isNullOrEmpty()
    val treeOpen = !wb.chat.tree.collapsed || empty
    val candidates = wb.editorStacks.filter { !wb.stacks[it]?.panels.isNullOrEmpty() || it == sideStack } + Workbench.BOTTOM
    val shellActions = listOf(
        ToolAction("tree", Sym.AccountTree, "文件树", checked = treeOpen) { env.layout(LayoutOp.ToggleRegion(Region.CHAT_TREE)) },
        ToolAction("toFiles", Sym.OpenInFull, "回到 Files") { env.layout(LayoutOp.ReturnToFiles) },
    )
    Column(modifier) {
        StackHeader(
            env, session, wb, sideStack, facts,
            leading = { StackSwitcher(env, wb, sideStack, candidates) { env.layout(LayoutOp.SetChatSideStack(it)) } },
            trailing = trailing, headerStyle = HeaderStyle.Tabs, shellActions = shellActions,
        )
        val treeCard: @Composable () -> Unit = {
            RegionCard(Modifier.fillMaxSize()) { env.explorerContent(ExplorerVariant.TREE_COLUMN, Modifier.fillMaxSize()) }
        }
        val stackCard: @Composable () -> Unit = {
            RegionCard(Modifier.fillMaxSize()) { StackBody(env, wb, sideStack, PanelPlacement.EDITOR, facts) }
        }
        Box(Modifier.fillMaxWidth().weight(1f)) {
            when {
                empty -> treeCard()
                !treeOpen -> stackCard()
                else -> RegionRow(
                    sideWidth = null,
                    auxWidth = wb.chat.tree.size.toFloat().dp,
                    sideLimits = 0f..0f,
                    auxLimits = Region.CHAT_TREE.minSize.toFloat()..(wb.chat.side.size.toFloat() * 0.6f).coerceAtLeast(Region.CHAT_TREE.minSize.toFloat()),
                    onResize = { _, width -> env.layout(LayoutOp.ResizeRegion(Region.CHAT_TREE, width.toDouble())) },
                    onCollapse = { env.layout(LayoutOp.SetRegionCollapsed(Region.CHAT_TREE, true)) },
                    onReset = { env.layout(LayoutOp.ResizeRegion(Region.CHAT_TREE, Region.CHAT_TREE.defaultSize)) },
                    onResizing = { env.runtime.resizing = it },
                    side = {},
                    center = stackCard,
                    aux = treeCard,
                    modifier = Modifier.fillMaxSize(),
                )
            }
        }
    }
}
