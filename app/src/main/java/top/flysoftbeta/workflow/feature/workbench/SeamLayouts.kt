package top.flysoftbeta.workflow.feature.workbench

import android.view.HapticFeedbackConstants
import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.ui.design.SeamCallbacks
import top.flysoftbeta.workflow.ui.design.seamGestures
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme
import kotlin.math.roundToInt

/**
 * Draws the 4×32dp primary pill of a pressed seam (docs/ui.md §3.2) at [center] (px along the axis).
 */
private fun Modifier.seamPill(horizontal: Boolean, center: () -> Float?, color: androidx.compose.ui.graphics.Color, density: Float): Modifier =
    drawWithContent {
        drawContent()
        val c = center() ?: return@drawWithContent
        val thick = 4 * density
        val long = 32 * density
        if (horizontal) {
            drawRoundRect(color, Offset(c - thick / 2, size.height / 2 - long / 2), Size(thick, long), CornerRadius(thick / 2))
        } else {
            drawRoundRect(color, Offset(size.width / 2 - long / 2, c - thick / 2), Size(long, thick), CornerRadius(thick / 2))
        }
    }

/**
 * `[side][center][aux]` with draggable seams (docs/ui.md §3.2). Side and aux widths are dp values from
 * the session's arrangement (null = not docked). While dragging, widths follow the finger locally;
 * the result commits once on release ([onResize]). Dragging below 50% of the minimum collapses the
 * region ([onCollapse], CLOCK_TICK); a double tap restores the default ([onReset]).
 */
@Composable
internal fun RegionRow(
    sideWidth: Dp?,
    auxWidth: Dp?,
    sideLimits: ClosedFloatingPointRange<Float>,
    auxLimits: ClosedFloatingPointRange<Float>,
    onResize: (side: Boolean, width: Float) -> Unit,
    onCollapse: (side: Boolean) -> Unit,
    onReset: (side: Boolean) -> Unit,
    onResizing: (Boolean) -> Unit,
    side: @Composable () -> Unit,
    center: @Composable () -> Unit,
    aux: @Composable () -> Unit,
    modifier: Modifier = Modifier,
) {
    val density = LocalDensity.current
    val view = LocalView.current
    val gap = WorkflowTheme.dimens.gap
    val colors = WorkflowTheme.colors
    var dragSide by remember { mutableStateOf<Float?>(null) }
    var dragAux by remember { mutableStateOf<Float?>(null) }
    var pressed by remember { mutableIntStateOf(-1) }
    val seams = remember { FloatArray(2) { Float.NaN } }
    var totalPx by remember { mutableIntStateOf(0) }
    val currentResize by rememberUpdatedState(onResize)
    val currentCollapse by rememberUpdatedState(onCollapse)
    val currentReset by rememberUpdatedState(onReset)
    val currentResizing by rememberUpdatedState(onResizing)
    val sideDp = sideWidth?.value?.let { dragSide ?: it }
    val auxDp = auxWidth?.value?.let { dragAux ?: it }
    val bandPx = with(density) { WorkflowTheme.dimens.dividerTouch.toPx() }
    val regionActions = buildList {
        fun region(side: Boolean, width: Float?, limits: ClosedFloatingPointRange<Float>, label: String) {
            if (width == null) return
            fun resize(delta: Float): Boolean {
                val next = (width + delta).coerceIn(limits)
                if (next == width) return false
                currentResize(side, next)
                return true
            }
            add(CustomAccessibilityAction("增大$label") { resize(24f) })
            add(CustomAccessibilityAction("减小$label") { resize(-24f) })
            add(CustomAccessibilityAction("收起$label") { currentCollapse(side); true })
            add(CustomAccessibilityAction("恢复${label}默认宽度") { currentReset(side); true })
        }
        region(true, sideDp, sideLimits, "侧栏")
        region(false, auxDp, auxLimits, "辅助区")
    }

    Layout(
        modifier = modifier
            .then(if (regionActions.isEmpty()) Modifier else Modifier.semantics {
                contentDescription = "调整区域布局"
                customActions = regionActions
            })
            .seamGestures(horizontal = true, bandPx = bandPx, callbacks = object : SeamCallbacks {
                override fun seams() = seams.map { if (it.isNaN()) -1e9f else it }
                override fun onPress(index: Int, pressed0: Boolean) { pressed = if (pressed0) index else -1 }
                override fun onDragStart(index: Int) = currentResizing(true)
                override fun onDrag(index: Int, position: Float) {
                    val gapPx = with(density) { gap.toPx() }
                    if (index == 0 && sideWidth != null) {
                        val proposed = with(density) { (position - gapPx / 2).toDp() }.value
                        if (proposed < sideLimits.start * 0.5f) {
                            view.performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
                            dragSide = null
                            currentCollapse(true)
                        } else dragSide = proposed.coerceIn(sideLimits)
                    } else if (index == 1 && auxWidth != null) {
                        val proposed = with(density) { (totalPx - position - gapPx / 2).toDp() }.value
                        if (proposed < auxLimits.start * 0.5f) {
                            view.performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
                            dragAux = null
                            currentCollapse(false)
                        } else dragAux = proposed.coerceIn(auxLimits)
                    }
                }
                override fun onDragEnd(index: Int) {
                    dragSide?.let { currentResize(true, it) }
                    dragAux?.let { currentResize(false, it) }
                    dragSide = null; dragAux = null
                    currentResizing(false)
                }
                override fun onDoubleTap(index: Int) = currentReset(index == 0)
            })
            .seamPill(true, { seams.getOrNull(pressed)?.takeUnless { it.isNaN() } }, colors.primary, density.density),
        // Each slot is wrapped so it always emits exactly one node (an undocked region emits nothing).
        content = { Box(propagateMinConstraints = true) { side() }; Box(propagateMinConstraints = true) { center() }; Box(propagateMinConstraints = true) { aux() } },
    ) { measurables, constraints ->
        val gapPx = gap.roundToPx()
        val w = constraints.maxWidth
        val h = constraints.maxHeight
        totalPx = w
        val sidePx = sideDp?.dp?.roundToPx()?.coerceAtMost(w / 2) ?: 0
        val auxPx = auxDp?.dp?.roundToPx()?.coerceAtMost(w / 2) ?: 0
        val centerPx = (w - sidePx - auxPx - (if (sideDp != null) gapPx else 0) - (if (auxDp != null) gapPx else 0)).coerceAtLeast(0)
        val sideP = measurables[0].measure(Constraints.fixed(sidePx, h))
        val centerP = measurables[1].measure(Constraints.fixed(centerPx, h))
        val auxP = measurables[2].measure(Constraints.fixed(auxPx, h))
        seams[0] = if (sideDp != null) sidePx + gapPx / 2f else Float.NaN
        seams[1] = if (auxDp != null) w - auxPx - gapPx / 2f else Float.NaN
        layout(w, h) {
            sideP.place(0, 0)
            centerP.place(if (sideDp != null) sidePx + gapPx else 0, 0)
            auxP.place(w - auxPx, 0)
        }
    }
}

