package top.flysoftbeta.workflow.agent.claude

import java.util.Base64
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonArray
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import top.flysoftbeta.workflow.agent.json.arr
import top.flysoftbeta.workflow.agent.json.bool
import top.flysoftbeta.workflow.agent.json.encode
import top.flysoftbeta.workflow.agent.json.get
import top.flysoftbeta.workflow.agent.json.obj
import top.flysoftbeta.workflow.agent.json.sanitizeDisplayText
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.json.strings
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.Decision
import top.flysoftbeta.workflow.agent.model.DecisionKind
import top.flysoftbeta.workflow.agent.model.PendingRequest
import top.flysoftbeta.workflow.agent.model.Question
import top.flysoftbeta.workflow.agent.model.QuestionOption
import top.flysoftbeta.workflow.agent.model.RequestKey
import top.flysoftbeta.workflow.agent.model.RequestKind
import top.flysoftbeta.workflow.agent.model.RequestResponse
import top.flysoftbeta.workflow.agent.model.RequestStatus
import top.flysoftbeta.workflow.agent.model.UserPart

/** Reads attachment bytes by environment path (the app maps `/workspace/...` to its files). */
fun interface AttachmentReader {
    suspend fun read(path: String, maxBytes: Long): ByteArray
}

/** Claude control-request ↔ neutral request mapping and stream-json message builders (pure). */
object ClaudeRequests {
    private val B = BackendKind.CLAUDE

    fun key(requestId: String) = RequestKey(B, JsonPrimitive(requestId))

    /** `can_use_tool` / `elicitation` / `request_user_dialog` → a card. Never auto-answered. */
    fun pending(requestId: String, subtype: String, request: JsonObject, raw: JsonObject, threadId: String?, turnId: String?, nowMs: Long): PendingRequest? {
        val base = PendingRequest(key(requestId), subtype, RequestKind.Unknown(subtype, request), emptyList(), threadId, turnId, receivedAtMs = nowMs, raw = raw)
        return when (subtype) {
            "can_use_tool" -> canUseTool(base, request)
            "elicitation" -> base.copy(
                kind = RequestKind.Elicitation(
                    server = request["mcp_server_name"].str,
                    message = request["message"].str ?: request["title"].str ?: "",
                    mode = request["mode"].str ?: "form",
                    url = request["url"].str,
                    schema = request["requested_schema"],
                    elicitationId = request["elicitation_id"].str,
                ),
                decisions = listOf(
                    Decision("accept", DecisionKind.ALLOW_ONCE),
                    Decision("decline", DecisionKind.DENY),
                    Decision("cancel", DecisionKind.ABORT),
                ),
            )
            // We declare no supportedDialogKinds, so the CLI fails closed; a stray dialog is shown but must not be answered.
            "request_user_dialog" -> base.copy(kind = RequestKind.UserDialog(request["dialog_kind"].str ?: "", request["payload"]))
            else -> null
        }
    }

    private fun canUseTool(base: PendingRequest, r: JsonObject): PendingRequest {
        val tool = r["tool_name"].str ?: ""
        val input = r["input"] ?: JsonObject(emptyMap())
        val toolUseId = r["tool_use_id"].str
        val suggestions = r["permission_suggestions"].arr ?: JsonArray(emptyList())
        val suppressAlways = r["suppress_always_allow_rule"].bool == true
        val remember = if (suppressAlways) emptyList() else suggestions.mapIndexed { i, s ->
            val destination = s["destination"].str
            Decision(
                id = "allowAlways:$i",
                kind = if (destination == "session" || destination == "cliArg") DecisionKind.ALLOW_SESSION else DecisionKind.ALLOW_PERSISTENT,
                detail = describeSuggestion(s),
                wire = s,
            )
        }
        val deny = listOf(Decision("deny", DecisionKind.DENY), Decision("denyAndStop", DecisionKind.ABORT))
        return when (tool) {
            "AskUserQuestion" -> base.copy(
                kind = RequestKind.UserInput(input["questions"].arr?.map { q ->
                    Question(
                        id = q["question"].str ?: "",
                        question = q["question"].str ?: "",
                        header = q["header"].str,
                        options = q["options"].arr?.map { QuestionOption(it["label"].str ?: "", it["description"].str, it["preview"].str) } ?: emptyList(),
                        multiSelect = q["multiSelect"].bool == true,
                        allowFreeText = true, // Claude always offers "Other"
                    )
                } ?: emptyList()),
                decisions = listOf(Decision("deny", DecisionKind.DENY)),
                itemId = toolUseId,
            )
            "ExitPlanMode" -> base.copy(
                kind = RequestKind.PlanApproval(input["plan"].str ?: ""),
                decisions = listOf(Decision("allow", DecisionKind.ALLOW_ONCE)) + remember + listOf(Decision("deny", DecisionKind.DENY)),
                itemId = toolUseId,
            )
            else -> base.copy(
                kind = RequestKind.ToolApproval(
                    tool = tool,
                    displayName = r["display_name"].str,
                    title = sanitizeDisplayText(r["title"].str),
                    description = sanitizeDisplayText(r["description"].str),
                    input = input,
                    blockedPath = r["blocked_path"].str,
                    reason = sanitizeDisplayText(r["decision_reason"].str),
                    reasonType = r["decision_reason_type"].str,
                    defaultToNo = r["default_to_no"].bool == true,
                    requiresUserInteraction = r["requires_user_interaction"].bool == true,
                    toolUseId = toolUseId,
                    agentId = r["agent_id"].str,
                    mcpServer = r["mcp_server"],
                ),
                // A card that is itself the interaction surface cannot be answered with one tap from here.
                decisions = if (r["requires_user_interaction"].bool == true) deny else listOf(Decision("allow", DecisionKind.ALLOW_ONCE)) + remember + deny,
                itemId = toolUseId,
            )
        }
    }

