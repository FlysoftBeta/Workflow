package top.flysoftbeta.workflow.agent.codex

import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import top.flysoftbeta.workflow.agent.codex.CodexProtocol.ServerNotification as N
import top.flysoftbeta.workflow.agent.codex.CodexProtocol.ServerRequest as R
import top.flysoftbeta.workflow.agent.json.arr
import top.flysoftbeta.workflow.agent.json.bool
import top.flysoftbeta.workflow.agent.json.double
import top.flysoftbeta.workflow.agent.json.encode
import top.flysoftbeta.workflow.agent.json.get
import top.flysoftbeta.workflow.agent.json.int
import top.flysoftbeta.workflow.agent.json.isNullish
import top.flysoftbeta.workflow.agent.json.long
import top.flysoftbeta.workflow.agent.json.obj
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.json.strings
import top.flysoftbeta.workflow.agent.model.AccountState
import top.flysoftbeta.workflow.agent.model.AgentEvent
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.CommandAction
import top.flysoftbeta.workflow.agent.model.Decision
import top.flysoftbeta.workflow.agent.model.DecisionKind
import top.flysoftbeta.workflow.agent.model.EffortOption
import top.flysoftbeta.workflow.agent.model.ItemDelta
import top.flysoftbeta.workflow.agent.model.LoginFlow
import top.flysoftbeta.workflow.agent.model.LoginState
import top.flysoftbeta.workflow.agent.model.MarkerItem
import top.flysoftbeta.workflow.agent.model.MarkerKind
import top.flysoftbeta.workflow.agent.model.McpServerStatus
import top.flysoftbeta.workflow.agent.model.ModelCatalog
import top.flysoftbeta.workflow.agent.model.ModelOption
import top.flysoftbeta.workflow.agent.model.Notice
import top.flysoftbeta.workflow.agent.model.NoticeLevel
import top.flysoftbeta.workflow.agent.model.PendingRequest
import top.flysoftbeta.workflow.agent.model.Question
import top.flysoftbeta.workflow.agent.model.QuestionOption
import top.flysoftbeta.workflow.agent.model.RateLimit
import top.flysoftbeta.workflow.agent.model.RateLimitState
import top.flysoftbeta.workflow.agent.model.RateLimitWindow
import top.flysoftbeta.workflow.agent.model.RequestKey
import top.flysoftbeta.workflow.agent.model.RequestKind
import top.flysoftbeta.workflow.agent.model.RequestStatus
import top.flysoftbeta.workflow.agent.model.RunState
import top.flysoftbeta.workflow.agent.model.ThreadSettings
import top.flysoftbeta.workflow.agent.model.TokenUsage
import top.flysoftbeta.workflow.agent.model.TurnPlan

/** What to do with a server request. Only [Ask] reaches the user; nothing here ever approves. */
sealed interface CodexDisposition {
    data class Ask(val request: PendingRequest) : CodexDisposition
    /** Host capability with a harmless factual answer (e.g. current time). */
    data class AutoResult(val record: PendingRequest, val result: JsonElement) : CodexDisposition
    /** Unsupported/unknown: answer with a JSON-RPC error. Recorded as REJECTED. */
    data class AutoError(val record: PendingRequest, val code: Long, val message: String) : CodexDisposition
}

/** Stateless mapping of Codex frames to [AgentEvent]s. */
object CodexEvents {
    private val B = BackendKind.CODEX

