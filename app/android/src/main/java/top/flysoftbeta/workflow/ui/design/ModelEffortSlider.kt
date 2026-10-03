package top.flysoftbeta.workflow.ui.design

import android.view.HapticFeedbackConstants
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.DropdownMenuPopup
import androidx.compose.material3.MenuAnchorPosition
import androidx.compose.material3.MenuDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.progressBarRangeInfo
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.setProgress
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlin.math.roundToInt
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** A reasoning effort level of a model. Labels: 极低 / 低 / 中 / 高 / 极高. */
@Immutable
data class EffortOption(val id: String, val label: String)

/** A model shown as one slider segment (max 4: recent / recommended). */
@Immutable
data class ModelOption(val id: String, val label: String, val efforts: List<EffortOption>)

@Immutable
data class ModelEffortSelection(val modelId: String, val effortId: String)

/** Standard effort labels (docs/ux/README.md §4.8), keyed by protocol ids. */
val StandardEffortLabels: Map<String, String> = linkedMapOf(
    "minimal" to "极低", "low" to "低", "medium" to "中", "high" to "高", "xhigh" to "极高",
)

private val TrackHeight = 24.dp
private val HandleWidth = 4.dp
private val HandleHeight = 44.dp
private val HandleGap = 6.dp
private val SegmentGap = 6.dp

/**
 * Models as segments, efforts as detents (picker.png). Dragging or tapping snaps detent by detent with a
 * CLOCK_TICK and reports every change immediately (applies to the next turn, no confirm). The handle
 * is the Expressive 4×44 bar. Labels under the segments collapse to only the selected model's label
 * when they do not fit.
 */
@Composable
fun ModelEffortSlider(
    models: List<ModelOption>,
    selection: ModelEffortSelection,
    onSelectionChange: (ModelEffortSelection) -> Unit,
    modifier: Modifier = Modifier,
    gapColor: Color = WorkflowTheme.colors.surfaceContainerHighest,
) {
    val colors = WorkflowTheme.colors
    val density = LocalDensity.current
    val view = LocalView.current
    val currentOnChange by rememberUpdatedState(onSelectionChange)
    val selectedModel = models.indexOfFirst { it.id == selection.modelId }.coerceAtLeast(0)
    val selectedEffort = models.getOrNull(selectedModel)?.efforts?.indexOfFirst { it.id == selection.effortId }?.coerceAtLeast(0) ?: 0

    // No BoxWithConstraints: the slider lives in a menu popup, which measures intrinsics.
    var widthPx by remember { mutableFloatStateOf(0f) }
    Box(modifier.fillMaxWidth().onSizeChanged { widthPx = it.width.toFloat() }) {
        val gapPx = with(density) { SegmentGap.toPx() }
        val geometry = remember(widthPx, models) {
            ModelSliderGeometry.layout(widthPx, gapPx, models.map { it.efforts.size })
        }
        val selectedIndex = geometry.indexOf(selectedModel, selectedEffort).coerceAtLeast(0)
        val targetX = geometry.detents.getOrNull(selectedIndex)?.x ?: 0f
        val handleX by animateFloatAsState(targetX, WorkflowTheme.motion.fastSpatialSpec(), label = "handle")
        val currentGeometry by rememberUpdatedState(geometry)
        val currentSelected by rememberUpdatedState(selectedIndex)

        fun select(index: Int) {
            val detent = currentGeometry.detents.getOrNull(index) ?: return
            if (index == currentSelected) return
            view.performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
            val model = models[detent.model]
            currentOnChange(ModelEffortSelection(model.id, model.efforts[detent.effort].id))
        }

        Column {
            Canvas(
                Modifier
                    .fillMaxWidth()
                    .height(HandleHeight)
                    .pointerInput(models) {
                        awaitEachGesture {
                            val down = awaitFirstDown()
                            down.consume()
                            select(ModelSliderGeometry.nearest(currentGeometry, down.position.x))
                            while (true) {
                                val event = awaitPointerEvent()
                                val change = event.changes.firstOrNull { it.id == down.id } ?: break
                                if (!change.pressed) break
                                change.consume()
                                select(ModelSliderGeometry.nearest(currentGeometry, change.position.x))
                            }
                        }
                    }
                    .semantics {
                        val current = models.getOrNull(selectedModel)
                        contentDescription = "模型与推理强度"
                        stateDescription = listOfNotNull(current?.label, current?.efforts?.getOrNull(selectedEffort)?.label).joinToString(" ")
                        val count = geometry.detents.size
                        progressBarRangeInfo = ProgressBarRangeInfo(selectedIndex.toFloat(), 0f..(count - 1).coerceAtLeast(0).toFloat(), steps = (count - 2).coerceAtLeast(0))
                        setProgress { value -> select(value.roundToInt()); true }
                    },
            ) {
                val trackTop = (size.height - TrackHeight.toPx()) / 2
                val trackH = TrackHeight.toPx()
                val radius = CornerRadius(trackH / 2)
                val handleGap = HandleGap.toPx()
                val dot = 2.dp.toPx()
                for (segment in geometry.segments) {
                    drawRoundRect(colors.secondaryContainer, Offset(segment.start, trackTop), Size(segment.end - segment.start, trackH), radius)
                    val activeEnd = minOf(segment.end, handleX)
                    if (activeEnd > segment.start) {
                        // Active part: primary, clipped to the segment's rounded shape by drawing a rounded rect.
                        drawRoundRect(colors.primary, Offset(segment.start, trackTop), Size(activeEnd - segment.start, trackH), radius)
                    }
                }
                geometry.detents.forEachIndexed { index, detent ->
                    if (index == selectedIndex) return@forEachIndexed
                    val onActive = detent.x < handleX
                    drawCircle(if (onActive) colors.onPrimary else colors.onSecondaryContainer, dot, Offset(detent.x, size.height / 2))
                }
                // Expressive handle: gap on both sides + 4dp bar.
                drawRect(gapColor, Offset(handleX - handleGap - HandleWidth.toPx() / 2, trackTop), Size(handleGap * 2 + HandleWidth.toPx(), trackH))
                drawRoundRect(
                    colors.primary, Offset(handleX - HandleWidth.toPx() / 2, 0f), Size(HandleWidth.toPx(), size.height),
                    CornerRadius(HandleWidth.toPx() / 2),
                )
            }
            SegmentLabels(models, geometry, selectedModel)
        }
    }
}

