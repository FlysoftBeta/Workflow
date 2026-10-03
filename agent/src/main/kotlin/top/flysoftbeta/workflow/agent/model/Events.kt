package top.flysoftbeta.workflow.agent.model

import kotlinx.serialization.json.JsonElement

/** Incremental change to an existing item. */
sealed interface ItemDelta {
    data class AgentText(val text: String) : ItemDelta
    data class ReasoningSummaryPart(val index: Int) : ItemDelta
    data class ReasoningSummary(val index: Int, val text: String) : ItemDelta
    data class ReasoningText(val index: Int, val text: String) : ItemDelta
    data class PlanText(val text: String) : ItemDelta
    data class CommandOutput(val text: String) : ItemDelta
    /** Codex `terminalInteraction`: the agent wrote to the process's stdin. */
    data class TerminalInput(val text: String) : ItemDelta
    data class FileChangeOutput(val text: String) : ItemDelta
    data class FileChangePatch(val changes: List<FileDelta>) : ItemDelta
    data class ToolArguments(val partialJson: String) : ItemDelta
    data class ToolProgress(val message: String) : ItemDelta
}

/**
 * Normalised backend event. Adapters map wire frames to these; [AgentReducer] folds them into
 * [AgentState]. Every event names its backend; thread-scoped events name the backend thread id.
 */
sealed interface AgentEvent {
    val backend: BackendKind

    // ---- backend / process ----
    data class ProcessChanged(override val backend: BackendKind, val state: ProcessState) : AgentEvent
    data class ServerInfo(override val backend: BackendKind, val info: JsonElement) : AgentEvent
    data class AccountChanged(override val backend: BackendKind, val account: AccountState) : AgentEvent
    data class LoginChanged(override val backend: BackendKind, val flow: LoginFlow?) : AgentEvent
    data class RateLimitsChanged(override val backend: BackendKind, val limits: RateLimitState, val merge: Boolean = false) : AgentEvent
    data class ModelsChanged(override val backend: BackendKind, val catalog: ModelCatalog) : AgentEvent
    data class McpServerChanged(override val backend: BackendKind, val status: McpServerStatus) : AgentEvent
    data class BackendNotice(override val backend: BackendKind, val notice: Notice) : AgentEvent
    data class Unknown(override val backend: BackendKind, val kind: String, val threadId: String?, val raw: JsonElement) : AgentEvent

    // ---- thread ----
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
    data class ThreadStatusChanged(override val backend: BackendKind, val threadId: String, val runState: RunState) : AgentEvent
    data class ThreadRenamed(override val backend: BackendKind, val threadId: String, val title: String?) : AgentEvent
    data class ThreadArchived(override val backend: BackendKind, val threadId: String, val archived: Boolean) : AgentEvent
    data class ThreadDeleted(override val backend: BackendKind, val threadId: String) : AgentEvent
    data class ThreadClosed(override val backend: BackendKind, val threadId: String) : AgentEvent
    data class ThreadSettingsChanged(override val backend: BackendKind, val threadId: String, val settings: ThreadSettings) : AgentEvent
    data class TokenUsageChanged(override val backend: BackendKind, val threadId: String, val turnId: String?, val usage: TokenUsage) : AgentEvent
    data class ThreadNotice(override val backend: BackendKind, val threadId: String, val notice: Notice) : AgentEvent
    /** History hydration: [turns] are complete, oldest first. [prepend] = older page loaded. */
    data class HistoryLoaded(
        override val backend: BackendKind,
        val threadId: String,
        val turns: List<Turn>,
        val prepend: Boolean = false,
        val cursor: String? = null,
    ) : AgentEvent

    /** Authoritative backend queue. Only previously acknowledged queued ids may disappear. */
    data class QueueUpdated(
        override val backend: BackendKind,
        val threadId: String,
        val messages: List<QueuedMessage>,
        val previouslyQueued: Set<String>,
    ) : AgentEvent

    // ---- turn ----
    /** Local: the user sent a message. Creates an optimistic QUEUED turn with the user's parts. */
    data class TurnSubmitted(
        override val backend: BackendKind,
        val threadId: String,
        val clientMessageId: String,
        val parts: List<UserPart>,
        val settings: TurnSettings?,
        val atMs: Long? = null,
    ) : AgentEvent
    /** Local: the backend accepted [clientMessageId] as turn [turnId] (Codex `turn/start` response). */
    data class TurnBound(override val backend: BackendKind, val threadId: String, val clientMessageId: String, val turnId: String) : AgentEvent
    data class TurnStarted(
        override val backend: BackendKind,
        val threadId: String,
        val turnId: String,
        val clientMessageId: String? = null,
        val atMs: Long? = null,
    ) : AgentEvent
    /** Local or backend: a queued turn was removed before it started. */
    data class TurnCancelled(override val backend: BackendKind, val threadId: String, val turnId: String) : AgentEvent
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
    data class TurnPlanUpdated(override val backend: BackendKind, val threadId: String, val turnId: String, val plan: TurnPlan) : AgentEvent
    data class TurnDiffUpdated(override val backend: BackendKind, val threadId: String, val turnId: String, val diff: String) : AgentEvent
    data class TurnProgress(override val backend: BackendKind, val threadId: String, val turnId: String, val thinkingTokens: Long?) : AgentEvent
    data class TurnNotice(override val backend: BackendKind, val threadId: String, val turnId: String?, val notice: Notice) : AgentEvent

    // ---- item ----
    data class ItemStarted(override val backend: BackendKind, val threadId: String, val turnId: String, val item: Item) : AgentEvent
    data class ItemUpdated(override val backend: BackendKind, val threadId: String, val turnId: String, val itemId: String, val delta: ItemDelta) : AgentEvent
    data class ItemCompleted(override val backend: BackendKind, val threadId: String, val turnId: String, val item: Item) : AgentEvent
    /** A tool call was refused (Claude `permission_denials`, denied `can_use_tool`). */
    data class ItemDeclined(override val backend: BackendKind, val threadId: String, val turnId: String?, val itemId: String) : AgentEvent

    // ---- server requests ----
    data class RequestOpened(val request: PendingRequest) : AgentEvent {
        override val backend: BackendKind get() = request.key.backend
    }
    data class RequestClosed(val key: RequestKey, val status: RequestStatus, val answer: String? = null) : AgentEvent {
        override val backend: BackendKind get() = key.backend
    }
}

data class QueuedMessage(val clientMessageId: String, val parts: List<UserPart>)
