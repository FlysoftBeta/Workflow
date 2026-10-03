package top.flysoftbeta.workflow.agent.model

import kotlinx.serialization.Serializable
import kotlinx.serialization.SerialName

import kotlinx.serialization.json.JsonElement

/** Incremental change to an existing item. */
@Serializable
sealed interface ItemDelta {
    @Serializable
    @SerialName("AgentText")
    data class AgentText(val text: String) : ItemDelta
    @Serializable
    @SerialName("ReasoningSummaryPart")
    data class ReasoningSummaryPart(val index: Int) : ItemDelta
    @Serializable
    @SerialName("ReasoningSummary")
    data class ReasoningSummary(val index: Int, val text: String) : ItemDelta
    @Serializable
    @SerialName("ReasoningText")
    data class ReasoningText(val index: Int, val text: String) : ItemDelta
    @Serializable
    @SerialName("PlanText")
    data class PlanText(val text: String) : ItemDelta
    @Serializable
    @SerialName("CommandOutput")
    data class CommandOutput(val text: String) : ItemDelta
    /** Codex `terminalInteraction`: the agent wrote to the process's stdin. */
    @Serializable
    @SerialName("TerminalInput")
    data class TerminalInput(val text: String) : ItemDelta
    @Serializable
    @SerialName("FileChangeOutput")
    data class FileChangeOutput(val text: String) : ItemDelta
    @Serializable
    @SerialName("FileChangePatch")
    data class FileChangePatch(val changes: List<FileDelta>) : ItemDelta
    @Serializable
    @SerialName("ToolArguments")
    data class ToolArguments(val partialJson: String) : ItemDelta
    @Serializable
    @SerialName("ToolProgress")
    data class ToolProgress(val message: String) : ItemDelta
}

/**
 * Normalised backend event. Adapters map wire frames to these; [AgentReducer] folds them into
 * [AgentState]. Every event names its backend; thread-scoped events name the backend thread id.
 */
@Serializable
sealed interface AgentEvent {
    val backend: BackendKind

    // ---- backend / process ----
    @Serializable
    @SerialName("ProcessChanged")
    data class ProcessChanged(override val backend: BackendKind, val state: ProcessState) : AgentEvent
    @Serializable
    @SerialName("ServerInfo")
    data class ServerInfo(override val backend: BackendKind, val info: JsonElement) : AgentEvent
    @Serializable
    @SerialName("AccountChanged")
    data class AccountChanged(override val backend: BackendKind, val account: AccountState) : AgentEvent
    @Serializable
    @SerialName("LoginChanged")
    data class LoginChanged(override val backend: BackendKind, val flow: LoginFlow?) : AgentEvent
    @Serializable
    @SerialName("RateLimitsChanged")
    data class RateLimitsChanged(override val backend: BackendKind, val limits: RateLimitState, val merge: Boolean = false) : AgentEvent
    @Serializable
    @SerialName("ModelsChanged")
    data class ModelsChanged(override val backend: BackendKind, val catalog: ModelCatalog) : AgentEvent
    @Serializable
    @SerialName("McpServerChanged")
    data class McpServerChanged(override val backend: BackendKind, val status: McpServerStatus) : AgentEvent
    @Serializable
    @SerialName("BackendNotice")
    data class BackendNotice(override val backend: BackendKind, val notice: Notice) : AgentEvent
    @Serializable
    @SerialName("Unknown")
    data class Unknown(override val backend: BackendKind, val kind: String, val threadId: String?, val raw: JsonElement) : AgentEvent

    // ---- thread ----
    @Serializable
    @SerialName("ThreadUpserted")
    data class ThreadUpserted(
        override val backend: BackendKind,
        val threadId: String,
        val title: String? = null,
        val preview: String? = null,
        val cwd: String? = null,
        val path: String? = null,
        val forkedFrom: String? = null,
        val ephemeral: Boolean? = null,
        val runState: RunState? = null,
        val settings: ThreadSettings? = null,
        val createdAtSec: Long? = null,
        val updatedAtSec: Long? = null,
        val raw: JsonElement? = null,
    ) : AgentEvent
    @Serializable
    @SerialName("ThreadStatusChanged")
    data class ThreadStatusChanged(override val backend: BackendKind, val threadId: String, val runState: RunState) : AgentEvent
    @Serializable
    @SerialName("ThreadRenamed")
    data class ThreadRenamed(override val backend: BackendKind, val threadId: String, val title: String?) : AgentEvent
    @Serializable
    @SerialName("ThreadArchived")
    data class ThreadArchived(override val backend: BackendKind, val threadId: String, val archived: Boolean) : AgentEvent
    @Serializable
    @SerialName("ThreadDeleted")
    data class ThreadDeleted(override val backend: BackendKind, val threadId: String) : AgentEvent
    @Serializable
    @SerialName("ThreadClosed")
    data class ThreadClosed(override val backend: BackendKind, val threadId: String) : AgentEvent
    @Serializable
    @SerialName("ThreadSettingsChanged")
    data class ThreadSettingsChanged(override val backend: BackendKind, val threadId: String, val settings: ThreadSettings) : AgentEvent
    @Serializable
    @SerialName("TokenUsageChanged")
    data class TokenUsageChanged(override val backend: BackendKind, val threadId: String, val turnId: String?, val usage: TokenUsage) : AgentEvent
    @Serializable
    @SerialName("ThreadNotice")
    data class ThreadNotice(override val backend: BackendKind, val threadId: String, val notice: Notice) : AgentEvent
    /** History hydration: [turns] are complete, oldest first. [prepend] = older page loaded. */
    @Serializable
    @SerialName("HistoryLoaded")
    data class HistoryLoaded(
        override val backend: BackendKind,
        val threadId: String,
        val turns: List<Turn>,
        val prepend: Boolean = false,
        val cursor: String? = null,
    ) : AgentEvent

