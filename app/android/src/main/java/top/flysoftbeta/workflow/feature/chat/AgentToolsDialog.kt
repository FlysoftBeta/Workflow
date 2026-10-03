package top.flysoftbeta.workflow.feature.chat

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.RateLimitWindow
import top.flysoftbeta.workflow.ui.design.InlineError
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter

/** Secondary controls live in the stack menu, leaving the main conversation compact. */
@Composable
internal fun AgentToolsDialog(c: ConversationController) {
    val entry = c.entry ?: return
    val console = c.toolsPage == "console"
    var method by remember { mutableStateOf(if (entry.backend == BackendKind.CODEX) "account/rateLimits/read" else "get_context_usage") }
    var parameters by remember { mutableStateOf("{}") }
    var result by remember { mutableStateOf<String?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var running by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    fun execute() {
        if (running) return
        running = true
        error = null
        scope.launch {
            try {
                if (console) {
                    val parsed = Json.parseToJsonElement(parameters)
                    require(parsed is JsonObject) { "参数必须是 JSON 对象" }
                    val response = c.hub.rawRequest(c.conversationId, method, parsed)
                    result = Json { prettyPrint = true }.encodeToString(JsonElement.serializer(), response).take(65_536)
                } else c.hub.refreshUsage(entry.backend)
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { error = failure.message ?: "请求未完成" }
            finally { running = false }
        }
    }
    AlertDialog(
        onDismissRequest = { c.toolsPage = null },
        title = { Text(if (console) "高级控制台" else "用量与后端状态", style = WorkflowTheme.text.titleMd) },
        text = {
            Column(Modifier.fillMaxWidth().heightIn(max = 480.dp).verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                if (console) {
                    OutlinedTextField(method, { method = it }, label = { Text("方法") }, singleLine = true, enabled = !running, modifier = Modifier.fillMaxWidth())
                    OutlinedTextField(parameters, { parameters = it }, label = { Text("JSON 参数") }, enabled = !running,
                        minLines = 3, maxLines = 8, textStyle = WorkflowTheme.text.mono, modifier = Modifier.fillMaxWidth())
                    result?.let { SelectionContainer { Text(it, style = WorkflowTheme.text.mono) } }
                } else {
                    val status = c.backend
                    Text(ChatText.backendName(entry.backend), style = WorkflowTheme.text.titleSm)
                    listOfNotNull(status?.account?.email, status?.account?.plan).forEach { Text(it, style = WorkflowTheme.text.body) }
                    val usage = c.thread?.usage
                    if (usage != null) {
                        Text("Token：${usage.totalTokens} · 输入 ${usage.inputTokens} · 输出 ${usage.outputTokens}", style = WorkflowTheme.text.body)
                        usage.contextWindow?.let { Text("上下文窗口：$it", style = WorkflowTheme.text.body) }
                        usage.costUsd?.let { Text("费用：USD $it", style = WorkflowTheme.text.body) }
                    }
                    val limits = status?.rateLimits?.limits.orEmpty()
                    if (limits.isEmpty() && usage == null) Text("后端尚未提供用量", style = WorkflowTheme.text.body)
                    limits.forEach { limit ->
                        Text(limit.name ?: limit.id, style = WorkflowTheme.text.label)
                        listOfNotNull(limit.primary, limit.secondary).forEach { window -> Text(windowLabel(window), style = WorkflowTheme.text.body) }
                        limit.status?.let { Text(it, style = WorkflowTheme.text.caption) }
                    }
                    if (status?.rateLimits?.ordinaryUsageAllowed == false) Text("常规模型额度已用尽", color = WorkflowTheme.colors.error)
                    status?.mcpServers?.values?.forEach { server ->
                        Text("${server.name} · ${server.status}", style = WorkflowTheme.text.body)
                        server.error?.let { Text(it, color = WorkflowTheme.colors.error, style = WorkflowTheme.text.caption) }
                    }
                    status?.notices?.takeLast(5)?.forEach { Text(it.message, style = WorkflowTheme.text.caption) }
                }
                error?.let { InlineError(it) }
                if (running) LinearProgressIndicator(Modifier.fillMaxWidth())
            }
        },
        confirmButton = { TextButton(onClick = { execute() }, enabled = c.available && !running && (!console || method.isNotBlank())) { Text(if (console) "执行" else "刷新") } },
        dismissButton = { TextButton(onClick = { c.toolsPage = null }) { Text("关闭") } },
    )
}

private fun windowLabel(window: RateLimitWindow): String = buildString {
    window.windowMinutes?.let { append("${it} 分钟 · ") }
    append(window.usedPercent?.let { "已用 ${it.toInt()}%" } ?: "用量未知")
    window.resetsAtEpochSec?.let {
        val reset = Instant.ofEpochSecond(it).atZone(ZoneId.systemDefault()).format(DateTimeFormatter.ofPattern("MM-dd HH:mm"))
        append(" · $reset 重置")
    }
}

@Composable
internal fun DeleteConversationDialog(c: ConversationController) {
    var running by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    AlertDialog(
        onDismissRequest = { if (!running) c.confirmingDelete = false },
        title = { Text("删除「${c.title}」？", style = WorkflowTheme.text.titleMd) },
        text = { Column { Text("对话将从所有会话中移除，未发送的文字和附件草稿也会丢弃。此操作无法撤销。", style = WorkflowTheme.text.body); error?.let { InlineError(it) } } },
        confirmButton = {
            TextButton(enabled = !running, onClick = {
                running = true
                // The panel may disappear during deletion; the process scope completes persistence.
                c.services.processScope.launch {
                    try { c.hub.deleteConversation(c.conversationId); c.confirmingDelete = false }
                    catch (cancelled: CancellationException) { throw cancelled }
                    catch (failure: Exception) { error = failure.message ?: "删除未完成" }
                    finally { running = false }
                }
            }) { Text("删除", color = WorkflowTheme.colors.error) }
        },
        dismissButton = { TextButton(enabled = !running, onClick = { c.confirmingDelete = false }) { Text("取消") } },
    )
}