    fun describeSuggestion(s: JsonElement): String {
        val destination = s["destination"].str ?: "session"
        return when (s["type"].str) {
            "addRules", "replaceRules" -> "${s["behavior"].str ?: "allow"} ${s["rules"].arr?.joinToString { r -> listOfNotNull(r["toolName"].str, r["ruleContent"].str?.let { "($it)" }).joinToString("") } ?: ""} → $destination"
            "setMode" -> "mode ${s["mode"].str} → $destination"
            "addDirectories" -> "directories ${s["directories"].strings().joinToString()} → $destination"
            "removeRules", "removeDirectories" -> "${s["type"].str} → $destination"
            else -> s.encode()
        }
    }

    /** The control_response payload for a user's answer on [request]. */
    fun answer(request: PendingRequest, response: RequestResponse): Answer {
        val raw = request.raw["request"].obj ?: JsonObject(emptyMap())
        val input = raw["input"] ?: JsonObject(emptyMap())
        fun decision(id: String): Decision = request.decisions.firstOrNull { it.id == id }
            ?: throw IllegalArgumentException("decision $id was not offered for ${request.key} (offered: ${request.decisions.map { it.id }})")
        fun deny(message: String?, interrupt: Boolean) = buildJsonObject {
            put("behavior", "deny")
            put("message", message ?: "The user declined this action.")
            if (interrupt) put("interrupt", true)
        }
        return when (val kind = request.kind) {
            is RequestKind.ToolApproval, is RequestKind.PlanApproval -> {
                val d = decision((response as? RequestResponse.Decide ?: throw IllegalArgumentException("expected a decision")).decisionId)
                when {
                    d.id == "allow" -> Answer(buildJsonObject { put("behavior", "allow"); put("updatedInput", input) }, d.id, denied = false)
                    d.id.startsWith("allowAlways:") -> Answer(buildJsonObject {
                        put("behavior", "allow"); put("updatedInput", input); put("updatedPermissions", JsonArray(listOf(d.wire ?: JsonNull)))
                    }, d.id, denied = false)
                    d.id == "denyAndStop" -> Answer(deny(response.message, interrupt = true), d.id, denied = true)
                    else -> Answer(deny(response.message, interrupt = false), d.id, denied = true)
                }
            }
            is RequestKind.UserInput -> when (response) {
                is RequestResponse.Answer -> {
                    val known = kind.questions.map { it.id }.toSet()
                    require(response.answers.keys.all { it in known }) { "unknown questions ${response.answers.keys - known}" }
                    Answer(buildJsonObject {
                        put("behavior", "allow")
                        put("updatedInput", buildJsonObject {
                            (input as? JsonObject)?.forEach { (k, v) -> if (k != "answers") put(k, v) }
                            // Multi-select answers are comma-separated (AskUserQuestionOutput.answers).
                            put("answers", buildJsonObject { response.answers.forEach { (q, a) -> put(q, a.joinToString(", ")) } })
                        })
                    }, "answered", denied = false)
                }
                is RequestResponse.Decide -> { decision(response.decisionId); Answer(deny(response.message, false), response.decisionId, denied = true) }
                else -> throw IllegalArgumentException("expected answers")
            }
            is RequestKind.Elicitation -> {
                val (action, content) = when (response) {
                    is RequestResponse.Elicit -> response.action to response.content
                    is RequestResponse.Decide -> decision(response.decisionId).id to null
                    else -> throw IllegalArgumentException("expected an elicitation answer")
                }
                require(action in setOf("accept", "decline", "cancel"))
                Answer(buildJsonObject { put("action", action); if (content != null) put("content", content) }, action, denied = action != "accept")
            }
            is RequestKind.UserDialog -> throw IllegalStateException("undeclared dialog kinds must not be answered")
            is RequestKind.Unknown -> when (response) {
                is RequestResponse.RawResult -> Answer(response.result as? JsonObject ?: throw IllegalArgumentException("result must be an object"), "raw", denied = false)
                else -> Answer(null, "rejected", denied = true, error = (response as? RequestResponse.Reject)?.message ?: "Rejected by user")
            }
            else -> throw IllegalArgumentException("unsupported request kind for Claude: $kind")
        }
    }

