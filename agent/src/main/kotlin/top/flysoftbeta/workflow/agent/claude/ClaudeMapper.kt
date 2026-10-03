package top.flysoftbeta.workflow.agent.claude

import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import top.flysoftbeta.workflow.agent.json.arr
import top.flysoftbeta.workflow.agent.json.bool
import top.flysoftbeta.workflow.agent.json.double
import top.flysoftbeta.workflow.agent.json.encode
import top.flysoftbeta.workflow.agent.json.get
import top.flysoftbeta.workflow.agent.json.isNullish
import top.flysoftbeta.workflow.agent.json.long
import top.flysoftbeta.workflow.agent.json.obj
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.json.strings
import top.flysoftbeta.workflow.agent.model.AccountState
import top.flysoftbeta.workflow.agent.model.AgentEvent
import top.flysoftbeta.workflow.agent.model.AgentMessageItem
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.CommandItem
import top.flysoftbeta.workflow.agent.model.EffortOption
import top.flysoftbeta.workflow.agent.model.FileChangeItem
import top.flysoftbeta.workflow.agent.model.FileChangeKind
import top.flysoftbeta.workflow.agent.model.FileDelta
import top.flysoftbeta.workflow.agent.model.Item
import top.flysoftbeta.workflow.agent.model.ItemDelta
import top.flysoftbeta.workflow.agent.model.ItemStatus
import top.flysoftbeta.workflow.agent.model.LoginFlow
import top.flysoftbeta.workflow.agent.model.LoginState
import top.flysoftbeta.workflow.agent.model.MarkerItem
import top.flysoftbeta.workflow.agent.model.MarkerKind
import top.flysoftbeta.workflow.agent.model.MessagePhase
import top.flysoftbeta.workflow.agent.model.ModelCatalog
import top.flysoftbeta.workflow.agent.model.ModelOption
import top.flysoftbeta.workflow.agent.model.Notice
import top.flysoftbeta.workflow.agent.model.NoticeLevel
import top.flysoftbeta.workflow.agent.model.PlanItem
import top.flysoftbeta.workflow.agent.model.PlanStep
import top.flysoftbeta.workflow.agent.model.PlanStepStatus
import top.flysoftbeta.workflow.agent.model.RateLimit
import top.flysoftbeta.workflow.agent.model.RateLimitState
import top.flysoftbeta.workflow.agent.model.RateLimitWindow
import top.flysoftbeta.workflow.agent.model.ReasoningItem
import top.flysoftbeta.workflow.agent.model.RunState
import top.flysoftbeta.workflow.agent.model.StreamText
import top.flysoftbeta.workflow.agent.model.SubAgentItem
import top.flysoftbeta.workflow.agent.model.ThreadSettings
import top.flysoftbeta.workflow.agent.model.TokenUsage
import top.flysoftbeta.workflow.agent.model.ToolCallItem
import top.flysoftbeta.workflow.agent.model.ToolKind
import top.flysoftbeta.workflow.agent.model.TurnError
import top.flysoftbeta.workflow.agent.model.TurnStatus
import top.flysoftbeta.workflow.agent.model.UnknownItem
import top.flysoftbeta.workflow.agent.model.UserMessageItem
import top.flysoftbeta.workflow.agent.model.UserPart
import top.flysoftbeta.workflow.agent.model.WebSearchItem

/**
 * Stateful mapping of one Claude Code session's stdout (stream-json) to [AgentEvent]s.
 *
 * Claude is content-block centric: `stream_event` carries raw Messages-API deltas, then one
 * `assistant` frame per completed block (same `message.id`, no block index), `user` frames carry
 * our echoed prompt and `tool_result`s, `result` ends the turn. Item ids: `<message id>:<index>`
 * for text/thinking, the `tool_use` id for tools, the prompt uuid for user messages. Turn id = the
 * client uuid of the prompt (also `command_lifecycle.command_uuid` / `result.user_message_uuid`).
 */
class ClaudeMapper(initialSessionId: String?) {
    private val b = BackendKind.CLAUDE

    var sessionId: String? = initialSessionId
        private set

    private var currentTurn: String? = null
    private val startedTurns = LinkedHashSet<String>()
    private val finishedTurns = HashSet<String>()

    private class Block(val index: Int, val type: String, val itemId: String, var completed: Boolean = false)
    private val blocks = HashMap<String, MutableList<Block>>()
    private val streaming = HashMap<String?, String>()