    /** Notifications mapped to neutral events (see CodexCoverage). */
    val MODELED_NOTIFICATIONS: Set<String> = setOf(
        N.ERROR, N.THREAD_STARTED, N.THREAD_STATUS_CHANGED, N.THREAD_ARCHIVED, N.THREAD_DELETED, N.THREAD_UNARCHIVED,
        N.THREAD_CLOSED, N.THREAD_REVERTED, N.THREAD_NAME_UPDATED, N.THREAD_SETTINGS_UPDATED, N.THREAD_TOKEN_USAGE_UPDATED,
        N.TURN_STARTED, N.HOOK_STARTED, N.TURN_COMPLETED, N.HOOK_COMPLETED, N.TURN_DIFF_UPDATED, N.TURN_PLAN_UPDATED,
        N.ITEM_STARTED, N.ITEM_AUTO_APPROVAL_REVIEW_STARTED, N.ITEM_AUTO_APPROVAL_REVIEW_COMPLETED,
        N.AUTO_APPROVAL_REVIEW_STRICT_REVIEW_REQUIRED, N.ITEM_COMPLETED, N.ITEM_AGENT_MESSAGE_DELTA, N.ITEM_PLAN_DELTA,
        N.ITEM_COMMAND_EXECUTION_OUTPUT_DELTA, N.ITEM_COMMAND_EXECUTION_TERMINAL_INTERACTION, N.ITEM_FILE_CHANGE_OUTPUT_DELTA,
        N.ITEM_FILE_CHANGE_PATCH_UPDATED, N.SERVER_REQUEST_RESOLVED, N.ITEM_MCP_TOOL_CALL_PROGRESS,
        N.MCP_SERVER_OAUTH_LOGIN_COMPLETED, N.MCP_SERVER_STARTUP_STATUS_UPDATED, N.ACCOUNT_RATE_LIMITS_UPDATED,
        N.ITEM_REASONING_SUMMARY_TEXT_DELTA, N.ITEM_REASONING_SUMMARY_PART_ADDED, N.ITEM_REASONING_TEXT_DELTA,
        N.THREAD_COMPACTED, N.MODEL_REROUTED, N.MODEL_VERIFICATION, N.MODEL_PROVIDER_AUTH_RECOVERY_STARTED,
        N.MODEL_PROVIDER_AUTH_RECOVERY_COMPLETED, N.MODEL_SAFETY_BUFFERING_UPDATED, N.WARNING, N.GUARDIAN_WARNING,
        N.DEPRECATION_NOTICE, N.CONFIG_WARNING, N.ACCOUNT_LOGIN_COMPLETED,
    )

    /** Notifications the backend reacts to by re-reading state (no direct event). */
    val REFRESH_NOTIFICATIONS: Set<String> = setOf(N.ACCOUNT_UPDATED, N.THREAD_QUEUE_CHANGED)

