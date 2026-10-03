package top.flysoftbeta.workflow.feature.chat.transcript

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Text
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.TextButton
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.withContext
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.json.get
import kotlinx.serialization.json.JsonArray
import top.flysoftbeta.workflow.core.layout.PanelView
import top.flysoftbeta.workflow.feature.chat.decodeThumbnail
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** Flat block rows keep a 100,000-character answer virtualized and give viewport anchors actual granularity. */
data class NativeRow(val key: String, val turn: TurnView, val kind: String, val block: MarkdownBlock? = null, val item: ItemView? = null, val group: List<ItemView> = emptyList())
data class NativeTranscriptDocument(val rows: List<NativeRow> = emptyList(), val hasEarlier: Boolean = false)
class NativeTranscriptProjector {
    private val markdown = MarkdownCache()
    fun project(turns: List<TurnView>, hasEarlier: Boolean): NativeTranscriptDocument = NativeTranscriptDocument(buildList {
        turns.forEach { turn ->
            fun row(kind: String, item: ItemView? = null) { add(NativeRow("${turn.id}/$kind/${item?.id.orEmpty()}", turn, kind, item = item)) }
            if (turn.timeLabel != null) row("time")
            if (turn.user != null) row("user")
            row("header")
            val expanded = turn.expanded ?: (turn.status == "running")
            var cursor = 0
            while (cursor < turn.items.size) {
                val item = turn.items[cursor++]
                val final = item.id == turn.final
                if (!final && !expanded) continue
                if (item.kind == "message") markdown.parse("${turn.id}/${item.id}", item.text.orEmpty()).forEachIndexed { i, block ->
                    add(NativeRow("${turn.id}/message/${item.id}/$i", turn, "markdown", block, item))
                } else if (item.kind == "command" || item.kind == "tool") {
                    val group = arrayListOf(item)
                    while (cursor < turn.items.size) {
                        val next = turn.items[cursor]
                        if (next.kind != item.kind || next.fields["category"] != item.fields["category"] || next.fields["server"] != item.fields["server"]) break
                        group += next; cursor++
                    }
                    if (group.size == 1) row("activity", item)
                    else add(NativeRow("${turn.id}/activities/${item.id}", turn, "activities", group = group))
                } else row("activity", item)
            }
            if (turn.error != null) row("error")
            if (!turn.changes.isNullOrEmpty()) row("changes")
            if (turn.isFinal) row("actions")
        }
    }, hasEarlier)
}

class NativeTranscriptState(initial: PanelView = PanelView()) {
    val list = LazyListState()
    var follow by mutableStateOf(initial.extras["chat.follow"] != "false")
    var restored by mutableStateOf(false)
    var initialAnchor: String? = initial.scrollAnchor
    var initialOffset: Int = initial.scrollOffset
    var jump by mutableIntStateOf(0)
    fun bottom() { follow = true; jump++ }
}

