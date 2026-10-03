package top.flysoftbeta.workflow.agent.model

import kotlinx.serialization.json.JsonElement

enum class LoginState { UNKNOWN, LOGGED_OUT, LOGGING_IN, LOGGED_IN }

/**
 * Account as the backend reports it. Never contains secrets: emails/plan names are shown to the
 * user, tokens are never read.
 */
data class AccountState(
    val state: LoginState = LoginState.UNKNOWN,
    /** `chatgpt`, `apiKey`, `claudeai`, `console`, `oauthToken`… as reported. */
    val method: String? = null,
    val email: String? = null,
    val plan: String? = null,
    val organization: String? = null,
    /** Backend needs an OpenAI account for the selected provider (Codex `requiresOpenaiAuth`). */
    val requiresAuth: Boolean? = null,
    val login: LoginFlow? = null,
    val raw: JsonElement? = null,
)

/** Login methods the UI can offer per backend. */
enum class LoginMethod {
    /** Codex `chatgptDeviceCode`: verification URL + user code; tablet friendly. */
    CODEX_DEVICE_CODE,
    /** Codex `chatgpt`: browser flow with localhost callback. */
    CODEX_BROWSER,
    /** Codex `apiKey`. */
    CODEX_API_KEY,
    /** Claude: `claude auth login` in a Workflow terminal tab (URL + pasted code). */
    CLAUDE_TERMINAL_LOGIN,
    /** Claude: long-lived token from `claude setup-token`, passed as `CLAUDE_CODE_OAUTH_TOKEN`. */
    CLAUDE_SETUP_TOKEN,
    /** Claude: `ANTHROPIC_API_KEY`. */
    CLAUDE_API_KEY,
}

sealed interface LoginFlow {
    val loginId: String?

    data class DeviceCode(override val loginId: String?, val verificationUrl: String, val userCode: String) : LoginFlow
    data class Browser(override val loginId: String?, val authUrl: String) : LoginFlow
    /** Run [argv] in a terminal inside the environment; the user completes it there. */
    data class Terminal(val argv: List<String>, val env: Map<String, String> = emptyMap()) : LoginFlow {
        override val loginId: String? get() = null
    }
    data class Progress(override val loginId: String?, val output: List<String>, val error: String? = null) : LoginFlow
    data class Completed(override val loginId: String?, val success: Boolean, val error: String?) : LoginFlow
}

data class RateLimitWindow(
    val usedPercent: Double?,
    val windowMinutes: Long? = null,
    val resetsAtEpochSec: Long? = null,
)

data class RateLimit(
    /** Codex limit id (`codex`, `base_model_inference`) or Claude `rateLimitType` (`five_hour`, `seven_day`…). */
    val id: String,
    val name: String? = null,
    val primary: RateLimitWindow? = null,
    val secondary: RateLimitWindow? = null,
    /** `allowed` / `allowed_warning` / `rejected` (Claude), `rate_limit_reached` (Codex). */
    val status: String? = null,
    val reached: Boolean = false,
    val raw: JsonElement? = null,
)

data class RateLimitState(
    val limits: List<RateLimit> = emptyList(),
    /** Codex `ordinaryUsageAllowed` (false = only reserve models usable). */
    val ordinaryUsageAllowed: Boolean? = null,
    /** Backend-authored upsell/banner copy (Codex `rateLimitUpsell`), rendered as data. */
    val upsell: JsonElement? = null,
    val raw: JsonElement? = null,
)