    fun notification(method: String, params: JsonElement?, raw: JsonObject): List<AgentEvent> {
        val p = params.obj ?: JsonObject(emptyMap())
        val thread = p["threadId"].str
        val turn = p["turnId"].str
        fun unknown() = listOf(AgentEvent.Unknown(B, method, thread, raw))
        fun needThread(block: (String) -> List<AgentEvent>): List<AgentEvent> = thread?.let(block) ?: unknown()
        fun needTurn(block: (String, String) -> List<AgentEvent>): List<AgentEvent> =
            if (thread != null && turn != null) block(thread, turn) else unknown()
        fun delta(d: ItemDelta): List<AgentEvent> = needTurn { t, u ->
            val item = p["itemId"].str ?: return@needTurn unknown()
            listOf(AgentEvent.ItemUpdated(B, t, u, item, d))
        }

        return when (method) {
            N.ERROR -> needThread { t ->
                val error = CodexItems.turnError(p["error"])
                val willRetry = p["willRetry"].bool == true
                listOf(AgentEvent.TurnNotice(B, t, turn, Notice(
                    level = if (willRetry) NoticeLevel.WARNING else NoticeLevel.ERROR,
                    message = error?.message ?: "error", code = error?.code, willRetry = willRetry, detail = error?.detail, raw = p,
                )))
            }
            N.THREAD_STARTED -> p["thread"].obj?.let { listOf(threadUpserted(it)) } ?: unknown()
            N.THREAD_STATUS_CHANGED -> needThread { t -> listOf(AgentEvent.ThreadStatusChanged(B, t, runState(p["status"]))) }
            N.THREAD_ARCHIVED -> needThread { t -> listOf(AgentEvent.ThreadArchived(B, t, true)) }
            N.THREAD_UNARCHIVED -> needThread { t -> listOf(AgentEvent.ThreadArchived(B, t, false)) }
            N.THREAD_DELETED -> needThread { t -> listOf(AgentEvent.ThreadDeleted(B, t)) }
            N.THREAD_CLOSED -> needThread { t -> listOf(AgentEvent.ThreadClosed(B, t)) }
            N.THREAD_REVERTED -> needThread { t -> listOf(AgentEvent.ThreadNotice(B, t, Notice(NoticeLevel.INFO, "Thread reverted", code = "reverted", raw = p))) }
            N.THREAD_NAME_UPDATED -> needThread { t -> listOf(AgentEvent.ThreadRenamed(B, t, p["threadName"].str)) }
            N.THREAD_SETTINGS_UPDATED -> needThread { t -> listOf(AgentEvent.ThreadSettingsChanged(B, t, settings(p["threadSettings"]))) }
            N.THREAD_TOKEN_USAGE_UPDATED -> needThread { t -> listOf(AgentEvent.TokenUsageChanged(B, t, turn, tokenUsage(p["tokenUsage"]))) }
            N.TURN_STARTED -> needThread { t ->
                val turnJson = p["turn"]
                val id = turnJson["id"].str ?: return@needThread unknown()
                listOf(AgentEvent.TurnStarted(B, t, id, atMs = turnJson["startedAt"].long?.times(1000)))
            }
            N.TURN_COMPLETED -> needThread { t ->
                val turnJson = p["turn"]
                val id = turnJson["id"].str ?: return@needThread unknown()
                listOf(AgentEvent.TurnCompleted(
                    backend = B, threadId = t, turnId = id,
                    status = CodexItems.turnStatus(turnJson["status"].str),
                    error = CodexItems.turnError(turnJson["error"]),
                    items = turnJson["items"].arr?.map { CodexItems.parse(it, completed = true) } ?: emptyList(),
                    durationMs = turnJson["durationMs"].long,
                    atMs = turnJson["completedAt"].long?.times(1000),
                ))
            }
            N.HOOK_STARTED, N.HOOK_COMPLETED -> needThread { t ->
                val run = p["run"]
                val text = listOfNotNull(run["eventName"].str, run["status"].str, run["statusMessage"].str).joinToString(" · ")
                val item = MarkerItem("hook:${run["id"].str ?: "?"}", MarkerKind.HOOK, text, raw = p)
                if (turn != null) listOf(if (method == N.HOOK_STARTED) AgentEvent.ItemStarted(B, t, turn, item) else AgentEvent.ItemCompleted(B, t, turn, item))
                else listOf(AgentEvent.ThreadNotice(B, t, Notice(NoticeLevel.INFO, "Hook: $text", code = "hook", raw = p)))
            }
            N.TURN_DIFF_UPDATED -> needTurn { t, u -> listOf(AgentEvent.TurnDiffUpdated(B, t, u, p["diff"].str ?: "")) }
            N.TURN_PLAN_UPDATED -> needTurn { t, u ->
                listOf(AgentEvent.TurnPlanUpdated(B, t, u, TurnPlan(CodexItems.planSteps(p["plan"]), p["explanation"].str)))
            }
            N.ITEM_STARTED -> needTurn { t, u -> p["item"]?.let { listOf(AgentEvent.ItemStarted(B, t, u, CodexItems.parse(it, completed = false))) } ?: unknown() }
            N.ITEM_COMPLETED -> needTurn { t, u -> p["item"]?.let { listOf(AgentEvent.ItemCompleted(B, t, u, CodexItems.parse(it, completed = true))) } ?: unknown() }
            N.ITEM_AUTO_APPROVAL_REVIEW_STARTED, N.ITEM_AUTO_APPROVAL_REVIEW_COMPLETED, N.AUTO_APPROVAL_REVIEW_STRICT_REVIEW_REQUIRED ->
                // Must not happen with approvalsReviewer=user; surfaced loudly if it does.
                listOf(if (thread != null) AgentEvent.TurnNotice(B, thread, turn, Notice(NoticeLevel.ERROR, "Approval was routed to automatic review", code = method, raw = p))
                else AgentEvent.BackendNotice(B, Notice(NoticeLevel.ERROR, "Approval was routed to automatic review", code = method, raw = p)))
            N.ITEM_AGENT_MESSAGE_DELTA -> delta(ItemDelta.AgentText(p["delta"].str ?: ""))
            N.ITEM_PLAN_DELTA -> delta(ItemDelta.PlanText(p["delta"].str ?: ""))
            N.ITEM_COMMAND_EXECUTION_OUTPUT_DELTA -> delta(ItemDelta.CommandOutput(p["delta"].str ?: ""))
            N.ITEM_COMMAND_EXECUTION_TERMINAL_INTERACTION -> delta(ItemDelta.TerminalInput(p["stdin"].str ?: ""))
            N.ITEM_FILE_CHANGE_OUTPUT_DELTA -> delta(ItemDelta.FileChangeOutput(p["delta"].str ?: ""))
            N.ITEM_FILE_CHANGE_PATCH_UPDATED -> delta(ItemDelta.FileChangePatch(CodexItems.changes(p["changes"])))
            N.ITEM_MCP_TOOL_CALL_PROGRESS -> delta(ItemDelta.ToolProgress(p["message"].str ?: ""))
            N.ITEM_REASONING_SUMMARY_TEXT_DELTA -> delta(ItemDelta.ReasoningSummary(p["summaryIndex"].int ?: 0, p["delta"].str ?: ""))
            N.ITEM_REASONING_SUMMARY_PART_ADDED -> delta(ItemDelta.ReasoningSummaryPart(p["summaryIndex"].int ?: 0))
            N.ITEM_REASONING_TEXT_DELTA -> delta(ItemDelta.ReasoningText(p["contentIndex"].int ?: 0, p["delta"].str ?: ""))
            N.SERVER_REQUEST_RESOLVED -> p["requestId"]?.takeUnless { it.isNullish }?.let {
                listOf(AgentEvent.RequestClosed(RequestKey(B, it), RequestStatus.RESOLVED))
            } ?: unknown()
            N.MCP_SERVER_OAUTH_LOGIN_COMPLETED -> listOf(AgentEvent.BackendNotice(B, Notice(
                if (p["success"].bool == false) NoticeLevel.WARNING else NoticeLevel.INFO,
                "MCP login ${if (p["success"].bool == false) "failed" else "completed"}: ${p["name"].str ?: ""}".trim(),
                code = method, detail = p["error"].str, raw = p,
            )))
            N.MCP_SERVER_STARTUP_STATUS_UPDATED -> listOf(AgentEvent.McpServerChanged(B, McpServerStatus(p["name"].str ?: "", p["status"].str ?: "", p["error"].str)))
            N.ACCOUNT_RATE_LIMITS_UPDATED -> listOf(AgentEvent.RateLimitsChanged(B, RateLimitState(limits = listOfNotNull(rateLimit(p["rateLimits"]))), merge = true))
            N.THREAD_COMPACTED -> needThread { t -> listOf(AgentEvent.ThreadNotice(B, t, Notice(NoticeLevel.INFO, "Context compacted", code = "compacted", raw = p))) }
            N.MODEL_REROUTED -> needThread { t ->
                listOf(AgentEvent.TurnNotice(B, t, turn, Notice(NoticeLevel.WARNING, "Model rerouted: ${p["fromModel"].str} → ${p["toModel"].str}", code = p["reason"].text(), raw = p)))
            }
            N.MODEL_VERIFICATION -> needThread { t -> listOf(AgentEvent.TurnNotice(B, t, turn, Notice(NoticeLevel.INFO, "Model verification", code = method, raw = p))) }
            N.MODEL_PROVIDER_AUTH_RECOVERY_STARTED, N.MODEL_PROVIDER_AUTH_RECOVERY_COMPLETED ->
                listOf(AgentEvent.BackendNotice(B, Notice(NoticeLevel.INFO, if (method == N.MODEL_PROVIDER_AUTH_RECOVERY_STARTED) "Refreshing sign-in" else "Sign-in refreshed", code = method, raw = p)))
            N.MODEL_SAFETY_BUFFERING_UPDATED -> needThread { t ->
                if (p["showBufferingUi"].bool == true) listOf(AgentEvent.TurnNotice(B, t, turn, Notice(NoticeLevel.INFO, "Safety check in progress", code = method, detail = p["reasons"].strings().joinToString(), raw = p)))
                else emptyList()
            }
            N.WARNING -> listOf(thread?.let { AgentEvent.ThreadNotice(B, it, Notice(NoticeLevel.WARNING, p["message"].str ?: "", code = method, raw = p)) }
                ?: AgentEvent.BackendNotice(B, Notice(NoticeLevel.WARNING, p["message"].str ?: "", code = method, raw = p)))
            N.GUARDIAN_WARNING -> needThread { t -> listOf(AgentEvent.ThreadNotice(B, t, Notice(NoticeLevel.WARNING, p["message"].str ?: "", code = method, raw = p))) }
            N.DEPRECATION_NOTICE -> listOf(AgentEvent.BackendNotice(B, Notice(NoticeLevel.INFO, p["summary"].str ?: "", code = method, detail = p["details"].str, raw = p)))
            N.CONFIG_WARNING -> listOf(AgentEvent.BackendNotice(B, Notice(NoticeLevel.WARNING, p["summary"].str ?: "", code = method, detail = listOfNotNull(p["path"].str, p["details"].str).joinToString("\n").ifEmpty { null }, raw = p)))
            N.ACCOUNT_LOGIN_COMPLETED -> listOf(AgentEvent.LoginChanged(B, LoginFlow.Completed(p["loginId"].str, p["success"].bool == true, p["error"].str)))
            in REFRESH_NOTIFICATIONS -> emptyList() // the backend re-reads the account
            else -> unknown()
        }
    }

