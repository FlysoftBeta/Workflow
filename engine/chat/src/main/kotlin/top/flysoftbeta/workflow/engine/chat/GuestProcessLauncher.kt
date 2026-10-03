package top.flysoftbeta.workflow.engine.chat

import java.io.File
import java.util.concurrent.ConcurrentHashMap
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.runInterruptible
import top.flysoftbeta.workflow.agent.process.*

/** Runs only inside the Engine guest; never inherits the launcher's credentials. */
class GuestProcessLauncher : ProcessLauncher, AutoCloseable {
    @Volatile private var closed = false
    private val children = ConcurrentHashMap.newKeySet<Process>()
    override suspend fun launch(spec: LaunchSpec): AgentProcess = runInterruptible(Dispatchers.IO) {
        val process = synchronized(this) {
            check(!closed) { "Chat service is stopping" }
            ProcessBuilder(spec.argv).directory(File(spec.cwd)).apply {
                environment().clear()
                environment().putAll(spec.env)
            }.start().also { children += it }
        }
        object : AgentProcess {
            override val stdin = process.outputStream
            override val stdout = process.inputStream
            override val stderr = process.errorStream
            override val pid: Long get() = process.pid()
            override val isAlive get() = process.isAlive
            override suspend fun awaitExit(): Int = runInterruptible(Dispatchers.IO) { process.waitFor().also { children -= process } }
            override fun kill(force: Boolean) {
                // Descendants include agent-started helpers. Do not leave them beyond service lifetime.
                process.descendants().use { descendants -> descendants.forEach { if (force) it.destroyForcibly() else it.destroy() } }
                if (force) process.destroyForcibly() else process.destroy()
            }
        }
    }
    @Synchronized override fun close() {
        closed = true
        children.forEach { process ->
            process.descendants().use { descendants -> descendants.forEach { it.destroyForcibly() } }
            process.destroyForcibly()
        }
        children.clear()
    }
}
