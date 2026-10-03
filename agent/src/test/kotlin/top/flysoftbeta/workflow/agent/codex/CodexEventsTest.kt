package top.flysoftbeta.workflow.agent.codex

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import top.flysoftbeta.workflow.agent.json.get
import top.flysoftbeta.workflow.agent.json.parseJson
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.model.AgentReducer
import top.flysoftbeta.workflow.agent.model.AgentState
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.DecisionKind
import top.flysoftbeta.workflow.agent.model.NoticeItem
import top.flysoftbeta.workflow.agent.model.PermissionPreset
import top.flysoftbeta.workflow.agent.model.RequestKind
import top.flysoftbeta.workflow.agent.model.RequestStatus
import top.flysoftbeta.workflow.agent.model.RunState
import top.flysoftbeta.workflow.agent.model.SliderPosition
import top.flysoftbeta.workflow.agent.model.ThreadKey
import top.flysoftbeta.workflow.agent.model.TurnSettings
import top.flysoftbeta.workflow.agent.model.TurnStatus
import top.flysoftbeta.workflow.agent.model.UserPart
import top.flysoftbeta.workflow.agent.testing.Fixtures

/** Pure mapper replays (no process): Codex notifications → events → reducer. */
class CodexEventsTest {
    private fun obj(s: String) = parseJson(s) as JsonObject

    private fun replayInbound(name: String, limit: Int = Int.MAX_VALUE): AgentState {
        var state = AgentState()
        for (e in Fixtures.envelopes(name).take(limit)) {
            if (e.dir != "in") continue
            val method = e.msg["method"].str ?: continue
            if (e.msg["id"] != null) continue
            state = AgentReducer.reduceAll(state, CodexEvents.notification(method, e.msg["params"], e.msg))
        }
        return state
    }

    @Test fun usageLimitFailureShape() {
        val envelopes = Fixtures.envelopes("codex/usage-limit.jsonl")
        val firstFailure = envelopes.indexOfFirst { it.msg["method"].str == "turn/completed" }
        // systemError survives the failed turn's completion (not reset to idle)…
        assertEquals(RunState.ERROR, replayInbound("codex/usage-limit.jsonl", firstFailure + 1).threads.values.single { it.turns.isNotEmpty() }.runState)
        // …and the research script archived the thread afterwards.
        val state = replayInbound("codex/usage-limit.jsonl")
        val thread = state.threads.values.single { it.turns.isNotEmpty() }
        assertTrue(thread.archived)
        assertTrue(thread.turns.isNotEmpty())
        for (turn in thread.turns) {
            assertEquals(TurnStatus.FAILED, turn.status)
            assertEquals("usageLimitExceeded", turn.error!!.code)
            assertEquals("usageLimitExceeded", turn.items.filterIsInstance<NoticeItem>().single().notice.code)
        }
    }

    @Test fun inboundOnlyReplayMatchesBackendReplay() {
        val state = replayInbound("codex/session.jsonl")
        val thread = state.threads.getValue(ThreadKey(BackendKind.CODEX, "01a0ddb6-d670-70f3-9a62-1320d2a3b41a"))
        assertEquals(listOf(TurnStatus.COMPLETED, TurnStatus.COMPLETED, TurnStatus.INTERRUPTED), thread.turns.map { it.status })
        assertEquals("workflow protocol research", thread.title)
        assertTrue(thread.archived)
        assertEquals("ready", state.backend(BackendKind.CODEX).mcpServers.getValue("node_repl").status)
        assertTrue(state.backend(BackendKind.CODEX).unknown.any { it.kind == "remoteControl/status/changed" })
        assertEquals("deprecationNotice", state.backend(BackendKind.CODEX).notices.single().code)
    }

    @Test fun commandDecisionsFollowAvailableDecisionsExactly() {
        val d = CodexEvents.commandDecisions(parseJson("""["accept",{"acceptWithExecpolicyAmendment":{"execpolicy_amendment":["git","status"]}},
            {"applyNetworkPolicyAmendment":{"network_policy_amendment":{"action":"allow","host":"example.com"}}},
            {"applyNetworkPolicyAmendment":{"network_policy_amendment":{"action":"deny","host":"evil.test"}}},"acceptForSession","decline","cancel","futureChoice"]"""))
        assertEquals(listOf("accept", "acceptWithExecpolicyAmendment", "applyNetworkPolicyAmendment", "applyNetworkPolicyAmendment#2", "acceptForSession", "decline", "cancel", "futureChoice"), d.map { it.id })
        assertEquals(listOf(DecisionKind.ALLOW_ONCE, DecisionKind.ALLOW_PERSISTENT, DecisionKind.ALLOW_PERSISTENT, DecisionKind.DENY, DecisionKind.ALLOW_SESSION, DecisionKind.DENY, DecisionKind.ABORT, DecisionKind.OTHER), d.map { it.kind })
        assertEquals("git status", d[1].detail)
        assertEquals("deny evil.test", d[3].detail)
        // Absent list: one-shot only, no persistent grant is invented.
        assertEquals(listOf("accept", "decline", "cancel"), CodexEvents.commandDecisions(null).map { it.id })
    }