    private fun JsonElement?.text(): String? = (this as? JsonPrimitive)?.content ?: this?.encode()

    fun runState(status: JsonElement?): RunState = when (status["type"].str) {
        "idle" -> RunState.IDLE
        "notLoaded" -> RunState.NOT_LOADED
        "systemError" -> RunState.ERROR
        "active" -> {
            val flags = status["activeFlags"].strings()
            when {
                "waitingOnApproval" in flags -> RunState.WAITING_APPROVAL
                "waitingOnUserInput" in flags -> RunState.WAITING_INPUT
                else -> RunState.RUNNING
            }
        }
        else -> RunState.IDLE
    }

    fun settings(json: JsonElement?): ThreadSettings = ThreadSettings(
        model = json["model"].str,
        effort = json["effort"].str ?: json["reasoningEffort"].str,
        approvalPolicy = CodexItems.policyText(json["approvalPolicy"]),
        sandbox = (json["sandboxPolicy"] ?: json["sandbox"]).let { it["type"].str ?: it.str },
        approvalsReviewer = json["approvalsReviewer"].str,
        raw = json,
    )

    /** `Thread` object (thread/started, start/resume/fork result, thread/list) → upsert. */
    fun threadUpserted(thread: JsonObject, response: JsonObject? = null): AgentEvent.ThreadUpserted = AgentEvent.ThreadUpserted(
        backend = B,
        threadId = thread["id"].str ?: "",
        title = thread["name"].str,
        preview = thread["preview"].str,
        cwd = thread["cwd"].str,
        path = thread["path"].str,
        forkedFrom = thread["forkedFromId"].str,
        ephemeral = thread["ephemeral"].bool,
        runState = thread["status"]?.let { runState(it) },
        settings = response?.let { settings(it) } ?: ThreadSettings(model = thread["model"].str, effort = thread["reasoningEffort"].str),
        createdAtSec = thread["createdAt"].long,
        updatedAtSec = thread["updatedAt"].long,
        raw = thread,
    )

