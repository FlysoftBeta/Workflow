package top.flysoftbeta.workflow.ui.design

import androidx.annotation.DrawableRes
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.gestures.animateScrollBy
import androidx.compose.foundation.gestures.scrollBy
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.material3.VerticalDivider
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.graphics.BlendMode
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.CompositingStrategy
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.layout.positionInParent
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.ui.design.dnd.DragDropState
import top.flysoftbeta.workflow.ui.design.dnd.DragPayload
import top.flysoftbeta.workflow.ui.design.dnd.DropTargetKind
import top.flysoftbeta.workflow.ui.design.dnd.PanelDragPayload
import top.flysoftbeta.workflow.ui.design.dnd.dragAutoScroll
import top.flysoftbeta.workflow.ui.design.dnd.dragSource
import top.flysoftbeta.workflow.ui.design.dnd.dropTarget
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme
import top.flysoftbeta.workflow.ui.design.theme.frame

/** One tab of a Stack (docs/ui.md §4.1). [caption] disambiguates equal names (parent directory). */
@Immutable
data class TabItem(
    val key: String,
    val title: String,
    @param:DrawableRes val icon: Int? = null,
    val dirty: Boolean = false,
    val caption: String? = null,
)

/**
 * A surfaced action of the focused Panel (save, undo, …), listed from high to low priority. With [menu]
 * the button opens an anchored [CompositeMenu] instead of calling [onClick] (the explorer's `＋`); when
 * collapsed into More it becomes a submenu.
 */
@Immutable
data class ToolAction(
    val key: String,
    @param:DrawableRes val icon: Int,
    val label: String,
    val enabled: Boolean = true,
    val checked: Boolean? = null,
    val menu: List<MenuGroup>? = null,
    val onClick: () -> Unit = {},
)

/** Renders one [ToolAction]: an icon button, or an icon button anchoring its menu. */
@Composable
fun ToolActionButton(action: ToolAction, modifier: Modifier = Modifier, badge: Color? = null) {
    val menu = action.menu
    if (menu == null) {
        WfIconButton(action.icon, action.label, action.onClick, modifier, enabled = action.enabled, checked = action.checked, badge = badge)
    } else {
        var open by remember { mutableStateOf(false) }
        Box(modifier) {
            WfIconButton(action.icon, action.label, { open = true }, enabled = action.enabled, checked = action.checked, badge = badge)
            CompositeMenu(expanded = open, onDismissRequest = { open = false }, groups = menu)
        }
    }
}

/** A collapsed [ToolAction] as a More-menu entry. */
fun ToolAction.asMenuEntry(): MenuEntry = when (val groups = menu) {
    null -> MenuEntry.Action(key, label, icon, enabled = enabled, onClick = onClick)
    else -> MenuEntry.Submenu(key, label, icon, groups, enabled = enabled)
}

/** Requests a one-time highlight pulse on [key] (re-opening an already open resource). Change [nonce] to repeat. */
@Immutable
data class TabPulse(val key: String, val nonce: Int)

/** Drag-and-drop wiring of a tab row: tabs become [PanelDragPayload] sources, the row a sort target. */
class StackTabDragDrop(
    val state: DragDropState,
    val stackId: String,
    val accepts: (DragPayload) -> Boolean = { it is PanelDragPayload },
    /** Single commit: [insertionIndex] counts positions with the dragged tab still in place. */
    val onDrop: (payload: DragPayload, insertionIndex: Int) -> Unit,
)

/**
 * The one-row Stack header: `[leading][tabs (scroll)][⌄n][surfaced actions][⋯][│ trailing]`.
 * Surfaced [actions] collapse into the top of the More menu (lowest priority first) when the tab area
 * would drop below 120dp. The active tab joins the content (surface); in the [focused] Stack it carries
 * a 2dp primary indicator. Long-pressing a tab without moving opens [tabMenuGroups].
 */
