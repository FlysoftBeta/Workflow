package top.flysoftbeta.workflow.feature.chat

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.agent.model.LoginState
import top.flysoftbeta.workflow.agent.model.ProcessState
import top.flysoftbeta.workflow.app.panel.PanelFrame
import top.flysoftbeta.workflow.app.panel.PanelPlacement
import top.flysoftbeta.workflow.ui.design.InlineError
import top.flysoftbeta.workflow.ui.design.RegionLoading
import top.flysoftbeta.workflow.ui.design.SymbolIcon
import top.flysoftbeta.workflow.ui.design.TextAction
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** Content max width of the conversation column (docs/ui.md §4.5). */
internal val ChatMaxWidth = 720.dp

@Composable
internal fun ConversationContent(c: ConversationController, frame: PanelFrame, modifier: Modifier) {
    val colors = WorkflowTheme.colors
    val entry = c.entry
    val backend = c.backend
    val turns = c.thread?.turns.orEmpty()
    val loggedOut = backend?.account?.state == LoginState.LOGGED_OUT
    val process = backend?.process
    val narrow = frame.placement == PanelPlacement.AUX
    LaunchedEffect(entry?.backend, c.available) {
        if (c.available) entry?.backend?.let { c.hub.warmUp(it) }
    }
    BoxWithConstraints(modifier.fillMaxSize().background(colors.surface)) {
        val maxCardHeight = maxHeight * 0.5f
        Column(Modifier.fillMaxSize()) {
            Box(Modifier.weight(1f).fillMaxWidth()) {
                when {
                    entry == null -> RegionLoading(true)
                    turns.isEmpty() && entry.backendThreadId == null -> StartPane(c, Modifier.fillMaxSize())
                    turns.isEmpty() && !c.available -> NeedsEnvironmentPane(c, Modifier.fillMaxSize())
                    turns.isEmpty() && loggedOut -> LoginPane(c, Modifier.fillMaxSize())
                    turns.isEmpty() && (process is ProcessState.Failed || process is ProcessState.Exited) -> BackendProblemPane(c, Modifier.fillMaxSize())
                    else -> {
                        TranscriptView(c, frame, Modifier.fillMaxSize())
                        if (turns.isEmpty()) RegionLoading(true)
                    }
                }
                if (!c.atBottom && turns.isNotEmpty()) {
                    JumpToBottom(c.openRequests.size, Modifier.align(Alignment.BottomCenter).padding(bottom = 8.dp)) { c.scrollToBottom() }
                }
            }
            Column(Modifier.fillMaxWidth().padding(horizontal = if (narrow) 4.dp else 8.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                val width = Modifier.widthIn(max = ChatMaxWidth + 16.dp).fillMaxWidth()
                if (c.openRequests.isNotEmpty()) RequestCards(c, width.heightIn(max = maxCardHeight))
                if (turns.isNotEmpty() && loggedOut) LoginPrompt(c, width)
                if (c.reviewerBlocked) InlineError("此对话的审批没有交给你，已停止发送", width)
                c.problem?.let { message ->
                    var details by remember { mutableStateOf(false) }
                    InlineError(message, width, listOfNotNull(
                        c.problemDetail?.let { TextAction("详情") { details = true } },
                        TextAction("关闭") { c.problem = null },
                    ))
                    if (details) DetailsDialog(message, c.problemDetail.orEmpty()) { details = false }
                }
                if (process is ProcessState.Failed && turns.isNotEmpty()) BackendProblemRow(c, width)
                Composer(c, frame, width.padding(bottom = 8.dp))
            }
        }
    }
    if (c.renaming) RenameDialog(c.title, onDismiss = { c.renaming = false }) { c.rename(it); c.renaming = false }
    if (c.showAllConversations) AllConversationsDialog(c)
    if (c.toolsPage != null) AgentToolsDialog(c)
    if (c.confirmingDelete) DeleteConversationDialog(c)
}

/** A native lazy transcript; the controller retains viewport state across panel reattachment. */
@Composable
private fun TranscriptView(c: ConversationController, frame: PanelFrame, modifier: Modifier) {
    top.flysoftbeta.workflow.feature.chat.transcript.NativeTranscript(
        c.transcript, c.transcriptState, c.callbacks, modifier,
    )
}

@Composable
private fun JumpToBottom(pending: Int, modifier: Modifier, onClick: () -> Unit) {
    val colors = WorkflowTheme.colors
    Row(
        modifier
            .height(36.dp)
            .clip(WorkflowShapes.full)
            .background(colors.surfaceContainerHighest)
            .clickable(onClick = onClick)
            .padding(horizontal = if (pending > 0) 14.dp else 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.Center,
    ) {
        if (pending > 0) {
            Text("$pending 个待确认", style = WorkflowTheme.text.label, color = colors.onSurface)
            Spacer(Modifier.width(4.dp))
        }
        SymbolIcon(Sym.ArrowDownward, if (pending > 0) null else "回到底部", size = 20.dp, tint = colors.onSurface)
    }
}

@Composable
private fun BackendProblemRow(c: ConversationController, modifier: Modifier) {
    var details by remember { mutableStateOf(false) }
    val name = ChatText.backendName(c.entry?.backend ?: return)
    InlineError("$name 未能启动", modifier, listOf(TextAction("重试") { c.retryBackend() }, TextAction("详情") { details = true }))
    if (details) DetailsDialog("$name 未能启动", backendDetail(c)) { details = false }
}

@Composable
private fun BackendProblemPane(c: ConversationController, modifier: Modifier) {
    Box(modifier, contentAlignment = Alignment.Center) {
        BackendProblemRow(c, Modifier.widthIn(max = 480.dp).padding(16.dp))
    }
}

internal fun backendDetail(c: ConversationController): String = when (val p = c.backend?.process) {
    is ProcessState.Failed -> listOf(p.message, p.stderrTail).filter { it.isNotBlank() }.joinToString("\n\n")
    is ProcessState.Exited -> "退出码 ${p.exitCode ?: "?"}\n\n${p.stderrTail}"
    else -> ""
}

@Composable
internal fun DetailsDialog(title: String, text: String, onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title, style = WorkflowTheme.text.titleMd) },
        text = {
            Box(Modifier.heightIn(max = 360.dp).fillMaxWidth().background(WorkflowTheme.colors.surfaceContainerLow, WorkflowShapes.sm)) {
                androidx.compose.foundation.text.selection.SelectionContainer {
                    Text(
                        text.ifBlank { "没有更多信息" },
                        Modifier.padding(8.dp).verticalScroll(rememberScrollState()),
                        style = WorkflowTheme.text.mono, color = WorkflowTheme.colors.onSurface,
                    )
                }
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text("关闭") } },
    )
}
