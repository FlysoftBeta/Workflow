package top.flysoftbeta.workflow.agent.claude

import kotlinx.serialization.json.JsonObject
import top.flysoftbeta.workflow.agent.UnknownRequestPolicy
import top.flysoftbeta.workflow.agent.model.PermissionPreset
import top.flysoftbeta.workflow.agent.process.LaunchSpec

/** App-registered hook callback (answers `hook_callback`); returns the hook output object. Not a user approval. */
fun interface ClaudeHookHandler {
    suspend fun handle(input: JsonObject): JsonObject
}

data class ClaudeHook(val event: String, val matcher: String?, val callbackId: String, val handler: ClaudeHookHandler)

data class ClaudeConfig(
    /** argv[0] inside the environment (`claude` on PATH, or an absolute path). */
    val executable: String = "claude",
    /** App-owned CLAUDE_CONFIG_DIR (credentials, transcripts). */
    val configDir: String,
    val cwd: String = "/workspace",
    /** Scratch dir for the CLI (sockets, temp files). */
    val tmpDir: String = "/tmp",
    /** Base environment; filtered (see [ClaudeLaunch.DENY_PREFIXES]). */
    val env: Map<String, String> = emptyMap(),
    /**
     * Explicit credentials/endpoint variables set by the app (`CLAUDE_CODE_OAUTH_TOKEN`,
     * `ANTHROPIC_API_KEY`, `ANTHROPIC_BASE_URL` for tests). Passed verbatim, never logged.
     */
    val credentials: Map<String, String> = emptyMap(),
    val defaultPermissions: PermissionPreset = PermissionPreset.ASK,
    val hooks: List<ClaudeHook> = emptyList(),
    val unknownRequests: UnknownRequestPolicy = UnknownRequestPolicy.REJECT,
    /** Largest file inlined as base64 (image/document). */
    val maxInlineBytes: Long = 20L * 1024 * 1024,
    val extraArgs: List<String> = emptyList(),
)

/** How a process attaches to a session. */
sealed interface ClaudeSession {
    data class New(val sessionId: String) : ClaudeSession
    data class Resume(val sessionId: String) : ClaudeSession
    /** Branch [fromSessionId] into [newSessionId]; [atMessageUuid] = keep history up to that message. */
    data class Fork(val fromSessionId: String, val newSessionId: String, val atMessageUuid: String?) : ClaudeSession
}

object ClaudeLaunch {
    /** Never forwarded from the base environment: host credentials, other agents, the host Claude session. */
    val DENY_PREFIXES = listOf("OPENAI_", "CODEX_", "ANTHROPIC_", "CLAUDE_", "CLAUDECODE", "LD_PRELOAD", "LD_LIBRARY_PATH")

    /** Modes that never let anything but the user approve. `bypassPermissions` and `auto` are refused. */
    val ALLOWED_MODES = setOf("default", "acceptEdits", "plan", "dontAsk")

    fun permissionMode(preset: PermissionPreset): String = when (preset) {
        PermissionPreset.ASK -> "default"
        PermissionPreset.AUTO_EDIT -> "acceptEdits"
        PermissionPreset.PLAN -> "plan"
        PermissionPreset.DENY_UNLISTED -> "dontAsk"
    }

    fun argv(config: ClaudeConfig, session: ClaudeSession, model: String?, effort: String?, preset: PermissionPreset?): List<String> {
        val mode = permissionMode(preset ?: config.defaultPermissions)
        check(mode in ALLOWED_MODES)
        val args = mutableListOf(
            config.executable, "-p",
            "--input-format", "stream-json",
            "--output-format", "stream-json",
            "--verbose",
            "--include-partial-messages",
            "--replay-user-messages",
            "--permission-prompt-tool", "stdio",
            "--permission-mode", mode,
        )
        if (model != null) args += listOf("--model", model)
        if (effort != null) args += listOf("--effort", effort)
        when (session) {
            is ClaudeSession.New -> args += listOf("--session-id", session.sessionId)
            is ClaudeSession.Resume -> args += listOf("--resume", session.sessionId)
            is ClaudeSession.Fork -> {
                args += listOf("--resume", session.fromSessionId, "--fork-session", "--session-id", session.newSessionId)
                session.atMessageUuid?.let { args += listOf("--resume-session-at", it) }
            }
        }
        val forbidden = setOf("--dangerously-skip-permissions", "--allow-dangerously-skip-permissions", "--permission-prompt-tool")
        require(config.extraArgs.none { it in forbidden || it == "bypassPermissions" || it == "auto" }) { "extraArgs may not weaken permissions" }
        args += config.extraArgs
        return args
    }

    fun env(config: ClaudeConfig): Map<String, String> {
        val env = LinkedHashMap<String, String>()
        config.env.forEach { (k, v) -> if (DENY_PREFIXES.none { k.startsWith(it) }) env[k] = v }
        env["CLAUDE_CONFIG_DIR"] = config.configDir
        env["TMPDIR"] = config.tmpDir
        env["CLAUDE_CODE_TMPDIR"] = config.tmpDir
        env["DISABLE_AUTOUPDATER"] = "1"
        env["DISABLE_TELEMETRY"] = "1"
        env["DISABLE_ERROR_REPORTING"] = "1"
        env["CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC"] = "1"
        env.putAll(config.credentials)
        return env
    }

    fun spec(config: ClaudeConfig, session: ClaudeSession, model: String?, effort: String?, preset: PermissionPreset?): LaunchSpec {
        val id = when (session) {
            is ClaudeSession.New -> session.sessionId
            is ClaudeSession.Resume -> session.sessionId
            is ClaudeSession.Fork -> session.newSessionId
        }
        return LaunchSpec(argv(config, session, model, effort, preset), env(config), config.cwd, "claude:$id")
    }

    /** `claude auth login` etc. run by the user in a terminal tab with the same environment. */
    fun loginArgv(config: ClaudeConfig): List<String> = listOf(config.executable, "auth", "login")
    fun setupTokenArgv(config: ClaudeConfig): List<String> = listOf(config.executable, "setup-token")
    fun logoutArgv(config: ClaudeConfig): List<String> = listOf(config.executable, "auth", "logout")

    /** Transcript location: `$CLAUDE_CONFIG_DIR/projects/<cwd with non-alphanumerics → "-">/<session>.jsonl`. */
    fun transcriptPath(configDir: String, cwd: String, sessionId: String): String =
        "$configDir/projects/${cwd.replace(Regex("[^A-Za-z0-9]"), "-")}/$sessionId.jsonl"
}
