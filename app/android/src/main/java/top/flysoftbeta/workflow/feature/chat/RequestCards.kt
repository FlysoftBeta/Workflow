package top.flysoftbeta.workflow.feature.chat

import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MenuAnchorPosition
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import top.flysoftbeta.workflow.agent.json.arr
import top.flysoftbeta.workflow.agent.json.get
import top.flysoftbeta.workflow.agent.json.obj
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.json.strings
import top.flysoftbeta.workflow.agent.model.Decision
import top.flysoftbeta.workflow.agent.model.DecisionKind
import top.flysoftbeta.workflow.agent.model.FileChangeItem
import top.flysoftbeta.workflow.agent.model.PendingRequest
import top.flysoftbeta.workflow.agent.model.Question
import top.flysoftbeta.workflow.agent.model.RequestKind
import top.flysoftbeta.workflow.agent.model.RequestResponse
import top.flysoftbeta.workflow.feature.chat.transcript.ReadableJson
import top.flysoftbeta.workflow.ui.design.CompositeMenu
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.MenuGroup
import top.flysoftbeta.workflow.ui.design.SymbolIcon
import top.flysoftbeta.workflow.ui.design.WfIconButton
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/**
 * Pending approvals and questions pinned above the composer (docs/ux/README.md §4.7): stacked in arrival
 * order, at most half the column, scrolling inside. Buttons are exactly the decisions the server
 * offered; nothing is preselected and a hardware Enter never approves.
 */
@Composable
internal fun RequestCards(c: ConversationController, modifier: Modifier) {
    Column(modifier.verticalScroll(rememberScrollState()).padding(bottom = 6.dp)) {
        c.openRequests.forEach { request ->
            RequestCard(c, request, Modifier.fillMaxWidth().padding(top = 6.dp))
        }
    }
}

@Composable
private fun RequestCard(c: ConversationController, request: PendingRequest, modifier: Modifier) {
    val colors = WorkflowTheme.colors
    val kind = request.kind
    Column(
        modifier
            .background(colors.surfaceContainerHigh, WorkflowShapes.md)
            .border(1.dp, WorkflowTheme.extendedColors.warning, WorkflowShapes.md)
            .padding(12.dp)
            .semantics { liveRegion = LiveRegionMode.Polite },
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            SymbolIcon(cardIcon(kind), null, size = 18.dp, tint = WorkflowTheme.extendedColors.warning)
            Spacer(Modifier.width(8.dp))
            Text(ChatText.requestTitle(kind), style = WorkflowTheme.text.titleSm, color = colors.onSurface)
        }
        Spacer(Modifier.height(8.dp))
        when (kind) {
            is RequestKind.CommandApproval -> {
                kind.networkHost?.let { Body("访问 $it") }
                kind.command?.let { MonoBlock(ChatText.unwrapShell(it), collapseAfter = 6) }
                kind.cwd?.let { cwd -> displayPath(c, cwd)?.takeIf { it.isNotEmpty() }?.let { Caption("位于 $it") } }
                kind.reason?.let { Body(it) }
                Decisions(c, request)
            }
            is RequestKind.FileChangeApproval -> {
                kind.reason?.let { Body(it) }
                val files = kind.changes.ifEmpty {
                    c.thread?.turns?.flatMap { it.items }?.filterIsInstance<FileChangeItem>()?.firstOrNull { it.id == request.itemId }?.changes.orEmpty()
                }
                files.forEach { delta -> FileLine(c, delta.path, delta.diff) }
                kind.grantRoot?.let { root -> Caption("并允许写入 ${displayPath(c, root) ?: root}") }
                Decisions(c, request)
            }
            is RequestKind.PermissionsApproval -> {
                kind.reason?.let { Body(it) }
                MonoBlock(ReadableJson.format(kind.permissions), collapseAfter = 8)
                Decisions(c, request)
            }
            is RequestKind.ToolApproval -> {
                listOfNotNull(kind.title, kind.description).distinct().forEach { Body(it) }
                val command = kind.input["command"].str
                val file = kind.input["file_path"].str ?: kind.input["path"].str
                when {
                    command != null -> MonoBlock(command, collapseAfter = 6)
                    file != null -> Body(displayPath(c, file) ?: file)
                    else -> kind.input?.let { MonoBlock(ReadableJson.format(it), collapseAfter = 8) }
                }
                kind.reason?.let { Caption(it) }
                Decisions(c, request)
            }
            is RequestKind.PlanApproval -> {
                Box(Modifier.heightIn(max = 240.dp).verticalScroll(rememberScrollState())) { Body(kind.plan) }
                Decisions(c, request)
            }
            is RequestKind.UserInput -> QuestionForm(c, request, kind.questions)
            is RequestKind.Elicitation -> ElicitationForm(c, request, kind)
            is RequestKind.UserDialog -> UnknownRequest(c, request, answerable = false)
            is RequestKind.Unknown -> UnknownRequest(c, request, answerable = true)
        }
    }
}

