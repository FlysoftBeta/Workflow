package top.flysoftbeta.workflow.agent.model

import kotlinx.serialization.json.JsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import top.flysoftbeta.workflow.agent.model.AgentEvent as E

class AgentReducerTest {
    private val B = BackendKind.CODEX
    private val key = ThreadKey(B, "th")

    private fun fold(vararg events: E, from: AgentState = AgentState()) = AgentReducer.reduceAll(from, events.toList())
    private fun AgentState.t() = threads.getValue(key)
    private fun request(id: Int, turn: String?, kind: RequestKind = RequestKind.CommandApproval("ls", "/", null)) = PendingRequest(
        RequestKey(B, JsonPrimitive(id)), "item/commandExecution/requestApproval", kind,
        listOf(Decision("accept", DecisionKind.ALLOW_ONCE)), threadId = "th", turnId = turn,
    )

    @Test fun queueSnapshotUpdatesOrderingWithoutCancellingInFlightOrRunningTurns() {
        val before = fold(
            E.TurnSubmitted(B, "th", "a", listOf(UserPart.Text("old")), null),
            E.TurnSubmitted(B, "th", "b", listOf(UserPart.Text("removed")), null),
            E.TurnSubmitted(B, "th", "sending", listOf(UserPart.Text("not acknowledged yet")), null),
            E.TurnStarted(B, "th", "live", "started"),
        )
        val updated = fold(E.QueueUpdated(B, "th", listOf(
            QueuedMessage("external", listOf(UserPart.Text("from another client"))),
            QueuedMessage("a", listOf(UserPart.Text("edited"))),
            QueuedMessage("started", listOf(UserPart.Text("stale snapshot"))),
        ), setOf("a", "b", "started")), from = before).t()
        assertEquals(listOf("sending", "external", "a"), updated.queuedTurns.map { it.clientMessageId })
        assertEquals("edited", (updated.turn("a")!!.items.single() as UserMessageItem).text)
        assertNull(updated.turn("b"))
        assertEquals(TurnStatus.RUNNING, updated.turn("live")!!.status)
    }

    @Test fun deltasAppendAndCompletionKeepsStreamedContentMissingFromPayload() {
        val s = fold(
            E.TurnStarted(B, "th", "t1"),
            E.ItemStarted(B, "th", "t1", CommandItem("c1", "ls")),
            E.ItemUpdated(B, "th", "t1", "c1", ItemDelta.CommandOutput("a\n")),
            E.ItemUpdated(B, "th", "t1", "c1", ItemDelta.CommandOutput("b\n")),
            E.ItemCompleted(B, "th", "t1", CommandItem("c1", "ls", exitCode = 0, status = ItemStatus.COMPLETED)),
            E.ItemUpdated(B, "th", "t1", "m1", ItemDelta.AgentText("he")), // placeholder created on first delta
            E.ItemUpdated(B, "th", "t1", "m1", ItemDelta.AgentText("llo")),
        )
        val turn = s.t().turn("t1")!!
        val cmd = turn.item("c1") as CommandItem
        assertEquals("a\nb\n", cmd.output.toString())
        assertEquals(0, cmd.exitCode)
        assertEquals(ItemStatus.COMPLETED, cmd.status)
        assertEquals("hello", (turn.item("m1") as AgentMessageItem).text.toString())
    }

    @Test fun interruptedTurnFinalisesDanglingItemsAndExpiresRequests() {
        val s = fold(
            E.TurnStarted(B, "th", "t1"),
            E.ItemStarted(B, "th", "t1", AgentMessageItem("m1", StreamText.of("1 2 3"), MessagePhase.FINAL)),
            E.ItemStarted(B, "th", "t1", ReasoningItem("r1")),
            E.RequestOpened(request(0, "t1")),
            E.RequestOpened(request(1, "other-turn")),
            E.TurnCompleted(B, "th", "t1", TurnStatus.INTERRUPTED),
        )
        val turn = s.t().turn("t1")!!
        assertEquals(TurnStatus.INTERRUPTED, turn.status)
        assertTrue(turn.items.all { it.status == ItemStatus.INCOMPLETE })
        assertEquals(RequestStatus.EXPIRED, s.requests.getValue(RequestKey(B, JsonPrimitive(0))).status)
        assertEquals("requests of other turns stay open", RequestStatus.PENDING, s.requests.getValue(RequestKey(B, JsonPrimitive(1))).status)
    }

