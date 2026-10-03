package top.flysoftbeta.workflow.agent.model

import kotlinx.serialization.Serializable
import kotlinx.serialization.SerialName

import kotlinx.serialization.json.JsonElement
import top.flysoftbeta.workflow.agent.json.encode

/**
 * Identity of a request the *server* sent to us. [rawId] is the JSON id exactly as received
 * (Codex: int or string; Claude: `request_id` string) and is echoed back verbatim. Server request
 * ids share the numeric space with our own request ids, so they live in their own map.
 */
@Serializable
@SerialName("RequestKey")
data class RequestKey(val backend: BackendKind, val rawId: JsonElement) {
    /** Stable text form, e.g. `codex:0` or `claude:"c071…"`. */
    val text: String get() = "${backend.id}:${rawId.encode()}"
    override fun toString(): String = text
}

@Serializable
enum class DecisionKind {
    /** Allow this one action. */
    ALLOW_ONCE,
    /** Allow for the rest of this session/thread (not written to disk). */
    ALLOW_SESSION,
    /** Allow and remember (writes a rule, exec-policy amendment, settings file, ...). Needs distinct wording. */
    ALLOW_PERSISTENT,
    /** Refuse; the agent continues. */
    DENY,
    /** Refuse and stop the turn. */
    ABORT,
    OTHER,
}

/**
 * One button on a request card. Only decisions the server offered are listed. [wire] is the exact
 * value sent back (e.g. Codex `{"acceptWithExecpolicyAmendment":{…}}`); the adapter never
 * synthesises decisions the server did not allow.
 */
@Serializable
@SerialName("Decision")
data class Decision(
    val id: String,
    val kind: DecisionKind,
    /** Backend-provided detail for persistent grants (rule text, amendment argv, destination). */
    val detail: String? = null,
    val wire: JsonElement? = null,
) {
    val persistent: Boolean get() = kind == DecisionKind.ALLOW_PERSISTENT || kind == DecisionKind.ALLOW_SESSION
}

@Serializable
@SerialName("QuestionOption")
data class QuestionOption(val label: String, val description: String? = null, val preview: String? = null)

@Serializable
@SerialName("Question")
data class Question(
    /** Answer key (Codex question id; Claude question text). */
    val id: String,
    val question: String,
    val header: String? = null,
    val options: List<QuestionOption> = emptyList(),
    val multiSelect: Boolean = false,
    val allowFreeText: Boolean = false,
    val secret: Boolean = false,
)

@Serializable
sealed interface RequestKind {
    @Serializable
    @SerialName("CommandApproval")
    data class CommandApproval(
        val command: String?,
        val cwd: String?,
        val reason: String?,
        val actions: List<CommandAction> = emptyList(),
        /** Network host the command wants to reach, if the approval is about network access. */
        val networkHost: String? = null,
    ) : RequestKind

    @Serializable
    @SerialName("FileChangeApproval")
    data class FileChangeApproval(
        val reason: String?,
        /** Directory the agent wants write access to for the rest of the session (Codex `grantRoot`). */
        val grantRoot: String?,
        val changes: List<FileDelta> = emptyList(),
    ) : RequestKind

    /** Codex `item/permissions/requestApproval`: extra filesystem/network permissions. */
    @Serializable
    @SerialName("PermissionsApproval")
    data class PermissionsApproval(val reason: String?, val cwd: String?, val permissions: JsonElement) : RequestKind

    /** Claude `can_use_tool` for an ordinary tool. */
    @Serializable
    @SerialName("ToolApproval")
    data class ToolApproval(
        val tool: String,
        val displayName: String?,
        val title: String?,
        val description: String?,
        val input: JsonElement?,
        val blockedPath: String?,
        /** Sanitised `decision_reason` (ANSI removed). */
        val reason: String?,
        val reasonType: String?,
        /** Do not preselect approve. */
        val defaultToNo: Boolean,
        /** Tool approval card is itself the interaction surface; one-tap approve must not be offered. */
        val requiresUserInteraction: Boolean,
        val toolUseId: String?,
        val agentId: String?,
        val mcpServer: JsonElement?,
    ) : RequestKind

    /** Codex `item/tool/requestUserInput` / Claude `AskUserQuestion`. */
    @Serializable
    @SerialName("UserInput")
    data class UserInput(val questions: List<Question>, val autoResolutionMs: Long? = null) : RequestKind

    /** Claude `ExitPlanMode`: approve a markdown plan. */
    @Serializable
    @SerialName("PlanApproval")
    data class PlanApproval(val plan: String) : RequestKind

    /** MCP elicitation (form or URL). */
    @Serializable
    @SerialName("Elicitation")
    data class Elicitation(
        val server: String?,
        val message: String,
        val mode: String,
        val url: String? = null,
        val schema: JsonElement? = null,
        val elicitationId: String? = null,
    ) : RequestKind

    /** Claude `request_user_dialog` of a kind we did not declare: must not be answered. */
    @Serializable
    @SerialName("UserDialog")
    data class UserDialog(val dialogKind: String, val payload: JsonElement?) : RequestKind

    /** Any request method this model does not know. */
    @Serializable
    @SerialName("Unknown")
    data class Unknown(val method: String, val params: JsonElement?) : RequestKind
}

@Serializable
enum class RequestStatus {
    PENDING,
    /** The user answered; the answer was written to the process. */
    ANSWERED,
    /** The server reported it resolved (Codex `serverRequest/resolved`). */
    RESOLVED,
    /** The server withdrew it (Claude `control_cancel_request`). */
    CANCELLED,
    /** The turn ended or the process exited before an answer; the card stays visible but disabled. */
    EXPIRED,
    /** Answered automatically with a protocol error because the method/subtype is unknown or unsupported. */
    REJECTED,
    ;

    val isOpen: Boolean get() = this == PENDING
}

@Serializable
@SerialName("PendingRequest")
data class PendingRequest(
    val key: RequestKey,
    val method: String,
    val kind: RequestKind,
    val decisions: List<Decision>,
    val threadId: String? = null,
    val turnId: String? = null,
    val itemId: String? = null,
    val status: RequestStatus = RequestStatus.PENDING,
    /** Short human summary of the answer (e.g. decision id) once answered. */
    val answer: String? = null,
    val receivedAtMs: Long? = null,
    val raw: JsonElement? = null,
)
