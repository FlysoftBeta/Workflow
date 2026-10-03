package top.flysoftbeta.workflow.agent.testing

import java.io.ByteArrayOutputStream
import java.io.InputStream
import java.io.OutputStream
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.LinkedBlockingQueue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.serialization.json.JsonObject
import top.flysoftbeta.workflow.agent.json.encode
import top.flysoftbeta.workflow.agent.json.parseJson
import top.flysoftbeta.workflow.agent.process.AgentProcess
import top.flysoftbeta.workflow.agent.process.LaunchSpec
import top.flysoftbeta.workflow.agent.process.ProcessLauncher

/** Blocking byte source fed by the test. */
class QueueInputStream : InputStream() {
    private val queue = LinkedBlockingQueue<ByteArray>()
    private var current: ByteArray? = null
    private var pos = 0
    private val eof = ByteArray(0)
    @Volatile private var ended = false

    fun push(bytes: ByteArray) { if (bytes.isNotEmpty()) queue.put(bytes) }
    fun end() = queue.put(eof)

    override fun read(): Int {
        val b = ByteArray(1)
        return if (read(b, 0, 1) < 0) -1 else b[0].toInt() and 0xff
    }

    override fun read(b: ByteArray, off: Int, len: Int): Int {
        if (ended) return -1
        var chunk = current
        if (chunk == null || pos >= chunk.size) {
            chunk = queue.take()
            if (chunk === eof) { ended = true; return -1 }
            current = chunk; pos = 0
        }
        val n = minOf(len, chunk.size - pos)
        System.arraycopy(chunk, pos, b, off, n)
        pos += n
        return n
    }
}

/** Splits what the client writes into lines and hands each to [onLine] on the writer's thread. */
class LineCapture(private val onLine: (String) -> Unit) : OutputStream() {
    private val buffer = ByteArrayOutputStream()
    @Synchronized override fun write(b: Int) {
        if (b == '\n'.code) { onLine(buffer.toString(Charsets.UTF_8)); buffer.reset() } else buffer.write(b)
    }
    @Synchronized override fun write(b: ByteArray, off: Int, len: Int) { for (i in off until off + len) write(b[i].toInt()) }
}

/**
 * In-memory child process. The test (or a [ScriptedServer]) reacts to every frame the client
 * writes by pushing frames to stdout. Records everything written for assertions.
 */
class FakeProcess(val spec: LaunchSpec, private val react: (FakeProcess, JsonObject) -> Unit) : AgentProcess {
    private val out = QueueInputStream()
    private val err = QueueInputStream()
    private val exit = CompletableDeferred<Int>()
    val written = CopyOnWriteArrayList<JsonObject>()

    override val stdin: OutputStream = LineCapture { line ->
        val frame = parseJson(line) as JsonObject
        written += frame
        react(this, frame)
    }
    override val stdout: InputStream get() = out
    override val stderr: InputStream get() = err
    override val pid: Long? = null
    override val isAlive: Boolean get() = !exit.isCompleted

    fun emit(frame: JsonObject) = out.push((frame.encode() + "\n").toByteArray())
    fun emitRaw(text: String) = out.push(text.toByteArray())

    fun exit(code: Int) {
        if (exit.isCompleted) return
        out.end(); err.end()
        exit.complete(code)
    }

    override suspend fun awaitExit(): Int = exit.await()
    override fun kill(force: Boolean) = exit(if (force) 137 else 143)
}

/** Hands out scripted processes in launch order and records the launch specs. */
class FakeLauncher(private val factory: (Int, LaunchSpec) -> FakeProcess) : ProcessLauncher {
    val launched = CopyOnWriteArrayList<FakeProcess>()
    override suspend fun launch(spec: LaunchSpec): AgentProcess = factory(launched.size, spec).also { launched += it }
}
