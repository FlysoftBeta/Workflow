package top.flysoftbeta.workflow.core.connection

import java.io.ByteArrayOutputStream
import java.io.IOException
import java.nio.ByteBuffer
import java.nio.charset.CodingErrorAction
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import top.flysoftbeta.workflow.core.json.Json
import kotlinx.serialization.json.JsonElement
import top.flysoftbeta.workflow.client.protocol.*

class WorkspaceRpcException(val code: Long, override val message: String, val data: Any?) : IOException(message) {
    /** The Engine's stable error kind (`data.kind`), such as `conflict` or `environment_preparing`. */
    val kind: String? get() = (data as? Map<*, *>)?.get("kind") as? String
}

/** The connection closed before or while this request ran; the connection status explains why. */
class WorkspaceClosedException(message: String, cause: Throwable? = null) : IOException(message, cause)

/** The Engine speaks another protocol or version. Retrying cannot help; version 1.0.0 has no adapter. */
class IncompatibleWorkspaceException(message: String) : IOException(message)

/** The Engine did not answer in time. Unlike a coroutine timeout, this is a failure, never a cancellation. */
class WorkspaceTimeoutException(val method: String, val timeoutMs: Long) : IOException("工作区响应超时")

/** Bounded JSONL transport. No vendor-agent messages, workspace reducers or persistence live here. */
class WorkspaceRpc(
    private val transport: WorkspaceTransport,
    private val scope: CoroutineScope,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) {
    private val ids = AtomicLong()
    private val waiting = ConcurrentHashMap<String, CompletableDeferred<JsonElement>>()
    private val writer = Mutex()
    private val handshake = Mutex()
    private var greeting: Pair<String, Map<String, Any?>>? = null
    private val mutableFailure = MutableStateFlow<String?>(null)
    val failure: StateFlow<String?> = mutableFailure.asStateFlow()
    private val output = transport.output.buffered(64 * 1024)
    private val transportClosed = AtomicBoolean()
    @Volatile private var closed = false
    private val reader = scope.launch(io) {
        try {
            val input = transport.input.buffered(64 * 1024)
            val line = ByteArrayOutputStream()
            val decoder = Charsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT).onUnmappableCharacter(CodingErrorAction.REPORT)
            while (isActive) {
                val byte = input.read()
                if (byte < 0) throw IOException("工作区连接已断开")
                if (byte != 10) {
                    if (line.size() >= MAX_FRAME_BYTES) throw IOException("工作区响应超过协议大小限制")
                    line.write(byte)
                    continue
                }
                if (line.size() == 0) continue
                val message = RpcEnvelope.parse(decoder.decode(ByteBuffer.wrap(line.toByteArray())).toString())
                line.reset()
                if (message.id == EnvelopeId.Missing) continue
                check(message.method == null) { "Workspace response is a request" }
                val id = (message.id as? EnvelopeId.Text)?.value ?: throw IOException("Invalid workspace response ID")
                val error = message.error?.let {
                    WorkspaceRpcException(it.code, it.message, it.data?.let { data -> Json.parse(data.json.toString()) })
                }
                val deferred = waiting.remove(id) ?: continue // Expired response; never reinterpret as a new request.
                if (error != null) deferred.completeExceptionally(error) else deferred.complete(message.result!!.json)
            }
        } catch (cancelled: CancellationException) { fail("工作区连接已关闭"); throw cancelled }
        catch (error: Exception) { fail(error.message ?: "工作区连接已断开") }
        finally { closeTransport() }
    }

    suspend fun hello(clientId: String): Map<String, Any?> = handshake.withLock {
        greeting?.let { (previous, response) ->
            check(previous == clientId) { "Workspace connection already belongs to another client" }
            check(!closed) { failure.value ?: "工作区连接已关闭" }
            return@withLock response
        }
        try {
            require(clientId.isNotBlank()) { "Workspace client id must not be empty" }
            val result = obj(request("hello", mapOf("protocol" to PROTOCOL, "clientId" to clientId)))
            if (result["protocol"] != PROTOCOL) throw IncompatibleWorkspaceException("工作区协议版本与应用不一致")
            if (WorkspaceWire.string(result, "engineVersion") != ENGINE_VERSION) throw IncompatibleWorkspaceException("工作区 Engine 版本与应用不一致")
            check(WorkspaceWire.string(result, "workspaceRoot").isNotBlank()) { "Workspace omitted its root" }
            val capabilities = obj(result["capabilities"])
            for (key in listOf("workspace", "files", "documents", "environment", "processes", "pty", "services")) {
                check(capabilities[key] == true) { "Workspace does not provide $key" }
            }
            check(WorkspaceWire.long(capabilities, "maxFrameBytes") == MAX_FRAME_BYTES.toLong() &&
                WorkspaceWire.long(capabilities, "maxBlobChunkBytes") == 65_536L) { "Workspace protocol limits do not match" }
            greeting = clientId to result
            result
        } catch (error: Exception) {
            fail(error.message ?: "工作区握手失败"); closeTransport(); reader.cancel()
            throw error
        }
    }

    suspend fun <P : WireValue, R : WireValue> request(method: RpcMethod<P, R>, params: P, timeoutMs: Long = 30_000): R =
        method.decodeResult(requestElement(method.name, params.json, timeoutMs))

    suspend fun request(method: String, params: Map<String, Any?> = emptyMap(), timeoutMs: Long = 30_000): Any? =
        Json.parse(requestElement(method, parseWire(Json.stringify(params)), timeoutMs).toString())

    private suspend fun requestElement(method: String, params: JsonElement, timeoutMs: Long): JsonElement {
        val id = "r${ids.incrementAndGet()}"
        val deferred = CompletableDeferred<JsonElement>()
        synchronized(waiting) {
            if (closed) throw WorkspaceClosedException(mutableFailure.value ?: "工作区连接已关闭")
            waiting[id] = deferred
        }
        try {
            val bytes = RpcEnvelope.request(id, method, params).toString().toByteArray(Charsets.UTF_8)
            require(bytes.size <= MAX_FRAME_BYTES) { "工作区请求超过协议大小限制" }
            return withTimeoutOrNull(timeoutMs) {
                withContext(io) {
                    writer.withLock {
                        if (closed) throw WorkspaceClosedException(mutableFailure.value ?: "工作区连接已关闭")
                        try { output.write(bytes); output.write(10); output.flush() }
                        catch (error: IOException) {
                            fail(error.message ?: "工作区写入失败"); closeTransport()
                            throw WorkspaceClosedException(mutableFailure.value ?: "工作区写入失败", error)
                        }
                    }
                }
                deferred.await()
            } ?: throw WorkspaceTimeoutException(method, timeoutMs)
        } finally { waiting.remove(id) }
    }

    private fun fail(message: String) = synchronized(waiting) {
        if (!closed) {
            closed = true
            mutableFailure.value = message
            waiting.values.forEach { it.completeExceptionally(WorkspaceClosedException(message)) }
            waiting.clear()
        }
    }

    private fun closeTransport() {
        if (transportClosed.compareAndSet(false, true)) runCatching { transport.close() }
    }

    fun close() {
        fail("工作区连接已关闭")
        closeTransport()
        reader.cancel()
    }

    companion object {
        const val PROTOCOL = "workflow.workspace/1"
        const val ENGINE_VERSION = "1.0.0"
        const val MAX_FRAME_BYTES = 32 * 1024 * 1024
        @Suppress("UNCHECKED_CAST") fun obj(value: Any?): Map<String, Any?> = value as? Map<String, Any?> ?: throw IOException("工作区响应不是对象")
    }
}
