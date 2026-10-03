package top.flysoftbeta.workflow.ui.design.dnd

import androidx.compose.runtime.Immutable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.layout.LayoutCoordinates

/** How a drop target interprets the pointer and what feedback it shows. */
@Immutable
sealed interface DropTargetKind {
    /** Stack content area: centre + four edges, previewed as the resulting rectangle. */
    data class Zones(val allowEdges: Boolean = true) : DropTargetKind

    /** A narrow outer band of the complete editor tree; centre delegates to the individual stacks. */
    data class EditorEdges(val bandPx: Float) : DropTargetKind

    /**
     * Sorting within a row (or column): [items] returns the item extents along the axis, relative to the
     * target's own origin, in display order. Feedback is a 2dp × 28dp insertion line; items do not move.
     */
    class Sort(val horizontal: Boolean = true, val items: () -> List<ClosedFloatingPointRange<Float>>) : DropTargetKind

    /** Whole-target feedback (composer "添加为附件", terminal "粘贴路径", folder rows). */
    data class Area(val style: AreaStyle, val label: String? = null) : DropTargetKind
}

enum class AreaStyle {
    /** 2dp primary outline + 8% primary fill + label (composer). */
    Outline,
    /** Translucent layer + centred label (terminal). */
    Scrim,
    /** No overlay; the target draws its own highlight from [DragDropState.isHovered] (tree folder → primaryContainer). */
    Self,
}

/** Delivered once, on release over an accepting target. Coordinates are target-local. */
@Immutable
data class DropResult(
    val zone: DropZone? = null,
    val insertionIndex: Int? = null,
    val position: Offset = Offset.Zero,
)

/** Visual feedback for the current hover, in host coordinates. */
@Immutable
sealed interface DropFeedback {
    data class Placement(val zone: DropZone, val preview: Rect) : DropFeedback
    data class Insertion(val index: Int, val line: Rect) : DropFeedback
    data class Area(val rect: Rect, val style: AreaStyle, val label: String?) : DropFeedback
    /** Over a target that does not accept this payload: the shadow shows ⃠. */
    data object Rejected : DropFeedback
}

internal class DropTargetEntry(val key: Any) {
    var kind: DropTargetKind = DropTargetKind.Zones()
    var priority: Int = 0
    var accepts: (DragPayload) -> Boolean = { true }
    var onDrop: (DragPayload, DropResult) -> Unit = { _, _ -> }
    var onHoverActivate: (() -> Unit)? = null
    /** Visible (clipped) bounds in host coordinates, for hit testing. */
    var bounds: Rect = Rect.Zero
    /** The layout's own origin in host coordinates (differs from bounds when clipped). */
    var origin: Offset = Offset.Zero
    /** Unclipped layout size. */
    var size: Size = Size.Zero
}

/** One drag in progress (or returning to its origin after a cancel). */
@Stable
class DragSession internal constructor(
    val payload: DragPayload,
    val sourceKey: Any?,
    /** Where the shadow returns on cancel (source centre), host coordinates. Null: vanish in place. */
    val origin: Offset?,
    /** Started by a platform (external) drag. */
    val external: Boolean,
    pointer: Offset,
) {
    var pointer by mutableStateOf(pointer)
        internal set
    var targetKey: Any? by mutableStateOf(null)
        internal set
    var feedback: DropFeedback? by mutableStateOf(null)
        internal set
    /** Cancelled; the shadow is animating back to [origin]. */
    var returning by mutableStateOf(false)
        internal set
}

/**
 * Holder of the drag-and-drop state of one [DragDropHost]. Sources start sessions, targets register
 * their bounds; on release exactly one `onDrop` of the target under the pointer is invoked (the single
 * commit, e.g. one `LayoutOp`), or the drag is cancelled and the shadow springs back.
 */
@Stable
class DragDropState {
    internal var hostCoordinates: LayoutCoordinates? = null
    internal val targets = LinkedHashMap<Any, DropTargetEntry>()

    /** Line of the sort indicator, in px; set by the host from the current density. */
    internal var insertionLineThickness = 4f
    internal var insertionLineLength = 56f

    var session: DragSession? by mutableStateOf(null)
        private set

    /** A drag is active (not counting the return animation after a cancel). */
    val isDragging: Boolean get() = session?.let { !it.returning } ?: false

    fun isActiveSource(key: Any): Boolean = session?.sourceKey == key

    /** True while the pointer hovers [key] with an accepted payload (for [AreaStyle.Self] targets). */
    fun isHovered(key: Any): Boolean {
        val s = session ?: return false
        return !s.returning && s.targetKey == key && s.feedback != null && s.feedback != DropFeedback.Rejected
    }

