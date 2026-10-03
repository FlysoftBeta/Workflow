package top.flysoftbeta.workflow.agent.model

import kotlinx.serialization.Serializable
import kotlinx.serialization.SerialName

import kotlinx.serialization.json.JsonElement

@Serializable
enum class ItemStatus {
    IN_PROGRESS,
    COMPLETED,
    /** Started but never completed (turn interrupted, process exited, stream aborted). */
    INCOMPLETE,
    FAILED,
    /** The user or a policy refused it. */
    DECLINED,
    ;

    val isFinal: Boolean get() = this != IN_PROGRESS
}

@Serializable
enum class MessagePhase {
    /** Interim narration while the agent works ("during answer" view). */
    COMMENTARY,
    /** The answer shown in the "final" view. */
    FINAL,
    /** Not yet known (Claude); resolved when the turn completes. */
    UNKNOWN,
}

/**
 * One rendered unit of a turn. [id] is the backend's item id (Codex `item.id`; Claude
 * `<message id>:<block index>` for text/thinking, the `tool_use` id for tools, the user uuid for
 * user messages). [parentId] nests sub-agent items under the tool call that spawned them.
 * [raw] keeps the last authoritative backend payload for the generic JSON view.
 */
@Serializable
sealed interface Item {
    val id: String
    val status: ItemStatus
    val parentId: String?
    val raw: JsonElement?

    fun withStatus(status: ItemStatus): Item
}

@Serializable
@SerialName("UserMessageItem")
data class UserMessageItem(
    override val id: String,
    val parts: List<UserPart>,
    val clientMessageId: String? = null,
    /** Optimistic copy created by the app before the backend echoed the message. */
    val local: Boolean = false,
    override val status: ItemStatus = ItemStatus.COMPLETED,
    override val parentId: String? = null,
    override val raw: JsonElement? = null,
) : Item {
    val text: String get() = parts.filterIsInstance<UserPart.Text>().joinToString("\n") { it.text }
    override fun withStatus(status: ItemStatus) = copy(status = status)
}

@Serializable
@SerialName("AgentMessageItem")
data class AgentMessageItem(
    override val id: String,
    val text: StreamText,
    val phase: MessagePhase = MessagePhase.UNKNOWN,
    override val status: ItemStatus = ItemStatus.IN_PROGRESS,
    override val parentId: String? = null,
    override val raw: JsonElement? = null,
) : Item {
    override fun withStatus(status: ItemStatus) = copy(status = status)
}

@Serializable
@SerialName("ReasoningItem")
data class ReasoningItem(
    override val id: String,
    /** Reasoning summary parts (Codex `summary[]`); collapsed "Thought for Ns" view. */
    val summary: List<StreamText> = emptyList(),
    /** Raw reasoning text parts (Codex `content[]`, Claude `thinking`). */
    val content: List<StreamText> = emptyList(),
    /** Claude `redacted_thinking` or hidden reasoning. */
    val redacted: Boolean = false,
    override val status: ItemStatus = ItemStatus.IN_PROGRESS,
    override val parentId: String? = null,
    override val raw: JsonElement? = null,
) : Item {
    override fun withStatus(status: ItemStatus) = copy(status = status)
}

@Serializable
enum class PlanStepStatus { PENDING, IN_PROGRESS, COMPLETED }

@Serializable
@SerialName("PlanStep")
data class PlanStep(val text: String, val status: PlanStepStatus)

/** Codex `plan` item (proposed plan markdown) or Claude `TodoWrite` (steps). */
@Serializable
@SerialName("PlanItem")
data class PlanItem(
    override val id: String,
    val text: StreamText = StreamText.EMPTY,
    val steps: List<PlanStep> = emptyList(),
    override val status: ItemStatus = ItemStatus.IN_PROGRESS,
    override val parentId: String? = null,
    override val raw: JsonElement? = null,
) : Item {
    override fun withStatus(status: ItemStatus) = copy(status = status)
}

@Serializable
@SerialName("CommandAction")
data class CommandAction(val kind: String, val command: String, val path: String? = null, val query: String? = null)

@Serializable
@SerialName("CommandItem")
data class CommandItem(
    override val id: String,
    val command: String,
    val cwd: String? = null,
    val output: StreamText = StreamText.EMPTY,
    /** True when the head of the output was dropped to bound memory. */
    val outputTruncated: Boolean = false,
    val exitCode: Int? = null,
    val durationMs: Long? = null,
    val processId: String? = null,
    val actions: List<CommandAction> = emptyList(),
    val description: String? = null,
    override val status: ItemStatus = ItemStatus.IN_PROGRESS,
    override val parentId: String? = null,
    override val raw: JsonElement? = null,
) : Item {
    override fun withStatus(status: ItemStatus) = copy(status = status)
}

@Serializable
enum class FileChangeKind { ADD, DELETE, UPDATE, MOVE }

@Serializable
@SerialName("FileDelta")
data class FileDelta(val path: String, val kind: FileChangeKind, val diff: String? = null, val movePath: String? = null)

