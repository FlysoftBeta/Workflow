package top.flysoftbeta.workflow.agent.codex

import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonArray
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import top.flysoftbeta.workflow.agent.json.arr
import top.flysoftbeta.workflow.agent.json.bool
import top.flysoftbeta.workflow.agent.json.encode
import top.flysoftbeta.workflow.agent.json.get
import top.flysoftbeta.workflow.agent.json.int
import top.flysoftbeta.workflow.agent.json.long
import top.flysoftbeta.workflow.agent.json.obj
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.json.strings
import top.flysoftbeta.workflow.agent.model.AgentMessageItem
import top.flysoftbeta.workflow.agent.model.CommandAction
import top.flysoftbeta.workflow.agent.model.CommandItem
import top.flysoftbeta.workflow.agent.model.FileChangeItem
import top.flysoftbeta.workflow.agent.model.FileChangeKind
import top.flysoftbeta.workflow.agent.model.FileDelta
import top.flysoftbeta.workflow.agent.model.ImageItem
import top.flysoftbeta.workflow.agent.model.ImageKind
import top.flysoftbeta.workflow.agent.model.Item
import top.flysoftbeta.workflow.agent.model.ItemStatus
import top.flysoftbeta.workflow.agent.model.MarkerItem
import top.flysoftbeta.workflow.agent.model.MarkerKind
import top.flysoftbeta.workflow.agent.model.MessagePhase
import top.flysoftbeta.workflow.agent.model.PlanItem
import top.flysoftbeta.workflow.agent.model.PlanStep
import top.flysoftbeta.workflow.agent.model.PlanStepStatus
import top.flysoftbeta.workflow.agent.model.ReasoningItem
import top.flysoftbeta.workflow.agent.model.StreamText
import top.flysoftbeta.workflow.agent.model.SubAgentItem
import top.flysoftbeta.workflow.agent.model.ToolCallItem
import top.flysoftbeta.workflow.agent.model.ToolKind
import top.flysoftbeta.workflow.agent.model.Turn
import top.flysoftbeta.workflow.agent.model.TurnError
import top.flysoftbeta.workflow.agent.model.TurnStatus
import top.flysoftbeta.workflow.agent.model.UnknownItem
import top.flysoftbeta.workflow.agent.model.UserMessageItem
import top.flysoftbeta.workflow.agent.model.UserPart
import top.flysoftbeta.workflow.agent.model.WebSearchItem

/** Codex `ThreadItem` / `UserInput` / `Turn` JSON ↔ neutral model. Tolerant: missing fields get defaults. */
object CodexItems {
    /** Item types this mapper renders natively; others become [UnknownItem]. */
    val KNOWN_TYPES = setOf(
        "userMessage", "hookPrompt", "agentMessage", "functionCallOutput", "plan", "reasoning", "commandExecution",
        "fileChange", "mcpToolCall", "dynamicToolCall", "collabAgentToolCall", "subAgentActivity", "webSearch",
        "imageView", "sleep", "imageGeneration", "enteredReviewMode", "exitedReviewMode", "contextCompaction",
    )

