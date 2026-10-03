package top.flysoftbeta.workflow.agent.codex

import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import top.flysoftbeta.workflow.agent.codex.CodexProtocol.ClientRequest as C
import top.flysoftbeta.workflow.agent.json.plusMember
import top.flysoftbeta.workflow.agent.json.toJson
import top.flysoftbeta.workflow.agent.model.LoginMethod
import top.flysoftbeta.workflow.agent.model.PermissionPreset
import top.flysoftbeta.workflow.agent.model.TurnSettings
import top.flysoftbeta.workflow.agent.model.UserPart

/**
 * Request builders. Invariant: every request that can start, resume, fork or run a thread carries
 * `approvalsReviewer: "user"` explicitly, because the server otherwise inherits the reviewer from
 * config.toml (observed: `auto_review`, i.e. a model approving on the user's behalf).
 */
object CodexParams {
    const val REVIEWER_USER = "user"

    /** Methods whose params must carry `approvalsReviewer: "user"`. */
    val REVIEWER_METHODS = setOf(C.THREAD_START, C.THREAD_RESUME, C.THREAD_FORK, C.TURN_START)

    data class Policy(val approvalPolicy: String, val sandboxMode: String, val sandboxPolicy: JsonObject)

    fun policy(preset: PermissionPreset): Policy = when (preset) {
        PermissionPreset.ASK, PermissionPreset.PLAN -> Policy("untrusted", "read-only", sandbox("readOnly"))
        PermissionPreset.AUTO_EDIT -> Policy("on-request", "workspace-write", sandbox("workspaceWrite"))
        PermissionPreset.DENY_UNLISTED -> Policy("never", "read-only", sandbox("readOnly"))
    }

    private fun sandbox(type: String) = buildJsonObject {
        put("type", type)
        put("networkAccess", false)
    }

    fun initialize(name: String, title: String, version: String, optOut: List<String>): JsonObject = buildJsonObject {
        put("clientInfo", buildJsonObject { put("name", name); put("title", title); put("version", version) })
        put("capabilities", buildJsonObject {
            put("experimentalApi", true)
            if (optOut.isNotEmpty()) put("optOutNotificationMethods", toJson(optOut))
        })
    }

    private fun threadCommon(cwd: String?, settings: TurnSettings, preset: PermissionPreset): Map<String, JsonElement> {
        val policy = policy(settings.permissions ?: preset)
        return buildMap {
            if (cwd != null) put("cwd", JsonPrimitive(cwd))
            settings.model?.let { put("model", JsonPrimitive(it)) }
            put("approvalPolicy", JsonPrimitive(policy.approvalPolicy))
            put("sandbox", JsonPrimitive(policy.sandboxMode))
            put("approvalsReviewer", JsonPrimitive(REVIEWER_USER))
        }
    }

    fun threadStart(cwd: String, settings: TurnSettings, preset: PermissionPreset, ephemeral: Boolean): JsonObject =
        JsonObject(threadCommon(cwd, settings, preset) + ("ephemeral" to JsonPrimitive(ephemeral)))

    fun threadResume(threadId: String, cwd: String?, settings: TurnSettings, preset: PermissionPreset): JsonObject =
        JsonObject(mapOf("threadId" to JsonPrimitive(threadId)) + threadCommon(cwd, settings, preset) + ("excludeTurns" to JsonPrimitive(true)))

    fun threadFork(threadId: String, lastTurnId: String?, cwd: String?, settings: TurnSettings, preset: PermissionPreset, ephemeral: Boolean): JsonObject =
        JsonObject(buildMap {
            put("threadId", JsonPrimitive(threadId))
            putAll(threadCommon(cwd, settings, preset))
            if (lastTurnId != null) put("lastTurnId", JsonPrimitive(lastTurnId))
            put("ephemeral", JsonPrimitive(ephemeral))
            put("excludeTurns", JsonPrimitive(true))
        })

    fun turnStart(threadId: String, parts: List<UserPart>, settings: TurnSettings?, clientMessageId: String): JsonObject = buildJsonObject {
        put("threadId", threadId)
        put("input", CodexItems.userInput(parts))
        put("clientUserMessageId", clientMessageId)
        settings?.model?.let { put("model", it) }
        settings?.effort?.let { put("effort", it) }
        settings?.permissions?.let {
            val policy = policy(it)
            put("approvalPolicy", policy.approvalPolicy)
            put("sandboxPolicy", policy.sandboxPolicy)
        }
        put("summary", "auto")
        put("approvalsReviewer", REVIEWER_USER)
    }

    fun turnSteer(threadId: String, expectedTurnId: String, parts: List<UserPart>, clientMessageId: String): JsonObject = buildJsonObject {
        put("threadId", threadId)
        put("expectedTurnId", expectedTurnId)
        put("input", CodexItems.userInput(parts))
        put("clientUserMessageId", clientMessageId)
    }

    fun queueAdd(threadId: String, parts: List<UserPart>, clientMessageId: String): JsonObject = buildJsonObject {
        put("threadId", threadId)
        put("clientUserMessageId", clientMessageId)
        put("input", CodexItems.userInput(parts))
    }

    fun login(method: LoginMethod, secret: String?): JsonObject = when (method) {
        LoginMethod.CODEX_DEVICE_CODE -> buildJsonObject { put("type", "chatgptDeviceCode") }
        LoginMethod.CODEX_BROWSER -> buildJsonObject { put("type", "chatgpt") }
        LoginMethod.CODEX_API_KEY -> buildJsonObject {
            put("type", "apiKey")
            put("apiKey", requireNotNull(secret?.takeIf { it.isNotBlank() }) { "API key required" })
        }
        else -> throw IllegalArgumentException("$method is not a Codex login method")
    }

    /** Generic console guard: forces `approvalsReviewer: "user"` on thread/turn requests. */
    fun enforceReviewer(method: String, params: JsonElement?): JsonElement? {
        if (method !in REVIEWER_METHODS) return params
        val obj = params as? JsonObject ?: JsonObject(emptyMap())
        return obj.plusMember("approvalsReviewer", JsonPrimitive(REVIEWER_USER))
    }

    val EMPTY_ARRAY = JsonArray(emptyList())
}
