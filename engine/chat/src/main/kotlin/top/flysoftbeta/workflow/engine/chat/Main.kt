package top.flysoftbeta.workflow.engine.chat

import java.io.*
import java.nio.ByteBuffer
import java.nio.charset.CodingErrorAction
import java.util.concurrent.LinkedBlockingQueue
import kotlinx.coroutines.*
import kotlinx.serialization.json.*
import top.flysoftbeta.workflow.agent.rpc.ChatWire
import top.flysoftbeta.workflow.core.connection.*

/** Sole service entrypoint. Stdout is exclusively the private bidirectional JSON-RPC channel. */
fun main() = runBlocking {
    val protocolOutput = System.out
    System.setOut(System.err)
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val writer = FrameWriter(protocolOutput)
    val callbackInput = ResponseInputStream()
    val transport = object : WorkspaceTransport {
        override val input: InputStream = callbackInput
        override val output: OutputStream = CallbackOutput(writer)
        override fun close() = callbackInput.close()
    }
    val rpc = WorkspaceRpc(transport, scope)
    val service = ChatService(rpc, scope)
    val initialized = scope.async { service.start() }
    try {
        withContext(Dispatchers.IO) {
            val input = System.`in`.buffered(64 * 1024)
            while (true) {
                val line = readFrame(input) ?: break
                val frame = ChatWire.json.parseToJsonElement(line).jsonObject
                require(frame["jsonrpc"]?.jsonPrimitive?.content == "2.0") { "Invalid private protocol" }
                val method = frame.stringOrNull("method")
                if (method == null) {
                    require(frame.containsKey("result") != frame.containsKey("error")) { "Invalid callback response" }
                    callbackInput.deliver((line + "\n").toByteArray(Charsets.UTF_8))
                    continue
                }
                val id = frame["id"] ?: continue
                scope.launch {
                    val response = try {
                        initialized.await()
                        require(method in setOf("chat.snapshot", "chat.watch", "chat.command")) { "Unknown chat method" }
                        val result = service.request(method, frame["params"]?.jsonObject ?: JsonObject(emptyMap()))
                        buildJsonObject { put("jsonrpc", "2.0"); put("id", id); put("result", result) }
                    } catch (cancelled: CancellationException) { throw cancelled }
                    catch (error: Exception) {
                        buildJsonObject {
                            put("jsonrpc", "2.0"); put("id", id)
                            putJsonObject("error") {
                                put("code", if (error is IllegalArgumentException) -32602 else -32000)
                                // Failure descriptions are returned only to the requesting UI, never diagnostic logs.
                                put("message", error.message ?: "Chat operation failed")
                                putJsonObject("data") { put("kind", "chat") }
                            }
                        }
                    }
                    writer.frame(response.toString())
                }
            }
        }
    } catch (_: Exception) {
        System.err.println("Chat service protocol closed")
    } finally {
        withContext(NonCancellable) { service.close() }
        rpc.close()
        scope.cancel()
    }
}

internal class FrameWriter(private val output: OutputStream) {
    @Synchronized fun frame(text: String) {
        val bytes = text.toByteArray(Charsets.UTF_8)
        require(bytes.size <= WorkspaceRpc.MAX_FRAME_BYTES) { "Private RPC frame exceeds limit" }
        output.write(bytes); output.write(10); output.flush()
    }
}

/** WorkspaceRpc already serializes its writes. Assemble whole frames before sharing stdout. */
private class CallbackOutput(private val writer: FrameWriter) : OutputStream() {
    private val buffer = ByteArrayOutputStream()
    override fun write(value: Int) {
        if (value == 10) {
            val text = buffer.toString(Charsets.UTF_8)
            buffer.reset()
            val frame = ChatWire.json.parseToJsonElement(text).jsonObject
            require(frame.string("method") in CALLBACKS) { "Disallowed Engine callback" }
            writer.frame(text)
        } else {
            require(buffer.size() < WorkspaceRpc.MAX_FRAME_BYTES)
            buffer.write(value)
        }
    }
    override fun write(bytes: ByteArray, offset: Int, length: Int) {
        for (i in offset until offset + length) write(bytes[i].toInt() and 255)
    }
    companion object {
        val CALLBACKS = setOf("hello", "workspace.snapshot", "workspace.watch", "workspace.command", "documents.read", "documents.write", "documents.quarantine", "files.read", "environment.tools.status")
    }
}

/** Bounded response lane prevents a callback response from being misread as an inbound chat command. */
internal class ResponseInputStream : InputStream() {
    private val queue = LinkedBlockingQueue<ByteArray>(4)
    private var current = ByteArray(0)
    private var offset = 0
    @Volatile private var closed = false
    fun deliver(bytes: ByteArray) {
        check(!closed) { "Callback stream closed" }
        queue.put(bytes)
    }
    override fun read(): Int {
        while (offset == current.size) {
            if (closed) return -1
            current = queue.take()
            offset = 0
        }
        return current[offset++].toInt() and 255
    }
    override fun read(bytes: ByteArray, start: Int, length: Int): Int {
        if (length == 0) return 0
        val first = read()
        if (first == -1) return -1
        bytes[start] = first.toByte()
        val count = minOf(length - 1, current.size - offset)
        current.copyInto(bytes, start + 1, offset, offset + count)
        offset += count
        return count + 1
    }
    override fun close() { closed = true; queue.clear(); queue.offer(ByteArray(0)) }
}

internal fun readFrame(input: InputStream): String? {
    val bytes = ByteArrayOutputStream()
    while (true) {
        val value = input.read()
        if (value < 0) {
            require(bytes.size() == 0) { "Truncated private RPC frame" }
            return null
        }
        if (value == 10) {
            if (bytes.size() == 0) continue
            return Charsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT).onUnmappableCharacter(CodingErrorAction.REPORT).decode(ByteBuffer.wrap(bytes.toByteArray())).toString()
        }
        require(bytes.size() < WorkspaceRpc.MAX_FRAME_BYTES) { "Private RPC frame exceeds limit" }
        bytes.write(value)
    }
}
