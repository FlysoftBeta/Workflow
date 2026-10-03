package top.flysoftbeta.workflow.feature.chat

import android.content.Intent
import android.net.Uri
import androidx.compose.animation.core.animateFloat
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.MenuAnchorPosition
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
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.ConversationEntry
import top.flysoftbeta.workflow.agent.model.LoginFlow
import top.flysoftbeta.workflow.agent.model.LoginMethod
import top.flysoftbeta.workflow.agent.model.LoginState
import top.flysoftbeta.workflow.agent.model.ProcessState
import top.flysoftbeta.workflow.ui.design.CompositeMenu
import top.flysoftbeta.workflow.ui.design.InlineError
import top.flysoftbeta.workflow.ui.design.InlineLoading
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.MenuGroup
import top.flysoftbeta.workflow.ui.design.SearchField
import top.flysoftbeta.workflow.ui.design.SymbolIcon
import top.flysoftbeta.workflow.ui.design.TextAction
import top.flysoftbeta.workflow.ui.design.WfIconButton
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/**
 * New conversation (docs/ui.md §6): the backend choice in the centre (remembered), nothing else
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
            account?.state == LoginState.LOGGED_OUT || account?.state == LoginState.LOGGING_IN && account.login != null -> LoginBody(c)
            c.backend?.process is ProcessState.Failed -> InlineError(
                "${ChatText.backendName(entry.backend)} 未能启动", Modifier.widthIn(max = 480.dp),
                listOf(TextAction("重试") { c.retryBackend() }),
            )
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

/** Compact prompt above the composer when an existing conversation's backend is logged out. */
@Composable
internal fun LoginPrompt(c: ConversationController, modifier: Modifier) {
    var open by remember { mutableStateOf(false) }
    val name = ChatText.backendName(c.entry?.backend ?: return)
    InlineError("$name 未登录", modifier, listOf(TextAction("登录") { open = true }))
    if (open) {
        AlertDialog(
            onDismissRequest = { open = false },
            confirmButton = { TextButton(onClick = { open = false }) { Text("关闭") } },
            text = { Column(horizontalAlignment = Alignment.CenterHorizontally) { LoginBody(c) } },
        )
    }
}

/** Login states inline (docs/ui.md §6 "居中 [登录 Codex]"): device code first, browser and API key as alternatives. */
@Composable
private fun LoginBody(c: ConversationController) {
    val entry = c.entry ?: return
    val kind = entry.backend
    val name = ChatText.backendName(kind)
    val colors = WorkflowTheme.colors
    val flow = c.backend?.account?.login
    var apiKey by remember { mutableStateOf<String?>(null) }
    val context = c.context.appContext
    fun open(url: String) = runCatching {
        context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
    }
    Column(Modifier.widthIn(max = 420.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        when (flow) {
            is LoginFlow.DeviceCode -> {
                Text("在浏览器中打开下面的链接，并输入代码", style = WorkflowTheme.text.body, color = colors.onSurfaceVariant, textAlign = TextAlign.Center)
                Spacer(Modifier.height(8.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        flow.verificationUrl, Modifier.weight(1f, fill = false).clip(WorkflowShapes.sm).clickable { open(flow.verificationUrl) }.padding(4.dp),
                        style = WorkflowTheme.text.body, color = colors.primary, maxLines = 1, overflow = TextOverflow.Ellipsis,
                    )
                    WfIconButton(Sym.OpenInNew, "打开链接", { open(flow.verificationUrl) })
                }
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(flow.userCode, style = WorkflowTheme.text.mono.copy(fontSize = 26.sp, lineHeight = 32.sp, letterSpacing = 2.sp), color = colors.onSurface)
                    WfIconButton(Sym.ContentCopy, "复制代码", { c.copy(flow.userCode) })
                }
                Spacer(Modifier.height(8.dp))
                Waiting { flow.loginId?.let(c::cancelLogin) }
            }
            is LoginFlow.Browser -> {
                Button(onClick = { open(flow.authUrl) }) { Text("在浏览器中继续") }
                Spacer(Modifier.height(8.dp))
                Waiting { flow.loginId?.let(c::cancelLogin) }
            }
            is LoginFlow.Terminal -> {
                Text("在终端中运行下面的命令完成登录", style = WorkflowTheme.text.body, color = colors.onSurfaceVariant)
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(flow.argv.joinToString(" "), style = WorkflowTheme.text.mono, color = colors.onSurface)
                    WfIconButton(Sym.ContentCopy, "复制命令", { c.copy(flow.argv.joinToString(" ")) })
                }
            }
            else -> {
                val failed = (flow as? LoginFlow.Completed)?.takeIf { !it.success && it.error != "cancelled" }?.error
                    ?: (flow as? LoginFlow.Progress)?.error
                if (failed != null) {
                    InlineError("登录没有完成", Modifier.fillMaxWidth())
                    Spacer(Modifier.height(8.dp))
                }
                val methods = c.hub.loginMethods(kind)
                val primary = methods.first()
                var more by remember { mutableStateOf(false) }
                if (apiKey == null) {
                    Button(onClick = { c.login(primary) }, enabled = c.backend?.process is ProcessState.Ready) { Text("登录 $name") }
                    Box {
                        TextButton(onClick = { more = true }) { Text("其他方式") }
                        CompositeMenu(
                            expanded = more, onDismissRequest = { more = false },
                            anchorPosition = MenuAnchorPosition.Below,
                            groups = listOf(MenuGroup("methods", methods.drop(1).map { method ->
                                MenuEntry.Action(method.name, ChatText.loginLabel(method), if (method.name.endsWith("API_KEY") || method == LoginMethod.CLAUDE_SETUP_TOKEN) Sym.Key else Sym.OpenInNew) {
                                    if (method == LoginMethod.CODEX_API_KEY || method == LoginMethod.CLAUDE_API_KEY || method == LoginMethod.CLAUDE_SETUP_TOKEN) apiKey = "" else c.login(method)
                                }
                            })),
                        )
                    }
                    InlineLoading(c.backend?.process !is ProcessState.Ready)
                } else {
                    OutlinedTextField(
                        value = apiKey ?: "", onValueChange = { apiKey = it }, singleLine = true,
                        label = { Text("API Key") }, visualTransformation = PasswordVisualTransformation(),
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                        modifier = Modifier.fillMaxWidth(),
                    )
                    Row(Modifier.fillMaxWidth().padding(top = 8.dp), horizontalArrangement = Arrangement.End) {
                        TextButton(onClick = { apiKey = null }) { Text("取消") }
                        Spacer(Modifier.width(8.dp))
                        Button(onClick = {
                            val method = methods.first { it == LoginMethod.CODEX_API_KEY || it == LoginMethod.CLAUDE_API_KEY }
                            c.login(method, apiKey)
                            apiKey = null
                        }, enabled = !apiKey.isNullOrBlank()) { Text("登录") }
                    }
                }
            }
        }
    }
}

@Composable
private fun Waiting(onCancel: () -> Unit) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        InlineLoading(true)
        Spacer(Modifier.width(8.dp))
        Text("等待确认…", style = WorkflowTheme.text.label, color = WorkflowTheme.colors.onSurfaceVariant)
        Spacer(Modifier.width(8.dp))
        TextButton(onClick = onCancel) { Text("取消") }
    }
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
