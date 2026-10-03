package top.flysoftbeta.workflow.ui.design

import androidx.annotation.DrawableRes
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

@Immutable
sealed interface AttachmentState {
    data object Ready : AttachmentState
    /** [progress] in 0..1, or null for indeterminate. */
    data class Uploading(val progress: Float?) : AttachmentState
    data object Failed : AttachmentState
}

/** A pending attachment of the composer (image thumbnail or file chip). */
@Immutable
data class AttachmentItem(
    val key: String,
    val name: String,
    val isImage: Boolean,
    @param:DrawableRes val icon: Int = if (isImage) Sym.Image else Sym.Draft,
    val thumbnail: ImageBitmap? = null,
    val state: AttachmentState = AttachmentState.Ready,
)

/**
 * The 56dp attachment strip above the composer text (docs/ui.md §4.8); show it only when non-empty.
 * Images are 56×56 thumbnails, files 40dp chips (name ≤160dp). Every item has a 20dp × at its top-end
 * corner; uploads show a progress ring; failures an error outline and ↻.
 */
@Composable
fun AttachmentStrip(
    items: List<AttachmentItem>,
    onRemove: (String) -> Unit,
    modifier: Modifier = Modifier,
    onRetry: (String) -> Unit = {},
    onOpen: (String) -> Unit = {},
) {
    Row(
        modifier.height(64.dp).horizontalScroll(rememberScrollState()).padding(horizontal = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        items.forEach { item ->
            Box(Modifier.padding(top = 6.dp, end = 6.dp)) {
                if (item.isImage) ImageAttachment(item, onOpen) else FileAttachment(item, onOpen, onRetry)
                RemoveBadge(Modifier.align(Alignment.TopEnd).offset(8.dp, (-8).dp)) { onRemove(item.key) }
            }
        }
    }
}

@Composable
private fun ImageAttachment(item: AttachmentItem, onOpen: (String) -> Unit) {
    val colors = WorkflowTheme.colors
    val failed = item.state == AttachmentState.Failed
    Box(
        Modifier
            .size(56.dp)
            .clip(WorkflowShapes.sm)
            .background(colors.surfaceContainerHighest)
            .then(if (failed) Modifier.border(1.5.dp, colors.error, WorkflowShapes.sm) else Modifier)
            .clickable { onOpen(item.key) }
            .semantics { contentDescription = item.name },
        contentAlignment = Alignment.Center,
    ) {
        if (item.thumbnail != null) {
            Image(item.thumbnail, null, Modifier.size(56.dp), contentScale = ContentScale.Crop)
        } else {
            SymbolIcon(item.icon, null, tint = colors.onSurfaceVariant)
        }
        ProgressOverlay(item.state)
    }
}

@Composable
private fun FileAttachment(item: AttachmentItem, onOpen: (String) -> Unit, onRetry: (String) -> Unit) {
    val colors = WorkflowTheme.colors
    val failed = item.state == AttachmentState.Failed
    Row(
        Modifier
            .height(40.dp)
            .clip(WorkflowShapes.sm)
            .background(colors.surfaceContainerHighest)
            .then(if (failed) Modifier.border(1.5.dp, colors.error, WorkflowShapes.sm) else Modifier)
            .clickable { if (failed) onRetry(item.key) else onOpen(item.key) }
            .padding(horizontal = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        when (val state = item.state) {
            is AttachmentState.Uploading -> UploadRing(state.progress)
            AttachmentState.Failed -> SymbolIcon(Sym.Refresh, "重试", size = 18.dp, tint = colors.error)
            AttachmentState.Ready -> SymbolIcon(item.icon, null, size = 18.dp, tint = colors.onSurfaceVariant)
        }
        Spacer(Modifier.width(8.dp))
        Text(
            item.name, Modifier.widthIn(max = 160.dp), style = WorkflowTheme.text.label, color = colors.onSurface,
            maxLines = 1, overflow = TextOverflow.Ellipsis,
        )
    }
}

@Composable
private fun ProgressOverlay(state: AttachmentState) {
    when (state) {
        is AttachmentState.Uploading -> Box(
            Modifier.size(56.dp).background(WorkflowTheme.colors.scrim.copy(alpha = 0.32f)),
            contentAlignment = Alignment.Center,
        ) { UploadRing(state.progress, color = WorkflowTheme.colors.inverseOnSurface) }
        AttachmentState.Failed -> SymbolIcon(Sym.Refresh, "重试", tint = WorkflowTheme.colors.error)
        AttachmentState.Ready -> Unit
    }
}

@Composable
private fun UploadRing(progress: Float?, color: androidx.compose.ui.graphics.Color = WorkflowTheme.colors.primary) {
    if (progress == null) {
        CircularProgressIndicator(Modifier.size(18.dp), color = color, strokeWidth = 2.dp)
    } else {
        CircularProgressIndicator(progress = { progress }, modifier = Modifier.size(18.dp), color = color, strokeWidth = 2.dp)
    }
}

@Composable
private fun RemoveBadge(modifier: Modifier, onClick: () -> Unit) {
    Box(
        modifier
            .size(20.dp)
            .clip(CircleShape)
            .background(WorkflowTheme.colors.inverseSurface)
            .clickable(onClick = onClick)
            .semantics { contentDescription = "移除" },
        contentAlignment = Alignment.Center,
    ) { SymbolIcon(Sym.Close, null, size = 14.dp, tint = WorkflowTheme.colors.inverseOnSurface) }
}
