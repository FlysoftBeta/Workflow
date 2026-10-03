package top.flysoftbeta.workflow.ui.design

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.Snackbar
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.SnackbarResult
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** A text action of a state/notice row. */
@Immutable
data class TextAction(val label: String, val onClick: () -> Unit)

/**
 * Centred empty state: optional short message plus text buttons (e.g. editor without tabs:
 * 打开文件 · 新建文件 · 新建终端). No illustration, no slogan (docs/ux/README.md §6).
 */
@Composable
fun EmptyState(actions: List<TextAction>, modifier: Modifier = Modifier, message: String? = null) {
    Column(modifier.fillMaxSize(), verticalArrangement = Arrangement.Center, horizontalAlignment = Alignment.CenterHorizontally) {
        if (message != null) {
            Text(message, style = WorkflowTheme.text.body, color = WorkflowTheme.colors.onSurfaceVariant)
            Spacer(Modifier.size(4.dp))
        }
        Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            actions.forEach { TextButton(onClick = it.onClick) { Text(it.label) } }
        }
    }
}

/** One-line error at the place it happened: error icon + short reason + [重试]/[详情]. Never a Toast. */
@Composable
fun InlineError(message: String, modifier: Modifier = Modifier, actions: List<TextAction> = emptyList()) {
    val colors = WorkflowTheme.colors
    Row(
        modifier.fillMaxWidth().heightIn(min = 36.dp).padding(horizontal = 12.dp).semantics { liveRegion = LiveRegionMode.Polite },
        verticalAlignment = Alignment.CenterVertically,
    ) {
        SymbolIcon(Sym.Error, null, size = 18.dp, tint = colors.error)
        Spacer(Modifier.width(8.dp))
        Text(message, Modifier.weight(1f, fill = false), style = WorkflowTheme.text.body, color = colors.onSurface, maxLines = 2, overflow = TextOverflow.Ellipsis)
        actions.forEach { TextButton(onClick = it.onClick) { Text(it.label) } }
    }
}

enum class NoticeTone { Tertiary, Error, Warning, Neutral }

/**
 * Temporary strip at the top of content (conflict bar 36dp tertiaryContainer, "另一个 VPN 正在运行"
 * errorContainer banner, "环境配置已更改" row). Message + text actions, no helper paragraphs.
 */
@Composable
fun NoticeBar(message: String, tone: NoticeTone, modifier: Modifier = Modifier, actions: List<TextAction> = emptyList()) {
    val colors = WorkflowTheme.colors
    val extended = WorkflowTheme.extendedColors
    val (container, content) = when (tone) {
        NoticeTone.Tertiary -> colors.tertiaryContainer to colors.onTertiaryContainer
        NoticeTone.Error -> colors.errorContainer to colors.onErrorContainer
        NoticeTone.Warning -> extended.warningContainer to extended.onWarningContainer
        NoticeTone.Neutral -> colors.surfaceContainerHigh to colors.onSurface
    }
    Row(
        modifier.fillMaxWidth().heightIn(min = 36.dp).background(container).padding(start = 12.dp, end = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(message, Modifier.weight(1f), style = WorkflowTheme.text.label, color = content, maxLines = 1, overflow = TextOverflow.Ellipsis)
        actions.forEach { TextButton(onClick = it.onClick) { Text(it.label, color = content) } }
    }
}

/**
 * Shows [content] only after [delayMillis] of continuous loading (nothing for fast loads, docs/ux/README.md §6).
 */
@Composable
fun DelayedVisibility(loading: Boolean, delayMillis: Long = 300, content: @Composable () -> Unit) {
    var show by remember { mutableStateOf(false) }
    LaunchedEffect(loading) {
        show = false
        if (loading) { delay(delayMillis); show = true }
    }
    AnimatedVisibility(show && loading, enter = fadeIn(WorkflowTheme.motion.fastEffectsSpec())) { content() }
}

/** Region-level Expressive loading indicator, shown after 300ms. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun RegionLoading(loading: Boolean, modifier: Modifier = Modifier) {
    Box(modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        DelayedVisibility(loading) { LoadingIndicator() }
    }
}

/** 16dp inline loading indicator (row level), shown after 300ms. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun InlineLoading(loading: Boolean, modifier: Modifier = Modifier) {
    DelayedVisibility(loading) { LoadingIndicator(modifier.size(16.dp)) }
}

/**
 * Snackbar host (docs/ux/README.md §6): bottom centre, max 480dp wide, above the IME. Place it at the bottom
 * of the window-level Box.
 */
@Composable
fun WfSnackbarHost(state: SnackbarHostState, modifier: Modifier = Modifier) {
    Box(modifier.fillMaxWidth().imePadding().padding(12.dp), contentAlignment = Alignment.BottomCenter) {
        SnackbarHost(state, Modifier.widthIn(max = 480.dp)) { data ->
            Snackbar(
                data,
                shape = WorkflowShapes.sm,
                containerColor = WorkflowTheme.colors.inverseSurface,
                contentColor = WorkflowTheme.colors.inverseOnSurface,
                actionColor = WorkflowTheme.colors.inversePrimary,
            )
        }
    }
}

/** Shows "message · 撤销"; returns true when the user tapped undo. */
suspend fun SnackbarHostState.showUndo(message: String, undoLabel: String = "撤销"): Boolean =
    showSnackbar(message, actionLabel = undoLabel, duration = SnackbarDuration.Long) == SnackbarResult.ActionPerformed

/** Small status dot (dirty, pending, in progress). */
@Composable
fun StatusDot(color: Color, modifier: Modifier = Modifier, size: androidx.compose.ui.unit.Dp = 8.dp) {
    Box(modifier.size(size).background(color, androidx.compose.foundation.shape.CircleShape))
}