    @Test fun reducerNeverAnswersRequests() {
        val opened = fold(E.TurnStarted(B, "th", "t1"), E.RequestOpened(request(0, "t1")))
        assertEquals(RunState.WAITING_APPROVAL, opened.t().runState)
        // Unrelated traffic does not close it.
        val later = fold(E.ItemUpdated(B, "th", "t1", "m", ItemDelta.AgentText("x")), E.TokenUsageChanged(B, "th", "t1", TokenUsage()), from = opened)
        assertEquals(RequestStatus.PENDING, later.requests.values.single().status)
        val answered = fold(E.RequestClosed(RequestKey(B, JsonPrimitive(0)), RequestStatus.ANSWERED, "accept"), from = later)
        assertEquals(RunState.RUNNING, answered.t().runState)
        // A server closure after the answer does not overwrite it; an answer after a server closure wins.
        val resolved = fold(E.RequestClosed(RequestKey(B, JsonPrimitive(0)), RequestStatus.RESOLVED), from = answered)
        assertEquals(RequestStatus.ANSWERED, resolved.requests.values.single().status)
        val race = fold(E.RequestClosed(RequestKey(B, JsonPrimitive(0)), RequestStatus.RESOLVED), E.RequestClosed(RequestKey(B, JsonPrimitive(0)), RequestStatus.ANSWERED, "accept"), from = later)
        assertEquals("accept", race.requests.values.single().answer)
    }

    @Test fun serverRequestIdsAreKeyedPerBackendAndVerbatim() {
        val codexInt = RequestKey(BackendKind.CODEX, JsonPrimitive(0))
        val codexString = RequestKey(BackendKind.CODEX, JsonPrimitive("0"))
        val claude = RequestKey(BackendKind.CLAUDE, JsonPrimitive(0))
        assertFalse(codexInt == codexString)
        assertFalse(codexInt == claude)
        assertEquals("codex:0", codexInt.text)
        assertEquals("codex:\"0\"", codexString.text)
    }

    @Test fun processExitFailsRunningTurnsAndExpiresRequestsOfThatBackendOnly() {
        val claudeKey = ThreadKey(BackendKind.CLAUDE, "cl")
        val s = fold(
            E.TurnSubmitted(B, "th", "q1", listOf(UserPart.Text("later")), null),
            E.TurnStarted(B, "th", "t1"),
            E.ItemStarted(B, "th", "t1", AgentMessageItem("m", StreamText.of("x"))),
            E.RequestOpened(request(0, "t1")),
            E.TurnStarted(BackendKind.CLAUDE, "cl", "c1"),
            E.ProcessChanged(B, ProcessState.Exited(9)),
        )
        val turn = s.t().turn("t1")!!
        assertEquals(TurnStatus.FAILED, turn.status)
        assertEquals("processExited", turn.error!!.code)
        assertEquals(ItemStatus.INCOMPLETE, turn.items.single().status)
        assertEquals(TurnStatus.CANCELLED, s.t().turn("q1")!!.status)
        assertEquals(RunState.NOT_LOADED, s.t().runState)
        assertEquals(RequestStatus.EXPIRED, s.requests.values.single().status)
        assertEquals(TurnStatus.RUNNING, s.threads.getValue(claudeKey).turn("c1")!!.status)
    }

    @Test fun optimisticTurnBindsWhateverArrivesFirst() {
        val submitted = E.TurnSubmitted(B, "th", "cm", listOf(UserPart.Image("/workspace/a.png")), TurnSettings(model = "m", effort = "low"))
        val echo = UserMessageItem("u1", listOf(UserPart.Image("a.png")), clientMessageId = "cm")
        val responseFirst = fold(submitted, E.TurnBound(B, "th", "cm", "T"), E.TurnStarted(B, "th", "T"), E.ItemCompleted(B, "th", "T", echo))
        val notificationFirst = fold(submitted, E.TurnStarted(B, "th", "T"), E.ItemCompleted(B, "th", "T", echo), E.TurnBound(B, "th", "cm", "T"))
        for (s in listOf(responseFirst, notificationFirst)) {
            val turn = s.t().turns.single()
            assertEquals("T", turn.id)
            assertEquals("cm", turn.clientMessageId)
            assertEquals(TurnSettings(model = "m", effort = "low"), turn.settings)
            val user = turn.items.filterIsInstance<UserMessageItem>().single()
            assertFalse(user.local)
            assertEquals("u1", user.id)
        }
        assertEquals(listOf(UserPart.Image("/workspace/a.png")), responseFirst.t().turns.single().items.filterIsInstance<UserMessageItem>().single().parts)
    }