    @Test fun serverRequestDispositions() {
        val approval = CodexEvents.request(JsonPrimitive("x1"), "item/fileChange/requestApproval",
            obj("""{"threadId":"t","turnId":"u","itemId":"i","reason":"write","grantRoot":"/workspace","startedAtMs":1}"""), obj("{}"), 5)
        approval as CodexDisposition.Ask
        assertEquals(JsonPrimitive("x1"), approval.request.key.rawId)
        assertEquals(listOf("accept", "acceptForSession", "decline", "cancel"), approval.request.decisions.map { it.id })
        assertEquals("/workspace", (approval.request.kind as RequestKind.FileChangeApproval).grantRoot)

        val questions = CodexEvents.request(JsonPrimitive(3), "item/tool/requestUserInput", obj("""{"threadId":"t","turnId":"u","itemId":"i","isBlocking":true,
            "questions":[{"id":"q1","header":"H","question":"Which?","isOther":true,"isSecret":false,"options":[{"label":"A","description":"a"}]}]}"""), obj("{}"), 5) as CodexDisposition.Ask
        val q = (questions.request.kind as RequestKind.UserInput).questions.single()
        assertEquals("q1", q.id)
        assertTrue(q.allowFreeText)

        val time = CodexEvents.request(JsonPrimitive(4), "currentTime/read", obj("""{"threadId":"t"}"""), obj("{}"), 1_790_000_000_999) as CodexDisposition.AutoResult
        assertEquals("""{"currentTimeAt":1790000000}""", time.result.toString())

        val unknown = CodexEvents.request(JsonPrimitive(5), "future/approveEverything", obj("{}"), obj("{}"), 5) as CodexDisposition.AutoError
        assertEquals(-32601, unknown.code)
        assertEquals(RequestStatus.REJECTED, unknown.record.status)

        val dynamic = CodexEvents.request(JsonPrimitive(6), "item/tool/call", obj("""{"threadId":"t","turnId":"u","callId":"c","tool":"x","arguments":{}}"""), obj("{}"), 5) as CodexDisposition.AutoResult
        assertEquals("false", dynamic.result["success"].toString())
    }

    @Test fun decisionResultsUseServerWireValues() {
        val ask = CodexEvents.request(JsonPrimitive(0), "item/permissions/requestApproval",
            obj("""{"threadId":"t","turnId":"u","itemId":"i","cwd":"/workspace","permissions":{"network":{"enabled":true}},"startedAtMs":1}"""), obj("{}"), 5) as CodexDisposition.Ask
        val session = ask.request.decisions.single { it.id == "grantSession" }
        assertEquals("""{"permissions":{"network":{"enabled":true}},"scope":"session"}""", CodexEvents.decisionResult(ask.request, session, null).toString())
        val decline = ask.request.decisions.single { it.id == "decline" }
        assertEquals("""{"permissions":{},"scope":"turn"}""", CodexEvents.decisionResult(ask.request, decline, null).toString())
    }

    @Test fun paramsAlwaysRouteApprovalsToTheUser() {
        for (preset in PermissionPreset.entries) {
            assertEquals("user", CodexParams.threadStart("/workspace", TurnSettings(permissions = preset), PermissionPreset.ASK, false)["approvalsReviewer"].str)
            assertEquals("user", CodexParams.threadResume("t", null, TurnSettings(), preset)["approvalsReviewer"].str)
            assertEquals("user", CodexParams.threadFork("t", "u", null, TurnSettings(), preset, true)["approvalsReviewer"].str)
        }
        val turn = CodexParams.turnStart("t", listOf(UserPart.Text("hi"), UserPart.Image("/workspace/a.png"), UserPart.File("/workspace/spec.pdf")), TurnSettings("m", "high", PermissionPreset.AUTO_EDIT), "cm")
        assertEquals("user", turn["approvalsReviewer"].str)
        assertEquals("on-request", turn["approvalPolicy"].str)
        assertEquals("workspaceWrite", turn["sandboxPolicy"]["type"].str)
        assertEquals("""[{"type":"text","text":"hi","text_elements":[]},{"type":"localImage","path":"/workspace/a.png"},{"type":"text","text":"Attached file: /workspace/spec.pdf","text_elements":[]}]""", turn["input"].toString())
        // The generic console cannot bypass it either.
        assertEquals("user", CodexParams.enforceReviewer("turn/start", obj("""{"threadId":"t","approvalsReviewer":"auto_review"}"""))["approvalsReviewer"].str)
        assertFalse(CodexParams.enforceReviewer("model/list", obj("{}")).toString().contains("approvalsReviewer"))
    }

    @Test fun modelCatalogSliderIncludesHiddenReserveOnlyWhenCurrent() {
        val result = Fixtures.envelopes("codex/handshake-hidden.jsonl").map { it.msg }
            .first { it["result"]["data"] != null }["result"]!!
        val catalog = CodexEvents.models(listOf(result))
        val hidden = catalog.models.filter { it.hidden }.map { it.id }
        assertTrue(hidden.isNotEmpty())
        val reserve = hidden.first()
        assertFalse(catalog.slider().any { it.model.id == reserve })
        assertTrue(catalog.slider(current = reserve).any { it.model.id == reserve })
        val astra = catalog.slider().first { it.model.id == "gpt-6-astra" }
        assertEquals(listOf("low", "medium", "high", "xhigh", "max", "ultra"), astra.detents)
        assertEquals(1, astra.defaultDetent) // medium
        assertEquals(SliderPosition("gpt-6-astra", "medium"), catalog.resolve(SliderPosition("gpt-6-astra", "nonexistent")))
        assertEquals(SliderPosition(catalog.defaultModel!!.id, catalog.defaultModel!!.defaultEffort), catalog.resolve(SliderPosition("gone-model", "low")))
        assertTrue(catalog.supports("gpt-6-astra", "image"))
    }
}
