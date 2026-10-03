package top.flysoftbeta.workflow.ui.design.dnd

import android.app.Activity
import android.content.ClipDescription
import android.content.ContextWrapper
import android.net.Uri
import androidx.activity.compose.BackHandler
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.spring
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.draganddrop.dragAndDropTarget
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draganddrop.DragAndDropEvent
import androidx.compose.ui.draganddrop.DragAndDropTarget
import androidx.compose.ui.draganddrop.mimeTypes
import androidx.compose.ui.draganddrop.toAndroidDragEvent
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInRoot
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import top.flysoftbeta.workflow.ui.design.SymbolIcon
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** Hover time after which [dropTarget]'s `onHoverActivate` fires. */
const val DRAG_HOVER_ACTIVATE_MILLIS = 600L

/**
 * Root of a drag-and-drop area (normally the whole window). Draws the drag shadow and target feedback
 * above [content], cancels an active drag on Back, and routes platform drags from other apps
 * ([acceptExternal] decides by MIME types) into the same targets as an [ExternalDragPayload].
 */
@Composable
fun DragDropHost(
    state: DragDropState,
    modifier: Modifier = Modifier,
    acceptExternal: (Set<String>) -> Boolean = { mimes -> mimes.isNotEmpty() },
    content: @Composable BoxScope.() -> Unit,
) {
    val density = LocalDensity.current
    SideEffect {
        with(density) {
            state.insertionLineThickness = 2.dp.toPx()
            state.insertionLineLength = 28.dp.toPx()
        }
    }
    val activity = LocalContext.current.findActivity()
    val currentAccept by rememberUpdatedState(acceptExternal)
    val external = remember(state, activity) { ExternalDropBridge(state, activity) }
    BackHandler(enabled = state.isDragging) { state.cancel() }

    // Hover activation (600ms dwell).
    LaunchedEffect(state) {
        snapshotFlow { state.session?.takeUnless { it.returning }?.targetKey to (state.session?.feedback != DropFeedback.Rejected) }
            .collectLatest { (key, accepted) ->
                if (key == null || !accepted) return@collectLatest
                delay(DRAG_HOVER_ACTIVATE_MILLIS)
                state.targets[key]?.onHoverActivate?.invoke()
            }
    }

    Box(
        modifier
            .onGloballyPositioned {
                state.hostCoordinates = it
                external.hostOffsetInRoot = it.positionInRoot()
            }
            .dragAndDropTarget(
                shouldStartDragAndDrop = { event -> currentAccept(event.mimeTypes()) },
                target = external,
            ),
    ) {
        CompositionLocalProvider(LocalDragDropState provides state) { content() }
        DragOverlay(state)
    }
}

private tailrec fun android.content.Context.findActivity(): Activity? = when (this) {
    is Activity -> this
    is ContextWrapper -> baseContext.findActivity()
    else -> null
}

/** Platform DnD → the same targets and feedback as in-app drags. */
private class ExternalDropBridge(private val state: DragDropState, private val activity: Activity?) : DragAndDropTarget {
    var hostOffsetInRoot: Offset = Offset.Zero

    private fun position(event: DragAndDropEvent): Offset {
        val drag = event.toAndroidDragEvent()
        return Offset(drag.x, drag.y) - hostOffsetInRoot
    }

    override fun onStarted(event: DragAndDropEvent) {
        state.start(ExternalDragPayload(emptyList(), event.mimeTypes()), sourceKey = null, origin = null, pointer = Offset(-1f, -1f), external = true)
    }

    override fun onEntered(event: DragAndDropEvent) = state.move(position(event))
    override fun onMoved(event: DragAndDropEvent) = state.move(position(event))
    override fun onExited(event: DragAndDropEvent) = state.move(Offset(-1f, -1f))

    override fun onDrop(event: DragAndDropEvent): Boolean {
        val drag = event.toAndroidDragEvent()
        state.move(position(event))
        val clip = drag.clipData
        val uris = buildList<Uri> {
            if (clip != null) for (i in 0 until clip.itemCount) clip.getItemAt(i).uri?.let(::add)
        }
        if (uris.isEmpty()) {
            state.cancel(); return false
        }
        // Content URIs from other apps need a grant scoped to this drop.
        activity?.requestDragAndDropPermissions(drag)
        val mimes = buildSet {
            val description: ClipDescription? = drag.clipDescription
            if (description != null) for (i in 0 until description.mimeTypeCount) add(description.getMimeType(i))
        }
        return state.drop(ExternalDragPayload(uris, mimes))
    }

    override fun onEnded(event: DragAndDropEvent) {
        if (state.session?.external == true) state.cancel()
    }
}