    @Test fun steerMessageJoinsTheRunningTurn() {
        val s = fold(
            E.TurnStarted(B, "th", "T"),
            E.ItemCompleted(B, "th", "T", UserMessageItem("u1", listOf(UserPart.Text("first")), clientMessageId = "c1")),
            E.ItemStarted(B, "th", "T", AgentMessageItem("m", StreamText.EMPTY)),
            E.TurnSubmitted(B, "th", "c2", listOf(UserPart.Text("steer")), null),
            E.TurnBound(B, "th", "c2", "T"),
        )
        val turn = s.t().turns.single()
        assertEquals(listOf("u1", "m", "local:c2"), turn.items.map { it.id })
        val echoed = fold(E.ItemCompleted(B, "th", "T", UserMessageItem("u2", listOf(UserPart.Text("steer")), clientMessageId = "c2")), from = s)
        assertEquals(listOf("u1", "m", "u2"), echoed.t().turns.single().items.map { it.id })
    }

    @Test fun unknownPhasesResolveAtTurnEndAndToolUseMarksCommentary() {
        val s = fold(
            E.TurnStarted(BackendKind.CLAUDE, "th", "t"),
            E.ItemCompleted(BackendKind.CLAUDE, "th", "t", AgentMessageItem("a", StreamText.of("I'll run it"), status = ItemStatus.COMPLETED)),
            E.ItemStarted(BackendKind.CLAUDE, "th", "t", CommandItem("tool", "ls")),
            E.ItemCompleted(BackendKind.CLAUDE, "th", "t", AgentMessageItem("b", StreamText.of("done"), status = ItemStatus.COMPLETED)),
            E.ItemCompleted(BackendKind.CLAUDE, "th", "t", AgentMessageItem("sub", StreamText.of("nested"), status = ItemStatus.COMPLETED, parentId = "tool")),
            E.TurnCompleted(BackendKind.CLAUDE, "th", "t", TurnStatus.COMPLETED),
        )
        val items = s.threads.getValue(ThreadKey(BackendKind.CLAUDE, "th")).turn("t")!!.items.filterIsInstance<AgentMessageItem>()
        assertEquals(listOf(MessagePhase.COMMENTARY, MessagePhase.FINAL, MessagePhase.COMMENTARY), items.map { it.phase })
        assertEquals(ItemStatus.INCOMPLETE, s.threads.getValue(ThreadKey(BackendKind.CLAUDE, "th")).turn("t")!!.item("tool")!!.status)
    }

    @Test fun declinedSurvivesFailedCompletion() {
        val s = fold(
            E.TurnStarted(B, "th", "t"),
            E.ItemStarted(B, "th", "t", CommandItem("c", "rm -rf x")),
            E.ItemDeclined(B, "th", null, "c"),
            E.ItemCompleted(B, "th", "t", CommandItem("c", "rm -rf x", status = ItemStatus.FAILED)),
        )
        assertEquals(ItemStatus.DECLINED, s.t().turn("t")!!.item("c")!!.status)
    }