    fun tokenUsage(json: JsonElement?): TokenUsage {
        val total = json["total"]
        return TokenUsage(
            inputTokens = total["inputTokens"].long ?: 0,
            cachedInputTokens = total["cachedInputTokens"].long ?: 0,
            outputTokens = total["outputTokens"].long ?: 0,
            reasoningTokens = total["reasoningOutputTokens"].long ?: 0,
            totalTokens = total["totalTokens"].long ?: 0,
            contextWindow = json["modelContextWindow"].long,
            raw = json,
        )
    }

    private fun window(json: JsonElement?): RateLimitWindow? = json.obj?.let {
        RateLimitWindow(it["usedPercent"].double, it["windowDurationMins"].long, it["resetsAt"].long)
    }

    fun rateLimit(json: JsonElement?): RateLimit? {
        val o = json.obj ?: return null
        return RateLimit(
            id = o["limitId"].str ?: "codex",
            name = o["limitName"].str,
            primary = window(o["primary"]),
            secondary = window(o["secondary"]),
            status = o["rateLimitReachedType"].str,
            reached = !o["rateLimitReachedType"].isNullish || (window(o["primary"])?.usedPercent ?: 0.0) >= 100.0,
            raw = o,
        )
    }

    /** `account/rateLimits/read` result. */
    fun rateLimits(result: JsonElement): RateLimitState {
        val byId = result["rateLimitsByLimitId"].obj
        val limits = byId?.values?.mapNotNull { rateLimit(it) } ?: listOfNotNull(rateLimit(result["rateLimits"]))
        return RateLimitState(limits, result["ordinaryUsageAllowed"].bool, result["rateLimitUpsell"]?.takeUnless { it.isNullish }, result)
    }

