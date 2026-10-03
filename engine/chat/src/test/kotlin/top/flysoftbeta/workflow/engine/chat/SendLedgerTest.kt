package top.flysoftbeta.workflow.engine.chat

import kotlinx.coroutines.*
import kotlinx.serialization.json.*
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.core.resource.ComposerDraft

class SendLedgerTest {
    private fun args(operationId: String, text: String = "hello") = buildJsonObject { put("operationId", operationId); put("text", text) }

    @Test fun concurrentClientsSubmitOneDraftOnceAndAcknowledgeExactDraft() = runBlocking {
        val documents = mutableMapOf<String, String>()
        val submitted = ComposerDraft("conversation", 3, "hello")
        var calls = 0
        val acknowledgements = mutableListOf<ComposerDraft>()
        val ledger = SendLedger({ documents[it] }, { key, body -> documents[key] = body }, { acknowledgements += it })
        val results = (1..10).map { client -> async { ledger.send("op-$client", args("op-$client"), submitted) { calls++; delay(5); "accepted-id" } } }.awaitAll()
        assertEquals(1, calls)
        assertTrue(results.all { it == "accepted-id" })
        assertTrue(acknowledgements.all { it == submitted })
        val reopened = SendLedger({ documents[it] }, { key, body -> documents[key] = body }, { acknowledgements += it })
        assertEquals("accepted-id", reopened.send("recreated-client", args("recreated-client"), submitted) { error("must not replay") })
    }

    @Test fun ambiguousSubmissionCannotReplayWithANewClientId() = runBlocking {
        val documents = mutableMapOf<String, String>()
        var calls = 0
        var acknowledgements = 0
        val ledger = SendLedger({ documents[it] }, { key, body -> documents[key] = body }, { acknowledgements++ })
        val draft = ComposerDraft("c", 1, "hello")
        assertTrue(runCatching { ledger.send("op", args("op"), draft) { calls++; error("lost vendor reply") } }.isFailure)
        val reopened = SendLedger({ documents[it] }, { key, body -> documents[key] = body }, { acknowledgements++ })
        assertTrue(runCatching { reopened.send("new-client", args("new-client"), draft) { calls++; "duplicated" } }.isFailure)
        assertEquals(1, calls)
        assertEquals(0, acknowledgements)
    }

    @Test fun changedArgumentsOrContentCannotReuseASubmittedRevision() = runBlocking {
        val documents = mutableMapOf<String, String>()
        val ledger = SendLedger({ documents[it] }, { key, body -> documents[key] = body }, {})
        val draft = ComposerDraft("c", 1, "hello")
        ledger.send("first", args("first"), draft) { "accepted" }
        assertTrue(runCatching { ledger.send("second", args("second", "changed"), draft) { "incorrect second dispatch" } }.isFailure)
        assertTrue(runCatching { ledger.send("third", args("third"), draft.copy(text = "changed")) { "incorrect third dispatch" } }.isFailure)
    }

    @Test fun aNewDraftRevisionAndAnotherConversationCanSendNormally() = runBlocking {
        val documents = mutableMapOf<String, String>()
        val ledger = SendLedger({ documents[it] }, { key, body -> documents[key] = body }, {})
        val draft = ComposerDraft("c", 1, "hello")
        var calls = 0
        ledger.send("op", args("op"), draft) { "accepted-${++calls}" }
        val next = draft.edit(text = "next")
        assertEquals("accepted-2", ledger.send("new-op", args("new-op", "next"), next) { "accepted-${++calls}" })
        assertEquals("accepted-3", ledger.send("other-conversation", args("other-conversation"), draft.copy(conversationId = "other")) { "accepted-${++calls}" })
    }

    @Test fun acceptedSubmissionSurvivesAcknowledgementFailureAndClientRecreation() = runBlocking {
        val documents = mutableMapOf<String, String>()
        val ledger = SendLedger({ documents[it] }, { key, body -> documents[key] = body }, { error("ack disconnected") })
        val draft = ComposerDraft("c", 1, "hello")
        assertTrue(runCatching { ledger.send("op", args("op"), draft) { "accepted" } }.isFailure)
        val reopened = SendLedger({ documents[it] }, { key, body -> documents[key] = body }, {})
        assertEquals("accepted", reopened.send("new-client", args("new-client"), draft) { error("must not replay") })
    }

    @Test fun interruptedIntentAcknowledgementCannotLeaveADispatchGap() = runBlocking {
        val documents = mutableMapOf<String, String>()
        val draft = ComposerDraft("c", 1, "hello")
        var calls = 0
        val ledger = SendLedger({ documents[it] }, { key, body -> documents[key] = body; error("lost durable intent receipt") }, {})
        assertTrue(runCatching { ledger.send("op", args("op"), draft) { calls++; "accepted" } }.isFailure)
        val reopened = SendLedger({ documents[it] }, { key, body -> documents[key] = body }, {})
        assertTrue(runCatching { reopened.send("new-client", args("new-client"), draft) { calls++; "duplicated" } }.isFailure)
        assertEquals(0, calls)
        assertEquals(1, documents.size)
    }
}
