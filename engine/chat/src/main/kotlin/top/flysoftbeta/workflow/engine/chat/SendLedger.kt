package top.flysoftbeta.workflow.engine.chat

import java.security.MessageDigest
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.serialization.json.*
import top.flysoftbeta.workflow.agent.rpc.ChatWire
import top.flysoftbeta.workflow.core.connection.WorkspaceRpc
import top.flysoftbeta.workflow.core.resource.ComposerDraft
import top.flysoftbeta.workflow.core.store.StateCodec
import top.flysoftbeta.workflow.core.store.WorkspaceStore

/** Durable composer identity prevents client recreation or RPC retries from submitting a second vendor turn. */
class SendLedger(
    private val read: suspend (String) -> String?,
    private val write: suspend (String, String) -> Unit,
    private val acknowledge: suspend (ComposerDraft) -> Unit,
) {
    constructor(rpc: WorkspaceRpc, store: WorkspaceStore) : this(
        read = { key -> WorkspaceRpc.obj(rpc.request("documents.read", mapOf("namespace" to "chat", "key" to key)))["document"] as? String },
        write = { key, text -> rpc.request("documents.write", mapOf("namespace" to "chat", "key" to key, "document" to text)); Unit },
        acknowledge = { store.acknowledgeComposer(it); Unit },
    )
    private val locks = Array(128) { Mutex() }
    suspend fun send(operationId: String, arguments: JsonObject, submitted: ComposerDraft, action: suspend () -> String): String {
        require(operationId.isNotBlank() && operationId.length <= 200) { "A bounded operationId is required" }
        // One durable record is the dispatch gate. Transport IDs are correlation only: a recreated
        // client may use a different UUID for the exact same Engine-owned composer revision.
        val identity = buildJsonObject { put("conversationId", submitted.conversationId); put("revision", submitted.revision) }
        val key = "send-${digest(identity.toString())}"
        return locks[(key.hashCode() and Int.MAX_VALUE) % locks.size].withLock {
            val semantic = buildJsonObject {
                put("arguments", JsonObject(arguments.filterKeys { it != "operationId" }))
                put("submitted", ChatWire.json.parseToJsonElement(StateCodec.encodeComposer(submitted)))
            }
            val fingerprint = digest(canonical(semantic).toString())
            val prior = read(key)?.let { ChatWire.json.parseToJsonElement(it).jsonObject }
            if (prior != null) {
                check(prior["fingerprint"]?.jsonPrimitive?.content == fingerprint) { "Composer revision was already submitted with different content or settings" }
                val accepted = prior["clientMessageId"]?.jsonPrimitive?.content
                check(accepted != null) { "Submission outcome is ambiguous; inspect the conversation before sending a new message" }
                acknowledge(submitted)
                return@withLock accepted
            }
            fun record(clientMessageId: String? = null) = buildJsonObject {
                put("fingerprint", fingerprint)
                put("state", if (clientMessageId == null) "pending" else "accepted")
                clientMessageId?.let { put("clientMessageId", it) }
            }.toString()
            write(key, record())
            // Once dispatch begins, failures are ambiguous. Never erase the pending record or replay it.
            val accepted = action()
            write(key, record(accepted))
            acknowledge(submitted)
            accepted
        }
    }
    private fun canonical(value: JsonElement): JsonElement = when (value) {
        is JsonObject -> JsonObject(value.toSortedMap().mapValues { canonical(it.value) })
        is JsonArray -> JsonArray(value.map(::canonical))
        else -> value
    }
    private fun digest(value: String) = MessageDigest.getInstance("SHA-256").digest(value.toByteArray()).joinToString("") { "%02x".format(it) }
}
