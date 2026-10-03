package top.flysoftbeta.workflow.feature.chat

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import top.flysoftbeta.workflow.agent.json.get
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.model.AgentMessageItem
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.CommandAction
import top.flysoftbeta.workflow.agent.model.CommandItem
import top.flysoftbeta.workflow.agent.model.Decision
import top.flysoftbeta.workflow.agent.model.DecisionKind
import top.flysoftbeta.workflow.agent.model.FileChangeItem
import top.flysoftbeta.workflow.agent.model.FileChangeKind
import top.flysoftbeta.workflow.agent.model.FileDelta
import top.flysoftbeta.workflow.agent.model.ItemStatus
import top.flysoftbeta.workflow.agent.model.MessagePhase
import top.flysoftbeta.workflow.agent.model.PendingRequest
import top.flysoftbeta.workflow.agent.model.RequestKey
import top.flysoftbeta.workflow.agent.model.RequestKind
import top.flysoftbeta.workflow.agent.model.RequestStatus
import top.flysoftbeta.workflow.agent.model.StreamText
import top.flysoftbeta.workflow.agent.model.ThreadKey
import top.flysoftbeta.workflow.agent.model.ThreadState
import top.flysoftbeta.workflow.agent.model.Turn
import top.flysoftbeta.workflow.agent.model.TurnStatus
import top.flysoftbeta.workflow.agent.model.UnknownItem
import top.flysoftbeta.workflow.agent.model.UserMessageItem
import top.flysoftbeta.workflow.agent.model.UserPart
import top.flysoftbeta.workflow.feature.chat.transcript.NativeTranscriptProjector
import top.flysoftbeta.workflow.feature.chat.transcript.TranscriptProjector
import top.flysoftbeta.workflow.feature.chat.transcript.TurnView
import top.flysoftbeta.workflow.platform.agent.AgentPaths

class TranscriptTest {
    private val key = ThreadKey(BackendKind.CODEX, "thread")
    private val paths = AgentPaths("/workspace")

    private fun user(text: String, id: String = "u-$text") = UserMessageItem(id, listOf(UserPart.Text(text), UserPart.Image("/workspace/pics/a.png", "image/png")))
    private fun message(text: String, phase: MessagePhase = MessagePhase.UNKNOWN, status: ItemStatus = ItemStatus.IN_PROGRESS) =
        AgentMessageItem("m1", StreamText.of(text), phase, status)
    private fun thread(vararg turns: Turn) = ThreadState(key, turns = turns.toList())
    private fun project(p: TranscriptProjector, t: ThreadState, requests: List<PendingRequest> = emptyList()) = p.project(t, requests, emptyMap())

    @Test fun streamingBecomesTextAppendsAndCompletionFinalizes() {
        val p = TranscriptProjector(paths)
        val running = Turn("t1", status = TurnStatus.RUNNING, items = listOf(user("hi"), message("Hello $")), startedAtMs = 1000)
        val v1 = project(p, thread(running))
        assertEquals("running", v1.single().status)
        assertEquals("/workspace/pics/a.png", v1.single().user!!.attachments.single().src)
        val v2 = project(p, thread(running.copy(items = listOf(user("hi"), message("Hello \$x^2\$")))))
        assertEquals("Hello \$x^2\$", v2.single().items.single().text)
        assertEquals(v1.single().id, v2.single().id)

        val done = running.copy(status = TurnStatus.COMPLETED, durationMs = 5000,
            items = listOf(user("hi"), message("Hello \$x^2\$", MessagePhase.FINAL, ItemStatus.COMPLETED)))
        val v3 = project(p, thread(done))
        assertEquals("m1", v3.single().final)
        assertEquals("completed", v3.single().status)
    }

    @Test fun optimisticTurnKeepsItsIdWhenBound() {
        val p = TranscriptProjector(paths)
        val local = Turn("c1", clientMessageId = "c1", status = TurnStatus.QUEUED, items = listOf(user("hi")), bound = false)
        val v1 = project(p, thread(local))
        assertEquals("c1", v1.single().id)
        val bound = local.copy(id = "turn-9", status = TurnStatus.RUNNING, bound = true)
        val v2 = project(p, thread(bound))
        assertEquals("c1", v2.single().id)
        assertEquals("turn-9", v2.single().backendTurnId)
        assertEquals(v1.single().id, v2.single().id)
    }

    @Test fun queuedFollowUpsStayOutOfTheTranscriptWhileRunning() {
        val p = TranscriptProjector(paths)
        val running = Turn("t1", status = TurnStatus.RUNNING, items = listOf(user("a")))
        val queued = Turn("c2", clientMessageId = "c2", status = TurnStatus.QUEUED, items = listOf(user("b")), bound = false)
        assertEquals(listOf("t1"), project(p, thread(running, queued)).map { it.id })
        val idle = running.copy(status = TurnStatus.COMPLETED)
        assertEquals(listOf("t1", "c2"), project(p, thread(idle, queued)).map { it.id })
    }

