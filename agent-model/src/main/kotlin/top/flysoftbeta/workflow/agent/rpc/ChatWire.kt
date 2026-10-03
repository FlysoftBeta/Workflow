package top.flysoftbeta.workflow.agent.rpc

import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.json.*
import top.flysoftbeta.workflow.agent.model.*

/** Engine-owned metadata, replaced as a unit on every update. */
@Serializable
data class ChatMetadata(
    val conversations: List<ConversationEntry> = emptyList(),
    val available: Set<BackendKind> = emptySet(),
    val loginMethods: Map<BackendKind, List<LoginMethod>> = emptyMap(),
    val permissions: PermissionPreset = PermissionPreset.ASK,
    val defaultBackend: BackendKind = BackendKind.CODEX,
    /** Changes on every backend process start; required when responding to a request. */
    val processEpochs: Map<BackendKind, String> = emptyMap(),
)

@Serializable
data class ChatSnapshot(val epoch: String, val revision: Long, val state: AgentState, val metadata: ChatMetadata)

@Serializable
data class ChatUpdate(
    val epoch: String,
    val revision: Long,
    val events: List<AgentEvent> = emptyList(),
    val metadata: ChatMetadata = ChatMetadata(),
    val resnapshot: Boolean = false,
)

/** Frozen snapshot transfer. Read subsequent chunks through chat.snapshot {transferId,offset}. */
@Serializable
data class ChatSnapshotChunk(val transferId: String, val offset: Int, val data: String, val nextOffset: Int, val eof: Boolean)

object ChatWire {
    // Structured map keys are encoded as alternating key/value arrays, retaining numeric/string IDs.
    val json = Json { encodeDefaults = true; ignoreUnknownKeys = true; allowStructuredMapKeys = true; classDiscriminator = "_type" }
    inline fun <reified T> encode(value: T): JsonElement = json.encodeToJsonElement(value)
    inline fun <reified T> decode(value: JsonElement): T = json.decodeFromJsonElement(value)
    inline fun <reified T> stringify(value: T): String = json.encodeToString(value)
    inline fun <reified T> parse(value: String): T = json.decodeFromString(value)
    fun snapshot(value: JsonElement): ChatSnapshot = decode(value)
    fun update(value: JsonElement): ChatUpdate = decode(value)
}