    private class Tool(val id: String, val name: String, var input: JsonElement?, val turnId: String, val parentId: String?)
    private val tools = HashMap<String, Tool>()
    private val denied = HashSet<String>()
    /** Last assistant/user uuid per turn: fork anchor for `--resume-session-at`. */
    private val lastUuid = HashMap<String, String>()
    private var markerSeq = 0

    fun lastMessageUuid(turnId: String): String? = lastUuid[turnId]
    fun isStarted(turnId: String) = turnId in startedTurns
    fun isFinished(turnId: String) = turnId in finishedTurns
    val runningTurn: String? get() = currentTurn

    /** Transcript mode: a prompt entry opens a turn (there is no `command_lifecycle` in transcripts). */
    fun beginTurn(turnId: String): List<AgentEvent> {
        val t = sessionId ?: return emptyList()
        currentTurn = turnId
        startedTurns += turnId
        return listOf(AgentEvent.TurnStarted(b, t, turnId, clientMessageId = turnId))
    }

    /** Transcript mode / process exit: closes [turnId] unless a `result` already did. */
    fun endTurn(turnId: String, status: TurnStatus, error: TurnError? = null): List<AgentEvent> {
        val t = sessionId ?: return emptyList()
        if (turnId in finishedTurns) return emptyList()
        finishedTurns += turnId
        if (currentTurn == turnId) currentTurn = null
        return listOf(
            if (turnId in startedTurns) AgentEvent.TurnCompleted(b, t, turnId, status, error)
            else AgentEvent.TurnCancelled(b, t, turnId),
        )
    }

    /** An app-registered hook ran (`hook_callback`): visible as a marker in the running turn. */
    fun hookMarker(callbackId: String, input: JsonObject): List<AgentEvent> {
        val t = sessionId ?: return emptyList()
        val text = listOfNotNull(input["hook_event_name"].str, input["tool_name"].str, callbackId).joinToString(" · ")
        val turn = currentTurn ?: return listOf(AgentEvent.ThreadNotice(b, t, Notice(NoticeLevel.INFO, "Hook: $text", "hook_callback", raw = input)))
        return listOf(AgentEvent.ItemCompleted(b, t, turn, MarkerItem("hook-${markerSeq++}", MarkerKind.HOOK, text, raw = input)))
    }

    /** Records that the user denied [toolUseId] so its error result renders as DECLINED. */
    fun markDenied(toolUseId: String) { denied += toolUseId }

    /** The turn a frame belongs to (explicit `user_message_uuid`, else the running command). */
    private fun turnOf(frame: JsonObject): String? = frame["user_message_uuid"].str ?: currentTurn

    private fun thread(frame: JsonObject? = null): String? {
        frame?.get("session_id").str?.takeIf { it.isNotEmpty() }?.let { if (sessionId == null) sessionId = it }
        return sessionId
    }

    fun map(frame: JsonObject): List<AgentEvent> {
        val t = thread(frame) ?: return listOf(AgentEvent.Unknown(b, frame["type"].str ?: "?", null, frame))
        val type = frame["type"].str
        return when (type) {
            "system" -> system(t, frame)
            "command_lifecycle" -> lifecycle(t, frame)
            "stream_event" -> streamEvent(t, frame)
            "assistant" -> assistant(t, frame)
            "user" -> user(t, frame, live = true)
            "result" -> result(t, frame)
            "rate_limit_event" -> listOf(AgentEvent.RateLimitsChanged(b, RateLimitState(listOfNotNull(rateLimit(frame["rate_limit_info"]))), merge = true))
            "auth_status" -> listOf(AgentEvent.LoginChanged(b, if (frame["isAuthenticating"].bool == true || frame["error"].str != null)
                LoginFlow.Progress(null, frame["output"].strings(), frame["error"].str) else LoginFlow.Completed(null, true, null)))
            "tool_progress" -> {
                val id = frame["tool_use_id"].str
                val tool = id?.let { tools[it] }
                if (tool == null) listOf(AgentEvent.Unknown(b, "tool_progress", t, frame))
                else listOf(AgentEvent.ItemUpdated(b, t, tool.turnId, tool.id, ItemDelta.ToolProgress("${frame["elapsed_time_seconds"].double?.toLong() ?: 0}s")))
            }
            else -> listOf(AgentEvent.Unknown(b, type ?: "?", t, frame))
        }
    }