@Composable
fun NativeTranscript(document: NativeTranscriptDocument, state: NativeTranscriptState, callbacks: TranscriptCallbacks, modifier: Modifier = Modifier) {
    val rows = document.rows
    val list = state.list
    var autoScrolling by remember { mutableStateOf(false) }
    val currentRows by rememberUpdatedState(rows)
    val last = rows.lastOrNull()?.key
    LaunchedEffect(state, rows, document.hasEarlier) {
        if (!state.restored && rows.isNotEmpty()) {
            val anchor = state.initialAnchor
            val index = rows.indexOfFirst { it.key == anchor }
            if (!state.follow && anchor != null && index < 0 && document.hasEarlier) {
                callbacks.earlier()
                return@LaunchedEffect
            }
            autoScrolling = true
            try {
                if (state.follow) list.scrollToItem(rows.lastIndex + if (document.hasEarlier) 1 else 0)
                else list.scrollToItem(index.coerceAtLeast(0) + if (document.hasEarlier) 1 else 0, state.initialOffset.coerceAtLeast(0))
                state.restored = true
            } finally { autoScrolling = false }
        }
    }
    // Includes last row's measured size: late font/math/image layout also follows the bottom.
    LaunchedEffect(state, state.restored, state.follow, state.jump, last) {
        if (!state.restored || !state.follow || rows.isEmpty()) return@LaunchedEffect
        snapshotFlow { Triple(list.layoutInfo.totalItemsCount, list.canScrollForward, list.layoutInfo.viewportEndOffset) }.distinctUntilChanged().collectLatest {
            if (state.follow && !list.isScrollInProgress) {
                autoScrolling = true
                try { list.scrollToItem(list.layoutInfo.totalItemsCount.coerceAtLeast(1) - 1, 100_000) }
                finally { autoScrolling = false }
            }
        }
    }
    LaunchedEffect(state, callbacks) {
        snapshotFlow { listOf(state.restored, list.isScrollInProgress, list.firstVisibleItemIndex, list.firstVisibleItemScrollOffset, autoScrolling, list.canScrollForward) }.collectLatest {
            val scrolling = list.isScrollInProgress
            if (!state.restored || autoScrolling) return@collectLatest
            if (scrolling) state.follow = !list.canScrollForward
            callbacks.layout(!list.canScrollForward)
            if (!scrolling) {
                delay(250)
                val first = list.layoutInfo.visibleItemsInfo.firstOrNull { it.index == list.firstVisibleItemIndex }
                callbacks.viewport((first?.key as? String)?.takeUnless { it == "earlier" } ?: currentRows.firstOrNull()?.key, list.firstVisibleItemScrollOffset, state.follow)
            }
        }
    }
    LazyColumn(state = list, modifier = modifier.fillMaxSize().testTag("native-transcript"), contentPadding = PaddingValues(horizontal = 12.dp, vertical = 16.dp), verticalArrangement = Arrangement.spacedBy(8.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        if (document.hasEarlier) item(key = "earlier") { TextButton(onClick = callbacks::earlier) { Text("加载更早的消息") } }
        items(rows, key = { it.key }, contentType = { it.kind }) { row ->
            Box(Modifier.widthIn(max = 720.dp).fillMaxWidth()) { TranscriptRow(row, callbacks) }
        }
    }
}

@Composable
private fun TranscriptRow(row: NativeRow, callbacks: TranscriptCallbacks) {
    val turn = row.turn
    val colors = WorkflowTheme.colors
    when (row.kind) {
        "time" -> Text(turn.timeLabel.orEmpty(), Modifier.fillMaxWidth().padding(top = 12.dp), style = WorkflowTheme.text.caption, color = colors.onSurfaceVariant)
        "user" -> UserBubble(turn, callbacks)
        "header" -> {
            var now by remember { mutableLongStateOf(System.currentTimeMillis()) }
            LaunchedEffect(turn.status) { while (turn.status == "running") { delay(1000); now = System.currentTimeMillis() } }
            val elapsed = turn.durationMs ?: (now - (turn.startedAt ?: now)).coerceAtLeast(0)
            val duration = if (elapsed >= 60_000) "${elapsed / 60_000}m ${(elapsed / 1000) % 60}s" else "${elapsed / 1000}s"
            val label = when (turn.status) { "running" -> "◌ 进行中 $duration"; "failed" -> "未完成 · $duration"; "interrupted" -> "已停止 · $duration"; else -> "用时 $duration" }
            val expanded = turn.expanded ?: (turn.status == "running")
            Text("$label ${if (expanded) "⌄" else "›"}", Modifier.fillMaxWidth().clickable { callbacks.toggle(turn.id, !expanded) }.padding(vertical = 6.dp), style = WorkflowTheme.text.label, color = colors.onSurfaceVariant)
        }
        "markdown" -> row.block?.let { MarkdownBlockContent(it, callbacks) }
        "activity" -> row.item?.let { ActivityRow(it, callbacks) }
        "activities" -> {
            var expanded by rememberSaveable(row.key) { mutableStateOf(false) }
            val label = when (row.group.firstOrNull()?.fields?.get("category").str) {
                "read" -> "读取 ${row.group.size} 个文件"
                "search" -> "执行 ${row.group.size} 次搜索"
                else -> "${row.group.size} 项${if (row.group.firstOrNull()?.kind == "command") "命令" else "工具调用"}"
            }
            Column {
                Text("${if (expanded) "⌄" else "›"} $label", Modifier.fillMaxWidth().clickable { expanded = !expanded }.padding(vertical = 5.dp), style = WorkflowTheme.text.label, color = colors.onSurfaceVariant)
                if (expanded) row.group.forEach { ActivityRow(it, callbacks) }
            }
        }
        "error" -> Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(turn.error.orEmpty(), Modifier.weight(1f), style = WorkflowTheme.text.body, color = colors.error)
            if (turn.canRetry) TextButton(onClick = { callbacks.action(turn.id, "retry") }) { Text("重试") }
        }
        "changes" -> Column(Modifier.fillMaxWidth().background(colors.surfaceContainerLow, WorkflowShapes.md).padding(10.dp)) {
            var all by rememberSaveable(turn.id) { mutableStateOf(false) }
            val files = turn.changes.orEmpty()
            var viewing by remember { mutableStateOf(false) }
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Text("已编辑 ${files.size} 个文件", Modifier.weight(1f), style = WorkflowTheme.text.label, color = colors.onSurface)
                TextButton(onClick = { viewing = true }) { Text("查看更改") }
            }
            if (viewing) AlertDialog(onDismissRequest = { viewing = false }, title = { Text("文件更改") },
                text = { Column(Modifier.heightIn(max = 440.dp).verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    files.forEach { file ->
                        TextButton(onClick = { callbacks.openPath(file.path, null, null) }) { Text(file.path) }
                        MarkdownBlockContent(MarkdownBlock.Code(file.diff ?: "后端未提供差异内容", "diff"), callbacks)
                    }
                } }, confirmButton = { TextButton(onClick = { viewing = false }) { Text("关闭") } })
            files.take(if (all) files.size else 3).forEach { file ->
                Text("${file.path}   +${file.added} −${file.removed}", Modifier.fillMaxWidth().clickable { callbacks.openPath(file.path, null, null) }.padding(vertical = 8.dp), style = WorkflowTheme.text.mono, color = colors.primary)
            }
            if (files.size > 3) TextButton(onClick = { all = !all }) { Text(if (all) "收起" else "再显示 ${files.size - 3} 个文件") }
        }
        "actions" -> Row(Modifier.fillMaxWidth().padding(bottom = 16.dp), horizontalArrangement = Arrangement.End) {
            TextButton(onClick = { callbacks.copyTurn(turn.id, "final") }) { Text("复制") }
            if (turn.canFork) TextButton(onClick = { callbacks.action(turn.id, "fork") }) { Text("分叉") }
            TextButton(onClick = { callbacks.copyTurn(turn.id, "markdown") }) { Text("Markdown") }
        }
    }
}

