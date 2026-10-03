package top.flysoftbeta.workflow.engine.chat

import java.util.Base64
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.agent.model.*
import top.flysoftbeta.workflow.agent.rpc.*

class ChatJournalTest {
    @Test fun reconnectReplaysOrderedEventsAndExpiredCursorRequiresSnapshot() = runBlocking {
        val journal = ChatJournal(maxEvents = 2)
        val initial = journal.snapshot()
        journal.event(AgentEvent.ProcessChanged(BackendKind.CODEX, ProcessState.Starting))
        val oldEpoch = journal.processEpoch(BackendKind.CODEX)
        journal.event(AgentEvent.ProcessChanged(BackendKind.CODEX, ProcessState.Ready))
        val update = journal.watch(initial.epoch, initial.revision, 0)
        assertFalse(update.resnapshot)
        assertEquals(journal.snapshot().state, update.events.fold(initial.state, AgentReducer::reduce))
        journal.event(AgentEvent.ProcessChanged(BackendKind.CODEX, ProcessState.Starting))
        assertNotEquals(oldEpoch, journal.processEpoch(BackendKind.CODEX))
        assertTrue(journal.watch(initial.epoch, initial.revision, 0).resnapshot)
        assertTrue(journal.watch("old-service", 0, 0).resnapshot)
    }
    @Test fun restartedBackendRejectsOldCardEvenWhenVendorReusesItsId() {
        val journal = ChatJournal()
        val key = RequestKey(BackendKind.CODEX, kotlinx.serialization.json.JsonPrimitive(7))
        val request = PendingRequest(key, "future/request", RequestKind.Unknown("future/request", null), emptyList())
        journal.event(AgentEvent.ProcessChanged(BackendKind.CODEX, ProcessState.Starting))
        val oldEpoch = journal.processEpoch(BackendKind.CODEX)!!
        journal.event(AgentEvent.RequestOpened(request))
        journal.validateResponse(key, oldEpoch)
        journal.event(AgentEvent.ProcessChanged(BackendKind.CODEX, ProcessState.Exited(1)))
        journal.event(AgentEvent.ProcessChanged(BackendKind.CODEX, ProcessState.Starting))
        journal.event(AgentEvent.RequestOpened(request))
        assertTrue(runCatching { journal.validateResponse(key, oldEpoch) }.isFailure)
        journal.validateResponse(key, journal.processEpoch(BackendKind.CODEX)!!)
    }
    @Test fun frozenSnapshotChunksSurviveLiveUpdates() {
        val journal = ChatJournal()
        val large = "界".repeat(500_000)
        journal.event(AgentEvent.Unknown(BackendKind.CODEX, "large", null, kotlinx.serialization.json.JsonPrimitive(large)))
        val expected = journal.snapshot()
        var chunk = ChatWire.decode<ChatSnapshotChunk>(journal.snapshotResult())
        val output = java.io.ByteArrayOutputStream()
        journal.event(AgentEvent.ProcessChanged(BackendKind.CODEX, ProcessState.Ready))
        while (true) {
            output.write(Base64.getDecoder().decode(chunk.data))
            if (chunk.eof) break
            chunk = ChatWire.decode(journal.snapshotResult(chunk.transferId, chunk.nextOffset))
        }
        assertEquals(expected, ChatWire.parse<ChatSnapshot>(output.toString(Charsets.UTF_8)))
    }
}