    // ------------------------------------------------------------------ system

    private fun system(t: String, f: JsonObject): List<AgentEvent> {
        val subtype = f["subtype"].str
        val turn = turnOf(f)
        fun notice(level: NoticeLevel, message: String, code: String? = subtype, retry: Boolean = false): AgentEvent =
            if (turn != null) AgentEvent.TurnNotice(b, t, turn, Notice(level, message, code, retry, raw = f))
            else AgentEvent.ThreadNotice(b, t, Notice(level, message, code, retry, raw = f))
        fun marker(kind: MarkerKind, text: String?): AgentEvent =
            if (turn != null) AgentEvent.ItemCompleted(b, t, turn, MarkerItem("marker-${f["uuid"].str ?: markerSeq++}", kind, text, raw = f))
            else AgentEvent.ThreadNotice(b, t, Notice(NoticeLevel.INFO, text ?: kind.name, subtype, raw = f))
        return when (subtype) {
            "init" -> {
                sessionId = f["session_id"].str ?: sessionId
                listOf(AgentEvent.ThreadUpserted(
                    backend = b, threadId = sessionId ?: t, cwd = f["cwd"].str,
                    settings = ThreadSettings(model = f["model"].str, approvalPolicy = f["permissionMode"].str, raw = f),
                    raw = f,
                ))
            }
            "status" -> buildList {
                f["permissionMode"].str?.let { add(AgentEvent.ThreadSettingsChanged(b, t, ThreadSettings(approvalPolicy = it))) }
                if (f["status"].str == "compacting") add(notice(NoticeLevel.INFO, "Compacting context"))
            }
            "session_state_changed" -> listOf(AgentEvent.ThreadStatusChanged(b, t, when (f["state"].str) {
                "running" -> RunState.RUNNING
                "requires_action" -> RunState.WAITING_APPROVAL
                else -> RunState.IDLE
            }))
            "thinking_tokens" -> turn?.let { listOf(AgentEvent.TurnProgress(b, t, it, f["estimated_tokens"].long)) } ?: emptyList()
            "api_retry" -> listOf(notice(NoticeLevel.WARNING,
                "Retrying request (${f["attempt"].long ?: "?"}/${f["max_retries"].long ?: "?"})", code = f["error"].str ?: f["error_status"].long?.toString(), retry = true))
            "compact_boundary" -> listOf(marker(MarkerKind.COMPACTION, f["compact_metadata"].let { m ->
                listOfNotNull(m["trigger"].str, m["pre_tokens"].long?.let { "$it" }, m["post_tokens"].long?.let { "→ $it" }).joinToString(" ")
            }))
            "hook_started", "hook_progress", "hook_response" -> listOf(marker(MarkerKind.HOOK,
                listOfNotNull(f["hook_event"].str ?: f["hook_event_name"].str, f["hook_name"].str, f["outcome"].str).joinToString(" · ").ifEmpty { subtype }))
            "local_command_output" -> listOf(marker(MarkerKind.LOCAL_COMMAND, f["content"].str ?: f["output"].str))
            "permission_denied" -> buildList {
                f["tool_use_id"].str?.let { denied += it; add(AgentEvent.ItemDeclined(b, t, tools[it]?.turnId, it)) }
                add(notice(NoticeLevel.WARNING, f["message"].str ?: "Permission denied"))
            }
            "notification", "informational" -> listOf(notice(NoticeLevel.INFO, f["message"].str ?: f["text"].str ?: f["title"].str ?: subtype))
            "model_refusal_fallback", "model_refusal_no_fallback" -> listOf(notice(NoticeLevel.WARNING, f["message"].str ?: "Model refused; ${if (subtype == "model_refusal_fallback") "switched model" else "no fallback"}"))
            "task_started", "task_progress", "task_updated", "task_notification" -> {
                val tool = f["tool_use_id"].str?.let { tools[it] }
                val text = listOfNotNull(f["description"].str, f["summary"].str, f["status"].str).joinToString(" · ").ifEmpty { subtype }
                if (tool != null) listOf(AgentEvent.ItemUpdated(b, t, tool.turnId, tool.id, ItemDelta.ToolProgress(text)))
                else listOf(AgentEvent.ThreadNotice(b, t, Notice(NoticeLevel.INFO, "Background task: $text", subtype, raw = f)))
            }
            else -> listOf(AgentEvent.Unknown(b, "system/$subtype", t, f))
        }
    }

