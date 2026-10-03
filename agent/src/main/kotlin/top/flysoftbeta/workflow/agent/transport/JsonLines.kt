package top.flysoftbeta.workflow.agent.transport

import java.io.ByteArrayOutputStream
import java.io.IOException
import java.io.InputStream
import java.nio.ByteBuffer
import java.nio.charset.CharacterCodingException
import java.nio.charset.CodingErrorAction
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.channels.ReceiveChannel
import kotlinx.coroutines.launch
import kotlinx.coroutines.runInterruptible
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import top.flysoftbeta.workflow.agent.json.encode
import top.flysoftbeta.workflow.agent.json.parseJson
import top.flysoftbeta.workflow.agent.process.AgentProcess

/** Splits a byte stream into `\n`-terminated UTF-8 lines with a hard size cap per line. */
class LineReader(private val input: InputStream, private val maxLineBytes: Int) {
    private val buffer = ByteArray(64 * 1024)
    private var start = 0
    private var end = 0
    private val line = ByteArrayOutputStream()
    private var oversized = false

    sealed interface Result {
        data class Line(val text: String) : Result
        data class TooLarge(val bytes: Long) : Result
        data class BadEncoding(val bytes: Int) : Result
        data object Eof : Result
    }

    private var droppedBytes = 0L

    /** Blocking. Returns the next line (without `\n` / `\r\n`), an error marker, or EOF. */
    fun next(): Result {
        while (true) {
            if (start == end) {
                val n = input.read(buffer)
                if (n < 0) {
                    if (line.size() > 0 && !oversized) return decode(stripCr(line.toByteArray())).also { line.reset() }
                    if (oversized) { oversized = false; return Result.TooLarge(droppedBytes) }
                    return Result.Eof
                }
                start = 0; end = n
            }
            var i = start
            while (i < end && buffer[i] != '\n'.code.toByte()) i++
            val chunk = i - start
            if (!oversized) {
                if (line.size() + chunk > maxLineBytes) {
                    oversized = true
                    droppedBytes = (line.size() + chunk).toLong()
                    line.reset()
                } else line.write(buffer, start, chunk)
            } else droppedBytes += chunk
            if (i < end) { // newline found
                start = i + 1
                if (oversized) { oversized = false; return Result.TooLarge(droppedBytes) }
                val bytes = line.toByteArray(); line.reset()
                val trimmed = stripCr(bytes)
                if (trimmed.isEmpty()) continue
                return decode(trimmed)
            }
            start = end
        }
    }

    private fun stripCr(bytes: ByteArray): ByteArray =
        if (bytes.isNotEmpty() && bytes.last() == '\r'.code.toByte()) bytes.copyOf(bytes.size - 1) else bytes

    private fun decode(bytes: ByteArray): Result = try {
        Result.Line(Charsets.UTF_8.newDecoder()
            .onMalformedInput(CodingErrorAction.REPORT)
            .onUnmappableCharacter(CodingErrorAction.REPORT)
            .decode(ByteBuffer.wrap(bytes)).toString())
    } catch (_: CharacterCodingException) {
        Result.BadEncoding(bytes.size)
    }
}

/** What the reader produced. Frames are always JSON objects; anything else is reported, not dropped silently. */
sealed interface LineEvent {
    data class Frame(val json: JsonObject) : LineEvent
    /** A line that is not a JSON object (kept for diagnostics, truncated). */
    data class Malformed(val preview: String, val reason: String) : LineEvent
    data class Closed(val error: Throwable?) : LineEvent
}

/**
 * Newline-delimited JSON over a process's stdio.
 *
 * Backpressure: [events] is a bounded channel; when the consumer is slow the reader suspends,
 * the pipe fills and the child blocks on write. Nothing is dropped. Writes are serialised by a
 * mutex and flushed per frame.
 */
class JsonLineChannel(
    private val process: AgentProcess,
    scope: CoroutineScope,
    maxFrameBytes: Int = DEFAULT_MAX_FRAME_BYTES,
    capacity: Int = 1024,
    private val stderrTailChars: Int = 32 * 1024,
    private val onStderr: ((String) -> Unit)? = null,
) {
    private val channel = Channel<LineEvent>(capacity)
    val events: ReceiveChannel<LineEvent> get() = channel
    private val writeLock = Mutex()
    private val stderrTail = StringBuilder()
    @Volatile private var closed = false

    val stderr: String get() = synchronized(stderrTail) { stderrTail.toString() }

    private val readerJob: Job = scope.launch(Dispatchers.IO) {
        val reader = LineReader(process.stdout, maxFrameBytes)
        var failure: Throwable? = null
        try {
            while (true) {
                val result = runInterruptible { reader.next() }
                when (result) {
                    is LineReader.Result.Eof -> break
                    is LineReader.Result.TooLarge -> channel.send(LineEvent.Malformed("", "frame of ${result.bytes} bytes exceeds $maxFrameBytes"))
                    is LineReader.Result.BadEncoding -> channel.send(LineEvent.Malformed("", "invalid UTF-8 (${result.bytes} bytes)"))
                    is LineReader.Result.Line -> channel.send(parse(result.text))
                }
            }
        } catch (e: IOException) {
            if (!closed) failure = e
        } finally {
            channel.trySend(LineEvent.Closed(failure))
            channel.close()
        }
    }

    private val stderrJob: Job = scope.launch(Dispatchers.IO) {
        runCatching {
            process.stderr.bufferedReader(Charsets.UTF_8).use { r ->
                while (true) {
                    val line = runInterruptible { r.readLine() } ?: break
                    synchronized(stderrTail) {
                        stderrTail.append(line).append('\n')
                        if (stderrTail.length > stderrTailChars) stderrTail.delete(0, stderrTail.length - stderrTailChars)
                    }
                    onStderr?.invoke(line)
                }
            }
        }
    }

    private fun parse(text: String): LineEvent = try {
        val element = parseJson(text)
        if (element is JsonObject) LineEvent.Frame(element) else LineEvent.Malformed(text.take(PREVIEW), "not a JSON object")
    } catch (e: Exception) {
        LineEvent.Malformed(text.take(PREVIEW), e.message ?: "invalid JSON")
    }

    suspend fun send(frame: JsonElement) {
        val bytes = (frame.encode() + "\n").toByteArray(Charsets.UTF_8)
        writeLock.withLock {
            check(!closed) { "channel closed" }
            runInterruptible(Dispatchers.IO) {
                process.stdin.write(bytes)
                process.stdin.flush()
            }
        }
    }

    /** Closes stdin (EOF for the child). Reading continues until the child closes stdout. */
    fun closeInput() {
        closed = true
        runCatching { process.stdin.close() }
    }

    suspend fun join() {
        readerJob.join()
        stderrJob.join()
    }

    companion object {
        const val DEFAULT_MAX_FRAME_BYTES = 64 * 1024 * 1024
        private const val PREVIEW = 2000
    }
}
