package top.flysoftbeta.workflow.feature.chat.transcript

import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonArray
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import top.flysoftbeta.workflow.agent.model.AgentMessageItem
import top.flysoftbeta.workflow.agent.model.CommandItem
import top.flysoftbeta.workflow.agent.model.FileChangeItem
import top.flysoftbeta.workflow.agent.model.FileChangeKind
import top.flysoftbeta.workflow.agent.model.FileDelta
import top.flysoftbeta.workflow.agent.model.ImageItem
import top.flysoftbeta.workflow.agent.model.Item
import top.flysoftbeta.workflow.agent.model.ItemStatus
import top.flysoftbeta.workflow.agent.model.MarkerItem
import top.flysoftbeta.workflow.agent.model.MarkerKind
import top.flysoftbeta.workflow.agent.model.MessagePhase
import top.flysoftbeta.workflow.agent.model.NoticeItem
import top.flysoftbeta.workflow.agent.model.NoticeLevel
import top.flysoftbeta.workflow.agent.model.PendingRequest
import top.flysoftbeta.workflow.agent.model.PlanItem
import top.flysoftbeta.workflow.agent.model.PlanStepStatus
import top.flysoftbeta.workflow.agent.model.ReasoningItem
import top.flysoftbeta.workflow.agent.model.RequestStatus
import top.flysoftbeta.workflow.agent.model.SubAgentItem
import top.flysoftbeta.workflow.agent.model.ThreadState
import top.flysoftbeta.workflow.agent.model.ToolCallItem
import top.flysoftbeta.workflow.agent.model.Turn
import top.flysoftbeta.workflow.agent.model.TurnStatus
import top.flysoftbeta.workflow.agent.model.UnknownItem
import top.flysoftbeta.workflow.agent.model.UserMessageItem
import top.flysoftbeta.workflow.agent.model.UserPart
import top.flysoftbeta.workflow.agent.model.WebSearchItem
import top.flysoftbeta.workflow.feature.chat.ChatText
import top.flysoftbeta.workflow.platform.agent.AgentPaths

/** One native transcript item. [text] is the streamable field. */
data class ItemView(
    val id: String,
    val kind: String,
    val status: String,
    val text: String? = null,
    val fields: Map<String, JsonElement> = emptyMap(),
) {
}

data class FileView(val path: String, val dir: String, val name: String, val added: Int, val removed: Int, val diff: String?) {
    fun toJson() = buildJsonObject {
        put("path", path); put("dir", dir); put("name", name); put("added", added); put("removed", removed)
        put("diff", diff?.let(::JsonPrimitive) ?: JsonNull)
    }
}

data class UserView(val text: String, val attachments: List<AttachmentView>)

data class AttachmentView(val name: String, val image: Boolean, val src: String?)

/** One native transcript turn. [id] is stable across the optimistic → bound transition. */
data class TurnView(
    val id: String,
    /** Backend turn id (fork / retry), used by fork / retry actions. */
    val backendTurnId: String,
    val status: String,
    val startedAt: Long?,
    val durationMs: Long?,
    val timeLabel: String?,
    val user: UserView?,
    val items: List<ItemView>,
    val final: String?,
    val error: String?,
    val changes: List<FileView>?,
    val canFork: Boolean,
    val canRetry: Boolean,
    /** Last expansion the user chose. */
    val expanded: Boolean? = null,
) {
    val isFinal: Boolean get() = status != "running"

}

/**
 * Pure projection of a thread (plus its settled requests) into the native transcript.
 * Queued turns are shown natively above the composer, except the next one while nothing runs.
 */