@Serializable
@SerialName("FileChangeItem")
data class FileChangeItem(
    override val id: String,
    val changes: List<FileDelta>,
    val output: StreamText = StreamText.EMPTY,
    /** Tool name for Claude (`Write`, `Edit`, ...). */
    val tool: String? = null,
    override val status: ItemStatus = ItemStatus.IN_PROGRESS,
    override val parentId: String? = null,
    override val raw: JsonElement? = null,
) : Item {
    override fun withStatus(status: ItemStatus) = copy(status = status)
}

@Serializable
enum class ToolKind { MCP, DYNAMIC, BUILTIN, FUNCTION_OUTPUT }

@Serializable
@SerialName("ToolCallItem")
data class ToolCallItem(
    override val id: String,
    val kind: ToolKind,
    val tool: String,
    val server: String? = null,
    val arguments: JsonElement? = null,
    /** Partial JSON streamed before [arguments] is final (Claude `input_json_delta`). */
    val argumentsText: StreamText = StreamText.EMPTY,
    val result: JsonElement? = null,
    val resultText: String? = null,
    val error: String? = null,
    val progress: List<String> = emptyList(),
    val durationMs: Long? = null,
    override val status: ItemStatus = ItemStatus.IN_PROGRESS,
    override val parentId: String? = null,
    override val raw: JsonElement? = null,
) : Item {
    override fun withStatus(status: ItemStatus) = copy(status = status)
}

/** A delegated agent: Codex `collabAgentToolCall` / `subAgentActivity`, Claude `Task`/`Agent`. */
@Serializable
@SerialName("SubAgentItem")
data class SubAgentItem(
    override val id: String,
    val tool: String,
    val description: String? = null,
    val prompt: String? = null,
    val agentType: String? = null,
    val model: String? = null,
    val threadIds: List<String> = emptyList(),
    val result: String? = null,
    val progress: List<String> = emptyList(),
    override val status: ItemStatus = ItemStatus.IN_PROGRESS,
    override val parentId: String? = null,
    override val raw: JsonElement? = null,
) : Item {
    override fun withStatus(status: ItemStatus) = copy(status = status)
}

@Serializable
@SerialName("WebSearchItem")
data class WebSearchItem(
    override val id: String,
    val query: String,
    val action: String? = null,
    val url: String? = null,
    val results: JsonElement? = null,
    override val status: ItemStatus = ItemStatus.IN_PROGRESS,
    override val parentId: String? = null,
    override val raw: JsonElement? = null,
) : Item {
    override fun withStatus(status: ItemStatus) = copy(status = status)
}

@Serializable
enum class ImageKind { VIEW, GENERATED }

@Serializable
@SerialName("ImageItem")
data class ImageItem(
    override val id: String,
    val kind: ImageKind,
    val path: String? = null,
    val url: String? = null,
    val prompt: String? = null,
    override val status: ItemStatus = ItemStatus.COMPLETED,
    override val parentId: String? = null,
    override val raw: JsonElement? = null,
) : Item {
    override fun withStatus(status: ItemStatus) = copy(status = status)
}

@Serializable
enum class MarkerKind { COMPACTION, REVIEW_ENTERED, REVIEW_EXITED, HOOK, HOOK_PROMPT, MODEL_REROUTED, INTERRUPTED, LOCAL_COMMAND, SLEEP }

@Serializable
@SerialName("MarkerItem")
data class MarkerItem(
    override val id: String,
    val kind: MarkerKind,
    val text: String? = null,
    override val status: ItemStatus = ItemStatus.COMPLETED,
    override val parentId: String? = null,
    override val raw: JsonElement? = null,
) : Item {
    override fun withStatus(status: ItemStatus) = copy(status = status)
}

@Serializable
enum class NoticeLevel { INFO, WARNING, ERROR }

@Serializable
@SerialName("Notice")
data class Notice(
    val level: NoticeLevel,
    val message: String,
    /** Backend error code (`usageLimitExceeded`, `authentication_failed`, ...). */
    val code: String? = null,
    val willRetry: Boolean = false,
    val detail: String? = null,
    val raw: JsonElement? = null,
)

@Serializable
@SerialName("NoticeItem")
data class NoticeItem(
    override val id: String,
    val notice: Notice,
    override val status: ItemStatus = ItemStatus.COMPLETED,
    override val parentId: String? = null,
    override val raw: JsonElement? = notice.raw,
) : Item {
    override fun withStatus(status: ItemStatus) = copy(status = status)
}

/** A backend item type this model does not know; rendered as collapsible JSON, never dropped. */
@Serializable
@SerialName("UnknownItem")
data class UnknownItem(
    override val id: String,
    val type: String,
    override val status: ItemStatus = ItemStatus.COMPLETED,
    override val parentId: String? = null,
    override val raw: JsonElement? = null,
) : Item {
    override fun withStatus(status: ItemStatus) = copy(status = status)
}
