package top.flysoftbeta.workflow.agent.model

import kotlinx.serialization.Serializable
import kotlinx.serialization.SerialName

import kotlinx.serialization.json.JsonElement

/** Backend thread identity: Codex `thread.id`, Claude `session_id`. */
@Serializable
@SerialName("ThreadKey")
data class ThreadKey(val backend: BackendKind, val id: String) {
    override fun toString(): String = "${backend.id}:$id"
}

@Serializable
sealed interface ProcessState {
    @Serializable
    @SerialName("Stopped")
    data object Stopped : ProcessState
    @Serializable
    @SerialName("Starting")
    data object Starting : ProcessState
    /** Handshake finished; requests may be sent. */
    @Serializable
    @SerialName("Ready")
    data object Ready : ProcessState
    @Serializable
    @SerialName("Exited")
    data class Exited(val exitCode: Int?, val stderrTail: String = "") : ProcessState
    @Serializable
    @SerialName("Failed")
    data class Failed(val message: String, val stderrTail: String = "") : ProcessState
}

@Serializable
@SerialName("McpServerStatus")
data class McpServerStatus(val name: String, val status: String, val error: String? = null)

/** Something the backend sent that this model does not interpret; kept for the generic console. */
@Serializable
@SerialName("UnknownRecord")
data class UnknownRecord(val kind: String, val threadId: String?, val raw: JsonElement)

@Serializable
@SerialName("BackendStatus")
data class BackendStatus(
    val backend: BackendKind,
    val process: ProcessState = ProcessState.Stopped,
    /** Codex initialize result / Claude initialize response (verbatim). */
    val serverInfo: JsonElement? = null,
    val account: AccountState = AccountState(),
    val rateLimits: RateLimitState? = null,
    val models: ModelCatalog? = null,
    val mcpServers: Map<String, McpServerStatus> = emptyMap(),
    /** Backend-wide notices (deprecation, config warnings, ...), newest last, bounded. */
    val notices: List<Notice> = emptyList(),
    /** Unknown notifications/messages, newest last, bounded. */
    val unknown: List<UnknownRecord> = emptyList(),
)

@Serializable
enum class RunState {
    NOT_LOADED,
    IDLE,
    RUNNING,
    WAITING_APPROVAL,
    WAITING_INPUT,
    ERROR,
    CLOSED,
}

/**
 * Effective thread settings as last reported by the backend. [raw] holds the verbatim payload
 * (Codex `threadSettings`, Claude `system/init`).
 */
@Serializable
@SerialName("ThreadSettings")
data class ThreadSettings(
    val model: String? = null,
    val effort: String? = null,
    /** Codex `approvalPolicy` (string or granular JSON as text) / Claude `permissionMode`. */
    val approvalPolicy: String? = null,
    val sandbox: String? = null,
    /** Codex `approvalsReviewer`. Anything but `user` is a safety violation the UI must flag. */
    val approvalsReviewer: String? = null,
    val raw: JsonElement? = null,
)

@Serializable
@SerialName("TokenUsage")
data class TokenUsage(
    val inputTokens: Long = 0,
    val cachedInputTokens: Long = 0,
    val outputTokens: Long = 0,
    val reasoningTokens: Long = 0,
    val totalTokens: Long = 0,
    val contextWindow: Long? = null,
    val costUsd: Double? = null,
    val raw: JsonElement? = null,
)

@Serializable
enum class TurnStatus {
    /** Submitted locally, not yet started by the backend. */
    QUEUED,
    RUNNING,
    COMPLETED,
    INTERRUPTED,
    FAILED,
    /** Removed from the queue before it started. */
    CANCELLED,
    ;

    val isFinal: Boolean get() = this != QUEUED && this != RUNNING
}

@Serializable
@SerialName("TurnError")
data class TurnError(val message: String, val code: String? = null, val detail: String? = null, val raw: JsonElement? = null)

@Serializable
@SerialName("TurnPlan")
data class TurnPlan(val steps: List<PlanStep>, val explanation: String? = null)

@Serializable
@SerialName("Turn")
data class Turn(
    /** Backend turn id (Codex `turn.id`); for Claude and for not-yet-bound local turns the client message id. */
    val id: String,
    val clientMessageId: String? = null,
    val status: TurnStatus = TurnStatus.RUNNING,
    val items: List<Item> = emptyList(),
    val error: TurnError? = null,
    val plan: TurnPlan? = null,
    /** Aggregated unified diff for the turn (Codex `turn/diff/updated`). */
    val diff: String? = null,
    val usage: TokenUsage? = null,
    /** Model/effort requested for this turn. */
    val settings: TurnSettings? = null,
    val startedAtMs: Long? = null,
    val completedAtMs: Long? = null,
    val durationMs: Long? = null,
    /** Claude `system/thinking_tokens` progress. */
    val thinkingTokens: Long? = null,
    /** False while the turn was created from an echo but not bound to a backend turn id yet. */
    val bound: Boolean = true,
) {
    fun item(id: String): Item? = items.firstOrNull { it.id == id }
    val finalMessage: AgentMessageItem?
        get() = items.lastOrNull { it is AgentMessageItem && it.phase == MessagePhase.FINAL } as? AgentMessageItem
}

@Serializable
@SerialName("ThreadState")
data class ThreadState(
    val key: ThreadKey,
    val title: String? = null,
    val preview: String? = null,
    val cwd: String? = null,
    /** Backend transcript/rollout path (Codex `thread.path`, Claude transcript file). */
    val path: String? = null,
    val forkedFrom: String? = null,
    val ephemeral: Boolean = false,
    val archived: Boolean = false,
    val deleted: Boolean = false,
    val runState: RunState = RunState.IDLE,
    val settings: ThreadSettings = ThreadSettings(),
    val turns: List<Turn> = emptyList(),
    val usage: TokenUsage? = null,
    val notices: List<Notice> = emptyList(),
    val unknown: List<UnknownRecord> = emptyList(),
    /** Opaque cursor for loading older turns (Codex `backwardsCursor`). */
    val historyCursor: String? = null,
    val createdAtSec: Long? = null,
    val updatedAtSec: Long? = null,
    val raw: JsonElement? = null,
) {
    fun turn(id: String): Turn? = turns.firstOrNull { it.id == id }
    val activeTurn: Turn? get() = turns.lastOrNull { it.status == TurnStatus.RUNNING }
    val queuedTurns: List<Turn> get() = turns.filter { it.status == TurnStatus.QUEUED }
}

@Serializable
@SerialName("AgentState")
data class AgentState(
    val backends: Map<BackendKind, BackendStatus> = emptyMap(),
    val threads: Map<ThreadKey, ThreadState> = emptyMap(),
    /** All server requests, in arrival order. Open ones have status PENDING. */
    val requests: Map<RequestKey, PendingRequest> = emptyMap(),
) {
    fun backend(kind: BackendKind): BackendStatus = backends[kind] ?: BackendStatus(kind)
    fun thread(key: ThreadKey): ThreadState? = threads[key]
    fun requestsFor(key: ThreadKey): List<PendingRequest> =
        requests.values.filter { it.key.backend == key.backend && it.threadId == key.id }
    val openRequests: List<PendingRequest> get() = requests.values.filter { it.status.isOpen }
}
