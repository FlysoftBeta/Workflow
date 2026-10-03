package top.flysoftbeta.workflow.agent.transport

import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.channels.ReceiveChannel
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import top.flysoftbeta.workflow.agent.json.encode
import top.flysoftbeta.workflow.agent.json.long
import top.flysoftbeta.workflow.agent.json.str

class RpcException(val code: Long, message: String, val data: JsonElement?) : Exception(message)

class ConnectionClosedException(message: String) : Exception(message)

/**
 * Codex App-Server JSON-RPC (JSONL, no `"jsonrpc"` member on the wire).
 *
 * Two independent id spaces:
 * - outbound: our numeric ids → [pendingOut]; a response completes the waiting [request] call;
 * - inbound: the server's request ids, kept as the raw [JsonElement] (int or string) in
 *   [pendingIn] and echoed verbatim by [respond]. A server request is never answered here;
 *   the owner decides (and for approvals, only the user decides).
 *
 * Responses are routed on the reader coroutine, not through [inbound], so a busy inbound
 * consumer can never block a pending [request].
 */
class JsonRpcConnection(private val lines: JsonLineChannel, scope: CoroutineScope, capacity: Int = 1024) {
    sealed interface Inbound {
        val raw: JsonObject
        data class Notification(val method: String, val params: JsonElement?, override val raw: JsonObject) : Inbound
        data class Request(val id: JsonElement, val method: String, val params: JsonElement?, override val raw: JsonObject) : Inbound
        /** Response for an id we are not waiting on, or a frame that fits no JSON-RPC shape. */
        data class Stray(val reason: String, override val raw: JsonObject) : Inbound
        data class Malformed(val preview: String, val reason: String) : Inbound {
            override val raw: JsonObject get() = JsonObject(emptyMap())
        }
    }

    private val nextId = AtomicLong(1)
    private val pendingOut = ConcurrentHashMap<Long, CompletableDeferred<JsonElement>>()
    private val pendingIn = ConcurrentHashMap<String, String>()
    private val channel = Channel<Inbound>(capacity)
    val inbound: ReceiveChannel<Inbound> get() = channel
    @Volatile var closedReason: String? = null
        private set

    /** Raw ids (JSON text) of server requests not answered yet. */
    val openServerRequests: Set<String> get() = pendingIn.keys.toSet()

    init {
        scope.launch {
            try {
                for (event in lines.events) {
                    when (event) {
                        is LineEvent.Frame -> route(event.json)
                        is LineEvent.Malformed -> channel.send(Inbound.Malformed(event.preview, event.reason))
                        is LineEvent.Closed -> closedReason = event.error?.message ?: "process closed stdout"
                    }
                }
            } finally {
                val reason = closedReason ?: "connection closed"
                closedReason = reason
                pendingOut.values.forEach { it.completeExceptionally(ConnectionClosedException(reason)) }
                pendingOut.clear()
                pendingIn.clear()
                channel.close()
            }
        }
    }

    private suspend fun route(frame: JsonObject) {
        val method = frame["method"].str
        val id = frame["id"]
        val isResponse = method == null && id != null && ("result" in frame || "error" in frame)
        when {
            method != null && id != null && id !is JsonNull -> {
                pendingIn[id.encode()] = method
                channel.send(Inbound.Request(id, method, frame["params"], frame))
            }
            method != null -> channel.send(Inbound.Notification(method, frame["params"], frame))
            isResponse -> {
                // Our ids are JSON numbers; a string id can never be ours.
                val waiter = id.long?.let { pendingOut.remove(it) }
                if (waiter == null) {
                    channel.send(Inbound.Stray("response for unknown id ${id.encode()}", frame))
                    return
                }
                val error = frame["error"] as? JsonObject
                if (error != null) {
                    waiter.completeExceptionally(RpcException(error["code"].long ?: -32000, error["message"].str ?: "error", error["data"]))
                } else waiter.complete(frame["result"] ?: JsonNull)
            }
            else -> channel.send(Inbound.Stray("not a JSON-RPC message", frame))
        }
    }

    /** Sends a request and suspends until its response. Throws [RpcException] / [ConnectionClosedException]. */
    suspend fun request(method: String, params: JsonElement?): JsonElement {
        closedReason?.let { throw ConnectionClosedException(it) }
        val id = nextId.getAndIncrement()
        val deferred = CompletableDeferred<JsonElement>()
        pendingOut[id] = deferred
        try {
            lines.send(buildJsonObject {
                put("id", id)
                put("method", method)
                if (params != null) put("params", params)
            })
            return deferred.await()
        } finally {
            pendingOut.remove(id)
        }
    }

    suspend fun notify(method: String, params: JsonElement? = null) {
        lines.send(buildJsonObject {
            put("method", method)
            if (params != null) put("params", params)
        })
    }

    /** Answers server request [id] (raw, verbatim). Fails if the id is not an open server request. */
    suspend fun respond(id: JsonElement, result: JsonElement) {
        take(id)
        lines.send(buildJsonObject { put("id", id); put("result", result) })
    }

    suspend fun respondError(id: JsonElement, code: Long, message: String, data: JsonElement? = null) {
        take(id)
        lines.send(buildJsonObject {
            put("id", id)
            put("error", buildJsonObject {
                put("code", code)
                put("message", message)
                if (data != null) put("data", data)
            })
        })
    }

    /** The server resolved request [id] itself (e.g. `serverRequest/resolved`); it must not be answered anymore. */
    fun forget(id: JsonElement) {
        pendingIn.remove(id.encode())
    }

    private fun take(id: JsonElement) {
        checkNotNull(pendingIn.remove(id.encode())) { "no open server request with id ${id.encode()}" }
    }

    companion object {
        const val METHOD_NOT_FOUND = -32601L
        const val INVALID_PARAMS = -32602L
        const val INTERNAL_ERROR = -32603L
    }
}
