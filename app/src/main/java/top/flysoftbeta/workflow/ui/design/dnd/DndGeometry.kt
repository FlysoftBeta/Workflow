package top.flysoftbeta.workflow.ui.design.dnd

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import kotlin.math.abs
import kotlin.math.min

/** Placement zone of a drop on a Stack's content area (docs/ui.md §4.10, dnd_placement.png). */
enum class DropZone { Center, Left, Top, Right, Bottom }

/**
 * Pure drag-and-drop geometry. No Android types; unit tested in `DndGeometryTest`.
 * All coordinates are in one space (the caller's), rects are `left, top, right, bottom`.
 */
object DndGeometry {
    /** Each edge zone covers the outer 25% of the target; the centre keeps the inner 50%. */
    const val EDGE_FRACTION = 0.25f

    /**
     * Resolves the zone under [point] (relative to the target's top-left) in a target of [size].
     * A point is in an edge zone when its normalized distance to that edge is < [edgeFraction]; in a
     * corner the nearer edge (by normalized distance) wins, ties prefer the horizontal edges (split right/left).
     * Points outside the target are clamped onto it. With [allowEdges] = false everything is [DropZone.Center].
     */
    fun resolveZone(size: Size, point: Offset, edgeFraction: Float = EDGE_FRACTION, allowEdges: Boolean = true): DropZone {
        if (!allowEdges || size.width <= 0f || size.height <= 0f) return DropZone.Center
        val nx = (point.x / size.width).coerceIn(0f, 1f)
        val ny = (point.y / size.height).coerceIn(0f, 1f)
        val left = nx
        val right = 1f - nx
        val top = ny
        val bottom = 1f - ny
        val nearest = min(min(left, right), min(top, bottom))
        if (nearest >= edgeFraction) return DropZone.Center
        return when (nearest) {
            left -> DropZone.Left
            right -> DropZone.Right
            top -> DropZone.Top
            else -> DropZone.Bottom
        }
    }

    /** Editor-tree perimeter in pixels, independent of tree size; centre belongs to child stacks. */
    fun outerEdge(size: Size, point: Offset, band: Float): DropZone? {
        if (size.width <= 0f || size.height <= 0f || band <= 0f ||
            point.x < 0f || point.y < 0f || point.x >= size.width || point.y >= size.height) return null
        val distances = listOf(point.x, point.y, size.width - point.x, size.height - point.y)
        val nearest = distances.indices.minBy { distances[it] }
        if (distances[nearest] > band) return null
        return listOf(DropZone.Left, DropZone.Top, DropZone.Right, DropZone.Bottom)[nearest]
    }

    /**
     * The rectangle the dropped panel will occupy after the drop (the placement preview):
     * the whole target for [DropZone.Center], otherwise the half on that side.
     */
    fun zonePreview(bounds: Rect, zone: DropZone): Rect = when (zone) {
        DropZone.Center -> bounds
        DropZone.Left -> Rect(bounds.left, bounds.top, bounds.center.x, bounds.bottom)
        DropZone.Right -> Rect(bounds.center.x, bounds.top, bounds.right, bounds.bottom)
        DropZone.Top -> Rect(bounds.left, bounds.top, bounds.right, bounds.center.y)
        DropZone.Bottom -> Rect(bounds.left, bounds.center.y, bounds.right, bounds.bottom)
    }

    /**
     * Insertion index for sorting along one axis. [items] are the item extents along that axis
     * (start to end, in display order, non-overlapping). Returns `i` in `0..items.size`:
     * the dragged item goes before `items[i]` (or at the end for `items.size`).
     * An item's midpoint splits "before" from "after".
     */
    fun insertionIndex(items: List<ClosedFloatingPointRange<Float>>, pointer: Float): Int {
        for ((index, item) in items.withIndex()) {
            val mid = (item.start + item.endInclusive) / 2f
            if (pointer < mid) return index
        }
        return items.size
    }

    /** Position of the insertion indicator for [index]: midway in the gap before item [index], or after the last item. */
    fun insertionIndicatorPosition(items: List<ClosedFloatingPointRange<Float>>, index: Int): Float {
        if (items.isEmpty()) return 0f
        return when {
            index <= 0 -> items.first().start
            index >= items.size -> items.last().endInclusive
            else -> (items[index - 1].endInclusive + items[index].start) / 2f
        }
    }

    /** True when inserting the item currently at [from] at insertion index [insertion] would not move it. */
    fun isNoOpMove(from: Int, insertion: Int): Boolean = insertion == from || insertion == from + 1

    /**
     * Converts an insertion index (computed with the dragged item still in the list) into the final index
     * after removing the item from [from] — the index a `move(from, to)` list operation expects.
     */
    fun targetIndexAfterRemoval(from: Int, insertion: Int): Int = if (insertion > from) insertion - 1 else insertion

    /**
     * Auto-scroll speed (px per second, signed) while dragging near the ends of a scrollable range
     * [start]..[end]: zero outside the [edge] band, growing linearly to [maxSpeed] at the very end.
     */
    fun autoScrollSpeed(pointer: Float, start: Float, end: Float, edge: Float, maxSpeed: Float): Float {
        if (end - start <= 2 * edge || edge <= 0f) return 0f
        return when {
            pointer < start + edge -> -maxSpeed * ((start + edge - pointer) / edge).coerceIn(0f, 1f)
            pointer > end - edge -> maxSpeed * ((pointer - (end - edge)) / edge).coerceIn(0f, 1f)
            else -> 0f
        }
    }

    /**
     * Picks the drop target under [point]: among [candidates] whose bounds contain it, the highest
     * priority wins, then the smallest area (the most specific target). Returns the index or -1.
     */
    fun pickTarget(candidates: List<Pair<Rect, Int>>, point: Offset): Int {
        var best = -1
        var bestPriority = Int.MIN_VALUE
        var bestArea = Float.MAX_VALUE
        for ((index, candidate) in candidates.withIndex()) {
            val (rect, priority) = candidate
            if (!rect.contains(point)) continue
            val area = abs(rect.width * rect.height)
            if (priority > bestPriority || (priority == bestPriority && area < bestArea)) {
                best = index; bestPriority = priority; bestArea = area
            }
        }
        return best
    }

    /**
     * Grid slot under [point] for a reorderable grid of [count] cells laid out row-major in [columns]
     * columns of [cellWidth] × [cellHeight] starting at the origin. Points left/above the grid clamp to the
     * first row/column, points past the last cell clamp to the last index. Returns -1 when [count] is 0.
     */
    fun gridSlot(point: Offset, columns: Int, cellWidth: Float, cellHeight: Float, count: Int): Int {
        if (count <= 0 || columns <= 0 || cellWidth <= 0f || cellHeight <= 0f) return -1
        val column = (point.x / cellWidth).toInt().coerceIn(0, columns - 1).let { if (point.x < 0) 0 else it }
        val row = if (point.y < 0) 0 else (point.y / cellHeight).toInt()
        return (row * columns + column).coerceIn(0, count - 1)
    }

    /** Display order while an item is dragged from [from] to slot [to]: the moved list's index for each original index. */
    fun reorderedIndex(index: Int, from: Int, to: Int): Int = when {
        index == from -> to
        from < to && index in (from + 1)..to -> index - 1
        from > to && index in to until from -> index + 1
        else -> index
    }
}