/**
 * An n-ary split of editor stacks (workspace.md §3): children by [weights] with 4dp seams. Children in
 * [fixed] get that size instead (IME focus: other stacks of the column shrink to their tab row). A drag
 * moves the seam between two weighted neighbours locally and commits once ([onCommit]); a double tap
 * resets ([onReset]).
 */
@Composable
internal fun WeightedSplit(
    horizontal: Boolean,
    weights: List<Double>,
    fixed: Map<Int, Dp>,
    minSize: Dp,
    onCommit: (List<Double>) -> Unit,
    onReset: () -> Unit,
    onResizing: (Boolean) -> Unit,
    children: List<@Composable () -> Unit>,
    modifier: Modifier = Modifier,
) {
    val density = LocalDensity.current
    val colors = WorkflowTheme.colors
    val gap = WorkflowTheme.dimens.gap
    var local by remember { mutableStateOf<List<Double>?>(null) }
    var pressed by remember { mutableIntStateOf(-1) }
    val geometry = remember { SplitGeometry() }
    val currentCommit by rememberUpdatedState(onCommit)
    val currentReset by rememberUpdatedState(onReset)
    val currentResizing by rememberUpdatedState(onResizing)
    val effective = local?.takeIf { it.size == weights.size } ?: weights
    val bandPx = with(density) { WorkflowTheme.dimens.dividerTouch.toPx() }
    val splitActions = buildList {
        for (index in 0 until (weights.size - 1).coerceAtLeast(0)) {
            if (index in fixed || index + 1 in fixed) continue
            fun resize(delta: Float): Boolean {
                val seam = geometry.seams.getOrNull(index) ?: return false
                val next = geometry.drag(index, seam + with(density) { delta.dp.toPx() }, effective, with(density) { minSize.toPx() }) ?: return false
                if (next == effective) return false
                currentCommit(next)
                return true
            }
            add(CustomAccessibilityAction("增大第 ${index + 1} 个窗格") { resize(24f) })
            add(CustomAccessibilityAction("减小第 ${index + 1} 个窗格") { resize(-24f) })
        }
        if (fixed.isEmpty() && weights.size > 1) {
            add(CustomAccessibilityAction("恢复拆分比例") { currentReset(); true })
        }
    }

    Layout(
        modifier = modifier
            .then(if (splitActions.isEmpty()) Modifier else Modifier.semantics {
                contentDescription = if (horizontal) "调整横向拆分" else "调整纵向拆分"
                customActions = splitActions
            })
            .seamGestures(horizontal, bandPx, object : SeamCallbacks {
                override fun seams() = geometry.seams.toList()
                override fun onPress(index: Int, pressed0: Boolean) { pressed = if (pressed0) index else -1 }
                override fun onDragStart(index: Int) = currentResizing(true)
                override fun onDrag(index: Int, position: Float) {
                    val next = geometry.drag(index, position, effective, with(density) { minSize.toPx() }) ?: return
                    local = next
                }
                override fun onDragEnd(index: Int) {
                    local?.let { currentCommit(it) }
                    local = null
                    currentResizing(false)
                }
                override fun onDoubleTap(index: Int) = currentReset()
            })
            .seamPill(horizontal, { geometry.seams.getOrNull(pressed) }, colors.primary, density.density),
        content = { children.forEach { child -> Box(propagateMinConstraints = true) { child() } } },
    ) { measurables, constraints ->
        val gapPx = gap.roundToPx()
        val main = if (horizontal) constraints.maxWidth else constraints.maxHeight
        val cross = if (horizontal) constraints.maxHeight else constraints.maxWidth
        val n = measurables.size
        val space = (main - gapPx * (n - 1)).coerceAtLeast(0)
        val fixedPx = fixed.mapValues { it.value.roundToPx().coerceAtMost(space) }
        val flexible = (space - fixedPx.values.sum()).coerceAtLeast(0)
        val flexWeight = effective.withIndex().filter { it.index !in fixedPx }.sumOf { it.value }.takeIf { it > 0 } ?: 1.0
        val sizes = IntArray(n) { i -> fixedPx[i] ?: (flexible * (effective.getOrElse(i) { 1.0 } / flexWeight)).roundToInt() }
        // Give rounding leftovers to the last flexible child.
        val lastFlex = (n - 1 downTo 0).firstOrNull { it !in fixedPx }
        if (lastFlex != null) sizes[lastFlex] += space - sizes.sum()
        val placeables = measurables.mapIndexed { i, m ->
            val s = sizes[i].coerceAtLeast(0)
            m.measure(if (horizontal) Constraints.fixed(s, cross) else Constraints.fixed(cross, s))
        }
        geometry.update(sizes, gapPx, fixedPx.keys)
        layout(if (horizontal) main else cross, if (horizontal) cross else main) {
            var pos = 0
            placeables.forEachIndexed { i, p ->
                if (horizontal) p.place(pos, 0) else p.place(0, pos)
                pos += sizes[i] + gapPx
            }
        }
    }
}