@Composable
private fun UserBubble(turn: TurnView, callbacks: TranscriptCallbacks) {
    val user = turn.user ?: return
    val colors = WorkflowTheme.colors
    var expanded by rememberSaveable(turn.id) { mutableStateOf(false) }
    var overflow by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxWidth().padding(top = 16.dp), horizontalAlignment = Alignment.End) {
        Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            user.attachments.forEach { attachment -> Attachment(attachment, callbacks) }
        }
        Column(Modifier.fillMaxWidth(.85f).background(colors.primaryContainer, WorkflowShapes.lg).padding(horizontal = 12.dp, vertical = 8.dp)) {
            SelectionContainer {
                Text(user.text, style = WorkflowTheme.text.chat, color = colors.onPrimaryContainer, maxLines = if (expanded) Int.MAX_VALUE else 10,
                    overflow = TextOverflow.Ellipsis, onTextLayout = { if (!expanded) overflow = it.hasVisualOverflow })
            }
            if (overflow || expanded) TextButton(onClick = { expanded = !expanded }) { Text(if (expanded) "收起" else "显示更多 ⌄") }
        }
    }
}

@Composable
private fun Attachment(attachment: AttachmentView, callbacks: TranscriptCallbacks) {
    val image by produceState<ImageBitmap?>(null, attachment.src) {
        if (attachment.image && attachment.src != null) value = withContext(Dispatchers.IO) {
            val source = attachment.src
            val data = if (source.startsWith("data:image/") && source.length < 2_000_000) runCatching { android.util.Base64.decode(source.substringAfter(','), android.util.Base64.DEFAULT) }.getOrNull()
            else callbacks.resource(java.net.URLDecoder.decode(source.removePrefix("/workspace/"), "UTF-8"), 8 * 1024 * 1024)
            data?.let { decodeThumbnail(it, 256) }
        }
    }
    val bitmap = image
    if (bitmap != null) Image(bitmap, attachment.name, Modifier.size(64.dp))
    else Text(attachment.name, Modifier.widthIn(max = 180.dp).background(WorkflowTheme.colors.surfaceContainerHigh, WorkflowShapes.sm).padding(8.dp), style = WorkflowTheme.text.caption, maxLines = 2)
}