    /** [completed] = the payload came from `item/completed` or a finished turn. */
    fun parse(json: JsonElement, completed: Boolean): Item {
        val o = json.obj ?: return UnknownItem("unknown", "invalid", raw = json)
        val id = o["id"].str ?: "unknown"
        val done = if (completed) ItemStatus.COMPLETED else ItemStatus.IN_PROGRESS
        return when (val type = o["type"].str) {
            "userMessage" -> UserMessageItem(
                id = id,
                parts = o["content"].arr?.map(::userPart) ?: emptyList(),
                clientMessageId = o["clientId"].str,
                status = ItemStatus.COMPLETED,
                raw = o,
            )
            "hookPrompt" -> MarkerItem(id, MarkerKind.HOOK_PROMPT, o["fragments"].arr?.mapNotNull { it["text"].str }?.joinToString("\n"), raw = o)
            "agentMessage" -> AgentMessageItem(id, StreamText.of(o["text"].str), phase(o["phase"].str), done, raw = o)
            "functionCallOutput" -> ToolCallItem(
                id = id, kind = ToolKind.FUNCTION_OUTPUT, tool = listOfNotNull(o["namespace"].str, o["name"].str).joinToString("."),
                result = o["output"], resultText = o["output"].str, status = ItemStatus.COMPLETED, raw = o,
            )
            "plan" -> PlanItem(id, StreamText.of(o["text"].str), status = done, raw = o)
            "reasoning" -> ReasoningItem(
                id = id,
                summary = o["summary"].strings().map(StreamText::of),
                content = o["content"].strings().map(StreamText::of),
                status = done,
                raw = o,
            )
            "commandExecution" -> CommandItem(
                id = id,
                command = o["command"].str ?: "",
                cwd = o["cwd"].str,
                output = StreamText.of(o["aggregatedOutput"].str),
                exitCode = o["exitCode"].int,
                durationMs = o["durationMs"].long,
                processId = o["processId"].str,
                actions = o["commandActions"].arr?.map(::commandAction) ?: emptyList(),
                status = status(o["status"].str, done),
                raw = o,
            )
            "fileChange" -> FileChangeItem(id, changes(o["changes"]), status = status(o["status"].str, done), raw = o)
            "mcpToolCall" -> ToolCallItem(
                id = id, kind = ToolKind.MCP, tool = o["tool"].str ?: "", server = o["server"].str,
                arguments = o["arguments"], result = o["result"].takeIf { it is JsonObject },
                error = o["error"]["message"].str, durationMs = o["durationMs"].long,
                status = status(o["status"].str, done), raw = o,
            )
            "dynamicToolCall" -> ToolCallItem(
                id = id, kind = ToolKind.DYNAMIC, tool = listOfNotNull(o["namespace"].str, o["tool"].str).joinToString("."),
                arguments = o["arguments"], result = o["contentItems"].takeIf { it is JsonArray },
                durationMs = o["durationMs"].long,
                status = when {
                    o["success"].bool == false -> ItemStatus.FAILED
                    else -> status(o["status"].str, done)
                },
                raw = o,
            )
            "collabAgentToolCall" -> SubAgentItem(
                id = id, tool = o["tool"].str ?: "collab", prompt = o["prompt"].str, model = o["model"].str,
                threadIds = o["receiverThreadIds"].strings(), status = status(o["status"].str, done), raw = o,
            )
            "subAgentActivity" -> SubAgentItem(
                id = id, tool = "subAgent", description = o["agentPath"].str, threadIds = listOfNotNull(o["agentThreadId"].str),
                status = when (o["kind"].str) {
                    "completed" -> ItemStatus.COMPLETED
                    "interrupted" -> ItemStatus.INCOMPLETE
                    else -> if (completed) ItemStatus.COMPLETED else ItemStatus.IN_PROGRESS
                },
                raw = o,
            )
            "webSearch" -> WebSearchItem(
                id = id, query = o["query"].str ?: "", action = o["action"]["type"].str, url = o["action"]["url"].str,
                results = o["results"], status = done, raw = o,
            )
            "imageView" -> ImageItem(id, ImageKind.VIEW, path = o["path"].str, raw = o)
            "imageGeneration" -> ImageItem(
                id, ImageKind.GENERATED, path = o["savedPath"].str, prompt = o["revisedPrompt"].str,
                status = when {
                    o["failure"] is JsonObject -> ItemStatus.FAILED
                    o["status"].str == "completed" || completed -> ItemStatus.COMPLETED
                    else -> ItemStatus.IN_PROGRESS
                },
                raw = o,
            )
            "sleep" -> MarkerItem(id, MarkerKind.SLEEP, o["durationMs"].long?.let { "${it}ms" }, raw = o)
            "enteredReviewMode" -> MarkerItem(id, MarkerKind.REVIEW_ENTERED, o["review"].str, raw = o)
            "exitedReviewMode" -> MarkerItem(id, MarkerKind.REVIEW_EXITED, o["review"].str, raw = o)
            "contextCompaction" -> MarkerItem(id, MarkerKind.COMPACTION, null, status = done, raw = o)
            else -> UnknownItem(id, type ?: "unknown", status = if (completed) ItemStatus.COMPLETED else ItemStatus.IN_PROGRESS, raw = o)
        }
    }

    fun phase(value: String?): MessagePhase = when (value) {
        "commentary" -> MessagePhase.COMMENTARY
        "final_answer" -> MessagePhase.FINAL
        null -> MessagePhase.UNKNOWN
        else -> MessagePhase.FINAL // research: unknown phases are treated as final
    }

    private fun status(value: String?, default: ItemStatus): ItemStatus = when (value) {
        "inProgress" -> ItemStatus.IN_PROGRESS
        "completed" -> ItemStatus.COMPLETED
        "failed" -> ItemStatus.FAILED
        "declined" -> ItemStatus.DECLINED
        "interrupted" -> ItemStatus.INCOMPLETE
        else -> default
    }

    private fun commandAction(json: JsonElement) = CommandAction(
        kind = json["type"].str ?: "unknown",
        command = json["command"].str ?: "",
        path = json["path"].str,
        query = json["query"].str,
    )