private fun cardIcon(kind: RequestKind): Int = when (kind) {
    is RequestKind.CommandApproval -> Sym.Terminal
    is RequestKind.FileChangeApproval -> Sym.EditDocument
    is RequestKind.PermissionsApproval -> Sym.Shield
    is RequestKind.ToolApproval -> Sym.DataObject
    is RequestKind.PlanApproval -> Sym.Checklist
    is RequestKind.UserInput, is RequestKind.Elicitation -> Sym.Info
    else -> Sym.Warning
}

private fun displayPath(c: ConversationController, path: String): String? = c.entry?.let { c.hub.paths(it.backend) }?.toWorkspace(path)

@Composable
private fun Body(text: String) {
    Text(text, Modifier.padding(bottom = 6.dp), style = WorkflowTheme.text.body, color = WorkflowTheme.colors.onSurface)
}

@Composable
private fun Caption(text: String) {
    Text(text, Modifier.padding(bottom = 6.dp), style = WorkflowTheme.text.caption, color = WorkflowTheme.colors.onSurfaceVariant)
}

@Composable
private fun MonoBlock(text: String, collapseAfter: Int) {
    var all by remember { mutableStateOf(false) }
    val lines = text.lines()
    val shown = if (all || lines.size <= collapseAfter) text else lines.take(collapseAfter).joinToString("\n")
    Column(Modifier.fillMaxWidth().padding(bottom = 6.dp).background(WorkflowTheme.colors.surfaceContainerLow, WorkflowShapes.sm).padding(8.dp)) {
        Text(shown, Modifier.horizontalScroll(rememberScrollState()), style = WorkflowTheme.text.mono, color = WorkflowTheme.colors.onSurface, softWrap = false)
        if (!all && lines.size > collapseAfter) {
            Text("显示全部 ${lines.size} 行", Modifier.clickable { all = true }.padding(top = 4.dp), style = WorkflowTheme.text.label, color = WorkflowTheme.colors.primary)
        }
    }
}

@Composable
private fun FileLine(c: ConversationController, path: String, diff: String?) {
    var open by remember { mutableStateOf(false) }
    val rel = displayPath(c, path) ?: path
    var added = 0
    var removed = 0
    diff?.lineSequence()?.forEach { l ->
        if (l.startsWith("+") && !l.startsWith("+++")) added++ else if (l.startsWith("-") && !l.startsWith("---")) removed++
    }
    Row(Modifier.fillMaxWidth().height(36.dp).clickable(enabled = diff != null) { open = !open }, verticalAlignment = Alignment.CenterVertically) {
        Text(rel.substringBeforeLast('/', "").let { if (it.isEmpty()) "" else "$it/" }, style = WorkflowTheme.text.caption, color = WorkflowTheme.colors.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.StartEllipsis)
        Text(rel.substringAfterLast('/'), Modifier.weight(1f), style = WorkflowTheme.text.body, color = WorkflowTheme.colors.onSurface, maxLines = 1)
        if (diff != null) {
            Text("+$added", style = WorkflowTheme.text.mono, color = WorkflowTheme.extendedColors.success)
            Spacer(Modifier.width(4.dp))
            Text("−$removed", style = WorkflowTheme.text.mono, color = WorkflowTheme.colors.error)
            Spacer(Modifier.width(4.dp))
            Text(if (open) "收起" else "查看", style = WorkflowTheme.text.label, color = WorkflowTheme.colors.primary)
        }
    }
    if (open && diff != null) MonoBlock(diff, collapseAfter = 40)
}