@Composable
private fun SegmentLabels(models: List<ModelOption>, geometry: ModelSliderGeometry.Layout, selectedModel: Int) {
    val measurer = rememberTextMeasurer()
    val style = WorkflowTheme.text.micro
    val colors = WorkflowTheme.colors
    val fits = geometry.segments.all { segment ->
        measurer.measure(models[segment.model].label, style).size.width <= segment.end - segment.start
    }
    Layout(
        content = {
            geometry.segments.forEach { segment ->
                val visible = fits || segment.model == selectedModel
                Text(
                    if (visible) models[segment.model].label else "",
                    style = style,
                    color = if (segment.model == selectedModel) colors.onSurface else colors.onSurfaceVariant,
                    maxLines = 1, overflow = TextOverflow.Ellipsis, textAlign = TextAlign.Center,
                )
            }
        },
        modifier = Modifier.fillMaxWidth().padding(top = 2.dp),
    ) { measurables, constraints ->
        val placeables = measurables.mapIndexed { index, m ->
            val segment = geometry.segments[index]
            // A lone (selected-only) label may extend beyond its segment, but stays inside the row.
            m.measure(constraints.copy(minWidth = 0, maxWidth = if (fits || !constraints.hasBoundedWidth) (segment.end - segment.start).toInt().coerceAtLeast(0) else constraints.maxWidth))
        }
        val height = placeables.maxOfOrNull { it.height } ?: 0
        val width = if (constraints.hasBoundedWidth) constraints.maxWidth else placeables.sumOf { it.width }
        layout(width, height) {
            placeables.forEachIndexed { index, p ->
                val segment = geometry.segments[index]
                val center = (segment.start + segment.end) / 2
                val x = (center - p.width / 2f).toInt().coerceIn(0, (width - p.width).coerceAtLeast(0))
                p.place(x, 0)
            }
        }
    }
}

