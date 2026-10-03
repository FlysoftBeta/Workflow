package top.flysoftbeta.workflow.agent.codex

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import top.flysoftbeta.workflow.agent.AgentStateStore
import top.flysoftbeta.workflow.agent.ThreadOptions
import top.flysoftbeta.workflow.agent.json.encode
import top.flysoftbeta.workflow.agent.json.get
import top.flysoftbeta.workflow.agent.json.parseJson
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.model.AgentMessageItem
import top.flysoftbeta.workflow.agent.model.AgentState
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.CommandItem
import top.flysoftbeta.workflow.agent.model.DecisionKind
import top.flysoftbeta.workflow.agent.model.ItemStatus
import top.flysoftbeta.workflow.agent.model.LoginState
import top.flysoftbeta.workflow.agent.model.MessagePhase
import top.flysoftbeta.workflow.agent.model.NoticeLevel
import top.flysoftbeta.workflow.agent.model.ReasoningItem
import top.flysoftbeta.workflow.agent.model.RequestKey
import top.flysoftbeta.workflow.agent.model.RequestKind
import top.flysoftbeta.workflow.agent.model.RequestResponse
import top.flysoftbeta.workflow.agent.model.RequestStatus
import top.flysoftbeta.workflow.agent.model.RunState
import top.flysoftbeta.workflow.agent.model.ThreadKey
import top.flysoftbeta.workflow.agent.model.ThreadState
import top.flysoftbeta.workflow.agent.model.TurnSettings
import top.flysoftbeta.workflow.agent.model.TurnStatus
import top.flysoftbeta.workflow.agent.model.UserMessageItem
import top.flysoftbeta.workflow.agent.model.UserPart
import top.flysoftbeta.workflow.agent.testing.Envelope
import top.flysoftbeta.workflow.agent.testing.FakeLauncher
import top.flysoftbeta.workflow.agent.testing.FakeProcess
import top.flysoftbeta.workflow.agent.testing.Fixtures
import top.flysoftbeta.workflow.agent.testing.ScriptedServer
import top.flysoftbeta.workflow.agent.testing.await

/**
 * Drives the real [CodexBackend] against the recorded Codex 0.157.0 session
 * (fixtures/codex/session.jsonl: markdown+math turn, command approval, interrupt, thread ops).
 */
