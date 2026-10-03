package top.flysoftbeta.workflow.ui.design

import android.view.HapticFeedbackConstants
import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.movableContentOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.ui.design.dnd.LocalDragDropState
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme
import top.flysoftbeta.workflow.ui.design.theme.frame

/** Hoisted state of a [DockingLayout]; persist it per Session. Widths are the docked widths in dp. */
@Stable
class DockingState(sideOpen: Boolean, auxOpen: Boolean, sideWidth: Dp, auxWidth: Dp) {
    var sideOpen by mutableStateOf(sideOpen)
    var auxOpen by mutableStateOf(auxOpen)
    var sideWidth by mutableStateOf(sideWidth)
    var auxWidth by mutableStateOf(auxWidth)
}

@Composable
fun rememberDockingState(sideOpen: Boolean = true, auxOpen: Boolean = true, spec: DockingSpec = DockingSpec()): DockingState =
    remember { DockingState(sideOpen, auxOpen, spec.sideDefault.dp, spec.auxDefault.dp) }

/**
 * Window-level region container of the Workbench (docs/ux/README.md §3.2): `[side][center][aux]` on the frame
 * color with 4dp seams. Docking follows [resolveDocking]; a region that cannot dock becomes an overlay
 * (side: drawer from the left, aux: slides in from the right, elevation 3) that closes on an outside tap
 * or Back and keeps its state (the content is moved, not recreated). Seams between docked regions are
 * draggable: below 50% of the minimum the region collapses (CLOCK_TICK), double tap restores the default.
 */
