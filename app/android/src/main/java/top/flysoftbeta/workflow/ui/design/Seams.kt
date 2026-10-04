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
 * A seam claims a drag only when the movement is predominantly along its axis: at least [SEAM_AXIS_RATIO]
 * times the movement across it, which is within about 27° of the axis.
 */
internal const val SEAM_AXIS_RATIO = 2f

internal enum class SeamClaim { Undecided, Claim, Release }

/**
 * Whether a pointer that went down on a seam's band is a seam drag, from its total movement [along] and
 * [across] the seam's drag axis. Within [slop] of the down position nothing is decided. Beyond it, a
 * movement steeper than [ratio] belongs to the content (a scroll or a scrollbar drag next to the seam);
 * otherwise the seam claims it once the movement along the axis exceeds [slop].
 */
internal fun seamClaim(along: Float, across: Float, slop: Float, ratio: Float = SEAM_AXIS_RATIO): SeamClaim {
    val a = abs(along)
    val c = abs(across)
    return when {
        a * a + c * c <= slop * slop -> SeamClaim.Undecided
        a < ratio * c -> SeamClaim.Release
        a > slop -> SeamClaim.Claim
        else -> SeamClaim.Undecided
    }
}

/**
 * Seam gestures observed in the Initial pass of the container, before the content (including an embedded
 * WebView) sees the touch. Until [seamClaim] decides, nothing is consumed and the content keeps receiving
 * the stream; a released gesture is left to the content entirely. Two quick taps on a seam = double tap.
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
                        when (seamClaim(moved, across, slop)) {
                            SeamClaim.Undecided -> continue
                            SeamClaim.Release -> break // not ours: the content scrolls or drags across the seam
                            SeamClaim.Claim -> { dragging = true; current.onDragStart(index) }
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
