package top.flysoftbeta.workflow.engine.chat

import java.util.ArrayDeque
import java.util.Base64
import java.util.UUID
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.withTimeoutOrNull
import kotlinx.serialization.json.JsonElement
import top.flysoftbeta.workflow.agent.model.*
import top.flysoftbeta.workflow.agent.rpc.*

/** Ordered replay journal. Clients only reduce authoritative events; gaps require a frozen snapshot. */
class ChatJournal(private val maxEvents: Int = 4096, private val maxBytes: Int = 8 * 1024 * 1024) {
    val epoch: String = UUID.randomUUID().toString()
    private var state = AgentState()
    private var metadata = ChatMetadata()
    private var revision = 0L
    private val changed = MutableStateFlow(0L)
    private data class Change(val revision: Long, val event: AgentEvent?, val bytes: Int, val metadata: ChatMetadata)
    private val history = ArrayDeque<Change>()
    private var historyBytes = 0
    private val transfers = LinkedHashMap<String, ByteArray>()

    @Synchronized fun event(event: AgentEvent) {
        state = AgentReducer.reduce(state, event)
        if (event is AgentEvent.ProcessChanged && event.state is ProcessState.Starting) {
            metadata = metadata.copy(processEpochs = metadata.processEpochs + (event.backend to UUID.randomUUID().toString()))
        }
        record(event)
    }
    @Synchronized fun metadata(value: ChatMetadata) {
        val updated = value.copy(processEpochs = metadata.processEpochs)
        if (updated == metadata) return
        metadata = updated
        record(null)
    }
    private fun record(event: AgentEvent?) {
        revision++
        val size = event?.let { ChatWire.stringify<AgentEvent>(it).toByteArray().size } ?: 0
        history.addLast(Change(revision, event, size, metadata))
        historyBytes += size
        while (history.size > maxEvents || historyBytes > maxBytes) historyBytes -= history.removeFirst().bytes
        changed.value = revision
    }
    @Synchronized fun snapshot(): ChatSnapshot = ChatSnapshot(epoch, revision, state, metadata)
    @Synchronized fun processEpoch(kind: BackendKind): String? = metadata.processEpochs[kind]

    @Synchronized fun validateResponse(key: RequestKey, processEpoch: String) {
        check(processEpoch == metadata.processEpochs[key.backend]) { "Approval belongs to an expired backend process" }
        check(state.requests[key]?.status?.isOpen == true) { "Request is no longer open" }
    }

    suspend fun watch(clientEpoch: String, after: Long, timeoutMs: Long): ChatUpdate {
        require(after >= 0 && timeoutMs in 0..30_000)
        if (clientEpoch == epoch && changed.value <= after) withTimeoutOrNull(timeoutMs) { changed.first { it > after } }
        return updates(clientEpoch, after)
    }
    @Synchronized private fun updates(clientEpoch: String, after: Long): ChatUpdate {
        if (clientEpoch != epoch || after > revision || (after < revision && (history.isEmpty() || after < history.first.revision - 1))) {
            return ChatUpdate(epoch, revision, metadata = metadata, resnapshot = true)
        }
        var bytes = 0
        val changes = history.asSequence().filter { it.revision > after }.takeWhile {
            bytes += it.bytes
            bytes <= UPDATE_BYTES
        }.take(512).toList()
        if (after < revision && changes.isEmpty()) return ChatUpdate(epoch, revision, metadata = metadata, resnapshot = true)
        // Metadata and request epochs are taken from exactly the last emitted revision.
        val next = changes.lastOrNull()?.revision ?: revision
        return ChatUpdate(epoch, next, changes.mapNotNull { it.event }, changes.lastOrNull()?.metadata ?: metadata)
    }
    @Synchronized fun snapshotResult(transferId: String? = null, offset: Int = 0): JsonElement {
        if (transferId != null) {
            val bytes = transfers[transferId] ?: error("Snapshot transfer expired; request a new snapshot")
            return chunk(transferId, bytes, offset)
        }
        require(offset == 0)
        val frozen = snapshot()
        val bytes = ChatWire.stringify(frozen).toByteArray(Charsets.UTF_8)
        if (bytes.size <= UPDATE_BYTES) return ChatWire.encode(frozen)
        require(bytes.size <= MAX_SNAPSHOT_BYTES) { "Chat snapshot exceeds transfer limit; load less history" }
        while (transfers.size >= 2) transfers.remove(transfers.keys.first())
        val id = UUID.randomUUID().toString()
        transfers[id] = bytes
        return chunk(id, bytes, 0)
    }
    private fun chunk(id: String, bytes: ByteArray, offset: Int): JsonElement {
        require(offset in 0..bytes.size)
        val next = (offset + CHUNK_BYTES).coerceAtMost(bytes.size)
        val result = ChatSnapshotChunk(id, offset, Base64.getEncoder().encodeToString(bytes.copyOfRange(offset, next)), next, next == bytes.size)
        return ChatWire.encode(result)
    }
    companion object {
        const val UPDATE_BYTES = 1024 * 1024
        const val CHUNK_BYTES = 65_536
        const val MAX_SNAPSHOT_BYTES = 64 * 1024 * 1024
    }
}
