package top.flysoftbeta.workflow.ui.design

import kotlin.math.abs

/**
 * Pure geometry of the segmented model + effort slider (docs/ui.md §4.8): every effort detent gets the
 * same width, a model's segment is as wide as its detents, segments are separated by [gap].
 * Unit tested in `ModelSliderGeometryTest`.
 */
object ModelSliderGeometry {
    data class Segment(val model: Int, val start: Float, val end: Float)
    data class Detent(val model: Int, val effort: Int, val x: Float)
    data class Layout(val segments: List<Segment>, val detents: List<Detent>) {
        fun indexOf(model: Int, effort: Int): Int = detents.indexOfFirst { it.model == model && it.effort == effort }
    }

    fun layout(width: Float, gap: Float, effortCounts: List<Int>): Layout {
        val counts = effortCounts.map { it.coerceAtLeast(1) }
        val total = counts.sum()
        if (total == 0 || width <= 0f) return Layout(emptyList(), emptyList())
        val unit = (width - gap * (counts.size - 1)).coerceAtLeast(0f) / total
        val segments = ArrayList<Segment>(counts.size)
        val detents = ArrayList<Detent>(total)
        var x = 0f
        counts.forEachIndexed { model, count ->
            val start = x
            val end = start + count * unit
            segments += Segment(model, start, end)
            for (effort in 0 until count) detents += Detent(model, effort, start + (effort + 0.5f) * unit)
            x = end + gap
        }
        return Layout(segments, detents)
    }

    /** Index of the detent nearest to [x] (ties go to the lower index); -1 for an empty layout. */
    fun nearest(layout: Layout, x: Float): Int {
        var best = -1
        var bestDistance = Float.MAX_VALUE
        layout.detents.forEachIndexed { index, detent ->
            val d = abs(detent.x - x)
            if (d < bestDistance) { best = index; bestDistance = d }
        }
        return best
    }
}
