package top.flysoftbeta.workflow.agent.model

import kotlinx.serialization.Serializable
import kotlinx.serialization.SerialName

import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject

@Serializable
enum class BackendKind(val id: String) {
    CODEX("codex"),
    CLAUDE("claude");

    companion object {
        fun of(id: String): BackendKind? = entries.firstOrNull { it.id == id }
    }
}

/**
 * One part of a user message. Paths are paths inside the agent environment (`/workspace/...`),
 * the same paths the agent's own tools see.
 */
@Serializable
sealed interface UserPart {
    @Serializable
    @SerialName("Text")
    data class Text(val text: String) : UserPart
    /** Image file inside the workspace. [mimeType] is required by backends that inline the bytes. */
    @Serializable
    @SerialName("Image")
    data class Image(val path: String, val mimeType: String? = null) : UserPart
    /** Any other file; PDFs are inlined as documents where the backend supports it, others are referenced by path. */
    @Serializable
    @SerialName("File")
    data class File(val path: String, val mimeType: String? = null) : UserPart
    /** Inline bytes from history (Claude base64 `image`/`document` blocks). [kind] is `image` or `document`. */
    @Serializable
    @SerialName("InlineData")
    data class InlineData(val kind: String, val mediaType: String?, val base64: String) : UserPart
    /** Remote image already addressed by URL (history only; the composer does not create these). */
    @Serializable
    @SerialName("ImageUrl")
    data class ImageUrl(val url: String) : UserPart
    /** Named skill / mention (Codex `skill` / `mention`). */
    @Serializable
    @SerialName("Reference")
    data class Reference(val kind: String, val name: String, val path: String) : UserPart
    /** Input kind the model does not understand (history), kept verbatim. */
    @Serializable
    @SerialName("Unknown")
    data class Unknown(val raw: JsonElement) : UserPart
}

/**
 * Settings the user picks for the next turn. `null` means "leave the backend's current value".
 */
@Serializable
@SerialName("TurnSettings")
data class TurnSettings(
    val model: String? = null,
    val effort: String? = null,
    val permissions: PermissionPreset? = null,
)

/**
 * Who may run what without asking. Presets are backend-neutral; adapters translate them.
 * No preset ever routes approvals to anything but the user.
 */
@Serializable
enum class PermissionPreset {
    /** Ask before any command or edit outside read-only access (Codex `untrusted` + read-only; Claude `default`). */
    ASK,
    /** Edits in the workspace are allowed; commands still ask (Codex `on-request` + workspace-write; Claude `acceptEdits`). */
    AUTO_EDIT,
    /** Plan only, no execution (Claude `plan`; Codex read-only + `untrusted`). */
    PLAN,
    /** Never prompt; anything not pre-approved is refused (Claude `dontAsk`; Codex `never` + read-only). */
    DENY_UNLISTED,
}

/** What the user answered on a request card. Validated by the adapter against the offered decisions. */
@Serializable
sealed interface RequestResponse {
    /** Pick one of [PendingRequest.decisions] by id. [message] is an optional reason (Claude deny). */
    @Serializable
    @SerialName("Decide")
    data class Decide(val decisionId: String, val message: String? = null) : RequestResponse
    /** Answers keyed by question id; each answer is a list (single-choice = one element, free text = one element). */
    @Serializable
    @SerialName("Answer")
    data class Answer(val answers: Map<String, List<String>>) : RequestResponse
    /** MCP elicitation form. [action] is one of accept/decline/cancel. */
    @Serializable
    @SerialName("Elicit")
    data class Elicit(val action: String, val content: JsonObject? = null) : RequestResponse
    /** Generic console: explicit raw JSON result written verbatim by the user. */
    @Serializable
    @SerialName("RawResult")
    data class RawResult(val result: JsonElement) : RequestResponse
    /** Generic console: reject with a protocol error. */
    @Serializable
    @SerialName("Reject")
    data class Reject(val message: String = "Rejected by user") : RequestResponse
}