/** The offered decisions: approve (Filled) · session (Tonal) · deny (Text) · ⌄ (abort / others). */
@Composable
private fun Decisions(c: ConversationController, request: PendingRequest) {
    val decisions = request.decisions
    if (decisions.isEmpty()) return
    val primary = decisions.filter { it.kind == DecisionKind.ALLOW_ONCE }
    val tonal = decisions.filter { it.kind == DecisionKind.ALLOW_SESSION || it.kind == DecisionKind.ALLOW_PERSISTENT }
    val deny = decisions.filter { it.kind == DecisionKind.DENY }
    val more = decisions.filter { it.kind == DecisionKind.ABORT || it.kind == DecisionKind.OTHER }
    var menu by remember { mutableStateOf(false) }
    // A hardware Enter must never approve (ui.md §4.7).
    val noEnter = Modifier.onPreviewKeyEvent { it.key == Key.Enter || it.key == Key.NumPadEnter }
    val decide = { d: Decision -> c.respond(request, RequestResponse.Decide(d.id)) }
    FlowRow(Modifier.fillMaxWidth().padding(top = 4.dp), horizontalArrangement = Arrangement.End, verticalArrangement = Arrangement.spacedBy(4.dp)) {
        deny.forEach { d -> TextButton(onClick = { decide(d) }, modifier = noEnter) { Text(ChatText.decisionLabel(d, request.kind)) } }
        tonal.forEach { d ->
            Spacer(Modifier.width(6.dp))
            FilledTonalButton(onClick = { decide(d) }, modifier = noEnter, contentPadding = ButtonDefaults.ContentPadding) { Text(ChatText.decisionLabel(d, request.kind)) }
        }
        primary.forEach { d ->
            Spacer(Modifier.width(6.dp))
            Button(onClick = { decide(d) }, modifier = noEnter) { Text(ChatText.decisionLabel(d, request.kind)) }
        }
        if (more.isNotEmpty()) Box {
            WfIconButton(Sym.KeyboardArrowDown, "更多选项", { menu = true })
            CompositeMenu(
                expanded = menu, onDismissRequest = { menu = false }, anchorPosition = MenuAnchorPosition.Above,
                groups = listOf(MenuGroup("more", more.map { d ->
                    MenuEntry.Action(d.id, ChatText.decisionLabel(d, request.kind), if (d.kind == DecisionKind.ABORT) Sym.Stop else null,
                        destructive = d.kind == DecisionKind.ABORT) { decide(d) }
                })),
            )
        }
    }
    tonal.firstOrNull { it.kind == DecisionKind.ALLOW_PERSISTENT && it.detail != null }?.let { d ->
        Caption("始终允许将记住：${d.detail}")
    }
}