    // ------------------------------------------------------------------ turns

    private fun lifecycle(t: String, f: JsonObject): List<AgentEvent> {
        val id = f["command_uuid"].str ?: return listOf(AgentEvent.Unknown(b, "command_lifecycle", t, f))
        return when (f["state"].str) {
            "started" -> {
                currentTurn = id
                startedTurns += id
                listOf(AgentEvent.TurnStarted(b, t, id, clientMessageId = id))
            }
            "completed" -> if (id in finishedTurns) emptyList() else {
                finishedTurns += id
                listOf(AgentEvent.TurnCompleted(b, t, id, TurnStatus.COMPLETED))
            }
            "cancelled" -> if (id in finishedTurns) emptyList() else {
                finishedTurns += id
                if (currentTurn == id) currentTurn = null
                listOf(if (id in startedTurns) AgentEvent.TurnCompleted(b, t, id, TurnStatus.INTERRUPTED) else AgentEvent.TurnCancelled(b, t, id))
            }
            else -> emptyList() // queued: the local TurnSubmitted already shows it
        }
    }

    private fun result(t: String, f: JsonObject): List<AgentEvent> {
        val turn = turnOf(f) ?: return listOf(AgentEvent.Unknown(b, "result", t, f))
        finishedTurns += turn
        if (currentTurn == turn) currentTurn = null
        val terminal = f["terminal_reason"].str
        val isError = f["is_error"].bool == true
        val status = when {
            terminal?.startsWith("aborted") == true -> TurnStatus.INTERRUPTED
            isError || f["subtype"].str != "success" -> TurnStatus.FAILED
            else -> TurnStatus.COMPLETED
        }
        val error = if (status == TurnStatus.FAILED) TurnError(
            message = f["result"].str ?: f["errors"].strings().joinToString("\n").ifEmpty { f["subtype"].str ?: "error" },
            code = terminal ?: f["subtype"].str,
            detail = f["api_error_status"]?.takeUnless { it.isNullish }?.encode(),
            raw = f,
        ) else null
        val usage = f["usage"]
        val events = mutableListOf<AgentEvent>()
        f["permission_denials"].arr?.forEach { d -> d["tool_use_id"].str?.let { denied += it; events += AgentEvent.ItemDeclined(b, t, turn, it) } }
        events += AgentEvent.TurnCompleted(
            backend = b, threadId = t, turnId = turn, status = status, error = error,
            durationMs = f["duration_ms"].long,
            usage = TokenUsage(
                inputTokens = usage["input_tokens"].long ?: 0,
                cachedInputTokens = usage["cache_read_input_tokens"].long ?: 0,
                outputTokens = usage["output_tokens"].long ?: 0,
                reasoningTokens = usage["output_tokens_details"]["thinking_tokens"].long ?: 0,
                totalTokens = (usage["input_tokens"].long ?: 0) + (usage["output_tokens"].long ?: 0),
                costUsd = f["total_cost_usd"].double,
                raw = usage,
            ),
        )
        return events
    }

    // ------------------------------------------------------------------ assistant content

    private fun streamEvent(t: String, f: JsonObject): List<AgentEvent> {
        val event = f["event"].obj ?: return emptyList()
        val parent = f["parent_tool_use_id"].str
        val turn = turnOf(f) ?: return listOf(AgentEvent.Unknown(b, "stream_event", t, f))
        return when (event["type"].str) {
            "message_start" -> {
                event["message"]["id"].str?.let { id -> streaming[parent] = id; blocks.getOrPut(id) { mutableListOf() } }
                emptyList()
            }
            "content_block_start" -> {
                val messageId = streaming[parent] ?: return emptyList()
                val index = event["index"].long?.toInt() ?: return emptyList()
                val block = event["content_block"].obj ?: return emptyList()
                startBlock(t, turn, messageId, index, block, parent)
            }
            "content_block_delta" -> {
                val messageId = streaming[parent] ?: return emptyList()
                val index = event["index"].long?.toInt() ?: return emptyList()
                val open = blocks[messageId]?.firstOrNull { it.index == index } ?: return emptyList()
                val delta = event["delta"]
                val d = when (delta["type"].str) {
                    "text_delta" -> ItemDelta.AgentText(delta["text"].str ?: "")
                    "thinking_delta" -> ItemDelta.ReasoningText(0, delta["thinking"].str ?: "")
                    "input_json_delta" -> ItemDelta.ToolArguments(delta["partial_json"].str ?: "")
                    else -> null // signature_delta, citations_delta: not rendered
                }
                if (d == null) emptyList() else listOf(AgentEvent.ItemUpdated(b, t, turn, open.itemId, d))
            }
            else -> emptyList() // content_block_stop, message_delta, message_stop: completion comes from `assistant`/`result`
        }
    }