    /** `account/read` result. */
    fun account(result: JsonElement): AccountState {
        val account = result["account"]
        return AccountState(
            state = if (account.isNullish) LoginState.LOGGED_OUT else LoginState.LOGGED_IN,
            method = account["type"].str,
            email = account["email"].str,
            plan = account["planType"].str,
            requiresAuth = result["requiresOpenaiAuth"].bool,
            raw = result,
        )
    }

    /** `account/login/start` result. */
    fun loginFlow(result: JsonElement): LoginFlow? = when (result["type"].str) {
        "chatgptDeviceCode" -> LoginFlow.DeviceCode(result["loginId"].str, result["verificationUrl"].str ?: "", result["userCode"].str ?: "")
        "chatgpt" -> LoginFlow.Browser(result["loginId"].str, result["authUrl"].str ?: "")
        "apiKey", "chatgptAuthTokens", "amazonBedrock" -> LoginFlow.Completed(null, true, null)
        else -> null
    }

    /** `model/list` result (all pages concatenated by the caller). */
    fun models(pages: List<JsonElement>): ModelCatalog = ModelCatalog(B, pages.flatMap { page ->
        page["data"].arr?.map { m ->
            ModelOption(
                id = m["id"].str ?: m["model"].str ?: "",
                displayName = m["displayName"].str ?: m["id"].str ?: "",
                description = m["description"].str,
                efforts = m["supportedReasoningEfforts"].arr?.map { EffortOption(it["reasoningEffort"].str ?: "", it["description"].str) } ?: emptyList(),
                defaultEffort = m["defaultReasoningEffort"].str,
                isDefault = m["isDefault"].bool == true,
                hidden = m["hidden"].bool == true,
                inputModalities = m["inputModalities"].strings().toSet(),
                upgradeTo = m["upgrade"].str,
                upgradeMessage = m["upgradeInfo"]["migrationMarkdown"].str,
                raw = m,
            )
        } ?: emptyList()
    })

    // ------------------------------------------------------------------ server requests