/** Questions: option chips (single or multi), "其他" free text; 提交 only when every question is answered. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun QuestionForm(c: ConversationController, request: PendingRequest, questions: List<Question>) {
    val picked = remember(request.key) { mutableStateMapOf<String, List<String>>() }
    val other = remember(request.key) { mutableStateMapOf<String, String>() }
    val otherOn = remember(request.key) { mutableStateMapOf<String, Boolean>() }
    questions.forEach { q ->
        q.header?.let { Caption(it) }
        Body(q.question)
        if (q.options.isNotEmpty()) FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            q.options.forEach { option ->
                val selected = option.label in picked[q.id].orEmpty()
                FilterChip(
                    selected = selected,
                    onClick = {
                        val current = picked[q.id].orEmpty()
                        picked[q.id] = when {
                            q.multiSelect -> if (selected) current - option.label else current + option.label
                            selected -> emptyList()
                            else -> listOf(option.label)
                        }
                        if (!q.multiSelect && !selected) otherOn[q.id] = false
                    },
                    label = { Text(option.label) },
                )
            }
            if (q.allowFreeText) FilterChip(
                selected = otherOn[q.id] == true,
                onClick = {
                    val on = otherOn[q.id] != true
                    otherOn[q.id] = on
                    if (on && !q.multiSelect) picked[q.id] = emptyList()
                },
                label = { Text("其他") },
            )
        }
        if (q.allowFreeText && (q.options.isEmpty() || otherOn[q.id] == true)) {
            OutlinedTextField(
                value = other[q.id].orEmpty(), onValueChange = { other[q.id] = it },
                modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
                visualTransformation = if (q.secret) PasswordVisualTransformation() else androidx.compose.ui.text.input.VisualTransformation.None,
                keyboardOptions = if (q.secret) KeyboardOptions(keyboardType = KeyboardType.Password) else KeyboardOptions.Default,
            )
        }
        Spacer(Modifier.height(6.dp))
    }
    fun answer(q: Question): List<String> = picked[q.id].orEmpty() +
        listOfNotNull(other[q.id]?.trim()?.takeIf { it.isNotEmpty() && (q.options.isEmpty() || otherOn[q.id] == true) })
    val complete = questions.all { answer(it).isNotEmpty() }
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
        Button(
            onClick = { c.respond(request, RequestResponse.Answer(questions.associate { it.id to answer(it) })) },
            enabled = complete,
            modifier = Modifier.onPreviewKeyEvent { it.key == Key.Enter },
        ) { Text("提交") }
    }
}

/** MCP elicitation: a form for string / number / boolean / enum fields (others as a text box), or a URL to visit. */
@Composable
private fun ElicitationForm(c: ConversationController, request: PendingRequest, kind: RequestKind.Elicitation) {
    kind.server?.let { Caption(it) }
    Body(kind.message)
    val respond = { action: String, content: JsonObject? -> c.respond(request, RequestResponse.Elicit(action, content)) }
    val url = kind.url
    if (kind.mode == "url" && url != null) {
        Text(url, Modifier.clickable {
            runCatching { c.context.appContext.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) }
        }.padding(vertical = 4.dp), style = WorkflowTheme.text.body, color = WorkflowTheme.colors.primary, maxLines = 2, overflow = TextOverflow.Ellipsis)
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
            TextButton(onClick = { respond("cancel", null) }) { Text("取消") }
            TextButton(onClick = { respond("decline", null) }) { Text("拒绝") }
            Button(onClick = { respond("accept", null) }, modifier = Modifier.onPreviewKeyEvent { it.key == Key.Enter }) { Text("已完成") }
        }
        return
    }
    val properties = kind.schema["properties"].obj ?: JsonObject(emptyMap())
    val required = kind.schema["required"].strings().toSet()
    val values = remember(request.key) { mutableStateMapOf<String, String>() }
    properties.forEach { (name, schema) ->
        val title = schema["title"].str ?: name
        val type = schema["type"].str
        val options = schema["enum"].arr?.mapNotNull { it.str }
        schema["description"].str?.let { Caption(it) }
        when {
            options != null -> {
                Caption(title + if (name in required) " *" else "")
                Row(Modifier.horizontalScroll(rememberScrollState())) {
                    options.forEach { option ->
                        FilterChip(selected = values[name] == option, onClick = { values[name] = option }, label = { Text(option) }, modifier = Modifier.padding(end = 6.dp))
                    }
                }
            }
            type == "boolean" -> Row(verticalAlignment = Alignment.CenterVertically) {
                Checkbox(checked = values[name] == "true", onCheckedChange = { values[name] = it.toString() })
                Text(title, style = WorkflowTheme.text.body, color = WorkflowTheme.colors.onSurface)
            }
            else -> OutlinedTextField(
                value = values[name].orEmpty(), onValueChange = { values[name] = it },
                label = { Text(title + if (name in required) " *" else "") },
                keyboardOptions = if (type == "number" || type == "integer") KeyboardOptions(keyboardType = KeyboardType.Number) else KeyboardOptions.Default,
                singleLine = type != "object" && type != "array",
                modifier = Modifier.fillMaxWidth().padding(vertical = 2.dp),
            )
        }
    }
    val content = runCatching { elicitationContent(properties, values) }.getOrNull()
    val complete = content != null && required.all { !values[it].isNullOrBlank() }
    Row(Modifier.fillMaxWidth().padding(top = 6.dp), horizontalArrangement = Arrangement.End) {
        TextButton(onClick = { respond("cancel", null) }) { Text("取消") }
        TextButton(onClick = { respond("decline", null) }) { Text("拒绝") }
        Button(onClick = { respond("accept", content) }, enabled = complete, modifier = Modifier.onPreviewKeyEvent { it.key == Key.Enter }) { Text("提交") }
    }
}

/** Typed form values; unsupported types are parsed as JSON typed by the user. Throws on invalid input. */
internal fun elicitationContent(properties: JsonObject, values: Map<String, String>): JsonObject = buildJsonObject {
    properties.forEach { (name, schema) ->
        val raw = values[name]?.takeIf { it.isNotBlank() } ?: return@forEach
        val value: JsonElement = when (schema["type"].str) {
            "boolean" -> JsonPrimitive(raw == "true")
            "integer" -> JsonPrimitive(raw.trim().toLong())
            "number" -> JsonPrimitive(raw.trim().toDouble())
            "string", null -> JsonPrimitive(raw)
            else -> top.flysoftbeta.workflow.agent.json.parseJson(raw)
        }
        put(name, value)
    }
}

@Composable
private fun UnknownRequest(c: ConversationController, request: PendingRequest, answerable: Boolean) {
    var details by remember { mutableStateOf(false) }
    Body("代理发来一个这里无法识别的请求。")
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End, verticalAlignment = Alignment.CenterVertically) {
        TextButton(onClick = { details = true }) { Text("详情") }
        if (answerable) {
            request.decisions.forEach { d ->
                TextButton(onClick = { c.respond(request, RequestResponse.Decide(d.id)) }) { Text(ChatText.decisionLabel(d, request.kind)) }
            }
            if (request.decisions.isEmpty()) TextButton(onClick = { c.respond(request, RequestResponse.Reject()) }) { Text("拒绝") }
        }
    }
    if (details) DetailsDialog("未识别的请求", request.raw?.let(ReadableJson::pretty) ?: request.method) { details = false }
}
