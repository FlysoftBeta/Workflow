package top.flysoftbeta.workflow.feature.chat

import androidx.compose.animation.core.animateFloat
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.ConversationEntry
import top.flysoftbeta.workflow.agent.model.LoginPhase
import top.flysoftbeta.workflow.agent.model.LoginState
import top.flysoftbeta.workflow.agent.model.LoginView
import top.flysoftbeta.workflow.agent.model.isPending
import top.flysoftbeta.workflow.agent.model.ProcessState
import top.flysoftbeta.workflow.ui.design.InlineError
import top.flysoftbeta.workflow.ui.design.InlineLoading
import top.flysoftbeta.workflow.ui.design.RegionLoading
import top.flysoftbeta.workflow.ui.design.SearchField
import top.flysoftbeta.workflow.ui.design.SignInContent
import top.flysoftbeta.workflow.ui.design.openSignInLink
import top.flysoftbeta.workflow.ui.design.SymbolIcon
import top.flysoftbeta.workflow.ui.design.TextAction
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/**
 * New conversation (docs/ux/README.md §6): the backend choice in the centre (remembered), nothing else
 * unless the chosen backend needs something — a login or the environment.
 */
@Composable
internal fun StartPane(c: ConversationController, modifier: Modifier) {
    val entry = c.entry ?: return
    val available by c.hub.available.collectAsState()
    Column(modifier.padding(16.dp), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.Center) {
        SingleChoiceSegmentedButtonRow(Modifier.height(40.dp)) {
            val kinds = BackendKind.entries
            kinds.forEachIndexed { index, kind ->
                SegmentedButton(
                    selected = entry.backend == kind,
                    onClick = { if (entry.backend != kind) c.setBackend(kind) },
                    shape = SegmentedButtonDefaults.itemShape(index, kinds.size),
                    icon = { SymbolIcon(ConversationController.backendIcon(kind), null, size = 18.dp) },
                    modifier = Modifier.widthIn(min = 148.dp),
                ) { Text(ChatText.backendName(kind), style = WorkflowTheme.text.label, maxLines = 1, softWrap = false) }
            }
        }
        Spacer(Modifier.height(20.dp))
        val account = c.backend?.account
        when {
            entry.backend !in available -> NeedsEnvironmentBody(c, available)
            LoginView.needsLogin(account) -> LoginBody(c)
            c.backend?.process is ProcessState.Failed -> InlineError(
                "${ChatText.backendName(entry.backend)} 未能启动", Modifier.widthIn(max = 480.dp),
                listOf(TextAction("重试") { c.retryBackend() }),
            )
        }
    }
}

/**
 * Before the conversation entry arrives: the hub waits for a preparing environment, so this shows that
 * preparation (with its retry) rather than a bare indicator, or the failure that stopped the open.
 */
@Composable
internal fun OpeningPane(c: ConversationController, modifier: Modifier) {
    val environment by c.services.environment.collectAsState()
    val failure = c.openFailure
    if (failure == null && environment.usable) { RegionLoading(true, modifier); return }
    Column(modifier.padding(16.dp), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.Center) {
        if (failure != null) {
            InlineError(failure, Modifier.widthIn(max = 480.dp), listOf(TextAction("重试") { c.retryOpen() }))
        } else {
            if (!environment.canRetry) {
                InlineLoading(true)
                Spacer(Modifier.height(12.dp))
            }
            Text(environment.description, style = WorkflowTheme.text.body, color = WorkflowTheme.colors.onSurfaceVariant, textAlign = TextAlign.Center)
            if (environment.canRetry) TextButton(onClick = c.services::retryEnvironment) { Text("重试环境") }
        }
    }
}

@Composable
internal fun NeedsEnvironmentPane(c: ConversationController, modifier: Modifier) {
    Column(modifier.padding(16.dp), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.Center) {
        val available by c.hub.available.collectAsState()
        NeedsEnvironmentBody(c, available)
    }
}