@OptIn(ExperimentalFoundationApi::class)
@Composable
fun StackTabBar(
    tabs: List<TabItem>,
    activeKey: String?,
    onSelect: (String) -> Unit,
    onClose: (String) -> Unit,
    modifier: Modifier = Modifier,
    focused: Boolean = true,
    actions: List<ToolAction> = emptyList(),
    moreGroups: List<MenuGroup> = emptyList(),
    tabMenuGroups: ((TabItem) -> List<MenuGroup>)? = null,
    leading: (@Composable RowScope.() -> Unit)? = null,
    trailing: (@Composable RowScope.() -> Unit)? = null,
    dragDrop: StackTabDragDrop? = null,
    pulse: TabPulse? = null,
) {
    val dimens = WorkflowTheme.dimens
    val density = LocalDensity.current
    var leadingWidth by remember { mutableIntStateOf(0) }
    var trailingWidth by remember { mutableIntStateOf(0) }
    val scroll = rememberScrollState()
    val extents = remember { mutableStateMapOf<String, ClosedFloatingPointRange<Float>>() }
    var viewportWidth by remember { mutableIntStateOf(0) }

    val hidden = tabs.count { tab ->
        val e = extents[tab.key] ?: return@count false
        e.start < scroll.value - 1 || e.endInclusive > scroll.value + viewportWidth + 1
    }

    BoxWithConstraints(modifier.fillMaxWidth().heightIn(min = dimens.bar).background(WorkflowTheme.colors.frame)) {
        val touch = dimens.iconButtonTouch
        val fixed = with(density) { (leadingWidth + trailingWidth).toDp() } + touch /* More */ +
            (if (hidden > 0) touch + 12.dp else 0.dp)
        val room = maxWidth - fixed - dimens.surfacedActionsMinTabSpace
        val visibleCount = (room / touch).toInt().coerceIn(0, actions.size)
        val shown = actions.take(visibleCount)
        val collapsed = actions.drop(visibleCount)
        val allMore = buildList {
            if (collapsed.isNotEmpty()) add(MenuGroup("surfaced", collapsed.map { it.asMenuEntry() }))
            addAll(moreGroups)
        }

        Row(Modifier.fillMaxWidth().height(dimens.bar), verticalAlignment = Alignment.CenterVertically) {
            if (leading != null) {
                Row(Modifier.onSizeChanged { leadingWidth = it.width }, verticalAlignment = Alignment.CenterVertically, content = leading)
            }
            Box(
                Modifier
                    .weight(1f)
                    .fillMaxHeight()
                    .onSizeChanged { viewportWidth = it.width }
                    .then(
                        if (dragDrop != null) {
                            Modifier
                                .dropTarget(
                                    dragDrop.state, key = "tabs:${dragDrop.stackId}",
                                    kind = DropTargetKind.Sort(horizontal = true) {
                                        tabs.mapNotNull { extents[it.key] }.map { (it.start - scroll.value)..(it.endInclusive - scroll.value) }
                                    },
                                    priority = 1,
                                    accepts = dragDrop.accepts,
                                    onDrop = { payload, result -> dragDrop.onDrop(payload, result.insertionIndex ?: tabs.size) },
                                )
                                .dragAutoScroll(dragDrop.state, horizontal = true) { scroll.scrollBy(it) }
                        } else Modifier
                    )
                    .edgeFades(scroll.value > 0, scroll.value < scroll.maxValue),
            ) {
                Row(Modifier.fillMaxHeight().horizontalScroll(scroll), verticalAlignment = Alignment.Bottom) {
                    tabs.forEachIndexed { index, tab ->
                        StackTab(
                            item = tab,
                            active = tab.key == activeKey,
                            focused = focused,
                            onClick = { onSelect(tab.key) },
                            onClose = { onClose(tab.key) },
                            menuGroups = tabMenuGroups?.let { { it(tab) } },
                            dragDrop = dragDrop,
                            index = index,
                            pulseNonce = pulse?.takeIf { it.key == tab.key }?.nonce,
                            modifier = Modifier.onGloballyPositioned {
                                val x = it.positionInParent().x
                                extents[tab.key] = x..(x + it.size.width)
                            },
                        )
                    }
                }
            }
            // Keep the active tab in view.
            LaunchedEffect(activeKey, extents[activeKey], viewportWidth) {
                val e = extents[activeKey] ?: return@LaunchedEffect
                val start = scroll.value
                val end = start + viewportWidth
                val delta = when {
                    e.start < start -> e.start - start
                    e.endInclusive > end -> e.endInclusive - end
                    else -> 0f
                }
                if (delta != 0f) scroll.animateScrollBy(delta)
            }
            if (hidden > 0) TabOverflowButton(hidden, tabs, activeKey, onSelect)
            shown.forEach { ToolActionButton(it) }
            if (allMore.isNotEmpty()) MoreMenuButton(allMore)
            if (trailing != null) {
                Row(Modifier.onSizeChanged { trailingWidth = it.width }, verticalAlignment = Alignment.CenterVertically) {
                    VerticalDivider(Modifier.height(20.dp).padding(horizontal = 0.dp), color = WorkflowTheme.colors.outlineVariant)
                    trailing()
                }
            }
        }
    }
}

