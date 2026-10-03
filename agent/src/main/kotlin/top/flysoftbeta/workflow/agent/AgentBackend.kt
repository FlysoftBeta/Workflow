package top.flysoftbeta.workflow.agent

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.serialization.json.JsonElement
import top.flysoftbeta.workflow.agent.model.AgentEvent
import top.flysoftbeta.workflow.agent.model.AgentReducer
import top.flysoftbeta.workflow.agent.model.AgentState
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.LoginFlow
import top.flysoftbeta.workflow.agent.model.LoginMethod
import top.flysoftbeta.workflow.agent.model.ModelCatalog
import top.flysoftbeta.workflow.agent.model.PermissionPreset
import top.flysoftbeta.workflow.agent.model.RequestKey
import top.flysoftbeta.workflow.agent.model.RequestResponse
import top.flysoftbeta.workflow.agent.model.TurnSettings
import top.flysoftbeta.workflow.agent.model.UserPart

/** Receives normalised events. Must be cheap and non-blocking; [AgentStateStore] is the standard sink. */
fun interface AgentEventSink {
    fun emit(event: AgentEvent)
}

/**
 * Process-lifetime owner of [AgentState] (the app's `AgentHub` holds one). Events are folded
 * synchronously so no event is lost between a backend emitting it and the UI subscribing.
 */
class AgentStateStore(initial: AgentState = AgentState()) : AgentEventSink {
    private val mutable = MutableStateFlow(initial)
    val state: StateFlow<AgentState> = mutable.asStateFlow()
    private val listeners = java.util.concurrent.CopyOnWriteArrayList<(AgentEvent) -> Unit>()

    override fun emit(event: AgentEvent) {
        synchronized(this) { mutable.value = AgentReducer.reduce(mutable.value, event) }
        listeners.forEach { it(event) }
    }

    /** Observes every event after it is reduced (index maintenance, persistence). */
    fun addListener(listener: (AgentEvent) -> Unit) { listeners += listener }
}

data class ThreadOptions(
    /** Working directory inside the environment (default `/workspace`). */
    val cwd: String,
    val settings: TurnSettings = TurnSettings(),
    val ephemeral: Boolean = false,
    val title: String? = null,
)

enum class SendMode {
    /** Start a turn when idle, otherwise queue behind the running turn. */
    AUTO,
    START,
    /** Inject into the running turn (Codex `turn/steer`; Claude: priority `now`). */
    STEER,
    /** Run after the current turn (Codex `thread/queue/add`; Claude: queued user message). */
    QUEUE,
}

/** A thread known to the backend (Codex `thread/list`); used to import into the app's index. */
data class ThreadSummary(
    val id: String,
    val title: String?,
    val preview: String?,
    val cwd: String?,
    val createdAtSec: Long?,
    val updatedAtSec: Long?,
    val archived: Boolean,
)

/**
 * Backend-neutral control surface the chat UI talks to. State is observed through the
 * [AgentEventSink]/[AgentStateStore], never returned piecemeal. Every method is main-safe
 * (suspends; IO happens on the process transport).
 */
interface AgentBackend {
    val kind: BackendKind

    /** Spawns / connects the backend process and performs the handshake. Idempotent. */
    suspend fun start()
    suspend fun stop()

    // account
    suspend fun refreshAccount()
    suspend fun refreshRateLimits()
    suspend fun refreshModels(): ModelCatalog
    /** Login methods this backend supports, in preferred order. */
    val loginMethods: List<LoginMethod>
    /** [secret] only for API-key methods; it is passed to the backend and never stored or logged here. */
    suspend fun login(method: LoginMethod, secret: String? = null): LoginFlow
    suspend fun cancelLogin(loginId: String)
    suspend fun logout()

    // threads
    suspend fun startThread(options: ThreadOptions): String
    suspend fun resumeThread(threadId: String, options: ThreadOptions): String
    /** Branch [threadId]; [atTurnId] = keep history up to and including that turn (null = everything). */
    suspend fun forkThread(threadId: String, atTurnId: String?, options: ThreadOptions): String
    suspend fun loadHistory(threadId: String, olderThan: String? = null)
    /** Backend-side list (null when the backend has none, e.g. Claude: use the app index). */
    suspend fun listThreads(cwd: String?): List<ThreadSummary>?
    suspend fun rename(threadId: String, title: String)
    suspend fun archive(threadId: String, archived: Boolean)
    suspend fun delete(threadId: String)
    suspend fun compact(threadId: String)

    // turns
    /** Returns the client message id (turn correlation key). */
    suspend fun send(threadId: String, parts: List<UserPart>, settings: TurnSettings? = null, mode: SendMode = SendMode.AUTO): String
    suspend fun cancelQueued(threadId: String, clientMessageId: String)
    suspend fun interrupt(threadId: String, cancelQueued: Boolean = false)
    suspend fun setPermissions(threadId: String, preset: PermissionPreset)

    // server requests
    /** Answers an open request with the user's explicit choice. Validated against the offered decisions. */
    suspend fun respond(key: RequestKey, response: RequestResponse)

    /** Generic console: any protocol method (Codex method / Claude control subtype). Safety fields are still enforced. */
    suspend fun rawRequest(method: String, params: JsonElement?, threadId: String? = null): JsonElement
}

/** Unknown server request handling. Default rejects with a protocol error; ASK_USER shows a generic card. */
enum class UnknownRequestPolicy { REJECT, ASK_USER }