    /** Maps a server request. Approvals and questions become cards; nothing is answered with consent here. */
    fun request(id: JsonElement, method: String, params: JsonElement?, raw: JsonObject, nowMs: Long): CodexDisposition {
        val p = params.obj ?: JsonObject(emptyMap())
        val key = RequestKey(B, id)
        fun card(kind: RequestKind, decisions: List<Decision>, itemId: String? = p["itemId"].str) = CodexDisposition.Ask(PendingRequest(
            key = key, method = method, kind = kind, decisions = decisions,
            threadId = p["threadId"].str ?: p["conversationId"].str, turnId = p["turnId"].str, itemId = itemId,
            receivedAtMs = nowMs, raw = raw,
        ))
        fun record(kind: RequestKind, status: RequestStatus) = PendingRequest(
            key = key, method = method, kind = kind, decisions = emptyList(),
            threadId = p["threadId"].str, turnId = p["turnId"].str, status = status, receivedAtMs = nowMs, raw = raw,
        )
        return when (method) {
            R.ITEM_COMMAND_EXECUTION_REQUEST_APPROVAL -> card(
                RequestKind.CommandApproval(
                    command = p["command"].str,
                    cwd = p["cwd"].str,
                    reason = p["reason"].str,
                    actions = p["commandActions"].arr?.map { CommandAction(it["type"].str ?: "unknown", it["command"].str ?: "", it["path"].str, it["query"].str) } ?: emptyList(),
                    networkHost = p["networkApprovalContext"]["host"].str,
                ),
                commandDecisions(p["availableDecisions"]),
            )
            R.ITEM_FILE_CHANGE_REQUEST_APPROVAL -> card(
                RequestKind.FileChangeApproval(p["reason"].str, p["grantRoot"].str),
                listOf(
                    Decision("accept", DecisionKind.ALLOW_ONCE, wire = JsonPrimitive("accept")),
                    Decision("acceptForSession", DecisionKind.ALLOW_SESSION, wire = JsonPrimitive("acceptForSession")),
                    Decision("decline", DecisionKind.DENY, wire = JsonPrimitive("decline")),
                    Decision("cancel", DecisionKind.ABORT, wire = JsonPrimitive("cancel")),
                ),
            )
            R.ITEM_PERMISSIONS_REQUEST_APPROVAL -> {
                val requested = p["permissions"] ?: JsonObject(emptyMap())
                card(
                    RequestKind.PermissionsApproval(p["reason"].str, p["cwd"].str, requested),
                    listOf(
                        Decision("grantTurn", DecisionKind.ALLOW_ONCE, wire = buildJsonObject { put("permissions", requested); put("scope", "turn") }),
                        Decision("grantSession", DecisionKind.ALLOW_SESSION, wire = buildJsonObject { put("permissions", requested); put("scope", "session") }),
                        Decision("decline", DecisionKind.DENY, wire = buildJsonObject { put("permissions", JsonObject(emptyMap())); put("scope", "turn") }),
                    ),
                )
            }
            R.ITEM_TOOL_REQUEST_USER_INPUT -> card(
                RequestKind.UserInput(
                    questions = p["questions"].arr?.map { q ->
                        Question(
                            id = q["id"].str ?: "",
                            question = q["question"].str ?: "",
                            header = q["header"].str,
                            options = q["options"].arr?.map { QuestionOption(it["label"].str ?: "", it["description"].str) } ?: emptyList(),
                            allowFreeText = q["isOther"].bool == true || q["options"].arr.isNullOrEmpty(),
                            secret = q["isSecret"].bool == true,
                        )
                    } ?: emptyList(),
                    autoResolutionMs = p["autoResolutionMs"].long,
                ),
                emptyList(),
            )
            R.MCP_SERVER_ELICITATION_REQUEST -> card(
                RequestKind.Elicitation(
                    server = p["serverName"].str ?: p["_meta"]["serverName"].str,
                    message = p["message"].str ?: p["description"].str ?: p["title"].str ?: "",
                    mode = p["mode"].str ?: "form",
                    url = p["url"].str,
                    schema = p["requestedSchema"],
                    elicitationId = p["elicitationId"].str,
                ),
                listOf(
                    Decision("accept", DecisionKind.ALLOW_ONCE, wire = JsonPrimitive("accept")),
                    Decision("decline", DecisionKind.DENY, wire = JsonPrimitive("decline")),
                    Decision("cancel", DecisionKind.ABORT, wire = JsonPrimitive("cancel")),
                ),
                itemId = null,
            )
            R.EXEC_COMMAND_APPROVAL -> card(
                RequestKind.CommandApproval(p["command"].strings().joinToString(" "), p["cwd"].str, p["reason"].str),
                legacyDecisions(),
                itemId = p["callId"].str,
            )
            R.APPLY_PATCH_APPROVAL -> card(RequestKind.FileChangeApproval(p["reason"].str, p["grantRoot"].str), legacyDecisions(), itemId = p["callId"].str)
            R.CURRENT_TIME_READ -> CodexDisposition.AutoResult(
                record(RequestKind.Unknown(method, params), RequestStatus.ANSWERED),
                buildJsonObject { put("currentTimeAt", nowMs / 1000) },
            )
            R.ITEM_TOOL_CALL -> CodexDisposition.AutoResult(
                record(RequestKind.Unknown(method, params), RequestStatus.REJECTED),
                buildJsonObject {
                    put("success", false)
                    put("contentItems", JsonArray(listOf(buildJsonObject { put("type", "inputText"); put("text", "Workflow registers no dynamic tools.") })))
                },
            )
            R.ACCOUNT_CHATGPT_AUTH_TOKENS_REFRESH, R.ATTESTATION_GENERATE -> CodexDisposition.AutoError(
                record(RequestKind.Unknown(method, params), RequestStatus.REJECTED), -32601, "$method is not supported by this client",
            )
            else -> CodexDisposition.AutoError(record(RequestKind.Unknown(method, params), RequestStatus.REJECTED), -32601, "method not found: $method")
        }
    }