class CodexBackendReplayTest {
    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())
    private val store = AgentStateStore()

    @After fun tearDown() = scope.cancel()

    private val thread = "01a0ddb6-d670-70f3-9a62-1320d2a3b41a"
    private val t1 = "01a0ddb6-d741-7293-9b32-d5164341d611"
    private val t2 = "01a0ddb6-fe01-77a2-a0a1-deff66d7d2c2"
    private val t3 = "01a0ddb7-1f2b-7f51-a263-e57807c042d3"

    /** The research client did not send approvalsReviewer, so the server echoed config's auto_review. */
    private fun asIfReviewerHonoured(envelopes: List<Envelope>) = envelopes.map { e ->
        if (e.dir != "in") e else e.copy(msg = parseJson(e.msg.encode().replace("\"approvalsReviewer\":\"auto_review\"", "\"approvalsReviewer\":\"user\"")) as JsonObject)
    }

    private fun backend(envelopes: List<Envelope>): Triple<CodexBackend, FakeLauncher, ScriptedServer> {
        val server = ScriptedServer(envelopes, ScriptedServer.Protocol.JSON_RPC)
        val launcher = FakeLauncher { _, spec -> FakeProcess(spec) { p, f -> server.react(p, f) }.also(server::start) }
        val ids = generateSequence(1) { it + 1 }.map { "client-msg-$it" }.iterator()
        val config = CodexConfig(
            executable = "/opt/workflow/libcodex.so", codexHome = "/home/work/.codex-app",
            env = mapOf("PATH" to "/usr/bin", "HOME" to "/home/work", "OPENAI_API_KEY" to "host-secret", "CLAUDE_CODE_ENTRYPOINT" to "x"),
        )
        return Triple(CodexBackend(launcher, config, store, scope, clock = { 1_790_426_138_000 }, newId = { ids.next() }), launcher, server)
    }

    private fun AgentState.t(): ThreadState = threads.getValue(ThreadKey(BackendKind.CODEX, thread))

    @Test fun fullSessionReplay() = runBlocking {
        val (codex, launcher, server) = backend(asIfReviewerHonoured(Fixtures.envelopes("codex/session.jsonl")))
        codex.start()

        // Launch contract: app-owned CODEX_HOME, host credentials never forwarded.
        val spec = launcher.launched.single().spec
        assertEquals(listOf("/opt/workflow/libcodex.so", "app-server"), spec.argv)
        assertEquals("/home/work/.codex-app", spec.env["CODEX_HOME"])
        assertFalse(spec.env.containsKey("OPENAI_API_KEY"))
        assertFalse(spec.env.keys.any { it.startsWith("CLAUDE_") })

        store.await(what = "account, models, limits") { s ->
            val b = s.backend(BackendKind.CODEX)
            b.account.state == LoginState.LOGGED_IN && b.models != null && b.rateLimits != null
        }
        val status = store.state.value.backend(BackendKind.CODEX)
        assertEquals("chatgpt", status.account.method)
        assertEquals("prolite", status.account.plan)
        assertEquals(false, status.rateLimits!!.ordinaryUsageAllowed)
        assertNotNull(status.rateLimits!!.upsell)
        assertEquals(7, status.models!!.models.size)
        assertEquals(listOf("low", "medium", "high", "xhigh", "max", "ultra"), status.models!!.model("gpt-6-astra")!!.efforts.map { it.id })

        val id = codex.startThread(ThreadOptions("/workspace", TurnSettings(model = "gpt-reserve")))
        assertEquals(thread, id)

        // ---- turn 1: markdown + math
        codex.send(thread, listOf(UserPart.Text("Answer in Markdown only…")), TurnSettings(model = "gpt-reserve", effort = "low"))
        store.await(what = "turn 1 completed") { it.t().turn(t1)?.status == TurnStatus.COMPLETED }
        val turn1 = store.state.value.t().turn(t1)!!
        assertEquals(1, turn1.items.count { it is UserMessageItem })
        assertFalse((turn1.items.first() as UserMessageItem).local)
        assertEquals("client-msg-1", turn1.clientMessageId)
        val reasoning = turn1.items.filterIsInstance<ReasoningItem>().single()
        assertEquals("**Planning concise markdown response**", reasoning.summary.single().toString())
        val answer = turn1.finalMessage!!
        assertEquals(ItemStatus.COMPLETED, answer.status)
        assertTrue(answer.text.toString().contains("\$e^{i\\pi}+1=0\$"))
        assertTrue(answer.text.toString().contains("\$\$x=\\frac{-b\\pm\\sqrt{b^2-4ac}}{2a}\$\$"))
        assertEquals(14781L, turn1.usage!!.totalTokens)

        // ---- turn 2: command approval — never answered without the user
        codex.send(thread, listOf(UserPart.Text("Create a file named hello.txt…")), TurnSettings(model = "gpt-reserve", effort = "low"))
        val key = RequestKey(BackendKind.CODEX, JsonPrimitive(0))
        store.await(what = "approval request") { it.requests[key]?.status == RequestStatus.PENDING }
        val request = store.state.value.requests.getValue(key)
        assertEquals(t2, request.turnId)
        assertEquals("exec-ce9fed29-e98f-49e8-8463-345c427750b8", request.itemId)
        assertEquals(listOf("accept", "acceptWithExecpolicyAmendment", "cancel"), request.decisions.map { it.id })
        assertEquals(DecisionKind.ALLOW_PERSISTENT, request.decisions[1].kind)
        assertEquals("/bin/zsh -lc printf '%s\\n' 'hi from codex' > hello.txt", request.decisions[1].detail)
        assertTrue((request.kind as RequestKind.CommandApproval).command!!.contains("hello.txt"))
        assertEquals(RunState.WAITING_APPROVAL, store.state.value.t().runState)
        Thread.sleep(200)
        val process = launcher.launched.single()
        assertTrue("nothing may answer id 0 before the user", process.written.none { it["id"] == JsonPrimitive(0) })
        assertEquals(RequestStatus.PENDING, store.state.value.requests.getValue(key).status)
        try {
            codex.respond(key, RequestResponse.Decide("decline"))
            fail("decline was not offered by the server")
        } catch (_: IllegalArgumentException) {}
        codex.respond(key, RequestResponse.Decide("accept"))
        val answerFrame = process.written.single { it["id"] == JsonPrimitive(0) }
        assertEquals("""{"id":0,"result":{"decision":"accept"}}""", answerFrame.encode())
        store.await(what = "turn 2 completed") { it.t().turn(t2)?.status == TurnStatus.COMPLETED }
        val turn2 = store.state.value.t().turn(t2)!!
        val command = turn2.items.filterIsInstance<CommandItem>().single()
        assertEquals(ItemStatus.COMPLETED, command.status)
        assertEquals(0, command.exitCode)
        val messages = turn2.items.filterIsInstance<AgentMessageItem>()
        assertEquals(listOf(MessagePhase.COMMENTARY, MessagePhase.FINAL), messages.map { it.phase })
        assertEquals("Created `hello.txt` containing `hi from codex`.", turn2.finalMessage!!.text.toString())
        val closed = store.state.value.requests.getValue(key)
        assertEquals(RequestStatus.ANSWERED, closed.status)
        assertEquals("accept", closed.answer)

        // ---- turn 3: interrupt closes the half-streamed message
        codex.send(thread, listOf(UserPart.Text("Without using tools, write the integers from 1 to 300…")), TurnSettings(model = "gpt-reserve", effort = "low"))
        store.await(what = "streaming") { s -> s.t().turn(t3)?.items?.any { it is AgentMessageItem && it.text.length > 20 } == true }
        codex.interrupt(thread)
        store.await(what = "turn 3 interrupted") { it.t().turn(t3)?.status == TurnStatus.INTERRUPTED }
        val partial = store.state.value.t().turn(t3)!!.items.filterIsInstance<AgentMessageItem>().single()
        assertEquals(ItemStatus.INCOMPLETE, partial.status)
        assertTrue(partial.text.toString().startsWith("1 2 3 4 5"))
        assertEquals(RunState.IDLE, store.state.value.t().runState)

        // ---- thread operations
        codex.loadHistory(thread)
        assertEquals(listOf(t1, t2, t3), store.state.value.t().turns.map { it.id })
        assertNotNull("live turn kept over the summary", store.state.value.t().turn(t2)!!.items.filterIsInstance<CommandItem>().singleOrNull())
        val listed = codex.listThreads("/workspace")
        assertTrue(listed.any { it.id == thread })
        codex.rename(thread, "workflow protocol research")
        store.await(what = "renamed") { it.t().title == "workflow protocol research" }
        val fork = codex.forkThread(thread, null, ThreadOptions("/workspace", ephemeral = true))
        assertEquals("01a0ddb7-338f-73c0-a5c2-ee9522e2a52f", fork)
        assertEquals(thread, store.state.value.threads.getValue(ThreadKey(BackendKind.CODEX, fork)).forkedFrom)
        codex.archive(thread, true)
        store.await(what = "archived") { it.t().archived }
        assertEquals(RunState.NOT_LOADED, store.state.value.t().runState)

        // ---- invariants over everything the client wrote
        for (frame in process.written) {
            val method = frame["method"].str
            if (method in CodexParams.REVIEWER_METHODS) assertEquals("$method must route approvals to the user", "user", frame["params"]["approvalsReviewer"].str)
        }
        assertEquals("exactly one answer, the user's", 1, process.written.count { it["method"] == null && it["result"]["decision"] != null })
        assertTrue(process.written.filter { it["method"].str == "turn/start" }.all { it["params"]["clientUserMessageId"].str!!.startsWith("client-msg-") })
    }

    @Test fun inheritedAutoReviewerBlocksSending() = runBlocking {
        val (codex, _, _) = backend(Fixtures.envelopes("codex/session.jsonl"))
        codex.start()
        codex.startThread(ThreadOptions("/workspace"))
        val notice = store.state.value.t().notices.single { it.code == "approvalsReviewerNotUser" }
        assertEquals(NoticeLevel.ERROR, notice.level)
        try {
            codex.send(thread, listOf(UserPart.Text("hi")))
            fail("sending must be refused while approvals are routed to auto_review")
        } catch (_: IllegalStateException) {}
        assertNull(store.state.value.t().activeTurn)
    }
}