@Composable
private fun NeedsEnvironmentBody(c: ConversationController, available: Set<BackendKind>) {
    val entry = c.entry ?: return
    val name = ChatText.backendName(entry.backend)
    val environment by c.services.environment.collectAsState()
    val message = when {
        entry.backend == BackendKind.CLAUDE && environment.usable -> when {
            environment.claudeFailure != null -> "Claude Code 安装失败：${environment.claudeFailure}"
            environment.claudeInstalling -> "正在安装 Claude Code" + (environment.claudeProgress?.let { " ${(it * 100).toInt()}%" } ?: "")
            else -> "正在准备 Claude Code"
        }
        else -> "$name：${environment.description}"
    }
    Text(message, style = WorkflowTheme.text.body, color = WorkflowTheme.colors.onSurfaceVariant, textAlign = TextAlign.Center)
    if (entry.backend == BackendKind.CLAUDE && environment.usable && environment.claudeFailure != null) {
        TextButton(onClick = c.services::installClaude) { Text("重试安装") }
    } else if (environment.canRetry) {
        TextButton(onClick = c.services::retryEnvironment) { Text("重试环境") }
    }
    if (entry.backendThreadId == null) {
        val other = BackendKind.entries.firstOrNull { it != entry.backend && it in available }
        if (other != null) TextButton(onClick = { c.setBackend(other) }) { Text("改用 ${ChatText.backendName(other)}") }
    }
}

@Composable
internal fun LoginPane(c: ConversationController, modifier: Modifier) {
    Column(modifier.padding(16.dp), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.Center) {
        LoginBody(c)
    }
}

/** Compact prompt above the composer while an existing conversation's backend needs sign-in; it stays during a login. */
@Composable
internal fun LoginPrompt(c: ConversationController, modifier: Modifier) {
    var open by remember { mutableStateOf(false) }
    val name = ChatText.backendName(c.entry?.backend ?: return)
    val account = c.backend?.account ?: return
    val message = when (val phase = LoginView.phase(account)) {
        is LoginPhase.Starting, is LoginPhase.Waiting, is LoginPhase.Terminal -> "正在登录 $name"
        is LoginPhase.Idle -> if (account.state == LoginState.UNKNOWN && phase.failure == null) "无法确认 $name 登录状态" else "$name 未登录"
        else -> "$name 未登录"
    }
    InlineError(message, modifier, listOf(TextAction(if (account.login.isPending) "继续" else "登录") { open = true }))
    if (open) {
        AlertDialog(
            onDismissRequest = { open = false },
            confirmButton = { TextButton(onClick = { open = false }) { Text("关闭") } },
            text = { Column(horizontalAlignment = Alignment.CenterHorizontally) { LoginBody(c) } },
        )
    }
}

/**
 * Login phases inline (docs/ux/conversations.md "Signing in"), from the shared sign-in that Settings also
 * uses: starting, waiting for the browser with Cancel, failure with Retry, and an unreadable account with
 * Check again. Device code is the primary method.
 */
@Composable
private fun LoginBody(c: ConversationController) {
    val signIn = c.signIn ?: return
    val account = c.backend?.account ?: return
    val chatConfiguration by c.hub.configuration.collectAsState()
    val context = c.context.appContext
    SignInContent(
        state = signIn,
        account = account,
        methods = chatConfiguration.loginMethods[signIn.kind].orEmpty(),
        ready = c.backend?.process is ProcessState.Ready,
        openUrl = { url -> openSignInLink(context, url) },
        copy = { _, text -> c.copy(text) },
    )
}

@Composable
internal fun RenameDialog(current: String, onDismiss: () -> Unit, onRename: (String) -> Unit) {
    var value by remember { mutableStateOf(current) }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("重命名对话", style = WorkflowTheme.text.titleMd) },
        text = { OutlinedTextField(value, { value = it }, singleLine = true, modifier = Modifier.fillMaxWidth()) },
        confirmButton = { Button(onClick = { onRename(value) }, enabled = value.isNotBlank()) { Text("确定") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("取消") } },
    )
}