    private fun startBlock(t: String, turn: String, messageId: String, index: Int, block: JsonObject, parent: String?): List<AgentEvent> {
        val type = block["type"].str ?: "unknown"
        val list = blocks.getOrPut(messageId) { mutableListOf() }
        list.firstOrNull { it.index == index }?.let { return emptyList() }
        val item: Item = when (type) {
            "text" -> AgentMessageItem("$messageId:$index", StreamText.EMPTY, MessagePhase.UNKNOWN, parentId = parent)
            "thinking" -> ReasoningItem("$messageId:$index", parentId = parent)
            "redacted_thinking" -> ReasoningItem("$messageId:$index", redacted = true, parentId = parent)
            "tool_use", "server_tool_use", "mcp_tool_use" -> {
                val id = block["id"].str ?: "$messageId:$index"
                val name = block["name"].str ?: ""
                tools[id] = Tool(id, name, block["input"], turn, parent)
                toolItem(id, name, null, parent, ItemStatus.IN_PROGRESS, block)
            }
            else -> UnknownItem("$messageId:$index", type, ItemStatus.IN_PROGRESS, parent, block)
        }
        list += Block(index, type, item.id)
        return listOf(AgentEvent.ItemStarted(b, t, turn, item))
    }

    private fun assistant(t: String, f: JsonObject): List<AgentEvent> {
        val message = f["message"].obj ?: return listOf(AgentEvent.Unknown(b, "assistant", t, f))
        val turn = turnOf(f) ?: return listOf(AgentEvent.Unknown(b, "assistant", t, f))
        val parent = f["parent_tool_use_id"].str
        f["uuid"].str?.let { lastUuid[turn] = it }
        val text = message["content"].arr?.mapNotNull { it["text"].str }?.joinToString("\n")
        val error = f["error"].str
        if (error != null && f["is_api_error_message"].bool == true) {
            val events = mutableListOf<AgentEvent>(AgentEvent.TurnNotice(b, t, turn, Notice(NoticeLevel.ERROR, text ?: error, code = error, raw = f)))
            if (error == "authentication_failed") events += AgentEvent.AccountChanged(b, AccountState(LoginState.LOGGED_OUT, raw = f))
            return events
        }
        val messageId = message["id"].str ?: (f["uuid"].str ?: "msg")
        val aborted = f["aborted"].bool == true || f["isAbortedMidStream"].bool == true
        val events = mutableListOf<AgentEvent>()
        val list = blocks.getOrPut(messageId) { mutableListOf() }
        for (content in message["content"].arr ?: JsonArray(emptyList())) {
            val block = content.obj ?: continue
            val type = block["type"].str ?: "unknown"
            val category = category(type)
            var open = list.firstOrNull { !it.completed && category(it.type) == category }
            if (open == null) {
                val index = f["apiBlockIndex"].long?.toInt()?.takeIf { i -> list.none { it.index == i } } ?: ((list.maxOfOrNull { it.index } ?: -1) + 1)
                events += startBlock(t, turn, messageId, index, block, parent)
                open = list.first { it.index == index }
            }
            open.completed = true
            val status = if (aborted) ItemStatus.INCOMPLETE else ItemStatus.COMPLETED
            events += when (type) {
                "text" -> AgentEvent.ItemCompleted(b, t, turn, AgentMessageItem(open.itemId, StreamText.of(block["text"].str), MessagePhase.UNKNOWN, status, parent, block))
                "thinking" -> AgentEvent.ItemCompleted(b, t, turn, ReasoningItem(open.itemId, content = listOf(StreamText.of(block["thinking"].str)), status = status, parentId = parent, raw = block))
                "redacted_thinking" -> AgentEvent.ItemCompleted(b, t, turn, ReasoningItem(open.itemId, redacted = true, status = status, parentId = parent, raw = block))
                "tool_use", "server_tool_use", "mcp_tool_use" -> {
                    val id = block["id"].str ?: open.itemId
                    val name = block["name"].str ?: ""
                    val tool = tools.getOrPut(id) { Tool(id, name, null, turn, parent) }
                    tool.input = block["input"]
                    // Arguments are final; the tool itself finishes with its tool_result.
                    AgentEvent.ItemStarted(b, t, turn, toolItem(id, name, tool.input, parent, ItemStatus.IN_PROGRESS, block))
                }
                "web_search_tool_result", "web_fetch_tool_result" -> {
                    val id = block["tool_use_id"].str
                    val tool = id?.let { tools[it] }
                    if (tool != null) AgentEvent.ItemCompleted(b, t, tool.turnId, finishedTool(tool, block["content"], null, false, null))
                    else AgentEvent.ItemCompleted(b, t, turn, UnknownItem(open.itemId, type, ItemStatus.COMPLETED, parent, block))
                }
                else -> AgentEvent.ItemCompleted(b, t, turn, UnknownItem(open.itemId, type, ItemStatus.COMPLETED, parent, block))
            }
        }
        return events
    }

