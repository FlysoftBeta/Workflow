package top.flysoftbeta.workflow.agent.transport

import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.channels.ReceiveChannel
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import top.flysoftbeta.workflow.agent.json.str

class ControlException(val subtype: String, message: String, val response: JsonObject?) : Exception(message)

/**
 * Claude Code stream-json + control protocol (`--input-format stream-json --output-format
 * stream-json`). One JSON object per line in both directions.
 *
 * - `control_request` we send carry our own `request_id`; the matching `control_response`
 *   completes [control]. The CLI echoes our *own* control responses back on stdout, so a
 *   `control_response` whose id we are not waiting on is ignored (counted in [ignoredResponses]).
 * - `control_request` from the CLI (can_use_tool, hook_callback, elicitation, …) are surfaced on
 *   [inbound] with their `request_id` kept verbatim and answered only through [respond] /
 *   [respondError] by the owner.
 * - `control_cancel_request` withdraws a CLI request; the id is forgotten and surfaced.
 * - `keep_alive` is dropped; everything else is an [Inbound.Message].
 */
class ControlConnection(
    private val lines: JsonLineChannel,
    scope: CoroutineScope,
    capacity: Int = 1024,
    private val newRequestId: () -> String = defaultIds(),
) {
    sealed interface Inbound {
        data class Message(val type: String, val raw: JsonObject) : Inbound
        data class ControlRequest(val requestId: String, val subtype: String, val request: JsonObject, val raw: JsonObject) : Inbound
        data class ControlCancel(val requestId: String, val raw: JsonObject) : Inbound
        data class Malformed(val preview: String, val reason: String) : Inbound
    }

    private val pendingOut = ConcurrentHashMap<String, CompletableDeferred<JsonObject>>()
    private val pendingIn = ConcurrentHashMap<String, String>()
    private val channel = Channel<Inbound>(capacity)
    val inbound: ReceiveChannel<Inbound> get() = channel
    private val ignored = AtomicLong()
    val ignoredResponses: Long get() = ignored.get()
    @Volatile var closedReason: String? = null
        private set

    val openInboundRequests: Set<String> get() = pendingIn.keys.toSet()

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
        when (val type = frame["type"].str) {
            "control_response" -> {
                val response = frame["response"] as? JsonObject
                val waiter = response?.get("request_id").str?.let { pendingOut.remove(it) }
                if (response == null || waiter == null) { ignored.incrementAndGet(); return }
                if (response["subtype"].str == "error") {
                    waiter.completeExceptionally(ControlException("error", response["error"].str ?: "control request failed", response))
                } else waiter.complete(response["response"] as? JsonObject ?: JsonObject(emptyMap()))
            }
            "control_request" -> {
                val id = frame["request_id"].str
                val request = frame["request"] as? JsonObject
                val subtype = request?.get("subtype").str
                if (id == null || request == null || subtype == null) {
                    channel.send(Inbound.Message("control_request", frame))
                    return
                }
                pendingIn[id] = subtype
                channel.send(Inbound.ControlRequest(id, subtype, request, frame))
            }
            "control_cancel_request" -> {
                val id = frame["request_id"].str
                if (id != null) pendingIn.remove(id)
                channel.send(if (id != null) Inbound.ControlCancel(id, frame) else Inbound.Message("control_cancel_request", frame))
            }
            "keep_alive" -> Unit
            else -> channel.send(Inbound.Message(type ?: "", frame))
        }
    }

    /** Sends `control_request{subtype, …fields}` and returns the success `response` payload. */
    suspend fun control(subtype: String, fields: JsonObject = JsonObject(emptyMap())): JsonObject {
        closedReason?.let { throw ConnectionClosedException(it) }
        val id = newRequestId()
        val deferred = CompletableDeferred<JsonObject>()
        pendingOut[id] = deferred
        try {
            lines.send(buildJsonObject {
                put("type", "control_request")
                put("request_id", id)
                put("request", JsonObject(LinkedHashMap<String, JsonElement>().apply {
                    put("subtype", kotlinx.serialization.json.JsonPrimitive(subtype))
                    putAll(fields)
                }))
            })
            return deferred.await()
        } finally {
            pendingOut.remove(id)
        }
    }

    /** Writes a stream-json message (e.g. a `user` message). */
    suspend fun send(message: JsonObject) = lines.send(message)

    suspend fun respond(requestId: String, response: JsonObject?) {
        take(requestId)
        lines.send(buildJsonObject {
            put("type", "control_response")
            put("response", buildJsonObject {
                put("subtype", "success")
                put("request_id", requestId)
                if (response != null) put("response", response)
            })
        })
    }

    suspend fun respondError(requestId: String, error: String) {
        take(requestId)
        lines.send(buildJsonObject {
            put("type", "control_response")
            put("response", buildJsonObject {
                put("subtype", "error")
                put("request_id", requestId)
                put("error", error)
            })
        })
    }

    /** Forget an inbound request without answering (e.g. undeclared `request_user_dialog`). */
    fun forget(requestId: String) {
        pendingIn.remove(requestId)
    }

    private fun take(requestId: String) {
        checkNotNull(pendingIn.remove(requestId)) { "no open control request $requestId" }
    }

    companion object {
        fun defaultIds(): () -> String {
            val counter = AtomicLong()
            val prefix = UUID.randomUUID().toString().substring(0, 8)
            return { "wf_${prefix}_${counter.incrementAndGet()}" }
        }
    }
}
