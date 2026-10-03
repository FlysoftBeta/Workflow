package top.flysoftbeta.workflow.ui.design

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import kotlin.math.abs

/** Callbacks of a set of draggable seams (dividers) inside one container. */
internal interface SeamCallbacks {
    /** Seam centres along the axis, in the container's px. */
    fun seams(): List<Float>
    fun onPress(index: Int, pressed: Boolean) {}
    fun onDragStart(index: Int) {}
    /** [position]: pointer position along the axis, container px. */
    fun onDrag(index: Int, position: Float)
    fun onDragEnd(index: Int) {}
    fun onDoubleTap(index: Int) {}
}

private class TapMemory { var index = -1; var time = 0L }

/**
 * Seam gestures observed in the Initial pass of the container, so the content keeps receiving touches:
 * a down within [bandPx]/2 of a seam only becomes a drag after moving past touch slop along the axis;
 * a gesture that moves across the axis first is left alone. Two quick taps on a seam = double tap.
 */
internal fun Modifier.seamGestures(horizontal: Boolean, bandPx: Float, callbacks: SeamCallbacks): Modifier = composed {
    val current by rememberUpdatedState(callbacks)
    val taps = remember { TapMemory() }
    pointerInput(horizontal, bandPx) {
        awaitEachGesture {
            val down = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
            val along = { p: Offset -> if (horizontal) p.x else p.y }
            val acrossOf = { p: Offset -> if (horizontal) p.y else p.x }
            val seams = current.seams()
            val index = seams.indices.minByOrNull { abs(seams[it] - along(down.position)) } ?: return@awaitEachGesture
            if (abs(seams[index] - along(down.position)) > bandPx / 2f) return@awaitEachGesture
            val slop = viewConfiguration.touchSlop
            var dragging = false
            current.onPress(index, true)
            try {
                while (true) {
                    val event = awaitPointerEvent(PointerEventPass.Initial)
                    val change = event.changes.firstOrNull { it.id == down.id } ?: break
                    if (!change.pressed) {
                        if (!dragging) {
                            val now = change.uptimeMillis
                            if (taps.index == index && now - taps.time < 300) {
                                taps.index = -1
                                current.onDoubleTap(index)
                            } else {
                                taps.index = index; taps.time = now
                            }
                        }
                        break
                    }
                    if (!dragging) {
                        val moved = along(change.position) - along(down.position)
                        val across = acrossOf(change.position) - acrossOf(down.position)
                        if (abs(across) > slop && abs(moved) <= slop) break // not ours: a scroll across the seam
                        if (abs(moved) > slop) {
                            dragging = true
                            current.onDragStart(index)
                        }
                    }
                    if (dragging) {
                        change.consume()
                        current.onDrag(index, along(change.position))
                    }
                }
            } finally {
                current.onPress(index, false)
                if (dragging) current.onDragEnd(index)
            }
        }
    }
}