    private fun category(type: String) = when (type) {
        "tool_use", "server_tool_use", "mcp_tool_use" -> "tool"
        "thinking", "redacted_thinking" -> "thinking"
        else -> type
    }

    // ------------------------------------------------------------------ user frames

    /** [live] = stdout frame (vs transcript entry). */
    internal fun user(t: String, f: JsonObject, live: Boolean): List<AgentEvent> {
        val message = f["message"]
        val content = message["content"]
        val uuid = f["uuid"].str
        val parent = f["parent_tool_use_id"].str
        val turn = turnOf(f)
        // Meta entries are hidden context (image source paths, local-command caveats).
        if (f["isMeta"].bool == true) return emptyList()
        if (content is JsonPrimitive) {
            val text = content.str ?: ""
            val stripped = localCommand(text) ?: if (live && f["isReplay"].bool == true && uuid !in startedTurns) text else null
            if (stripped != null) {
                return listOf(if (turn != null && turn !in finishedTurns) AgentEvent.ItemCompleted(b, t, turn, MarkerItem("marker-${uuid ?: markerSeq++}", MarkerKind.LOCAL_COMMAND, stripped, raw = f))
                else AgentEvent.ThreadNotice(b, t, Notice(NoticeLevel.INFO, stripped, "local_command", raw = f)))
            }
        }
        val blocks = content.arr ?: (content as? JsonPrimitive)?.let { JsonArray(listOf(JsonObject(mapOf("type" to JsonPrimitive("text"), "text" to it)))) } ?: return emptyList()
        val results = blocks.filter { it["type"].str == "tool_result" }
        if (results.isNotEmpty()) {
            val structured = f["tool_use_result"] ?: f["toolUseResult"]
            val meta = f["tool_result_meta"].arr
            uuid?.let { u -> turn?.let { lastUuid[it] = u } }
            return results.mapNotNull { r ->
                val id = r["tool_use_id"].str ?: return@mapNotNull null
                val tool = tools[id] ?: return@mapNotNull AgentEvent.Unknown(b, "tool_result", t, f)
                val refused = id in denied || f["toolDenialKind"].str != null ||
                    meta?.any { it["id"].str == id && it["non_execution_kind"].str != null } == true
                AgentEvent.ItemCompleted(b, t, tool.turnId, finishedTool(tool, r["content"], structured, r["is_error"].bool == true, if (refused) ItemStatus.DECLINED else null))
            }
        }
        val parts = blocks.mapNotNull(::userPart)
        val onlyText = parts.filterIsInstance<UserPart.Text>().joinToString("\n") { it.text }
        if (parts.size == 1 && onlyText == "[Request interrupted by user]" || onlyText == "[Request interrupted by user for tool use]") {
            val target = turn ?: return emptyList()
            return listOf(AgentEvent.ItemCompleted(b, t, target, MarkerItem("marker-${uuid ?: markerSeq++}", MarkerKind.INTERRUPTED, onlyText, raw = f)))
        }
        val id = uuid ?: return listOf(AgentEvent.Unknown(b, "user", t, f))
        val ownPrompt = id in startedTurns || (live && f["isReplay"].bool == true)
        val targetTurn = if (ownPrompt) id else turn ?: id
        return listOf(AgentEvent.ItemCompleted(b, t, targetTurn, UserMessageItem(
            id = id, parts = parts, clientMessageId = if (ownPrompt) id else null, status = ItemStatus.COMPLETED, parentId = parent, raw = f,
        )))
    }