    internal fun start(payload: DragPayload, sourceKey: Any?, origin: Offset?, pointer: Offset, external: Boolean = false) {
        session = DragSession(payload, sourceKey, origin, external, pointer).also { update(it) }
    }

    internal fun move(pointer: Offset) {
        val s = session ?: return
        if (s.returning) return
        s.pointer = pointer
        update(s)
    }

    /** Releases the drag: commits to the hovered target or cancels. Returns true when a target took it. */
    internal fun drop(payloadOverride: DragPayload? = null): Boolean {
        val s = session ?: return false
        if (s.returning) return false
        update(s)
        val entry = s.targetKey?.let { targets[it] }
        val feedback = s.feedback
        if (entry == null || feedback == null || feedback == DropFeedback.Rejected) {
            cancel()
            return false
        }
        val payload = payloadOverride ?: s.payload
        val local = s.pointer - entry.origin
        val result = when (feedback) {
            is DropFeedback.Placement -> DropResult(zone = feedback.zone, position = local)
            is DropFeedback.Insertion -> DropResult(insertionIndex = feedback.index, position = local)
            else -> DropResult(position = local)
        }
        session = null
        entry.onDrop(payload, result)
        return true
    }

    /** Cancels the drag (Back, release over nothing). In-app drags animate back to their origin. */
    fun cancel() {
        val s = session ?: return
        if (s.origin == null || s.external) {
            session = null
        } else {
            s.targetKey = null
            s.feedback = null
            s.returning = true
        }
    }

    /** Ends a drag without feedback (released in place: the source shows its menu instead). */
    internal fun clear() {
        session = null
    }

    internal fun finishReturn(s: DragSession) {
        if (session === s) session = null
    }

    private fun update(s: DragSession) {
        // Overlapping targets may accept different payloads (a terminal's "粘贴路径" area over the Stack's
        // placement zones): the best target that accepts the payload wins; only when none accepts does the
        // topmost one show the rejection.
        val all = targets.values.filter { entry ->
            val kind = entry.kind
            kind !is DropTargetKind.EditorEdges ||
                DndGeometry.outerEdge(entry.size, s.pointer - entry.origin, kind.bandPx) != null
        }
        val accepting = all.filter { it.accepts(s.payload) }
        val acceptIndex = DndGeometry.pickTarget(accepting.map { it.bounds to it.priority }, s.pointer)
        val entries = if (acceptIndex >= 0) accepting else all
        val index = if (acceptIndex >= 0) acceptIndex else DndGeometry.pickTarget(all.map { it.bounds to it.priority }, s.pointer)
        if (index < 0) {
            s.targetKey = null
            s.feedback = null
            return
        }
        val entry = entries[index]
        s.targetKey = entry.key
        if (!entry.accepts(s.payload)) {
            s.feedback = DropFeedback.Rejected
            return
        }
        val local = s.pointer - entry.origin
        s.feedback = when (val kind = entry.kind) {
            is DropTargetKind.Zones -> {
                val zone = DndGeometry.resolveZone(entry.size, local, allowEdges = kind.allowEdges)
                DropFeedback.Placement(zone, DndGeometry.zonePreview(entry.bounds, zone))
            }
            is DropTargetKind.EditorEdges -> {
                val zone = DndGeometry.outerEdge(entry.size, local, kind.bandPx) ?: return
                DropFeedback.Placement(zone, DndGeometry.zonePreview(entry.bounds, zone))
            }
            is DropTargetKind.Sort -> {
                val items = kind.items()
                val axis = if (kind.horizontal) local.x else local.y
                val insertion = DndGeometry.insertionIndex(items, axis)
                val at = DndGeometry.insertionIndicatorPosition(items, insertion)
                val half = insertionLineThickness / 2f
                val line = if (kind.horizontal) {
                    val cy = entry.bounds.center.y
                    val x = (entry.origin.x + at).coerceIn(entry.bounds.left + half, entry.bounds.right - half)
                    Rect(x - half, cy - insertionLineLength / 2f, x + half, cy + insertionLineLength / 2f)
                } else {
                    val cx = entry.bounds.center.x
                    val y = (entry.origin.y + at).coerceIn(entry.bounds.top + half, entry.bounds.bottom - half)
                    Rect(cx - insertionLineLength / 2f, y - half, cx + insertionLineLength / 2f, y + half)
                }
                DropFeedback.Insertion(insertion, line)
            }
            is DropTargetKind.Area -> DropFeedback.Area(entry.bounds, kind.style, kind.label)
        }
    }
}

/** The state of the nearest [DragDropHost]; null outside one. */
val LocalDragDropState = compositionLocalOf<DragDropState?> { null }