@Composable
private fun ActivityRow(item: ItemView, callbacks: TranscriptCallbacks) {
    val colors = WorkflowTheme.colors
    if (item.kind in setOf("notice", "marker", "record")) {
        SelectionContainer { Text(item.text ?: item.fields["text"].str.orEmpty(), Modifier.fillMaxWidth().padding(vertical = 4.dp), style = WorkflowTheme.text.caption, color = if (item.fields["level"].str == "error") colors.error else colors.onSurfaceVariant) }
        return
    }
    var expanded by rememberSaveable(item.id) { mutableStateOf(false) }
    var allOutput by rememberSaveable(item.id) { mutableStateOf(false) }
    val title = item.fields["title"].str ?: when (item.kind) { "reasoning" -> if (item.status == "running") "思考中…" else "已思考"; "plan" -> "计划"; "image" -> item.fields["caption"].str.orEmpty(); else -> "活动" }
    Column(Modifier.fillMaxWidth()) {
        Text("${if (expanded) "⌄" else "›"} $title", Modifier.fillMaxWidth().clickable { expanded = !expanded }.padding(vertical = 5.dp), style = WorkflowTheme.text.label, color = colors.onSurfaceVariant, maxLines = if (expanded) 4 else 1, overflow = TextOverflow.Ellipsis)
        if (expanded) {
            val detail = when (item.kind) {
                "command" -> listOfNotNull(item.fields["command"].str, item.fields["cwd"].str, item.text?.let { if (allOutput) it else it.lineSequence().take(12).joinToString("\n") }, item.fields["exitCode"].str?.let { "退出码 $it" }).joinToString("\n")
                "tool" -> listOfNotNull(item.fields["args"].str, item.fields["result"].str).joinToString("\n\n")
                "unknown" -> item.fields["detail"].str.orEmpty()
                "plan" -> (item.fields["steps"] as? JsonArray).orEmpty().joinToString("\n") { step ->
                    "${if (step["status"].str == "done") "✓" else if (step["status"].str == "running") "◌" else "□"} ${step["text"].str.orEmpty()}"
                }
                "files" -> (item.fields["files"] as? JsonArray).orEmpty().joinToString("\n\n") { file ->
                    listOfNotNull(file["path"].str, file["diff"].str).joinToString("\n")
                }
                else -> item.text ?: item.fields["text"].str.orEmpty()
            }
            if (item.kind == "image") Attachment(AttachmentView(title, true, item.fields["src"].str), callbacks)
            else if (item.kind == "reasoning") {
                val blocks by produceState<List<MarkdownBlock>>(emptyList(), detail) {
                    value = withContext(Dispatchers.Default) { NativeMarkdownParser.parse(detail) }
                }
                blocks.forEach { MarkdownBlockContent(it, callbacks) }
            } else detail.chunked(4000).forEachIndexed { index, part -> MarkdownBlockContent(MarkdownBlock.Code(part, if (item.kind == "unknown") "json" else "", detail, index > 0), callbacks) }
            if (item.kind == "command" && item.text.orEmpty().lineSequence().take(13).count() > 12) TextButton(onClick = { allOutput = !allOutput }) { Text(if (allOutput) "收起输出" else "显示全部输出") }
            item.fields["url"].str?.let { url -> TextButton(onClick = { openLink(url, callbacks) }) { Text("打开结果") } }
        }
    }
}