    fun changes(json: JsonElement?): List<FileDelta> = json.arr?.map { change ->
        val kind = change["kind"]
        val movePath = kind["move_path"].str
        FileDelta(
            path = change["path"].str ?: "",
            kind = when (kind["type"].str) {
                "add" -> FileChangeKind.ADD
                "delete" -> FileChangeKind.DELETE
                else -> if (movePath != null) FileChangeKind.MOVE else FileChangeKind.UPDATE
            },
            diff = change["diff"].str,
            movePath = movePath,
        )
    } ?: emptyList()

    fun userPart(json: JsonElement): UserPart = when (json["type"].str) {
        "text" -> UserPart.Text(json["text"].str ?: "")
        "localImage" -> UserPart.Image(json["path"].str ?: "")
        "image" -> json["url"].str?.let { UserPart.ImageUrl(it) } ?: UserPart.Unknown(json)
        "skill", "mention" -> UserPart.Reference(json["type"].str!!, json["name"].str ?: "", json["path"].str ?: "")
        else -> UserPart.Unknown(json)
    }

    /**
     * Neutral parts → Codex `UserInput[]`. Images become `localImage` (the server reads the file in
     * its own environment); other files are referenced by path in a text element, since Codex has
     * no generic file input and its tools read workspace files directly.
     */
    fun userInput(parts: List<UserPart>): JsonArray = buildJsonArray {
        for (part in parts) when (part) {
            is UserPart.Text -> add(buildJsonObject { put("type", "text"); put("text", part.text); put("text_elements", JsonArray(emptyList())) })
            is UserPart.Image -> add(buildJsonObject { put("type", "localImage"); put("path", part.path) })
            is UserPart.ImageUrl -> add(buildJsonObject { put("type", "image"); put("url", part.url) })
            is UserPart.File -> add(buildJsonObject {
                put("type", "text"); put("text", "Attached file: ${part.path}"); put("text_elements", JsonArray(emptyList()))
            })
            is UserPart.Reference -> add(buildJsonObject { put("type", part.kind); put("name", part.name); put("path", part.path) })
            is UserPart.InlineData -> error("Codex input does not accept inline ${part.kind} data; save it to the workspace first")
            is UserPart.Unknown -> add(part.raw)
        }
    }

    fun turnStatus(value: String?): TurnStatus = when (value) {
        "completed" -> TurnStatus.COMPLETED
        "interrupted" -> TurnStatus.INTERRUPTED
        "failed" -> TurnStatus.FAILED
        "inProgress" -> TurnStatus.RUNNING
        else -> TurnStatus.COMPLETED
    }

    fun turnError(json: JsonElement?): TurnError? {
        val o = json.obj ?: return null
        val info = o["codexErrorInfo"]
        val code = info.str ?: info.obj?.keys?.firstOrNull()
        return TurnError(o["message"].str ?: "error", code, o["additionalDetails"].str, raw = o)
    }

    fun planSteps(json: JsonElement?): List<PlanStep> = json.arr?.map {
        PlanStep(
            it["step"].str ?: "",
            when (it["status"].str) {
                "completed" -> PlanStepStatus.COMPLETED
                "inProgress" -> PlanStepStatus.IN_PROGRESS
                else -> PlanStepStatus.PENDING
            },
        )
    } ?: emptyList()

    /** History turn (`thread/turns/list`, `thread/read`) → finished [Turn]. */
    fun turn(json: JsonElement): Turn {
        val o = json.obj ?: JsonObject(emptyMap())
        val status = turnStatus(o["status"].str)
        val items = o["items"].arr?.map { parse(it, completed = true) } ?: emptyList()
        val clientId = items.filterIsInstance<UserMessageItem>().firstOrNull()?.clientMessageId
        return Turn(
            id = o["id"].str ?: "unknown",
            clientMessageId = clientId,
            status = status,
            items = if (status == TurnStatus.RUNNING) items else items.map { if (it.status == ItemStatus.IN_PROGRESS) it.withStatus(ItemStatus.INCOMPLETE) else it },
            error = turnError(o["error"]),
            startedAtMs = o["startedAt"].long?.times(1000),
            completedAtMs = o["completedAt"].long?.times(1000),
            durationMs = o["durationMs"].long,
        )
    }

    /** Stable text for a Codex approval policy (string enum or granular object). */
    fun policyText(json: JsonElement?): String? = when (json) {
        null -> null
        is JsonPrimitive -> json.str
        else -> json.encode()
    }
}
