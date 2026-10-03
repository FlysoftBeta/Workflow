package top.flysoftbeta.workflow.agent.process

import java.io.InputStream
import java.io.OutputStream

/**
 * What to start. [argv] and [cwd] are interpreted by the launcher: the Debian environment launcher
 * (engine workstream) runs them inside the guest, where the workspace is `/workspace`. [env] is
 * the complete environment of the child; launchers must not merge in their own environment
 * (host tokens such as `OPENAI_API_KEY` or `CLAUDE_CODE_*` must never leak into an agent).
 */
data class LaunchSpec(
    val argv: List<String>,
    val env: Map<String, String>,
    val cwd: String,
    /** Short name for logs and service leases (`codex`, `claude:<session>`). */
    val label: String,
) {
    init {
        require(argv.isNotEmpty()) { "argv must not be empty" }
    }
}

/** A running child with piped stdio. Streams are raw bytes; framing is the transport's job. */
interface AgentProcess {
    val stdin: OutputStream
    val stdout: InputStream
    val stderr: InputStream
    val pid: Long?
    val isAlive: Boolean
    /** Suspends until the process exits; returns the exit status (128+signal for signals where known). */
    suspend fun awaitExit(): Int
    /** SIGTERM (or the platform equivalent); [force] = SIGKILL. Idempotent. */
    fun kill(force: Boolean = false)
}

/** Port implemented by the platform (`:app/platform`), the Engine. Production agents always use its guest process channel. */
fun interface ProcessLauncher {
    suspend fun launch(spec: LaunchSpec): AgentProcess
}