class TranscriptProjector(
    private val paths: AgentPaths?,
    private val zone: ZoneId = ZoneId.systemDefault(),
    private val canFork: Boolean = true,
) {
    private var cache = java.util.IdentityHashMap<Turn, TurnView>()

    fun project(thread: ThreadState?, requests: List<PendingRequest>, expanded: Map<String, Boolean>, today: LocalDate = LocalDate.now(zone)): List<TurnView> {
        if (thread == null) return emptyList()
        val running = thread.turns.any { it.status == TurnStatus.RUNNING }
        val firstQueued = thread.turns.firstOrNull { it.status == TurnStatus.QUEUED }
        val visible = thread.turns.filter { t ->
            t.status != TurnStatus.CANCELLED && (t.status != TurnStatus.QUEUED || (!running && t === firstQueued))
        }
        val settled = requests.filter { !it.status.isOpen }
        val nextCache = java.util.IdentityHashMap<Turn, TurnView>()
        val out = ArrayList<TurnView>(visible.size)
        var previousStart: Long? = null
        visible.forEachIndexed { index, turn ->
            val turnRequests = settled.filter { it.turnId == turn.id }
            val last = index == visible.lastIndex
            val base = cache[turn]?.takeIf { turnRequests.isEmpty() && !last } ?: projectTurn(turn, turnRequests, last)
            nextCache[turn] = base
            val label = timeLabel(previousStart, turn.startedAtMs, today)
            previousStart = turn.startedAtMs ?: previousStart
            var view = base.copy(timeLabel = label, expanded = expanded[base.id])
            if (last && thread.notices.isNotEmpty()) {
                view = view.copy(items = view.items + thread.notices.takeLast(3).mapIndexed { i, n ->
                    ItemView("thread-notice-$i", "notice", "done", fields = mapOf("level" to JsonPrimitive(level(n.level)), "text" to JsonPrimitive(ChatText.notice(n))))
                })
            }
            out += view
        }
        cache = nextCache
        return out
    }

    private fun projectTurn(turn: Turn, requests: List<PendingRequest>, last: Boolean): TurnView {
        val userItems = turn.items.filterIsInstance<UserMessageItem>()
        val first = userItems.firstOrNull()
        val items = ArrayList<ItemView>()
        for (item in turn.items) {
            if (item === first) continue
            project(item)?.let { items += it }
            requests.filter { it.itemId == item.id }.forEach { items += record(it) }
        }
        requests.filter { r -> r.itemId == null || turn.items.none { it.id == r.itemId } }
            .sortedBy { it.receivedAtMs ?: 0 }
            .forEach { items += record(it) }
        val status = when (turn.status) {
            TurnStatus.QUEUED, TurnStatus.RUNNING -> "running"
            TurnStatus.COMPLETED -> "completed"
            TurnStatus.INTERRUPTED, TurnStatus.CANCELLED -> "interrupted"
            TurnStatus.FAILED -> "failed"
        }
        val final = if (turn.status.isFinal) turn.finalMessage?.id else null
        val changes = if (turn.status.isFinal) aggregate(turn.items.filterIsInstance<FileChangeItem>().filter { it.status == ItemStatus.COMPLETED }) else null
        return TurnView(
            id = turn.clientMessageId ?: turn.id,
            backendTurnId = turn.id,
            status = status,
            startedAt = turn.startedAtMs,
            durationMs = turn.durationMs ?: turn.completedAtMs?.let { end -> turn.startedAtMs?.let { end - it } },
            timeLabel = null,
            user = first?.let(::user),
            items = items,
            final = final,
            error = if (turn.status == TurnStatus.FAILED) ChatText.turnError(turn.error) else null,
            changes = changes?.takeIf { it.isNotEmpty() },
            canFork = canFork && turn.status.isFinal && turn.bound,
            canRetry = last && turn.status == TurnStatus.FAILED && first != null,
        )
    }

    private fun user(item: UserMessageItem): UserView = UserView(
        text = item.text,
        attachments = item.parts.mapNotNull { part ->
            when (part) {
                is UserPart.Image -> AttachmentView(name(part.path), true, workspaceUrl(part.path))
                is UserPart.File -> AttachmentView(name(part.path), false, null)
                is UserPart.InlineData -> AttachmentView(if (part.kind == "image") "图片" else "文档", part.kind == "image",
                    if (part.kind == "image" && part.base64.length < 2_000_000) "data:${part.mediaType ?: "image/png"};base64,${part.base64}" else null)
                is UserPart.ImageUrl -> AttachmentView("图片", true, null)
                is UserPart.Reference -> AttachmentView(part.name, false, null)
                is UserPart.Text, is UserPart.Unknown -> null
            }
        },
    )

    private fun project(item: Item): ItemView? {
        val status = status(item.status)
        return when (item) {
            is UserMessageItem -> ItemView(item.id, "record", "done", fields = mapOf("text" to JsonPrimitive("引导：" + item.text.lineSequence().first().take(120))))
            is AgentMessageItem -> ItemView(item.id, "message", status, item.text.toString(),
                mapOf("phase" to JsonPrimitive(if (item.phase == MessagePhase.FINAL) "final" else "commentary")))
            is ReasoningItem -> {
                val text = when {
                    item.redacted -> ""
                    item.summary.isNotEmpty() -> item.summary.joinToString("\n\n")
                    else -> item.content.joinToString("\n\n")
                }
                ItemView(item.id, "reasoning", status, text)
            }
            is PlanItem -> ItemView(item.id, "plan", status, fields = mapOf(
                "steps" to buildJsonArray {
                    item.steps.forEach { s -> add(buildJsonObject { put("text", s.text); put("status", when (s.status) { PlanStepStatus.PENDING -> "pending"; PlanStepStatus.IN_PROGRESS -> "running"; PlanStepStatus.COMPLETED -> "done" }) }) }
                },
                "text" to JsonPrimitive(item.text.toString()),
            ))
            is CommandItem -> command(item, status)
            is FileChangeItem -> {
                val files = item.changes.map(::file)
                ItemView(item.id, "files", status, fields = mapOf(
                    "title" to JsonPrimitive(ChatText.filesTitle(files.map { it.name }, item.status)),
                    "files" to JsonArray(files.map { it.toJson() }),
                ))
            }
            is ToolCallItem -> ItemView(item.id, "tool", status, fields = mapOf(
                "title" to JsonPrimitive(ChatText.toolTitle(item.server, item.tool, item.status)),
                "server" to (item.server?.let(::JsonPrimitive) ?: JsonNull),
                "args" to JsonPrimitive(ReadableJson.format(item.arguments ?: item.argumentsText.toString().takeIf { it.isNotBlank() }?.let(::JsonPrimitive), 60)),
                "result" to JsonPrimitive(item.error ?: item.resultText ?: ReadableJson.format(item.result, 80)),
            ))
            is SubAgentItem -> ItemView(item.id, "agent", status, fields = mapOf(
                "title" to JsonPrimitive(item.description ?: item.agentType ?: "子代理"),
                "text" to JsonPrimitive(item.result ?: item.progress.lastOrNull() ?: ""),
            ))
            is WebSearchItem -> ItemView(item.id, "search", status, fields = mapOf(
                "title" to JsonPrimitive(ChatText.searchTitle(item.query, item.status)),
                "query" to JsonPrimitive(item.query),
                "url" to (item.url?.let(::JsonPrimitive) ?: JsonNull),
            ))
            is ImageItem -> {
                val src = item.path?.let { p -> paths?.toWorkspace(p)?.let(::encodeWorkspaceUrl) }
                ItemView(item.id, "image", status, fields = mapOf(
                    "src" to (src?.let(::JsonPrimitive) ?: JsonNull),
                    "caption" to JsonPrimitive(item.prompt ?: item.path?.let(::name) ?: ""),
                ))
            }
            is MarkerItem -> ItemView(item.id, "marker", "done", fields = mapOf("text" to JsonPrimitive(ChatText.marker(item.kind, item.text))))
            is NoticeItem -> ItemView(item.id, "notice", "done", fields = mapOf("level" to JsonPrimitive(level(item.notice.level)), "text" to JsonPrimitive(ChatText.notice(item.notice))))
            is UnknownItem -> ItemView(item.id, "unknown", status, fields = mapOf(
                "title" to JsonPrimitive("未识别的活动"),
                "detail" to JsonPrimitive(item.raw?.let(ReadableJson::pretty) ?: item.type),
            ))
        }
    }

    private fun command(item: CommandItem, status: String): ItemView {
        val kinds = item.actions.map { it.kind }.toSet()
        val category = when {
            kinds.isEmpty() -> "run"
            kinds.all { it == "read" } -> "read"
            kinds.all { it == "listFiles" } -> "list"
            kinds.all { it == "search" } -> "search"
            else -> "run"
        }
        val shown = item.actions.singleOrNull()?.command?.takeIf { it.isNotBlank() } ?: ChatText.unwrapShell(item.command)
        val title = when (category) {
            "read" -> ChatText.readTitle(item.actions.mapNotNull { it.path?.let(::name) }.ifEmpty { listOf(shown) }, item.status)
            "list" -> ChatText.listTitle(item.actions.firstOrNull()?.path?.let { displayPath(it) } ?: shown, item.status)
            "search" -> ChatText.searchTitle(item.actions.firstOrNull()?.query ?: shown, item.status)
            else -> ChatText.runTitle(shown, item.status)
        }
        val failed = item.status == ItemStatus.FAILED || (item.exitCode != null && item.exitCode != 0)
        val output = item.output.toString().let { if (it.length > MAX_OUTPUT) it.takeLast(MAX_OUTPUT) else it }
        return ItemView(item.id, "command", if (failed && status == "done") "failed" else status, output, mapOf(
            "category" to JsonPrimitive(category),
            "title" to JsonPrimitive(title),
            "command" to JsonPrimitive(shown),
            "cwd" to (item.cwd?.let { cwd -> displayPath(cwd).takeIf { it.isNotEmpty() } }?.let(::JsonPrimitive) ?: JsonNull),
            "exitCode" to (item.exitCode?.let(::JsonPrimitive) ?: JsonNull),
            "truncated" to JsonPrimitive(item.outputTruncated || item.output.length > MAX_OUTPUT),
        ))
    }

    private fun record(request: PendingRequest) = ItemView(
        "req:" + request.key.text, "record", "done",
        fields = mapOf("text" to JsonPrimitive(ChatText.requestRecord(request))),
    )

    private fun file(delta: FileDelta): FileView {
        val rel = paths?.toWorkspace(delta.path) ?: delta.path
        var added = 0
        var removed = 0
        delta.diff?.lineSequence()?.forEach { line ->
            when {
                line.startsWith("+++") || line.startsWith("---") -> Unit
                line.startsWith("+") -> added++
                line.startsWith("-") -> removed++
            }
        }
        if (delta.diff == null && delta.kind == FileChangeKind.DELETE) removed = 0
        return FileView(rel, rel.substringBeforeLast('/', "").let { if (it.isEmpty()) "" else "$it/" }, name(rel), added, removed, delta.diff?.take(64_000))
    }

    private fun aggregate(items: List<FileChangeItem>): List<FileView> {
        val byPath = LinkedHashMap<String, FileView>()
        items.flatMap { it.changes }.map(::file).forEach { f ->
            val prev = byPath[f.path]
            byPath[f.path] = if (prev == null) f else prev.copy(added = prev.added + f.added, removed = prev.removed + f.removed, diff = listOfNotNull(prev.diff, f.diff).joinToString("\n").ifEmpty { null })
        }
        return byPath.values.toList()
    }

    private fun displayPath(path: String): String = paths?.toWorkspace(path) ?: path

    private fun workspaceUrl(agentPath: String): String? = paths?.toWorkspace(agentPath)?.let(::encodeWorkspaceUrl)

    private fun timeLabel(previous: Long?, current: Long?, today: LocalDate): String? {
        if (previous == null || current == null || current - previous < 30 * 60_000) return null
        return ChatText.timeSeparator(Instant.ofEpochMilli(current).atZone(zone).toLocalDateTime(), today)
    }

    private fun level(level: NoticeLevel) = when (level) {
        NoticeLevel.INFO -> "info"
        NoticeLevel.WARNING -> "warning"
        NoticeLevel.ERROR -> "error"
    }

    private fun status(status: ItemStatus) = when (status) {
        ItemStatus.IN_PROGRESS -> "running"
        ItemStatus.COMPLETED -> "done"
        ItemStatus.INCOMPLETE -> "incomplete"
        ItemStatus.FAILED -> "failed"
        ItemStatus.DECLINED -> "declined"
    }

    companion object {
        const val MAX_OUTPUT = 32 * 1024

        fun name(path: String) = path.trimEnd('/').substringAfterLast('/')

        /** Resource identifier decoded by the injected Engine resource reader; never fetched as a URL. */
        fun encodeWorkspaceUrl(relative: String): String =
            "/workspace/" + relative.split('/').joinToString("/") { java.net.URLEncoder.encode(it, "UTF-8").replace("+", "%20") }
    }
}

