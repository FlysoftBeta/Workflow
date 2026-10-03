package top.flysoftbeta.workflow.ui.design

import androidx.annotation.DrawableRes
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import kotlin.math.roundToInt
import top.flysoftbeta.workflow.ui.design.dnd.DRAG_PICKUP_MILLIS
import top.flysoftbeta.workflow.ui.design.dnd.DndGeometry
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

private val LocalAppTileDragging = staticCompositionLocalOf { false }

/** Launcher tile geometry (docs/ux/README.md §3.1). */
object AppGridMetrics {
    val tileWidth = 88.dp
    val tileHeight = 92.dp
    val iconSize = 56.dp
    val columnWidth = 104.dp
    const val MAX_COLUMNS = 10
}

/** Built-in app glyph on `primaryContainer` (Workbench, Proxy, Settings). */
@Composable
fun BuiltInAppIcon(@DrawableRes glyph: Int, modifier: Modifier = Modifier) {
    Box(
        modifier.size(AppGridMetrics.iconSize).clip(WorkflowShapes.lg).background(WorkflowTheme.colors.primaryContainer),
        contentAlignment = Alignment.Center,
    ) { SymbolIcon(glyph, null, size = 28.dp, tint = WorkflowTheme.colors.onPrimaryContainer) }
}

/**
 * One Launcher tile: 56dp icon + caption in an 88×92 cell. [icon] is a slot (built-in glyph or a cached
 * third-party icon bitmap loaded off the main thread). [badge] draws the 8dp pending dot.
 */