    /** Authoritative backend queue. Only previously acknowledged queued ids may disappear. */
    @Serializable
    @SerialName("QueueUpdated")
    data class QueueUpdated(
        override val backend: BackendKind,
        val threadId: String,
        val messages: List<QueuedMessage>,
        val previouslyQueued: Set<String>,
    ) : AgentEvent

    // ---- turn ----
    /** Local: the user sent a message. Creates an optimistic QUEUED turn with the user's parts. */
    @Serializable
    @SerialName("TurnSubmitted")
    data class TurnSubmitted(
        override val backend: BackendKind,
        val threadId: String,
        val clientMessageId: String,
        val parts: List<UserPart>,
        val settings: TurnSettings?,
        val atMs: Long? = null,
    ) : AgentEvent
    /** Local: the backend accepted [clientMessageId] as turn [turnId] (Codex `turn/start` response). */
    @Serializable
    @SerialName("TurnBound")
    data class TurnBound(override val backend: BackendKind, val threadId: String, val clientMessageId: String, val turnId: String) : AgentEvent
    @Serializable
    @SerialName("TurnStarted")
    data class TurnStarted(
        override val backend: BackendKind,
        val threadId: String,
        val turnId: String,
        val clientMessageId: String? = null,
        val atMs: Long? = null,
    ) : AgentEvent
    /** Local or backend: a queued turn was removed before it started. */
    @Serializable
    @SerialName("TurnCancelled")
    data class TurnCancelled(override val backend: BackendKind, val threadId: String, val turnId: String) : AgentEvent
    @Serializable
    @SerialName("TurnCompleted")
    data class TurnCompleted(
        override val backend: BackendKind,
        val threadId: String,
        val turnId: String,
        val status: TurnStatus,
        val error: TurnError? = null,
        /** Authoritative items the backend attached to the completion (upserted, never used to delete). */
        val items: List<Item> = emptyList(),
        val durationMs: Long? = null,
        val usage: TokenUsage? = null,
        val atMs: Long? = null,
    ) : AgentEvent
    @Serializable
    @SerialName("TurnPlanUpdated")
    data class TurnPlanUpdated(override val backend: BackendKind, val threadId: String, val turnId: String, val plan: TurnPlan) : AgentEvent
    @Serializable
    @SerialName("TurnDiffUpdated")
    data class TurnDiffUpdated(override val backend: BackendKind, val threadId: String, val turnId: String, val diff: String) : AgentEvent
    @Serializable
    @SerialName("TurnProgress")
    data class TurnProgress(override val backend: BackendKind, val threadId: String, val turnId: String, val thinkingTokens: Long?) : AgentEvent
    @Serializable
    @SerialName("TurnNotice")
    data class TurnNotice(override val backend: BackendKind, val threadId: String, val turnId: String?, val notice: Notice) : AgentEvent

    // ---- item ----
    @Serializable
    @SerialName("ItemStarted")
    data class ItemStarted(override val backend: BackendKind, val threadId: String, val turnId: String, val item: Item) : AgentEvent
    @Serializable
    @SerialName("ItemUpdated")
    data class ItemUpdated(override val backend: BackendKind, val threadId: String, val turnId: String, val itemId: String, val delta: ItemDelta) : AgentEvent
    @Serializable
    @SerialName("ItemCompleted")
    data class ItemCompleted(override val backend: BackendKind, val threadId: String, val turnId: String, val item: Item) : AgentEvent
    /** A tool call was refused (Claude `permission_denials`, denied `can_use_tool`). */
    @Serializable
    @SerialName("ItemDeclined")
    data class ItemDeclined(override val backend: BackendKind, val threadId: String, val turnId: String?, val itemId: String) : AgentEvent

    // ---- server requests ----
    @Serializable
    @SerialName("RequestOpened")
    data class RequestOpened(val request: PendingRequest) : AgentEvent {
        override val backend: BackendKind get() = request.key.backend
    }
    @Serializable
    @SerialName("RequestClosed")
    data class RequestClosed(val key: RequestKey, val status: RequestStatus, val answer: String? = null) : AgentEvent {
        override val backend: BackendKind get() = key.backend
    }
}

@Serializable
@SerialName("QueuedMessage")
data class QueuedMessage(val clientMessageId: String, val parts: List<UserPart>)