    private fun legacyDecisions() = listOf(
        Decision("approved", DecisionKind.ALLOW_ONCE, wire = JsonPrimitive("approved")),
        Decision("approved_for_session", DecisionKind.ALLOW_SESSION, wire = JsonPrimitive("approved_for_session")),
        Decision("denied", DecisionKind.DENY, wire = buildJsonObject { put("denied", buildJsonObject { put("rejection", "Declined by user") }) }),
        Decision("abort", DecisionKind.ABORT, wire = JsonPrimitive("abort")),
    )

    /** `availableDecisions` → buttons, in server order. Absent list → one-shot accept/decline/cancel only. */
    fun commandDecisions(available: JsonElement?): List<Decision> {
        val list = available.arr ?: return listOf(
            Decision("accept", DecisionKind.ALLOW_ONCE, wire = JsonPrimitive("accept")),
            Decision("decline", DecisionKind.DENY, wire = JsonPrimitive("decline")),
            Decision("cancel", DecisionKind.ABORT, wire = JsonPrimitive("cancel")),
        )
        val used = HashMap<String, Int>()
        return list.map { wire ->
            val base: Decision = when {
                wire.str == "accept" -> Decision("accept", DecisionKind.ALLOW_ONCE, wire = wire)
                wire.str == "acceptForSession" -> Decision("acceptForSession", DecisionKind.ALLOW_SESSION, wire = wire)
                wire.str == "decline" -> Decision("decline", DecisionKind.DENY, wire = wire)
                wire.str == "cancel" -> Decision("cancel", DecisionKind.ABORT, wire = wire)
                wire.obj?.containsKey("acceptWithExecpolicyAmendment") == true -> Decision(
                    "acceptWithExecpolicyAmendment", DecisionKind.ALLOW_PERSISTENT,
                    detail = wire["acceptWithExecpolicyAmendment"]["execpolicy_amendment"].strings().joinToString(" "), wire = wire,
                )
                wire.obj?.containsKey("applyNetworkPolicyAmendment") == true -> {
                    val amendment = wire["applyNetworkPolicyAmendment"]["network_policy_amendment"]
                    val action = amendment["action"].str
                    Decision(
                        "applyNetworkPolicyAmendment", if (action == "deny") DecisionKind.DENY else DecisionKind.ALLOW_PERSISTENT,
                        detail = "${action ?: "allow"} ${amendment["host"].str ?: ""}".trim(), wire = wire,
                    )
                }
                else -> Decision(wire.str ?: wire.obj?.keys?.firstOrNull() ?: "other", DecisionKind.OTHER, detail = wire.encode(), wire = wire)
            }
            val n = used.merge(base.id, 1, Int::plus)!!
            if (n == 1) base else base.copy(id = "${base.id}#$n")
        }
    }

    /** The JSON-RPC result for a decision on [request]. */
    fun decisionResult(request: PendingRequest, decision: Decision, message: String?): JsonElement = when (request.method) {
        R.ITEM_PERMISSIONS_REQUEST_APPROVAL -> decision.wire ?: JsonNull
        R.MCP_SERVER_ELICITATION_REQUEST -> buildJsonObject { put("action", decision.wire ?: JsonPrimitive(decision.id)); put("content", JsonNull) }
        R.EXEC_COMMAND_APPROVAL, R.APPLY_PATCH_APPROVAL -> buildJsonObject {
            val wire = if (decision.id == "denied" && message != null) buildJsonObject { put("denied", buildJsonObject { put("rejection", message) }) } else decision.wire
            put("decision", wire ?: JsonNull)
        }
        else -> buildJsonObject { put("decision", decision.wire ?: JsonNull) }
    }
}