    /** [payload] = success response; [error] = send an error response instead. */
    data class Answer(val payload: JsonObject?, val summary: String, val denied: Boolean, val error: String? = null)

    /** `initialize` request fields: our hooks and no prompt suggestions / dialog kinds. */
    fun initialize(hooks: List<ClaudeHook>): JsonObject = buildJsonObject {
        if (hooks.isNotEmpty()) put("hooks", buildJsonObject {
            hooks.groupBy { it.event }.forEach { (event, list) ->
                put(event, JsonArray(list.map { h -> buildJsonObject {
                    h.matcher?.let { put("matcher", it) }
                    put("hookCallbackIds", JsonArray(listOf(JsonPrimitive(h.callbackId))))
                } }))
            }
        })
        put("promptSuggestions", false)
    }

    /**
     * Neutral parts → Messages-API content blocks. Images and PDFs are inlined as base64 (read via
     * [reader]); other files are referenced as `@path` so the CLI loads them itself.
     */
    suspend fun content(parts: List<UserPart>, reader: AttachmentReader, maxBytes: Long): JsonArray {
        val mentions = parts.filterIsInstance<UserPart.File>().filterNot { isPdf(it) }.joinToString(" ") { "@${it.path}" }
        return buildJsonArray {
            var mentionAdded = mentions.isEmpty()
            for (part in parts) when (part) {
                is UserPart.Text -> {
                    val text = if (!mentionAdded) { mentionAdded = true; part.text + "\n\n" + mentions } else part.text
                    add(buildJsonObject { put("type", "text"); put("text", text) })
                }
                is UserPart.Image -> add(inline("image", part.mimeType ?: mimeFor(part.path) ?: "image/png", reader.read(part.path, maxBytes)))
                is UserPart.File -> if (isPdf(part)) add(inline("document", "application/pdf", reader.read(part.path, maxBytes)))
                is UserPart.InlineData -> add(buildJsonObject {
                    put("type", part.kind)
                    put("source", buildJsonObject { put("type", "base64"); put("media_type", part.mediaType ?: "application/octet-stream"); put("data", part.base64) })
                })
                is UserPart.ImageUrl -> add(buildJsonObject { put("type", "image"); put("source", buildJsonObject { put("type", "url"); put("url", part.url) }) })
                is UserPart.Reference -> add(buildJsonObject { put("type", "text"); put("text", "@${part.path}") })
                is UserPart.Unknown -> add(part.raw)
            }
            if (!mentionAdded) add(buildJsonObject { put("type", "text"); put("text", mentions) })
        }
    }

    private fun isPdf(file: UserPart.File) = file.mimeType == "application/pdf" || file.path.endsWith(".pdf", ignoreCase = true)

    private fun inline(type: String, mediaType: String, bytes: ByteArray) = buildJsonObject {
        put("type", type)
        put("source", buildJsonObject {
            put("type", "base64")
            put("media_type", mediaType)
            put("data", Base64.getEncoder().encodeToString(bytes))
        })
    }

    fun mimeFor(path: String): String? = when (path.substringAfterLast('.', "").lowercase()) {
        "png" -> "image/png"
        "jpg", "jpeg" -> "image/jpeg"
        "gif" -> "image/gif"
        "webp" -> "image/webp"
        "pdf" -> "application/pdf"
        else -> null
    }

    fun userMessage(clientMessageId: String, content: JsonArray, priority: String?): JsonObject = buildJsonObject {
        put("type", "user")
        put("uuid", clientMessageId)
        put("session_id", "")
        put("parent_tool_use_id", JsonNull)
        put("message", buildJsonObject { put("role", "user"); put("content", content) })
        if (priority != null) put("priority", priority)
    }

    /** Record for a control request answered with an error (unknown subtype, SDK MCP, …). */
    fun rejected(requestId: String, subtype: String, request: JsonObject, raw: JsonObject, threadId: String?, nowMs: Long) = PendingRequest(
        key(requestId), subtype, RequestKind.Unknown(subtype, request), emptyList(), threadId, status = RequestStatus.REJECTED, receivedAtMs = nowMs, raw = raw,
    )
}
