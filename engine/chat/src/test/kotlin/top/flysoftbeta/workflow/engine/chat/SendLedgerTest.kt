package top.flysoftbeta.workflow.engine.chat

import kotlinx.coroutines.*
import kotlinx.serialization.json.*
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.core.resource.ComposerDraft

class SendLedgerTest {
    @Test fun concurrentRetriesSubmitOnceAndAcknowledgeExactDraft() = runBlocking {
        val documents = mutableMapOf<String, String>()
        val submitted = ComposerDraft("conversation", 3, "hello")
        var calls = 0
        val acknowledgements = mutableListOf<ComposerDraft>()
        val ledger = SendLedger({ documents[it] }, { key, body -> documents[key] = body }, { acknowledgements += it })
        val args = buildJsonObject { put("text", "hello") }
        val results = (1..10).map { async { ledger.send("op", args, submitted) { calls++; delay(5); "accepted-id" } } }.awaitAll()
        assertEquals(1, calls)
        assertTrue(results.all { it == "accepted-id" })
        assertTrue(acknowledgements.all { it == submitted })
        val reopened = SendLedger({ documents[it] }, { key, body -> documents[key] = body }, { acknowledgements += it })
        assertEquals("accepted-id", reopened.send("op", args, submitted) { error("must not replay") })
    }
    @Test fun ambiguousSubmissionNeverReplaysAndChangedArgumentsAreRejected() = runBlocking {
        val documents = mutableMapOf<String, String>()
        var calls = 0
        var acknowledgements = 0
        val ledger = SendLedger({ documents[it] }, { key, body -> documents[key] = body }, { acknowledgements++ })
        val draft = ComposerDraft("c", 1, "hello")
        val args = buildJsonObject { put("text", "hello") }
        assertTrue(runCatching { ledger.send("op", args, draft) { calls++; error("lost vendor reply") } }.isFailure)
        assertTrue(runCatching { ledger.send("op", args, draft) { calls++; "duplicated" } }.isFailure)
        assertTrue(runCatching { ledger.send("op", buildJsonObject { put("text", "different") }, draft) { calls++; "different" } }.isFailure)
        assertEquals(1, calls)
        assertEquals(0, acknowledgements)
    }
    @Test fun acceptedSubmissionSurvivesAcknowledgementFailure() = runBlocking {
        val documents = mutableMapOf<String, String>()
        var failAck = true
        val ledger = SendLedger({ documents[it] }, { key, body -> documents[key] = body }, { if (failAck) error("ack disconnected") })
        val args = buildJsonObject { put("text", "hello") }
        val draft = ComposerDraft("c", 1, "hello")
        assertTrue(runCatching { ledger.send("op", args, draft) { "accepted" } }.isFailure)
        failAck = false
        assertEquals("accepted", ledger.send("op", args, draft) { error("must not replay") })
    }
}
