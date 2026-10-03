package top.flysoftbeta.workflow.platform.agent

import java.io.ByteArrayOutputStream
import kotlinx.coroutines.*
import top.flysoftbeta.workflow.agent.process.LaunchSpec
import top.flysoftbeta.workflow.platform.engine.EngineController
import top.flysoftbeta.workflow.platform.engine.Guest

internal data class GuestResult(val exitCode: Int, val output: ByteArray, val error: String)
internal suspend fun runGuest(controller: EngineController, argv: List<String>, limit: Int = 64 * 1024, timeout: Long = 60_000): GuestResult = withTimeout(timeout) {
    val process = EngineProcessLauncher(controller).launch(LaunchSpec(argv, emptyMap(), Guest.HOME, "chat-tool"))
    try {
        coroutineScope {
            val output = async(Dispatchers.IO) { process.stdout.use { input ->
                val bytes = ByteArrayOutputStream(); val chunk = ByteArray(16 * 1024)
                while (true) { val count = input.read(chunk); if (count < 0) break; require(bytes.size() + count <= limit) { "后端文件超过读取限制" }; bytes.write(chunk, 0, count) }
                bytes.toByteArray()
            } }
            val error = async(Dispatchers.IO) { process.stderr.use { input ->
                val bytes = ByteArray(4096); val text = StringBuilder()
                while (true) { val count = input.read(bytes); if (count < 0) break; text.append(String(bytes, 0, count, Charsets.UTF_8)); if (text.length > 8192) text.delete(0, text.length - 8192) }
                text.toString()
            } }
            GuestResult(process.awaitExit(), output.await(), error.await())
        }
    } finally { if (process.isAlive) process.kill(true) }
}