@Composable
fun AppGridCell(
    label: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    badge: Boolean = false,
    icon: @Composable () -> Unit,
) {
    Column(
        modifier
            .size(AppGridMetrics.tileWidth, AppGridMetrics.tileHeight)
            .clip(WorkflowShapes.md)
            .clickable(onClick = onClick)
            .semantics { role = Role.Button }
            .padding(top = 6.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(Modifier.shadow(if (LocalAppTileDragging.current) 3.dp else 0.dp, WorkflowShapes.lg, clip = false)) {
            icon()
            if (badge) Box(Modifier.align(Alignment.TopEnd).offset(2.dp, (-2).dp).size(8.dp).background(WorkflowTheme.colors.tertiary, CircleShape))
        }
        Spacer(Modifier.height(6.dp))
        Text(
            label, Modifier.fillMaxWidth().padding(horizontal = 4.dp), style = WorkflowTheme.text.caption,
            color = WorkflowTheme.colors.onSurface, textAlign = TextAlign.Center, maxLines = 1, overflow = TextOverflow.Ellipsis,
        )
    }
}

/** The dashed "添加" tile. */
@Composable
fun AddAppCell(label: String, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val outline = WorkflowTheme.colors.outline
    AppGridCell(label, onClick, modifier) {
        Box(
            Modifier.size(AppGridMetrics.iconSize).drawBehind {
                val stroke = 1.5.dp.toPx()
                drawRoundRect(
                    outline, topLeft = Offset(stroke / 2, stroke / 2), size = size.copy(size.width - stroke, size.height - stroke),
                    cornerRadius = CornerRadius(20.dp.toPx()),
                    style = Stroke(stroke, pathEffect = PathEffect.dashPathEffect(floatArrayOf(6.dp.toPx(), 4.dp.toPx()))),
                )
            },
            contentAlignment = Alignment.Center,
        ) { SymbolIcon(top.flysoftbeta.workflow.ui.design.icons.Sym.Add, null, tint = WorkflowTheme.colors.onSurfaceVariant) }
    }
}

/**
 * Launcher grid with long-press reordering (docs/ux/README.md §3.1): columns of 104dp (max 10), centred and
 * top-aligned. A 300ms long press lifts a tile (1.08× + shadow); releasing in place calls
 * [onLongPress] (context menu); dragging past slop reorders live — other tiles spring aside — and the
 * release commits once via [onMove]. [trailing] cells (the "添加" tile) are not movable.
 */
@Composable
fun <T> ReorderableAppGrid(
    items: List<T>,
    key: (T) -> Any,
    onMove: (from: Int, to: Int) -> Unit,
    onLongPress: (T) -> Unit,
    modifier: Modifier = Modifier,
    trailing: (@Composable () -> Unit)? = null,
    cell: @Composable (item: T) -> Unit,
) {
    val density = LocalDensity.current
    val haptics = LocalHapticFeedback.current
    var dragFrom by remember { mutableIntStateOf(-1) }
    var dragTo by remember { mutableIntStateOf(-1) }
    var dragOffset by remember { mutableStateOf(Offset.Zero) }
    val currentItems by rememberUpdatedState(items)
    val currentOnMove by rememberUpdatedState(onMove)
    val currentOnLongPress by rememberUpdatedState(onLongPress)

    BoxWithConstraints(modifier.fillMaxWidth()) {
        val columnPx = with(density) { AppGridMetrics.columnWidth.toPx() }
        val rowPx = with(density) { (AppGridMetrics.tileHeight + 8.dp).toPx() }
        val columns = (maxWidth / AppGridMetrics.columnWidth).toInt().coerceIn(1, AppGridMetrics.MAX_COLUMNS)
        val gridWidth = AppGridMetrics.columnWidth * columns
        val startPx = with(density) { ((maxWidth - gridWidth) / 2).toPx() }
        val cellInset = with(density) { ((AppGridMetrics.columnWidth - AppGridMetrics.tileWidth) / 2).toPx() }
        val total = items.size + if (trailing != null) 1 else 0
        val rows = (total + columns - 1) / columns
        fun slotOffset(slot: Int) = Offset(startPx + (slot % columns) * columnPx + cellInset, (slot / columns) * rowPx)

        Box(Modifier.fillMaxWidth().height(with(density) { (rows * rowPx).toDp() })) {
            items.forEachIndexed { index, item ->
                key(key(item)) {
                    val dragging = index == dragFrom
                    val slot = if (dragFrom >= 0 && dragTo >= 0) DndGeometry.reorderedIndex(index, dragFrom, dragTo) else index
                    val target = slotOffset(slot)
                    val animated = remember { Animatable(target, Offset.VectorConverter) }
                    LaunchedEffect(target, dragging) {
                        if (!dragging) animated.animateTo(target, spring(dampingRatio = 0.9f, stiffness = 700f))
                    }
                    val lift by animateFloatAsState(if (dragging) 1.08f else 1f, label = "lift")
                    val position = if (dragging) slotOffset(index) + dragOffset else animated.value
                    Box(
                        Modifier
                            .offset { IntOffset(position.x.roundToInt(), position.y.roundToInt()) }
                            .zIndex(if (dragging) 1f else 0f)
                            .graphicsLayer {
                                scaleX = lift; scaleY = lift
                                shape = WorkflowShapes.md
                                clip = false
                            }
                            .semantics(mergeDescendants = true) {
                                customActions = buildList {
                                    add(CustomAccessibilityAction("显示操作") { currentOnLongPress(item); true })
                                    if (index > 0) add(CustomAccessibilityAction("向前移动") { currentOnMove(index, index - 1); true })
                                    if (index < items.lastIndex) add(CustomAccessibilityAction("向后移动") { currentOnMove(index, index + 1); true })
                                }
                            }
                            .pointerInput(index) {
                                awaitEachGesture {
                                    val down = awaitFirstDown(requireUnconsumed = false)
                                    val slop = viewConfiguration.touchSlop
                                    val aborted = withTimeoutOrNull(DRAG_PICKUP_MILLIS) {
                                        while (true) {
                                            val e = awaitPointerEvent()
                                            val c = e.changes.firstOrNull { it.id == down.id } ?: return@withTimeoutOrNull true
                                            if (!c.pressed || c.isConsumed || (c.position - down.position).getDistance() > slop) return@withTimeoutOrNull true
                                        }
                                        @Suppress("UNREACHABLE_CODE") false
                                    }
                                    if (aborted == true) return@awaitEachGesture
                                    haptics.performHapticFeedback(HapticFeedbackType.LongPress)
                                    dragFrom = index; dragTo = index; dragOffset = Offset.Zero
                                    var moved = false
                                    var released = false
                                    var total = Offset.Zero
                                    try {
                                        while (true) {
                                            val e = awaitPointerEvent(PointerEventPass.Initial)
                                            e.changes.forEach { it.consume() }
                                            val c = e.changes.firstOrNull { it.id == down.id } ?: break
                                            if (!c.pressed) { released = true; break }
                                            // The tile follows the finger, so its local space moves: accumulate the
                                            // per-event change (both positions are mapped with the current layout).
                                            total += c.position - c.previousPosition
                                            val delta = total
                                            if (!moved && delta.getDistance() > slop) moved = true
                                            if (moved) {
                                                dragOffset = delta
                                                val center = slotOffset(index) + delta + Offset(columnPx / 2 - cellInset, rowPx / 2)
                                                dragTo = DndGeometry.gridSlot(
                                                    Offset(center.x - startPx, center.y), columns, columnPx, rowPx, currentItems.size,
                                                )
                                            }
                                        }
                                    } finally {
                                        val from = dragFrom
                                        val to = dragTo
                                        dragFrom = -1; dragTo = -1; dragOffset = Offset.Zero
                                        if (released && !moved) currentItems.getOrNull(index)?.let(currentOnLongPress)
                                        else if (released && from >= 0 && to >= 0 && from != to) currentOnMove(from, to)
                                    }
                                }
                            },
                    ) {
                        CompositionLocalProvider(LocalAppTileDragging provides dragging) { cell(item) }
                    }
                }
            }
            if (trailing != null) {
                val p = slotOffset(items.size)
                Box(Modifier.offset { IntOffset(p.x.roundToInt(), p.y.roundToInt()) }) { trailing() }
            }
        }
    }
}
