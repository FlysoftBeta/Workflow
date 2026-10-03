package top.flysoftbeta.workflow.ui.design.dnd

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.CancellationException

/** Long-press duration that picks an item up (docs/ux/README.md §4.1, §4.10). */
const val DRAG_PICKUP_MILLIS = 300L

private class CoordinatesHolder { var coordinates: LayoutCoordinates? = null }

private fun DragDropState.toHost(source: LayoutCoordinates?, local: Offset): Offset {
    val host = hostCoordinates
    return if (host != null && source != null && host.isAttached && source.isAttached) host.localPositionOf(source, local) else local
}

/**
 * Makes the element a drag source: a 300ms long press picks it up (LONG_PRESS haptic, the element
 * drops to 40% opacity, a shadow chip follows the finger). Releasing without moving past touch slop
 * calls [onLongPressWithoutMove] instead (show the same context menu). Taps and scrolls that start
 * before the pickup are left to other handlers (clickable, scroll containers).
 *
 * [payload] is read at pickup time; returning null disables the drag for that gesture.
 */
fun Modifier.dragSource(
    state: DragDropState,
    key: Any,
    enabled: Boolean = true,
    onLongPressWithoutMove: (() -> Unit)? = null,
    payload: () -> DragPayload?,
): Modifier = composed {
    val holder = remember { CoordinatesHolder() }
    val haptics = LocalHapticFeedback.current
    val currentPayload by rememberUpdatedState(payload)
    val currentLongPress by rememberUpdatedState(onLongPressWithoutMove)
    this
        .semantics {
            if (enabled && currentLongPress != null) {
                customActions = listOf(CustomAccessibilityAction("移动、排序及其他操作") {
                    currentLongPress?.invoke()
                    true
                })
            }
        }
        .onGloballyPositioned { holder.coordinates = it }
        .graphicsLayer { alpha = if (state.isActiveSource(key) && state.session?.returning == false) 0.4f else 1f }
        .pointerInput(state, key, enabled) {
            if (!enabled) return@pointerInput
            awaitEachGesture {
                val down = awaitFirstDown(requireUnconsumed = false)
                val slop = viewConfiguration.touchSlop
                var position = down.position
                // Wait for the pickup; a release, a move beyond slop or a consumer (scroll) aborts it.
                val aborted = withTimeoutOrNull(DRAG_PICKUP_MILLIS) {
                    while (true) {
                        val event = awaitPointerEvent()
                        val change = event.changes.firstOrNull { it.id == down.id } ?: return@withTimeoutOrNull true
                        if (!change.pressed || change.isConsumed) return@withTimeoutOrNull true
                        position = change.position
                        if ((change.position - down.position).getDistance() > slop) return@withTimeoutOrNull true
                    }
                    @Suppress("UNREACHABLE_CODE") false
                }
                if (aborted == true) return@awaitEachGesture
                val value = currentPayload() ?: return@awaitEachGesture
                haptics.performHapticFeedback(HapticFeedbackType.LongPress)
                val source = holder.coordinates
                val origin = source?.let { state.toHost(it, Offset(it.size.width / 2f, it.size.height / 2f)) }
                state.start(value, key, origin, state.toHost(source, position))
                val pickup = position
                var moved = false
                try {
                    while (true) {
                        // Initial pass: consume before clickable / scroll containers see the events.
                        val event = awaitPointerEvent(PointerEventPass.Initial)
                        event.changes.forEach { it.consume() }
                        val change = event.changes.firstOrNull { it.id == down.id }
                        val stillOurs = state.session?.sourceKey == key && state.session?.returning == false
                        if (change == null || !change.pressed) {
                            if (stillOurs) {
                                if (moved) state.drop() else { state.clear(); currentLongPress?.invoke() }
                            }
                            break
                        }
                        if (!stillOurs) continue // cancelled (Back): swallow the rest of the gesture
                        if (!moved && (change.position - pickup).getDistance() > slop) moved = true
                        if (moved) state.move(state.toHost(holder.coordinates, change.position))
                    }
                } catch (e: CancellationException) {
                    if (state.session?.sourceKey == key) state.cancel()
                    throw e
                }
            }
        }
}

/**
 * Registers the element as a drop target of [state]. [onDrop] is the single commit callback, invoked
 * once on release over this target when [accepts] returned true. [onHoverActivate] fires after the
 * pointer rests on the target for 600ms (expand a folder, open a collapsed region).
 * When targets overlap, the higher [priority] wins, then the smaller one.
 */
fun Modifier.dropTarget(
    state: DragDropState,
    key: Any,
    kind: DropTargetKind,
    priority: Int = 0,
    accepts: (DragPayload) -> Boolean = { true },
    onHoverActivate: (() -> Unit)? = null,
    onDrop: (DragPayload, DropResult) -> Unit,
): Modifier = composed {
    val entry = remember(state, key) { DropTargetEntry(key) }
    entry.kind = kind
    entry.priority = priority
    entry.accepts = accepts
    entry.onDrop = onDrop
    entry.onHoverActivate = onHoverActivate
    DisposableEffect(state, key) {
        state.targets[key] = entry
        onDispose { if (state.targets[key] === entry) state.targets.remove(key) }
    }
    this.onGloballyPositioned { coordinates ->
        val host = state.hostCoordinates
        if (host != null && host.isAttached) {
            entry.bounds = host.localBoundingBoxOf(coordinates, clipBounds = true)
            entry.origin = host.localPositionOf(coordinates, Offset.Zero)
        }
        entry.size = Size(coordinates.size.width.toFloat(), coordinates.size.height.toFloat())
    }
}

/**
 * Scrolls a list or tab row while a drag hovers within 32dp of its ends (docs/ux/README.md §4.10).
 * [scrollBy] is typically `scrollState::scrollBy` / `lazyListState::scrollBy`.
 */
fun Modifier.dragAutoScroll(
    state: DragDropState,
    horizontal: Boolean,
    scrollBy: suspend (Float) -> Float,
): Modifier = composed {
    val holder = remember { CoordinatesHolder() }
    val density = LocalDensity.current
    val edge = with(density) { 32.dp.toPx() }
    val maxSpeed = with(density) { 900.dp.toPx() }
    val currentScroll by rememberUpdatedState(scrollBy)
    val dragging = state.isDragging
    LaunchedEffect(dragging) {
        if (!dragging) return@LaunchedEffect
        var last = withFrameNanos { it }
        while (state.isDragging) {
            val now = withFrameNanos { it }
            val dt = (now - last) / 1_000_000_000f
            last = now
            val coordinates = holder.coordinates ?: continue
            val host = state.hostCoordinates ?: continue
            if (!coordinates.isAttached || !host.isAttached) continue
            val pointer = state.session?.pointer ?: continue
            val bounds = host.localBoundingBoxOf(coordinates, clipBounds = true)
            val inCross = if (horizontal) pointer.y in bounds.top..bounds.bottom else pointer.x in bounds.left..bounds.right
            if (!inCross) continue
            val speed = if (horizontal) {
                DndGeometry.autoScrollSpeed(pointer.x, bounds.left, bounds.right, edge, maxSpeed)
            } else {
                DndGeometry.autoScrollSpeed(pointer.y, bounds.top, bounds.bottom, edge, maxSpeed)
            }
            if (speed != 0f) {
                currentScroll(speed * dt)
                state.move(pointer) // refresh hover/insertion against the moved content
            }
        }
    }
    this.onGloballyPositioned { holder.coordinates = it }
}