private class SplitGeometry {
    var sizes = IntArray(0)
    var gap = 0
    var fixed: Set<Int> = emptySet()
    var seams = FloatArray(0)

    fun update(sizes: IntArray, gap: Int, fixed: Set<Int>) {
        this.sizes = sizes; this.gap = gap; this.fixed = fixed
        var pos = 0f
        seams = FloatArray((sizes.size - 1).coerceAtLeast(0)) { i ->
            pos += sizes[i]
            val c = pos + gap / 2f
            pos += gap
            c
        }
    }

    /** New weights when seam [index] is dragged to [position] (px), or null when not draggable. */
    fun drag(index: Int, position: Float, weights: List<Double>, minPx: Float): List<Double>? {
        if (index in fixed || index + 1 in fixed || index + 1 >= sizes.size) return null
        val start = (0 until index).sumOf { sizes[it] + gap }.toFloat()
        val pair = (sizes[index] + sizes[index + 1]).toFloat()
        if (pair <= 0f) return null
        val first = (position - gap / 2f - start).coerceIn(minOf(minPx, pair / 2), maxOf(pair - minPx, pair / 2))
        val sum = weights[index] + weights[index + 1]
        return weights.toMutableList().apply {
            this[index] = sum * first / pair
            this[index + 1] = sum - this[index]
        }
    }
}

/**
 * The centre column of Files: editor area above the bottom (terminal) stack (docs/ui.md §3.2). The
 * bottom height is a fraction of the column; collapsed it shows only its tab row ([collapsedHeight]).
 * [imeFocus] overrides: EDITOR shrinks the bottom to its tab row, BOTTOM shrinks the editor area.
 */
