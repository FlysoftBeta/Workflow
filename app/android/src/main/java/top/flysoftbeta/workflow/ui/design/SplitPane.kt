package top.flysoftbeta.workflow.ui.design

import android.view.HapticFeedbackConstants
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.layoutId
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.progressBarRangeInfo
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.setProgress
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlin.math.roundToInt
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

enum class SplitOrientation {
    /** Panes side by side; the divider is vertical. */
    Horizontal,
    /** Panes stacked; the divider is horizontal. */
    Vertical,
}

enum class SplitSide { First, Second }

/**
 * Hoisted state of a [SplitPane]. [fraction] is the first pane's share of the space (excluding the seam);
 * callers persist [fraction] and [collapsed] per Session (docs/ux/README.md §3.2).
 */
@Stable
class SplitPaneState(fraction: Float, val defaultFraction: Float = fraction, collapsed: SplitSide? = null) {
    var fraction by mutableFloatStateOf(fraction)
    var collapsed by mutableStateOf(collapsed)
    /** True while the divider is being dragged (WebView / sora relayout should wait until false + 100ms). */
    var isDragging by mutableStateOf(false)
        internal set
    /** Finger is down on the divider: the pill handle is shown. */
    var isPressed by mutableStateOf(false)
        internal set

    fun restoreDefault() {
        fraction = defaultFraction
        collapsed = null
    }
}

@Composable
fun rememberSplitPaneState(fraction: Float, defaultFraction: Float = fraction, collapsed: SplitSide? = null): SplitPaneState =
    remember { SplitPaneState(fraction, defaultFraction, collapsed) }

/**
 * Two regions separated by the 4dp seam (docs/ux/README.md §3.2). The seam becomes a draggable divider:
 * a 20dp touch band that only starts dragging after moving past touch slop along the split axis
 * (other touches go to the content), a 4×32dp primary pill while pressed, double tap restores
 * [SplitPaneState.defaultFraction], and dragging a [collapsible] side below 50% of its minimum snaps it
 * collapsed ([collapsedSize]: 0 for side regions, the tab row height for a bottom Stack) with CLOCK_TICK.
 * Sizes are re-clamped to min/max whenever the container size changes.
 */