    @Test fun historyLoadKeepsRicherLiveTurnsAndPendingLocalTurns() {
        val live = fold(
            E.TurnStarted(B, "th", "t1"),
            E.ItemCompleted(B, "th", "t1", UserMessageItem("u", listOf(UserPart.Text("q")))),
            E.ItemCompleted(B, "th", "t1", ReasoningItem("r", status = ItemStatus.COMPLETED)),
            E.ItemCompleted(B, "th", "t1", AgentMessageItem("m", StreamText.of("a"), MessagePhase.FINAL, ItemStatus.COMPLETED)),
            E.TurnCompleted(B, "th", "t1", TurnStatus.COMPLETED),
            E.TurnSubmitted(B, "th", "pending", listOf(UserPart.Text("next")), null),
        )
        val history = listOf(
            Turn("t0", status = TurnStatus.COMPLETED),
            Turn("t1", status = TurnStatus.COMPLETED, items = listOf(UserMessageItem("u", listOf(UserPart.Text("q"))))),
        )
        val s = fold(E.HistoryLoaded(B, "th", history, cursor = "c"), from = live)
        assertEquals(listOf("t0", "t1", "pending"), s.t().turns.map { it.id })
        assertEquals(3, s.t().turn("t1")!!.items.size)
        assertEquals("c", s.t().historyCursor)
        val older = fold(E.HistoryLoaded(B, "th", listOf(Turn("t-1", status = TurnStatus.COMPLETED), Turn("t0", status = TurnStatus.COMPLETED)), prepend = true), from = s)
        assertEquals(listOf("t-1", "t0", "t1", "pending"), older.t().turns.map { it.id })
    }

    @Test fun commandOutputIsBounded() {
        var s = fold(E.TurnStarted(B, "th", "t"), E.ItemStarted(B, "th", "t", CommandItem("c", "yes")))
        val chunk = "y".repeat(64 * 1024)
        repeat(10) { s = AgentReducer.reduce(s, E.ItemUpdated(B, "th", "t", "c", ItemDelta.CommandOutput(chunk))) }
        val cmd = s.t().turn("t")!!.item("c") as CommandItem
        assertTrue(cmd.outputTruncated)
        assertEquals(AgentReducer.MAX_COMMAND_OUTPUT, cmd.output.length)
    }

    @Test fun unknownEventsArePreservedBounded() {
        var s = fold(E.ThreadUpserted(B, "th"))
        repeat(AgentReducer.MAX_UNKNOWN + 5) { s = AgentReducer.reduce(s, E.Unknown(B, "future/thing", "th", JsonPrimitive(it))) }
        assertEquals(AgentReducer.MAX_UNKNOWN, s.backend(B).unknown.size)
        assertEquals(JsonPrimitive(AgentReducer.MAX_UNKNOWN + 4), s.t().unknown.last().raw)
    }

    @Test fun rateLimitUpdatesMergeById() {
        val s = fold(
            E.RateLimitsChanged(B, RateLimitState(listOf(RateLimit("codex", reached = true), RateLimit("base_model_inference")), ordinaryUsageAllowed = false)),
            E.RateLimitsChanged(B, RateLimitState(listOf(RateLimit("codex", reached = false))), merge = true),
        )
        val limits = s.backend(B).rateLimits!!
        assertEquals(listOf("codex", "base_model_inference"), limits.limits.map { it.id })
        assertFalse(limits.limits.first().reached)
        assertEquals(false, limits.ordinaryUsageAllowed)
    }

    @Test fun authenticatedAccountRefreshClearsPendingLogin() {
        val pending = fold(E.LoginChanged(B, LoginFlow.DeviceCode("l1", "https://auth.example/device", "TEST-CODE")))
        val authenticated = fold(E.AccountChanged(B, AccountState(LoginState.LOGGED_IN, "chatgpt")), from = pending)
        assertNull(authenticated.backend(B).account.login)
        assertEquals(LoginState.LOGGED_IN, authenticated.backend(B).account.state)
    }

    @Test fun loginFlowTransitions() {
        val s = fold(
            E.AccountChanged(B, AccountState(LoginState.LOGGED_OUT)),
            E.LoginChanged(B, LoginFlow.DeviceCode("l1", "https://auth.example/device", "ABCD-EFGH")),
        )
        assertEquals(LoginState.LOGGING_IN, s.backend(B).account.state)
        val done = fold(E.LoginChanged(B, LoginFlow.Completed("l1", true, null)), E.AccountChanged(B, AccountState(LoginState.LOGGED_IN, "chatgpt")), from = s)
        assertEquals(LoginState.LOGGED_IN, done.backend(B).account.state)
        assertNull(done.backend(B).account.login)
    }
}