@Composable
fun DockingLayout(
    state: DockingState,
    side: @Composable () -> Unit,
    center: @Composable () -> Unit,
    aux: @Composable () -> Unit,
    modifier: Modifier = Modifier,
    spec: DockingSpec = DockingSpec(),
) {
    val colors = WorkflowTheme.colors
    val density = LocalDensity.current
    val view = LocalView.current
    val sideContent = remember { movableContentOf(side) }
    val auxContent = remember { movableContentOf(aux) }
    var pressedSeam by remember { mutableIntStateOf(-1) }
    val dragging = LocalDragDropState.current?.isDragging == true

    BoxWithConstraints(modifier.fillMaxSize().background(colors.frame)) {
        val window = maxWidth.value
        val decision = resolveDocking(window, state.sideOpen, state.auxOpen, state.sideWidth.value, state.auxWidth.value, spec)
        val gap = spec.gap
        val gapPx = with(density) { gap.dp.toPx() }
        val widthPx = with(density) { maxWidth.toPx() }
        val sidePx = with(density) { decision.sideWidth.dp.toPx() }
        val auxPx = with(density) { decision.auxWidth.dp.toPx() }
        // Seam centres (px): 0 = side|center, 1 = center|aux.
        val seams = buildList {
            add(if (decision.sideDocked) gapPx + sidePx + gapPx / 2 else Float.NaN)
            add(if (decision.auxDocked) widthPx - gapPx - auxPx - gapPx / 2 else Float.NaN)
        }
        val bandPx = with(density) { WorkflowTheme.dimens.dividerTouch.toPx() }
        Row(
            Modifier
                .fillMaxSize()
                .padding(gap.dp)
                .seamGestures(horizontal = true, bandPx = bandPx, callbacks = object : SeamCallbacks {
                    override fun seams() = seams.map { if (it.isNaN()) -1e9f else it - gapPx }
                    override fun onPress(index: Int, pressed: Boolean) { pressedSeam = if (pressed) index else -1 }
                    override fun onDrag(index: Int, position: Float) {
                        val x = position + gapPx
                        if (index == 0) {
                            val proposed = with(density) { (x - gapPx - gapPx / 2).toDp() }.value
                            if (proposed < spec.sideMin * 0.5f) {
                                if (state.sideOpen) view.performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
                                state.sideOpen = false
                            } else {
                                state.sideWidth = proposed.coerceIn(spec.sideMin, spec.sideMaxFraction * window).dp
                            }
                        } else {
                            val proposed = with(density) { (widthPx - gapPx - x - gapPx / 2).toDp() }.value
                            if (proposed < spec.auxMin * 0.5f) {
                                if (state.auxOpen) view.performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
                                state.auxOpen = false
                            } else {
                                state.auxWidth = proposed.coerceIn(spec.auxMin, spec.auxMaxFraction * window).dp
                            }
                        }
                    }
                    override fun onDoubleTap(index: Int) {
                        if (index == 0) state.sideWidth = spec.sideDefault.dp else state.auxWidth = spec.auxDefault.dp
                    }
                }),
        ) {
            if (decision.sideDocked) {
                Box(Modifier.width(decision.sideWidth.dp).fillMaxHeight()) { sideContent() }
                Spacer(Modifier.width(gap.dp))
            }
            Box(Modifier.weight(1f).fillMaxHeight()) { center() }
            if (decision.auxDocked) {
                Spacer(Modifier.width(gap.dp))
                Box(Modifier.width(decision.auxWidth.dp).fillMaxHeight()) { auxContent() }
            }
        }
        if (pressedSeam >= 0 && !seams[pressedSeam].isNaN()) {
            val pill = with(density) { Size(4.dp.toPx(), 32.dp.toPx()) }
            Canvas(Modifier.fillMaxSize()) {
                drawRoundRect(
                    colors.primary, Offset(seams[pressedSeam] - pill.width / 2, size.height / 2 - pill.height / 2), pill,
                    CornerRadius(pill.width / 2),
                )
            }
        }

        val sideOverlay = state.sideOpen && !decision.sideDocked
        val auxOverlay = state.auxOpen && !decision.auxDocked
        OverlayScrim(visible = sideOverlay || auxOverlay, strong = decision.narrow) {
            if (sideOverlay) state.sideOpen = false
            if (auxOverlay) state.auxOpen = false
        }
        AnimatedVisibility(
            visible = sideOverlay,
            enter = slideInHorizontally(WorkflowTheme.motion.defaultSpatialSpec()) { -it },
            exit = slideOutHorizontally(WorkflowTheme.motion.defaultSpatialSpec()) { -it },
            modifier = Modifier.align(Alignment.CenterStart),
        ) {
            Surface(
                Modifier.width(decision.sideOverlayWidth.dp).fillMaxHeight(),
                color = colors.frame, shadowElevation = 3.dp, tonalElevation = 0.dp,
            ) { Box(Modifier.padding(gap.dp)) { if (sideOverlay) sideContent() } }
        }
        AnimatedVisibility(
            visible = auxOverlay,
            enter = slideInHorizontally(WorkflowTheme.motion.defaultSpatialSpec()) { it },
            exit = slideOutHorizontally(WorkflowTheme.motion.defaultSpatialSpec()) { it },
            modifier = Modifier.align(Alignment.CenterEnd),
        ) {
            Surface(
                Modifier.width(decision.auxOverlayWidth.dp).fillMaxHeight(),
                color = colors.frame, shadowElevation = 3.dp,
            ) { Box(Modifier.padding(gap.dp)) { if (auxOverlay) auxContent() } }
        }
        BackHandler(enabled = (sideOverlay || auxOverlay) && !dragging) {
            if (auxOverlay) state.auxOpen = false else state.sideOpen = false
        }
    }
}

@Composable
private fun OverlayScrim(visible: Boolean, strong: Boolean, onDismiss: () -> Unit) {
    AnimatedVisibility(visible, enter = fadeIn(WorkflowTheme.motion.fastEffectsSpec()), exit = fadeOut(WorkflowTheme.motion.fastEffectsSpec())) {
        Box(
            Modifier
                .fillMaxSize()
                .background(WorkflowTheme.colors.scrim.copy(alpha = if (strong) 0.32f else 0.16f))
                .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null, onClick = onDismiss),
        )
    }
}

/**
 * A region's content card: `surface`, `md` corners (docs/ux/README.md §3.2). Put the Stack's tab row above it
 * on the frame; the active tab shares the card's color.
 */
@Composable
fun RegionCard(modifier: Modifier = Modifier, content: @Composable () -> Unit) {
    Surface(modifier, shape = WorkflowShapes.md, color = WorkflowTheme.colors.surface, content = content)
}