@Composable
fun SplitPane(
    state: SplitPaneState,
    orientation: SplitOrientation,
    first: @Composable () -> Unit,
    second: @Composable () -> Unit,
    modifier: Modifier = Modifier,
    firstMin: Dp = 0.dp,
    secondMin: Dp = 0.dp,
    firstMax: Dp = Dp.Infinity,
    secondMax: Dp = Dp.Infinity,
    collapsible: Set<SplitSide> = emptySet(),
    collapsedSize: Dp = 0.dp,
    gap: Dp = WorkflowTheme.dimens.gap,
) {
    val dimens = WorkflowTheme.dimens
    val colors = WorkflowTheme.colors
    val view = LocalView.current
    val horizontal = orientation == SplitOrientation.Horizontal
    val firstCollapse by animateFloatAsState(
        if (state.collapsed == SplitSide.First) 1f else 0f, WorkflowTheme.motion.defaultSpatialSpec(), label = "firstCollapse",
    )
    val secondCollapse by animateFloatAsState(
        if (state.collapsed == SplitSide.Second) 1f else 0f, WorkflowTheme.motion.defaultSpatialSpec(), label = "secondCollapse",
    )
    // Geometry of the last layout, used by the gesture handler (px).
    val geometry = remember { SplitGeometry() }
    val currentCollapsible by rememberUpdatedState(collapsible)

    val bandPx = with(LocalDensity.current) { dimens.dividerTouch.toPx() }
    Layout(
        modifier = modifier.seamGestures(horizontal, bandPx, object : SeamCallbacks {
            override fun seams() = listOf(geometry.seamCenter)
            override fun onPress(index: Int, pressed: Boolean) { state.isPressed = pressed }
            override fun onDragStart(index: Int) { state.isDragging = true }
            override fun onDrag(index: Int, position: Float) {
                geometry.dragTo(position, state, currentCollapsible) { view.performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK) }
            }
            override fun onDragEnd(index: Int) { state.isDragging = false }
            override fun onDoubleTap(index: Int) = state.restoreDefault()
        }),
        content = {
            Box(Modifier.layoutId("first")) { first() }
            Box(Modifier.layoutId("second")) { second() }
            Box(
                Modifier
                    .layoutId("handle")
                    .background(if (state.isPressed) colors.primary else androidx.compose.ui.graphics.Color.Transparent, CircleShape)
                    .semantics {
                        contentDescription = "分隔条"
                        progressBarRangeInfo = ProgressBarRangeInfo(state.fraction, 0f..1f)
                        setProgress { value -> state.fraction = value; state.collapsed = null; true }
                        customActions = listOf(
                            CustomAccessibilityAction("增大") { state.collapsed = null; state.fraction = (state.fraction + 0.05f).coerceAtMost(1f); true },
                            CustomAccessibilityAction("减小") { state.collapsed = null; state.fraction = (state.fraction - 0.05f).coerceAtLeast(0f); true },
                        ) + currentCollapsible.map { side ->
                            CustomAccessibilityAction(if (side == SplitSide.First) "收起前一区域" else "收起后一区域") { state.collapsed = side; true }
                        }
                    },
            )
        },
    ) { measurables, constraints ->
        val total = if (horizontal) constraints.maxWidth else constraints.maxHeight
        val cross = if (horizontal) constraints.maxHeight else constraints.maxWidth
        val gapPx = gap.roundToPx()
        val space = (total - gapPx).coerceAtLeast(0)
        val fMin = firstMin.roundToPx().coerceAtMost(space)
        val sMin = secondMin.roundToPx().coerceAtMost(space)
        val fMax = if (firstMax == Dp.Infinity) space else firstMax.roundToPx()
        val sMax = if (secondMax == Dp.Infinity) space else secondMax.roundToPx()
        val collapsedPx = collapsedSize.roundToPx()
        val lower = maxOf(fMin, space - sMax)
        val upper = minOf(fMax, space - sMin).coerceAtLeast(lower)
        val normal = (state.fraction * space).roundToInt().coerceIn(lower, upper)
        var firstSize = normal
        firstSize = (firstSize + (collapsedPx - firstSize) * firstCollapse).roundToInt()
        firstSize = (firstSize + ((space - collapsedPx) - firstSize) * secondCollapse).roundToInt()
        firstSize = firstSize.coerceIn(0, space)
        val secondSize = space - firstSize
        geometry.update(space, gapPx, firstSize, fMin, sMin, fMax, sMax, collapsedPx)

        fun fixed(main: Int) = if (horizontal) Constraints.fixed(main, cross) else Constraints.fixed(cross, main)
        val firstP = measurables.first { it.layoutId == "first" }.measure(fixed(firstSize))
        val secondP = measurables.first { it.layoutId == "second" }.measure(fixed(secondSize))
        val pillLong = dimens.dividerHandleLength.roundToPx()
        val pillThick = dimens.dividerHandleThickness.roundToPx()
        val handleP = measurables.first { it.layoutId == "handle" }.measure(
            if (horizontal) Constraints.fixed(pillThick, pillLong) else Constraints.fixed(pillLong, pillThick)
        )
        val w = if (horizontal) total else cross
        val h = if (horizontal) cross else total
        layout(w, h) {
            val seamStart = firstSize
            if (horizontal) {
                firstP.place(0, 0)
                secondP.place(firstSize + gapPx, 0)
                handleP.place(seamStart + (gapPx - pillThick) / 2, (cross - pillLong) / 2)
            } else {
                firstP.place(0, 0)
                secondP.place(0, firstSize + gapPx)
                handleP.place((cross - pillLong) / 2, seamStart + (gapPx - pillThick) / 2)
            }
        }
    }
}

/** Mutable last-layout geometry and the drag logic (px). */
private class SplitGeometry {
    var space = 0; var gap = 0; var first = 0
    var fMin = 0; var sMin = 0; var fMax = 0; var sMax = 0; var collapsedPx = 0
    val seamCenter: Float get() = first + gap / 2f

    fun update(space: Int, gap: Int, first: Int, fMin: Int, sMin: Int, fMax: Int, sMax: Int, collapsedPx: Int) {
        this.space = space; this.gap = gap; this.first = first
        this.fMin = fMin; this.sMin = sMin; this.fMax = fMax; this.sMax = sMax; this.collapsedPx = collapsedPx
    }

    /** Applies a divider position [seam] (px along the axis) to [state]. */
    fun dragTo(seam: Float, state: SplitPaneState, collapsible: Set<SplitSide>, tick: () -> Unit) {
        if (space <= 0) return
        val proposedFirst = seam - gap / 2f
        val proposedSecond = space - proposedFirst
        val newCollapsed = when {
            SplitSide.First in collapsible && proposedFirst < fMin * 0.5f -> SplitSide.First
            SplitSide.Second in collapsible && proposedSecond < sMin * 0.5f -> SplitSide.Second
            else -> null
        }
        if (newCollapsed != state.collapsed) {
            state.collapsed = newCollapsed
            tick()
        }
        if (newCollapsed == null) {
            val lower = maxOf(fMin, space - sMax).toFloat()
            val upper = minOf(fMax, space - sMin).toFloat().coerceAtLeast(lower)
            state.fraction = proposedFirst.coerceIn(lower, upper) / space
        }
    }
}