@Composable
private fun DragOverlay(state: DragDropState) {
    val session = state.session ?: return
    val colors = WorkflowTheme.colors
    val density = LocalDensity.current
    val feedback = session.feedback

    // Placement preview springs between zones/regions.
    val preview = remember(session) { Animatable(Rect.Zero, Rect.VectorConverter) }
    val placement = feedback as? DropFeedback.Placement
    LaunchedEffect(placement?.preview) {
        val target = placement?.preview ?: return@LaunchedEffect
        if (preview.value == Rect.Zero) preview.snapTo(target) else preview.animateTo(target, spring(stiffness = Spring.StiffnessMediumLow, dampingRatio = Spring.DampingRatioNoBouncy))
    }
    LaunchedEffect(placement == null) { if (placement == null) preview.snapTo(Rect.Zero) }

    val corner = with(density) { CornerRadius(12.dp.toPx()) }
    val stroke = with(density) { 2.dp.toPx() }
    Canvas(Modifier.fillMaxSize()) {
        when (feedback) {
            is DropFeedback.Placement -> {
                val r = preview.value.takeUnless { it == Rect.Zero } ?: feedback.preview
                val inset = r.deflate(stroke)
                drawRoundRect(colors.primary.copy(alpha = 0.12f), inset.topLeft, inset.size, corner)
                drawRoundRect(colors.primary, inset.topLeft, inset.size, corner, style = Stroke(stroke))
            }
            is DropFeedback.Insertion -> drawRoundRect(colors.primary, feedback.line.topLeft, feedback.line.size, CornerRadius(stroke / 2))
            is DropFeedback.Area -> when (feedback.style) {
                AreaStyle.Outline -> {
                    val inset = feedback.rect.deflate(stroke / 2)
                    drawRoundRect(colors.primary.copy(alpha = 0.08f), inset.topLeft, inset.size, corner)
                    drawRoundRect(colors.primary, inset.topLeft, inset.size, corner, style = Stroke(stroke))
                }
                AreaStyle.Scrim -> drawRect(colors.surface.copy(alpha = 0.72f), feedback.rect.topLeft, feedback.rect.size)
                AreaStyle.Self -> Unit
            }
            DropFeedback.Rejected, null -> Unit
        }
    }
    (feedback as? DropFeedback.Area)?.label?.let { label -> AreaLabel(feedback.rect, label) }
    DragShadow(state, session)
}

@Composable
private fun AreaLabel(rect: Rect, label: String) {
    Layout(content = {
        Surface(shape = WorkflowShapes.sm, color = WorkflowTheme.colors.primary, contentColor = WorkflowTheme.colors.onPrimary) {
            Text(label, Modifier.padding(horizontal = 10.dp, vertical = 4.dp), style = WorkflowTheme.text.labelActive)
        }
    }, modifier = Modifier.fillMaxSize()) { measurables, constraints ->
        val placeable = measurables.first().measure(constraints.copy(minWidth = 0, minHeight = 0))
        layout(constraints.maxWidth, constraints.maxHeight) {
            // Top-centre of the target: the shadow chip rides 32dp above the finger, usually lower in the area.
            placeable.place((rect.center.x - placeable.width / 2f).toInt(), (rect.top + 8.dp.roundToPx()).toInt())
        }
    }
}

@Composable
private fun DragShadow(state: DragDropState, session: DragSession) {
    val density = LocalDensity.current
    val motion = WorkflowTheme.motion
    val position = remember(session) { Animatable(session.pointer, Offset.VectorConverter) }
    val lift = remember(session) { Animatable(0.9f) }
    LaunchedEffect(session) { lift.animateTo(1f, motion.fastSpatialSpec()) }
    LaunchedEffect(session) {
        snapshotFlow { session.pointer to session.returning }.collectLatest { (pointer, returning) ->
            if (returning) {
                val origin = session.origin
                if (origin != null) position.animateTo(origin, motion.defaultSpatialSpec())
                state.finishReturn(session)
            } else {
                position.snapTo(pointer)
            }
        }
    }
    if (session.external && session.targetKey == null) return // external drags have the platform shadow
    val rejected = session.feedback == DropFeedback.Rejected
    val above = with(density) { 32.dp.toPx() }
    Layout(content = {
        Surface(
            shape = WorkflowShapes.sm,
            color = WorkflowTheme.colors.surfaceContainerHighest,
            contentColor = WorkflowTheme.colors.onSurface,
            shadowElevation = 3.dp,
            modifier = Modifier.graphicsLayer { scaleX = lift.value; scaleY = lift.value },
        ) {
            Row(Modifier.height(32.dp).padding(horizontal = 10.dp), verticalAlignment = Alignment.CenterVertically) {
                SymbolIcon(
                    if (rejected) Sym.Block else session.payload.icon, null, size = 16.dp,
                    tint = if (rejected) WorkflowTheme.colors.error else WorkflowTheme.colors.onSurfaceVariant,
                )
                Spacer(Modifier.width(6.dp))
                Text(
                    session.payload.label, Modifier.widthIn(max = 200.dp), style = WorkflowTheme.text.label,
                    maxLines = 1, overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }, modifier = Modifier.fillMaxSize()) { measurables, constraints ->
        val placeable = measurables.first().measure(constraints.copy(minWidth = 0, minHeight = 0))
        layout(constraints.maxWidth, constraints.maxHeight) {
            val p = position.value
            placeable.place((p.x - placeable.width / 2f).toInt(), (p.y - above - placeable.height / 2f).toInt())
        }
    }
}