@Composable
internal fun CenterColumn(
    bottomFraction: Float,
    bottomCollapsed: Boolean,
    collapsedHeight: Dp,
    editorMin: Dp,
    imeFocus: ImeFocus?,
    onResize: (Float) -> Unit,
    onCollapse: (Boolean) -> Unit,
    onReset: () -> Unit,
    onResizing: (Boolean) -> Unit,
    editor: @Composable () -> Unit,
    bottom: @Composable () -> Unit,
    modifier: Modifier = Modifier,
) {
    val density = LocalDensity.current
    val view = LocalView.current
    val colors = WorkflowTheme.colors
    val gap = WorkflowTheme.dimens.gap
    var local by remember { mutableStateOf<Float?>(null) }
    var pressed by remember { mutableStateOf(false) }
    var seam by remember { mutableFloatStateOf(Float.NaN) }
    var total by remember { mutableIntStateOf(0) }
    val currentResize by rememberUpdatedState(onResize)
    val currentCollapse by rememberUpdatedState(onCollapse)
    val currentReset by rememberUpdatedState(onReset)
    val currentResizing by rememberUpdatedState(onResizing)
    val bandPx = with(density) { WorkflowTheme.dimens.dividerTouch.toPx() }
    val expandedMin = 120.dp
    val columnActions = buildList {
        // IME focus supplies a temporary geometry; do not persist adjustments to that geometry.
        if (imeFocus == null) {
            if (!bottomCollapsed) {
                fun resize(delta: Float): Boolean {
                    if (total <= 0 || seam.isNaN()) return false
                    val gapPx = with(density) { gap.toPx() }
                    val min = with(density) { expandedMin.toPx() }
                    val max = total - with(density) { editorMin.toPx() } - gapPx
                    if (max < min) return false
                    val height = total - seam - gapPx / 2
                    val next = (height + with(density) { delta.dp.toPx() }).coerceIn(min, max)
                    if (next == height) return false
                    currentResize((next / total).coerceIn(0.05f, 0.95f))
                    return true
                }
                add(CustomAccessibilityAction("增大底部区域") { resize(24f) })
                add(CustomAccessibilityAction("减小底部区域") { resize(-24f) })
            }
            add(CustomAccessibilityAction(if (bottomCollapsed) "展开底部区域" else "收起底部区域") {
                currentCollapse(!bottomCollapsed)
                true
            })
            add(CustomAccessibilityAction("恢复底部区域默认大小") { currentReset(); true })
        }
    }

    Layout(
        modifier = modifier
            .then(if (columnActions.isEmpty()) Modifier else Modifier.semantics {
                contentDescription = "调整编辑器与底部区域"
                customActions = columnActions
            })
            .seamGestures(false, bandPx, object : SeamCallbacks {
                override fun seams() = listOf(if (seam.isNaN()) -1e9f else seam)
                override fun onPress(index: Int, pressed0: Boolean) { pressed = pressed0 }
                override fun onDragStart(index: Int) = currentResizing(true)
                override fun onDrag(index: Int, position: Float) {
                    if (total <= 0) return
                    val gapPx = with(density) { gap.toPx() }
                    val height = total - position - gapPx / 2
                    val min = with(density) { expandedMin.toPx() }
                    if (height < min * 0.5f) {
                        if (!bottomCollapsed || local != null) view.performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
                        local = null
                        if (!bottomCollapsed) currentCollapse(true)
                    } else {
                        if (bottomCollapsed) currentCollapse(false)
                        val max = total - with(density) { editorMin.toPx() } - gapPx
                        local = (height.coerceIn(min, maxOf(min, max)) / total).coerceIn(0.05f, 0.95f)
                    }
                }
                override fun onDragEnd(index: Int) {
                    local?.let { currentResize(it) }
                    local = null
                    currentResizing(false)
                }
                override fun onDoubleTap(index: Int) = currentReset()
            })
            .seamPill(false, { seam.takeIf { pressed && !it.isNaN() } }, colors.primary, density.density),
        content = { Box(propagateMinConstraints = true) { editor() }; Box(propagateMinConstraints = true) { bottom() } },
    ) { measurables, constraints ->
        val gapPx = gap.roundToPx()
        val h = constraints.maxHeight
        val w = constraints.maxWidth
        total = h
        val bar = collapsedHeight.roundToPx()
        val bottomPx = when {
            imeFocus == ImeFocus.EDITOR -> bar
            imeFocus == ImeFocus.BOTTOM -> (h - gapPx - bar).coerceAtLeast(bar)
            bottomCollapsed && local == null -> bar
            else -> ((local ?: bottomFraction) * h).roundToInt()
                .coerceIn(minOf(bar, h), (h - gapPx - editorMin.roundToPx()).coerceAtLeast(bar))
        }
        val editorPx = (h - gapPx - bottomPx).coerceAtLeast(0)
        val editorP = measurables[0].measure(Constraints.fixed(w, editorPx))
        val bottomP = measurables[1].measure(Constraints.fixed(w, bottomPx))
        seam = editorPx + gapPx / 2f
        layout(w, h) {
            editorP.place(0, 0)
            bottomP.place(0, editorPx + gapPx)
        }
    }
}

/** Which part of the centre column holds IME focus (docs/ui.md §2.3). */
internal enum class ImeFocus { EDITOR, BOTTOM }
