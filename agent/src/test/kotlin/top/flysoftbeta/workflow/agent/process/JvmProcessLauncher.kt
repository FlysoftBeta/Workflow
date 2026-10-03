package top.flysoftbeta.workflow.agent.process

import java.io.File
import java.io.InputStream
import java.io.OutputStream
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.runInterruptible

/** Host launcher backed by [ProcessBuilder]. Used by tests and host smoke runs. */
class JvmProcessLauncher : ProcessLauncher {
    override suspend fun launch(spec: LaunchSpec): AgentProcess = runInterruptible(Dispatchers.IO) {
        val builder = ProcessBuilder(spec.argv).directory(File(spec.cwd))
        builder.environment().apply { clear(); putAll(spec.env) }
        JvmAgentProcess(builder.start())
    }
}

class JvmAgentProcess(private val process: Process) : AgentProcess {
    override val stdin: OutputStream get() = process.outputStream
    override val stdout: InputStream get() = process.inputStream
    override val stderr: InputStream get() = process.errorStream
    override val pid: Long? get() = runCatching { process.pid() }.getOrNull()
    override val isAlive: Boolean get() = process.isAlive
    override suspend fun awaitExit(): Int = runInterruptible(Dispatchers.IO) { process.waitFor() }
    override fun kill(force: Boolean) {
        if (force) process.destroyForcibly() else process.destroy()
    }

    /** Terminates gracefully, escalating to SIGKILL after [graceMs]. Blocking; call off the main thread. */
    fun terminate(graceMs: Long) {
        process.destroy()
        if (!process.waitFor(graceMs, TimeUnit.MILLISECONDS)) process.destroyForcibly()
    }
}
