package top.flysoftbeta.workflow.agent.testing

import kotlinx.coroutines.flow.first
import kotlinx.coroutines.withTimeout
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import top.flysoftbeta.workflow.agent.AgentStateStore
import top.flysoftbeta.workflow.agent.json.get
import top.flysoftbeta.workflow.agent.json.parseJson
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.model.AgentState

/** One recorded frame: research transcript envelope `{t, phase, dir, msg}`. */
data class Envelope(val phase: String, val dir: String, val msg: JsonObject)

object Fixtures {
    fun lines(name: String): List<String> =
        (Fixtures::class.java.getResourceAsStream("/fixtures/$name") ?: error("missing fixture $name"))
            .bufferedReader().readLines().filter { it.isNotBlank() }

    fun envelopes(name: String): List<Envelope> = lines(name).map { line ->
        val o = parseJson(line) as JsonObject
        Envelope(o["phase"].str ?: "", o["dir"].str ?: "", (o["msg"] as? JsonObject) ?: JsonObject(emptyMap()))
    }

    fun resource(path: String): String =
        (Fixtures::class.java.getResourceAsStream(path) ?: error("missing resource $path")).bufferedReader().readText()
}

suspend fun AgentStateStore.await(timeoutMs: Long = 10_000, what: String = "condition", predicate: (AgentState) -> Boolean): AgentState =
    try {
        withTimeout(timeoutMs) { state.first(predicate) }
    } catch (e: Exception) {
        throw AssertionError("timed out waiting for $what; state=${state.value.threads.values.map { t -> t.key to t.turns.map { it.id to it.status } }}", e)
    }

/**
 * Replays a recorded transcript as the server side of a fake process.
 *
 * Each recorded client frame starts a step that owns the server frames recorded after it. When
 * the client under test writes a frame, the first unconsumed step with the same key (JSON-RPC
 * method / control subtype / answered request id / user message) is emitted, with recorded ids
 * of *our* requests rewritten to the ids the client actually used. Server-originated ids are
 * emitted verbatim. Unscripted requests get the last recorded response of the same kind.
 */
class ScriptedServer(envelopes: List<Envelope>, private val protocol: Protocol) {
    enum class Protocol { JSON_RPC, CLAUDE_CONTROL }

    private class Step(val out: JsonObject, val inbound: MutableList<JsonObject> = mutableListOf(), var consumed: Boolean = false)

    private val preamble = mutableListOf<JsonObject>()
    private val steps = mutableListOf<Step>()
    val unmatched = java.util.concurrent.CopyOnWriteArrayList<JsonObject>()

    init {
        for (e in envelopes) when (e.dir) {
            "out" -> steps += Step(e.msg)
            "in" -> (steps.lastOrNull()?.inbound ?: preamble) += e.msg
        }
    }

    fun start(process: FakeProcess) = preamble.forEach(process::emit)

    val remainingSteps: Int get() = steps.count { !it.consumed }

    @Synchronized
    fun react(process: FakeProcess, frame: JsonObject) {
        when (protocol) {
            Protocol.JSON_RPC -> jsonRpc(process, frame)
            Protocol.CLAUDE_CONTROL -> control(process, frame)
        }
    }

    private fun jsonRpc(process: FakeProcess, frame: JsonObject) {
        val method = frame["method"].str
        val id = frame["id"]
        val step = when {
            method != null && id != null -> steps.firstOrNull { !it.consumed && it.out["method"].str == method && it.out["id"] != null }
            method != null -> steps.firstOrNull { !it.consumed && it.out["method"].str == method && it.out["id"] == null }
            id != null -> steps.firstOrNull { !it.consumed && it.out["method"] == null && it.out["id"] == id }
            else -> null
        }
        if (step == null) {
            unmatched += frame
            if (method != null && id != null) {
                val previous = steps.lastOrNull { it.out["method"].str == method && it.out["id"] != null }
                val response = previous?.inbound?.firstOrNull { it["method"] == null && it["id"] == previous.out["id"] }
                process.emit(if (response != null) JsonObject(response + ("id" to id))
                else JsonObject(mapOf("id" to id, "error" to JsonObject(mapOf("code" to JsonPrimitive(-32601), "message" to JsonPrimitive("unscripted $method"))))))
            }
            return
        }
        step.consumed = true
        val recordedId = step.out["id"]
        for (msg in step.inbound) {
            val isOurResponse = method != null && id != null && msg["method"] == null && msg["id"] == recordedId && ("result" in msg || "error" in msg)
            process.emit(if (isOurResponse) JsonObject(msg + ("id" to id)) else msg)
        }
    }

    private fun subtype(frame: JsonObject) = frame["request"]["subtype"].str

    private fun control(process: FakeProcess, frame: JsonObject) {
        val type = frame["type"].str
        val step = when (type) {
            "control_request" -> steps.firstOrNull { !it.consumed && it.out["type"].str == "control_request" && subtype(it.out) == subtype(frame) }
            "control_response" -> steps.firstOrNull { !it.consumed && it.out["type"].str == "control_response" &&
                it.out["response"]["request_id"] == frame["response"]["request_id"] }
            "user" -> steps.firstOrNull { !it.consumed && it.out["type"].str == "user" }
            else -> null
        }
        val ourId = frame["request_id"]
        if (step == null) {
            unmatched += frame
            if (type == "control_request") {
                val previous = steps.lastOrNull { it.out["type"].str == "control_request" && subtype(it.out) == subtype(frame) }
                val recorded = previous?.out?.get("request_id")
                val response = previous?.inbound?.firstOrNull { it["type"].str == "control_response" && it["response"]["request_id"] == recorded }
                process.emit(if (response != null) rewrite(response, ourId!!)
                else JsonObject(mapOf("type" to JsonPrimitive("control_response"), "response" to JsonObject(mapOf(
                    "subtype" to JsonPrimitive("error"), "request_id" to ourId!!, "error" to JsonPrimitive("unscripted ${subtype(frame)}"))))))
            }
            return
        }
        step.consumed = true
        val recordedId = step.out["request_id"]
        for (msg in step.inbound) {
            val isOurResponse = type == "control_request" && msg["type"].str == "control_response" && msg["response"]["request_id"] == recordedId
            process.emit(if (isOurResponse) rewrite(msg, ourId!!) else msg)
        }
    }

    private fun rewrite(response: JsonObject, id: JsonElement): JsonObject {
        val inner = response["response"] as JsonObject
        return JsonObject(response + ("response" to JsonObject(inner + ("request_id" to id))))
    }
}