/** Readable, non-JSON rendering of structured values for tool rows and request cards. */
object ReadableJson {
    fun format(value: JsonElement?, maxLines: Int = 40): String {
        if (value == null || value is JsonNull) return ""
        val lines = ArrayList<String>()
        fun emit(element: JsonElement, indent: String, prefix: String) {
            when (element) {
                is JsonObject -> {
                    if (prefix.isNotEmpty()) lines += indent + prefix.trimEnd()
                    val inner = if (prefix.isNotEmpty()) "$indent  " else indent
                    element.forEach { (k, v) ->
                        if (v is JsonObject || v is JsonArray) emit(v, inner, "$k:") else emit(v, inner, "$k: ")
                    }
                }
                is JsonArray -> {
                    if (prefix.isNotEmpty()) lines += indent + prefix.trimEnd()
                    val inner = if (prefix.isNotEmpty()) "$indent  " else indent
                    element.forEach { v -> emit(v, inner, "- ") }
                }
                is JsonPrimitive -> {
                    val text = if (element.isString) element.content else element.toString()
                    val parts = text.lines()
                    lines += indent + prefix + parts.first()
                    parts.drop(1).forEach { lines += "$indent  $it" }
                }
            }
        }
        emit(value, "", "")
        return if (lines.size > maxLines) (lines.take(maxLines) + "…").joinToString("\n") else lines.joinToString("\n")
    }

    /** Verbatim, indented JSON: only for "未识别的活动 / 请求" details. */
    fun pretty(value: JsonElement): String = PrettyJson.encodeToString(JsonElement.serializer(), value)

    private val PrettyJson = kotlinx.serialization.json.Json { prettyPrint = true }
}