    private fun localCommand(text: String): String? {
        Regex("<local-command-(stdout|stderr)>([\\s\\S]*?)</local-command-\\1>").find(text)?.let { return it.groupValues[2].trim() }
        val name = Regex("<command-name>([\\s\\S]*?)</command-name>").find(text)?.groupValues?.get(1) ?: return null
        val args = Regex("<command-args>([\\s\\S]*?)</command-args>").find(text)?.groupValues?.get(1)
        return listOfNotNull(name.trim(), args?.trim()?.takeIf { it.isNotEmpty() }).joinToString(" ")
    }

    fun userPart(block: JsonElement): UserPart? = when (block["type"].str) {
        "text" -> UserPart.Text(block["text"].str ?: "")
        "image", "document" -> {
            val source = block["source"]
            when (source["type"].str) {
                "base64" -> UserPart.InlineData(block["type"].str!!, source["media_type"].str, source["data"].str ?: "")
                "url" -> UserPart.ImageUrl(source["url"].str ?: "")
                else -> UserPart.Unknown(block)
            }
        }
        null -> null
        else -> UserPart.Unknown(block)
    }

    // ------------------------------------------------------------------ tools

    private fun toolItem(id: String, name: String, input: JsonElement?, parent: String?, status: ItemStatus, raw: JsonElement?): Item = when {
        name == "Bash" -> CommandItem(id, command = input["command"].str ?: "", description = input["description"].str, status = status, parentId = parent, raw = raw)
        name == "Write" -> FileChangeItem(id, listOfNotNull(input["file_path"].str?.let { FileDelta(it, FileChangeKind.UPDATE) }), tool = name, status = status, parentId = parent, raw = raw)
        name == "Edit" || name == "MultiEdit" -> FileChangeItem(id, listOfNotNull(input["file_path"].str?.let { FileDelta(it, FileChangeKind.UPDATE, editDiff(input)) }), tool = name, status = status, parentId = parent, raw = raw)
        name == "NotebookEdit" -> FileChangeItem(id, listOfNotNull(input["notebook_path"].str?.let { FileDelta(it, FileChangeKind.UPDATE) }), tool = name, status = status, parentId = parent, raw = raw)
        name == "TodoWrite" -> PlanItem(id, steps = todos(input), status = status, parentId = parent, raw = raw)
        name == "Task" || name == "Agent" -> SubAgentItem(id, name, input["description"].str, input["prompt"].str, input["subagent_type"].str, input["model"].str, status = status, parentId = parent, raw = raw)
        name == "WebSearch" || name == "web_search" -> WebSearchItem(id, input["query"].str ?: "", status = status, parentId = parent, raw = raw)
        name.startsWith("mcp__") -> {
            val rest = name.removePrefix("mcp__")
            ToolCallItem(id, ToolKind.MCP, rest.substringAfter("__"), rest.substringBefore("__"), input, status = status, parentId = parent, raw = raw)
        }
        else -> ToolCallItem(id, ToolKind.BUILTIN, name, arguments = input, status = status, parentId = parent, raw = raw)
    }

    private fun finishedTool(tool: Tool, content: JsonElement?, structured: JsonElement?, isError: Boolean, forced: ItemStatus?): Item {
        val status = forced ?: if (isError) ItemStatus.FAILED else ItemStatus.COMPLETED
        val text = resultText(content)
        return when (val item = toolItem(tool.id, tool.name, tool.input, tool.parentId, status, structured ?: content)) {
            is CommandItem -> {
                val stdout = structured["stdout"].str
                val stderr = structured["stderr"].str
                val output = if (stdout != null || stderr != null) listOfNotNull(stdout, stderr?.takeIf { it.isNotEmpty() }).joinToString("\n") else text
                item.copy(output = StreamText.of(output), exitCode = if (structured["interrupted"].bool == true) null else null)
            }
            is FileChangeItem -> {
                val path = structured["filePath"].str
                val kind = if (structured["type"].str == "create") FileChangeKind.ADD else FileChangeKind.UPDATE
                val diff = structuredPatch(path ?: item.changes.firstOrNull()?.path, structured["structuredPatch"]) ?: item.changes.firstOrNull()?.diff
                val changes = (path ?: item.changes.firstOrNull()?.path)?.let { listOf(FileDelta(it, kind, diff)) } ?: item.changes
                item.copy(changes = changes, output = StreamText.of(text))
            }
            is SubAgentItem -> item.copy(result = text)
            is WebSearchItem -> item.copy(results = content)
            is ToolCallItem -> item.copy(result = content, resultText = text, error = if (isError) text else null)
            else -> item
        }
    }