/**
 * The model popover anchored above the composer's model chip: 320dp, `xl`, surfaceContainerHighest,
 * elevation 3. Header: optional ⚡ speed toggle ([speedActive] non-null only when the backend supports
 * speed tiers), "Model · Effort", › (full model list). Outside tap / Back dismiss.
 */
@Composable
fun ModelEffortPopover(
    expanded: Boolean,
    onDismissRequest: () -> Unit,
    models: List<ModelOption>,
    selection: ModelEffortSelection,
    onSelectionChange: (ModelEffortSelection) -> Unit,
    onOpenModelList: () -> Unit,
    speedActive: Boolean? = null,
    onSpeedToggle: () -> Unit = {},
) {
    DropdownMenuPopup(
        expanded = expanded,
        onDismissRequest = onDismissRequest,
        modifier = Modifier.padding(MenuShadowPadding),
        popupPositionProvider = MenuDefaults.rememberDropdownMenuPopupPositionProvider(MenuAnchorPosition.Above),
    ) {
        ModelEffortPanel(models, selection, onSelectionChange, onOpenModelList, speedActive, onSpeedToggle)
    }
}

/** The popover's content, usable inline (gallery, tests). */
@Composable
fun ModelEffortPanel(
    models: List<ModelOption>,
    selection: ModelEffortSelection,
    onSelectionChange: (ModelEffortSelection) -> Unit,
    onOpenModelList: () -> Unit,
    speedActive: Boolean? = null,
    onSpeedToggle: () -> Unit = {},
    modifier: Modifier = Modifier,
) {
    val colors = WorkflowTheme.colors
    val model = models.firstOrNull { it.id == selection.modelId }
    val effort = model?.efforts?.firstOrNull { it.id == selection.effortId }
    Surface(
        modifier.width(320.dp),
        shape = WorkflowShapes.xl,
        color = colors.surfaceContainerHighest,
        shadowElevation = 3.dp,
    ) {
        Column(Modifier.padding(start = 16.dp, end = 16.dp, top = 4.dp, bottom = 10.dp)) {
            Row(Modifier.fillMaxWidth().height(36.dp), verticalAlignment = Alignment.CenterVertically) {
                Box(Modifier.size(WorkflowTheme.dimens.iconButtonTouch), contentAlignment = Alignment.Center) {
                    if (speedActive != null) {
                        WfIconButton(if (speedActive) Sym.BoltFill else Sym.Bolt, "速度", onSpeedToggle, checked = speedActive)
                    }
                }
                Text(
                    listOfNotNull(model?.label, effort?.label).joinToString(" · "),
                    Modifier.weight(1f), style = WorkflowTheme.text.titleSm, color = colors.onSurface,
                    textAlign = TextAlign.Center, maxLines = 1, overflow = TextOverflow.Ellipsis,
                )
                WfIconButton(Sym.ChevronRight, "全部模型", onOpenModelList)
            }
            ModelEffortSlider(models, selection, onSelectionChange)
        }
    }
}

/** Composer chip showing the current model and effort ("GPT-5.5 高 ⌄"). */
@Composable
fun ModelChip(label: String, onClick: () -> Unit, modifier: Modifier = Modifier, enabled: Boolean = true) {
    val colors = WorkflowTheme.colors
    Row(
        modifier
            .height(32.dp)
            .clip(WorkflowShapes.full)
            .background(colors.surfaceContainerHighest)
            .clickable(enabled = enabled, onClick = onClick)
            .padding(start = 12.dp, end = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(label, style = WorkflowTheme.text.label, color = if (enabled) colors.onSurface else colors.onSurface.copy(alpha = 0.38f), maxLines = 1)
        Spacer(Modifier.width(2.dp))
        SymbolIcon(Sym.KeyboardArrowDown, null, size = 18.dp, tint = colors.onSurfaceVariant)
    }
}