    @Test fun settledRequestsBecomeRecordsAfterTheirItem() {
        val p = TranscriptProjector(paths)
        val command = CommandItem("exec", "/bin/zsh -lc \"touch a\"", actions = listOf(CommandAction("unknown", "touch a")), exitCode = 0, status = ItemStatus.COMPLETED)
        val turn = Turn("t1", status = TurnStatus.COMPLETED, items = listOf(user("go"), command, message("ok", MessagePhase.FINAL, ItemStatus.COMPLETED)))
        val request = PendingRequest(
            RequestKey(BackendKind.CODEX, JsonPrimitive(0)), "item/commandExecution/requestApproval",
            RequestKind.CommandApproval("touch a", null, null), listOf(Decision("accept", DecisionKind.ALLOW_ONCE)),
            threadId = "thread", turnId = "t1", itemId = "exec", status = RequestStatus.ANSWERED, answer = "accept",
        )
        val items = project(p, thread(turn), listOf(request)).single().items
        assertEquals(listOf("command", "record", "message"), items.map { it.kind })
        assertEquals("已批准 · 运行 touch a", items[1].fields["text"].str)
        assertEquals("已运行 touch a", items[0].fields["title"].str)
    }

    @Test fun fileChangesAggregateIntoTheCardAndUnknownItemsKeepJson() {
        val p = TranscriptProjector(paths)
        val files = FileChangeItem("patch", listOf(FileDelta("/workspace/app/Main.kt", FileChangeKind.UPDATE, "@@\n-a\n+b\n+c\n")), status = ItemStatus.COMPLETED)
        val unknown = UnknownItem("x", "futureThing", raw = JsonObject(mapOf("type" to JsonPrimitive("futureThing"))))
        val view = project(p, thread(Turn("t1", status = TurnStatus.COMPLETED, items = listOf(user("go"), files, unknown)))).single()
        val card = view.changes!!.single()
        assertEquals("app/Main.kt", card.path)
        assertEquals(2, card.added)
        assertEquals(1, card.removed)
        assertTrue(view.items.last().fields["detail"].str!!.contains("futureThing"))
    }

    @Test fun newAndOlderTurnsAreAppendedAndPrepended() {
        val p = TranscriptProjector(paths)
        val t1 = Turn("t1", status = TurnStatus.COMPLETED, items = listOf(user("a")))
        val t2 = Turn("t2", status = TurnStatus.COMPLETED, items = listOf(user("b")))
        val t0 = Turn("t0", status = TurnStatus.COMPLETED, items = listOf(user("z")))
        val native = NativeTranscriptProjector()
        val v1 = native.project(project(p, thread(t1)), false).rows.map { it.key }
        val appended = native.project(project(p, thread(t1, t2)), false).rows.map { it.key }
        assertEquals(v1, appended.take(v1.size))
        val prepended = native.project(project(p, thread(t0, t1, t2)), false).rows.map { it.key }
        assertEquals(appended, prepended.takeLast(appended.size))
    }

    @Test fun railGroupsByDay() {
        val zone = java.time.ZoneOffset.UTC
        val today = java.time.LocalDate.of(2026, 9, 29)
        fun at(date: java.time.LocalDate) = date.atStartOfDay(zone).toInstant().toEpochMilli() + 3600_000
        val entries = listOf(
            top.flysoftbeta.workflow.agent.model.ConversationEntry("a", BackendKind.CODEX, null, "A", "", 0, at(today)),
            top.flysoftbeta.workflow.agent.model.ConversationEntry("b", BackendKind.CODEX, null, "B", "", 0, at(today.minusDays(1))),
            top.flysoftbeta.workflow.agent.model.ConversationEntry("c", BackendKind.CLAUDE, null, "C", "", 0, at(today.minusDays(20))),
            top.flysoftbeta.workflow.agent.model.ConversationEntry("d", BackendKind.CODEX, null, "D", "", 0, at(today), archived = true),
        )
        val rows = RailModel.rows(entries, "", archived = false, today = today, zone = zone)
        assertEquals(listOf("今天", "a", "昨天", "b", "更早", "c"), rows.map {
            when (it) { is RailModel.Row.Header -> it.label; is RailModel.Row.Conversation -> it.entry.id }
        })
        assertEquals(listOf("d"), RailModel.rows(entries, "", archived = true, today = today, zone = zone).map { (it as RailModel.Row.Conversation).entry.id })
    }
}