private fun Modifier.edgeFades(start: Boolean, end: Boolean, width: Dp = 16.dp): Modifier =
    if (!start && !end) this else this
        .graphicsLayer(compositingStrategy = CompositingStrategy.Offscreen)
        .drawWithContent {
            drawContent()
            val fade = width.toPx()
            if (start) drawRect(
                Brush.horizontalGradient(0f to Color.Transparent, 1f to Color.Black, startX = 0f, endX = fade),
                size = size.copy(width = fade), blendMode = BlendMode.DstIn,
            )
            if (end) drawRect(
                Brush.horizontalGradient(0f to Color.Black, 1f to Color.Transparent, startX = size.width - fade, endX = size.width),
                topLeft = androidx.compose.ui.geometry.Offset(size.width - fade, 0f),
                size = size.copy(width = fade), blendMode = BlendMode.DstIn,
            )
        }

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun StackTab(
    item: TabItem,
    active: Boolean,
    focused: Boolean,
    onClick: () -> Unit,
    onClose: () -> Unit,
    menuGroups: (() -> List<MenuGroup>)?,
    dragDrop: StackTabDragDrop?,
    index: Int,
    pulseNonce: Int?,
    modifier: Modifier = Modifier,
) {
    val dimens = WorkflowTheme.dimens
    val colors = WorkflowTheme.colors
    val text = WorkflowTheme.text
    val haptics = LocalHapticFeedback.current
    var menuOpen by remember { mutableStateOf(false) }
    val pulse = remember { Animatable(0f) }
    LaunchedEffect(pulseNonce) {
        if (pulseNonce == null) return@LaunchedEffect
        pulse.snapTo(0.28f)
        pulse.animateTo(0f, tween(600))
    }
    val gestures = when {
        dragDrop != null -> Modifier
            .dragSource(
                dragDrop.state, key = "tab:${dragDrop.stackId}:${item.key}",
                onLongPressWithoutMove = if (menuGroups != null) ({ menuOpen = true }) else null,
            ) { PanelDragPayload(item.key, dragDrop.stackId, index, item.title, item.icon ?: Sym.Draft) }
            .clickable(onClick = onClick)
        menuGroups != null -> Modifier.combinedClickable(
            onClick = onClick,
            onLongClick = { haptics.performHapticFeedback(HapticFeedbackType.LongPress); menuOpen = true },
        )
        else -> Modifier.clickable(onClick = onClick)
    }
    Box(modifier) {
        Box(
            Modifier
                .height(dimens.bar)
                .widthIn(min = dimens.tabMinWidth, max = dimens.tabMaxWidth)
                .clip(WorkflowShapes.tab)
                .background(if (active) colors.surface else Color.Transparent)
                .background(colors.primary.copy(alpha = pulse.value))
                .drawWithContent {
                    drawContent()
                    // 2dp primary indicator on the focused Stack's active tab (drawn, so it never widens the tab).
                    if (active && focused) drawRect(colors.primary, size = size.copy(height = 2.dp.toPx()))
                }
                .then(gestures)
                .semantics {
                    role = Role.Tab
                    selected = active
                    if (item.dirty) stateDescription = "未保存"
                },
        ) {
            Row(
                Modifier.fillMaxHeight().padding(start = 10.dp, end = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                if (item.icon != null) {
                    SymbolIcon(item.icon, null, size = dimens.tabIcon, tint = if (active) colors.primary else colors.onSurfaceVariant)
                    Spacer(Modifier.width(6.dp))
                }
                Text(
                    item.title,
                    Modifier.weight(1f, fill = false),
                    style = if (active) text.labelActive else text.label,
                    color = if (active) colors.onSurface else colors.onSurfaceVariant,
                    maxLines = 1, overflow = TextOverflow.Ellipsis,
                )
                if (item.caption != null) {
                    Spacer(Modifier.width(4.dp))
                    Text(item.caption, style = text.caption, color = colors.onSurfaceVariant, maxLines = 1)
                }
                Spacer(Modifier.width(2.dp))
                TabTrailingSlot(item.dirty, focused && active, onClose)
            }
        }
        if (menuGroups != null) {
            CompositeMenu(expanded = menuOpen, onDismissRequest = { menuOpen = false }, groups = if (menuOpen) menuGroups() else emptyList())
        }
    }
}

/** The trailing slot uses the toolbar touch size; the dirty dot and close glyph stay visually compact. */
@Composable
private fun TabTrailingSlot(dirty: Boolean, emphasized: Boolean, onClose: () -> Unit) {
    val colors = WorkflowTheme.colors
    Box(
        Modifier
            .width(WorkflowTheme.dimens.iconButtonTouch)
            .fillMaxHeight()
            .clip(CircleShape)
            .clickable(onClick = onClose)
            .semantics { contentDescription = if (dirty) "关闭未保存标签" else "关闭标签" },
        contentAlignment = Alignment.Center,
    ) {
        if (dirty) {
            Box(Modifier.size(WorkflowTheme.dimens.dirtyDot).background(if (emphasized) colors.primary else colors.onSurfaceVariant, CircleShape))
        } else {
            SymbolIcon(Sym.Close, "关闭", size = 16.dp, tint = colors.onSurfaceVariant)
        }
    }
}

@Composable
private fun TabOverflowButton(hidden: Int, tabs: List<TabItem>, activeKey: String?, onSelect: (String) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Box {
        Row(
            Modifier
                .height(WorkflowTheme.dimens.iconButtonTouch)
                .clip(WorkflowShapes.sm)
                .clickable { open = true }
                .padding(horizontal = 6.dp)
                .semantics { this.role = Role.DropdownList },
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.Center,
        ) {
            SymbolIcon(Sym.KeyboardArrowDown, "全部标签", size = WorkflowTheme.dimens.iconSmall, tint = WorkflowTheme.colors.onSurfaceVariant)
            Text("$hidden", style = WorkflowTheme.text.caption, color = WorkflowTheme.colors.onSurfaceVariant)
        }
        ItemListMenu(
            expanded = open,
            onDismissRequest = { open = false },
            items = tabs.map { ListMenuItem(it.key, it.title, it.icon, it.caption, dot = it.dirty) },
            selectedKey = activeKey,
            onSelect = { open = false; onSelect(it) },
        )
    }
}

/** A clickable header row of bar height for regions without tabs (file browser, conversation list, conversation column). */
@Composable
fun RegionHeader(
    modifier: Modifier = Modifier,
    leading: (@Composable RowScope.() -> Unit)? = null,
    title: (@Composable RowScope.() -> Unit)? = null,
    actions: List<ToolAction> = emptyList(),
    moreGroups: List<MenuGroup> = emptyList(),
    trailing: (@Composable RowScope.() -> Unit)? = null,
) {
    val dimens = WorkflowTheme.dimens
    val density = LocalDensity.current
    var leadingWidth by remember { mutableIntStateOf(0) }
    var trailingWidth by remember { mutableIntStateOf(0) }
    BoxWithConstraints(modifier.fillMaxWidth().heightIn(min = dimens.bar).background(WorkflowTheme.colors.frame)) {
        val fixed = with(density) { (leadingWidth + trailingWidth).toDp() } + dimens.iconButtonTouch
        val room = maxWidth - fixed - if (title == null) 0.dp else dimens.surfacedActionsMinTabSpace
        val visibleCount = (room / dimens.iconButtonTouch).toInt().coerceIn(0, actions.size)
        val allMore = buildList {
            if (visibleCount < actions.size) add(MenuGroup("surfaced", actions.drop(visibleCount).map { it.asMenuEntry() }))
            addAll(moreGroups)
        }
        Row(Modifier.fillMaxWidth().heightIn(min = dimens.bar), verticalAlignment = Alignment.CenterVertically) {
            if (leading != null) Row(Modifier.onSizeChanged { leadingWidth = it.width }, verticalAlignment = Alignment.CenterVertically, content = leading)
            Row(Modifier.weight(1f).padding(horizontal = 8.dp), verticalAlignment = Alignment.CenterVertically) { title?.invoke(this) }
            actions.take(visibleCount).forEach { ToolActionButton(it) }
            if (allMore.isNotEmpty()) MoreMenuButton(allMore)
            if (trailing != null) Row(Modifier.onSizeChanged { trailingWidth = it.width }, verticalAlignment = Alignment.CenterVertically) {
                VerticalDivider(Modifier.height(20.dp), color = WorkflowTheme.colors.outlineVariant)
                trailing()
            }
        }
    }
}

/**
 * Session chip of the top-left corner cluster (docs/ui.md §2.1): 64–120dp wide, the persistent name or
 * the start time of a temporary session. Tap opens the session view, long press names it.
 */
@OptIn(ExperimentalFoundationApi::class)
@Composable
fun SessionChip(label: String, onClick: () -> Unit, onLongClick: () -> Unit, modifier: Modifier = Modifier) {
    val haptics = LocalHapticFeedback.current
    Row(
        modifier
            .padding(horizontal = 2.dp)
            .heightIn(min = WorkflowTheme.dimens.iconButtonTouch)
            .widthIn(min = 64.dp, max = 120.dp)
            .clip(WorkflowShapes.sm)
            .background(WorkflowTheme.colors.surfaceContainerHigh)
            .combinedClickable(
                interactionSource = remember { MutableInteractionSource() },
                indication = androidx.compose.material3.ripple(),
                onClick = onClick,
                onLongClick = { haptics.performHapticFeedback(HapticFeedbackType.LongPress); onLongClick() },
            )
            .padding(start = 8.dp, end = 2.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            label, Modifier.weight(1f, fill = false), style = WorkflowTheme.text.label,
            color = WorkflowTheme.colors.onSurface, maxLines = 1, overflow = TextOverflow.Ellipsis,
        )
        SymbolIcon(Sym.ArrowDropDown, null, size = 18.dp, tint = WorkflowTheme.colors.onSurfaceVariant)
    }
}
