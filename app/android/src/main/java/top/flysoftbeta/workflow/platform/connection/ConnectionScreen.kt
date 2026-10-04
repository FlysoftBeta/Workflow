package top.flysoftbeta.workflow.platform.connection

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import top.flysoftbeta.workflow.core.config.Density as ConfigDensity
import top.flysoftbeta.workflow.core.config.ThemeMode as ConfigTheme
import top.flysoftbeta.workflow.ui.design.DelayedVisibility
import top.flysoftbeta.workflow.ui.design.theme.ThemeMode
import top.flysoftbeta.workflow.ui.design.theme.UiDensity
import top.flysoftbeta.workflow.ui.design.theme.WorkflowDesignTheme
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** First launch explicitly chooses a connection before any workspace state or process is opened. */
@Composable
fun ConnectionScreen(manager: WorkspaceConnectionManager, status: ConnectionStatus) {
    ConnectionTheme(manager) {
        Surface(Modifier.fillMaxSize(), color = WorkflowTheme.colors.surface) {
            Box(Modifier.fillMaxSize().padding(24.dp), contentAlignment = Alignment.Center) {
                Column(Modifier.widthIn(max = 440.dp).fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(16.dp)) {
                    Text("连接工作区", style = WorkflowTheme.text.titleMd)
                    ConnectionStatusContent(manager, status)
                }
            }
        }
    }
}

/**
 * Keeps the Workbench of a lost connection visible but inert (docs/ux/workbench.md): no input, drag or
 * accessibility focus reaches it while [overlay] explains the connection.
 */
@Composable
fun InertWhileReconnecting(inert: Boolean, overlay: @Composable () -> Unit, content: @Composable () -> Unit) {
    Box(Modifier.fillMaxSize()) {
        Box(if (inert) Modifier.fillMaxSize().clearAndSetSemantics {} else Modifier.fillMaxSize()) { content() }
        if (inert) overlay()
    }
}

/** The reconnect card over the inert Workbench. It consumes every pointer event so nothing below responds. */
@Composable
fun ConnectionOverlay(manager: WorkspaceConnectionManager, status: ConnectionStatus) {
    ConnectionTheme(manager) {
        Box(
            Modifier.fillMaxSize()
                .background(WorkflowTheme.colors.scrim.copy(alpha = 0.32f))
                .pointerInput(Unit) { awaitPointerEventScope { while (true) awaitPointerEvent().changes.forEach { it.consume() } } }
                .padding(24.dp)
                .testTag("connection:overlay"),
            contentAlignment = Alignment.Center,
        ) {
            Surface(
                Modifier.widthIn(max = 440.dp).fillMaxWidth().semantics { liveRegion = LiveRegionMode.Polite },
                shape = WorkflowShapes.lg, color = WorkflowTheme.colors.surfaceContainerHigh, tonalElevation = 3.dp, shadowElevation = 6.dp,
            ) {
                Column(Modifier.padding(24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    val title = when (status) {
                        is ConnectionStatus.Failed -> "工作区离线"
                        is ConnectionStatus.Reconnecting -> "正在重新连接工作区"
                        else -> "正在连接工作区"
                    }
                    Text(title, style = WorkflowTheme.text.titleSm)
                    ConnectionStatusContent(manager, status, overlay = true)
                }
            }
        }
    }
}

@Composable
private fun ConnectionTheme(manager: WorkspaceConnectionManager, content: @Composable () -> Unit) {
    val local by manager.localConfig.collectAsState()
    val theme = when (local.appearance.theme) { ConfigTheme.SYSTEM -> ThemeMode.System; ConfigTheme.LIGHT -> ThemeMode.Light; ConfigTheme.DARK -> ThemeMode.Dark }
    val density = if (local.appearance.density == ConfigDensity.STANDARD) UiDensity.Standard else UiDensity.Compact
    WorkflowDesignTheme(themeMode = theme, density = density, content = content)
}

@Composable
private fun ConnectionStatusContent(manager: WorkspaceConnectionManager, status: ConnectionStatus, overlay: Boolean = false) {
    val muted = WorkflowTheme.colors.onSurfaceVariant
    when (status) {
        ConnectionStatus.Loading -> Progress()
        is ConnectionStatus.Connecting -> {
            Text("正在连接「${status.name}」", style = WorkflowTheme.text.body)
            Text(status.phase.label, style = WorkflowTheme.text.caption, color = muted)
            Progress()
        }
        is ConnectionStatus.Reconnecting -> {
            Text(status.cause, style = WorkflowTheme.text.body, color = muted)
            val phase = status.phase
            if (phase != null) {
                Text("第 ${status.attempt} 次尝试 · ${phase.label}", style = WorkflowTheme.text.caption, color = muted)
                Progress()
            } else {
                Text("第 ${status.attempt} 次尝试 · ${countdown(status.retryAt)} 秒后重试", style = WorkflowTheme.text.caption, color = muted)
            }
            Actions(primary = "立即重试", onPrimary = manager::retry, detail = status.detail, focus = overlay)
        }
        is ConnectionStatus.Failed -> {
            Text(status.message, style = WorkflowTheme.text.body, color = WorkflowTheme.colors.error)
            if (!status.retryable) Text("重试无法解决此问题，请更新应用。", style = WorkflowTheme.text.caption, color = muted)
            Actions(primary = if (status.retryable) "重试" else null, onPrimary = manager::retry, detail = status.detail, focus = overlay)
        }
        ConnectionStatus.Configure -> {
            Text("使用此设备内置的工作区，开始编辑文件、运行终端或对话。", style = WorkflowTheme.text.body)
            Button(onClick = { manager.useEmbedded() }, modifier = Modifier.fillMaxWidth()) { Text("使用此设备") }
        }
        is ConnectionStatus.Connected -> Unit
    }
}

@Composable
private fun Progress() {
    DelayedVisibility(true) { LinearProgressIndicator(Modifier.fillMaxWidth()) }
}

/** Whole seconds until [retryAt], ticking while the card is shown. */
@Composable
private fun countdown(retryAt: Long?): Long {
    var now by remember { mutableLongStateOf(System.currentTimeMillis()) }
    LaunchedEffect(retryAt) { while (true) { now = System.currentTimeMillis(); delay(250) } }
    return ((retryAt ?: now) - now + 999).coerceAtLeast(0) / 1000
}

/** The primary action and, when the Engine left diagnostic output, a Details toggle showing it selectable. */
@Composable
private fun Actions(primary: String?, onPrimary: () -> Unit, detail: String?, focus: Boolean) {
    var expanded by remember { mutableStateOf(false) }
    val requester = remember { FocusRequester() }
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
        if (primary != null) Button(onClick = onPrimary, modifier = Modifier.focusRequester(requester)) { Text(primary) }
        if (!detail.isNullOrBlank()) TextButton(onClick = { expanded = !expanded }) { Text(if (expanded) "收起详情" else "详情") }
    }
    if (focus && primary != null) LaunchedEffect(Unit) { runCatching { requester.requestFocus() } }
    if (expanded && detail != null) {
        SelectionContainer {
            Text(
                detail,
                Modifier.fillMaxWidth().heightIn(max = 200.dp).background(WorkflowTheme.colors.surfaceContainerHighest, WorkflowShapes.sm)
                    .verticalScroll(rememberScrollState()).padding(8.dp),
                style = WorkflowTheme.text.mono, color = WorkflowTheme.colors.onSurface,
            )
        }
    }
}