    private fun resultText(content: JsonElement?): String? = when (content) {
        null -> null
        is JsonPrimitive -> content.str
        is JsonArray -> content.mapNotNull { it["text"].str }.joinToString("\n").ifEmpty { null }
        else -> content.encode()
    }

    private fun todos(input: JsonElement?): List<PlanStep> = input["todos"].arr?.map {
        PlanStep(it["content"].str ?: "", when (it["status"].str) {
            "completed" -> PlanStepStatus.COMPLETED
            "in_progress" -> PlanStepStatus.IN_PROGRESS
            else -> PlanStepStatus.PENDING
        })
    } ?: emptyList()

    /** Minimal unified diff from Edit's old/new strings (display only). */
    private fun editDiff(input: JsonElement?): String? {
        val edits = input["edits"].arr?.map { it["old_string"].str to it["new_string"].str }
            ?: listOf(input["old_string"].str to input["new_string"].str)
        if (edits.all { it.first == null && it.second == null }) return null
        return edits.joinToString("\n") { (old, new) ->
            (old?.lines()?.joinToString("\n") { "-$it" } ?: "") + "\n" + (new?.lines()?.joinToString("\n") { "+$it" } ?: "")
        }
    }

    private fun structuredPatch(path: String?, patch: JsonElement?): String? {
        val hunks = patch.arr?.takeIf { it.isNotEmpty() } ?: return null
        val out = StringBuilder()
        if (path != null) out.append("--- ").append(path).append("\n+++ ").append(path).append('\n')
        for (h in hunks) {
            out.append("@@ -${h["oldStart"].long ?: 0},${h["oldLines"].long ?: 0} +${h["newStart"].long ?: 0},${h["newLines"].long ?: 0} @@\n")
            h["lines"].strings().forEach { out.append(it).append('\n') }
        }
        return out.toString()
    }

    companion object {
        fun rateLimit(info: JsonElement?): RateLimit? {
            val o = info.obj ?: return null
            val utilization = o["utilization"].double
            return RateLimit(
                id = o["rateLimitType"].str ?: "claude",
                primary = RateLimitWindow(utilization?.let { if (it <= 1.0) it * 100 else it }, null, o["resetsAt"].long),
                status = o["status"].str,
                reached = o["status"].str == "rejected",
                raw = o,
            )
        }

        /** `list_models` / `initialize.models` → catalog. Claude accepts text, images and PDFs on every model. */
        fun models(models: JsonElement?): ModelCatalog = ModelCatalog(BackendKind.CLAUDE, models.arr?.mapIndexed { i, m ->
            ModelOption(
                id = m["value"].str ?: "",
                displayName = m["displayName"].str ?: m["value"].str ?: "",
                description = m["description"].str,
                efforts = if (m["supportsEffort"].bool == true) m["supportedEffortLevels"].strings().map { EffortOption(it) } else emptyList(),
                defaultEffort = if (m["supportsEffort"].bool == true) "medium".takeIf { "medium" in m["supportedEffortLevels"].strings() } else null,
                isDefault = i == 0 && m["value"].str == "default",
                inputModalities = setOf("text", "image", "pdf"),
                resolvedModel = m["resolvedModel"].str,
                raw = m,
            )
        } ?: emptyList())

        /** `initialize` response `account` → account state (no secrets are present in it). */
        fun account(response: JsonElement?): AccountState {
            val a = response["account"]
            val token = a["tokenSource"].str
            val apiKey = a["apiKeySource"].str
            val loggedIn = (token != null && token != "none") || (apiKey != null && apiKey != "none")
            return AccountState(
                state = if (loggedIn) LoginState.LOGGED_IN else LoginState.LOGGED_OUT,
                method = token?.takeIf { it != "none" } ?: apiKey,
                email = a["email"].str,
                plan = a["subscriptionType"].str,
                organization = a["organization"].str,
                raw = a,
            )
        }
    }
}