/** "全部对话…" from the title switcher (Files' aux): search + the full list. */
@Composable
internal fun AllConversationsDialog(c: ConversationController) {
    val conversations by c.hub.conversations.collectAsState()
    var query by remember { mutableStateOf("") }
    AlertDialog(
        onDismissRequest = { c.showAllConversations = false },
        title = { Text("全部对话", style = WorkflowTheme.text.titleMd) },
        text = {
            Column(Modifier.heightIn(max = 480.dp)) {
                SearchField(query, { query = it }, Modifier.fillMaxWidth().height(36.dp))
                Spacer(Modifier.height(8.dp))
                val list = top.flysoftbeta.workflow.agent.model.ConversationIndexing.search(conversations, query)
                LazyColumn {
                    items(list, key = { it.id }) { e ->
                        ConversationRow(e, selected = e.id == c.conversationId, running = false, pending = false,
                            onClick = { c.showAllConversations = false; if (e.id != c.conversationId) c.switchTo(e.id) }, onLongClick = null)
                    }
                }
            }
        },
        confirmButton = { TextButton(onClick = { c.showAllConversations = false }) { Text("关闭") } },
    )
}

/** One conversation row (rail, "全部对话"): backend glyph + title + age, dots for running / pending. */
@Composable
internal fun ConversationRow(
    entry: ConversationEntry,
    selected: Boolean,
    running: Boolean,
    pending: Boolean,
    onClick: () -> Unit,
    onLongClick: (() -> Unit)?,
    zone: ZoneId = ZoneId.systemDefault(),
) {
    val colors = WorkflowTheme.colors
    val now = System.currentTimeMillis()
    val today = LocalDate.now(zone)
    val date = Instant.ofEpochMilli(entry.updatedAtMs).atZone(zone).toLocalDate()
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = WorkflowTheme.dimens.treeRow)
            .padding(horizontal = 4.dp)
            .clip(WorkflowShapes.sm)
            .background(if (selected) colors.secondaryContainer else androidx.compose.ui.graphics.Color.Transparent)
            .combinedClickable(onClick = onClick, onLongClick = onLongClick)
            .padding(horizontal = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        SymbolIcon(ConversationController.backendIcon(entry.backend), ChatText.backendName(entry.backend), size = 14.dp, tint = colors.onSurfaceVariant)
        Spacer(Modifier.width(8.dp))
        Text(
            entry.title ?: ChatText.untitled(entry.preview), Modifier.weight(1f),
            style = if (selected) WorkflowTheme.text.labelActive else WorkflowTheme.text.label,
            color = if (selected) colors.onSecondaryContainer else colors.onSurface, maxLines = 1, overflow = TextOverflow.Ellipsis,
        )
        Spacer(Modifier.width(6.dp))
        when {
            pending -> top.flysoftbeta.workflow.ui.design.StatusDot(colors.tertiary)
            running -> PulsingDot()
            else -> Text(ChatText.relative(entry.updatedAtMs, now, today, date), style = WorkflowTheme.text.caption, color = colors.onSurfaceVariant, maxLines = 1)
        }
    }
}

@Composable
private fun PulsingDot() {
    val transition = androidx.compose.animation.core.rememberInfiniteTransition(label = "running")
    val alpha by transition.animateFloat(
        0.35f, 1f,
        androidx.compose.animation.core.infiniteRepeatable(androidx.compose.animation.core.tween(700), androidx.compose.animation.core.RepeatMode.Reverse),
        label = "alpha",
    )
    Box(Modifier.size(8.dp).clip(WorkflowShapes.full).background(WorkflowTheme.colors.primary.copy(alpha = alpha)))
}
