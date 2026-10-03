package top.flysoftbeta.workflow.agent.claude

import kotlinx.coroutines.runBlocking
import kotlinx.serialization.json.JsonObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import top.flysoftbeta.workflow.agent.json.arr
import top.flysoftbeta.workflow.agent.json.encode
import top.flysoftbeta.workflow.agent.json.get
import top.flysoftbeta.workflow.agent.json.parseJson
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.model.AgentMessageItem
import top.flysoftbeta.workflow.agent.model.CommandItem
import top.flysoftbeta.workflow.agent.model.DecisionKind
import top.flysoftbeta.workflow.agent.model.FileChangeItem
import top.flysoftbeta.workflow.agent.model.FileChangeKind
import top.flysoftbeta.workflow.agent.model.ItemStatus
import top.flysoftbeta.workflow.agent.model.MarkerItem
import top.flysoftbeta.workflow.agent.model.MarkerKind
import top.flysoftbeta.workflow.agent.model.MessagePhase
import top.flysoftbeta.workflow.agent.model.PermissionPreset
import top.flysoftbeta.workflow.agent.model.ReasoningItem
import top.flysoftbeta.workflow.agent.model.RequestKind
import top.flysoftbeta.workflow.agent.model.RequestResponse
import top.flysoftbeta.workflow.agent.model.TurnStatus
import top.flysoftbeta.workflow.agent.model.UserMessageItem
import top.flysoftbeta.workflow.agent.model.UserPart
import top.flysoftbeta.workflow.agent.testing.Fixtures

class ClaudeUnitTest {
    private fun obj(s: String) = parseJson(s) as JsonObject

    private fun canUseTool(request: String) = obj("""{"type":"control_request","request_id":"r1","request":$request}""").let {
        ClaudeRequests.pending("r1", it["request"]["subtype"].str!!, it["request"] as JsonObject, it, "s", "t", 1)!!
    }

    @Test fun toolApprovalOffersOnlyWhatTheCliAllows() {
        val req = canUseTool("""{"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"rm x"},"tool_use_id":"tu",
            "decision_reason":"\u001b[31mdangerous\u001b[0m","default_to_no":true,"suppress_always_allow_rule":true,
            "permission_suggestions":[{"type":"addRules","rules":[{"toolName":"Bash","ruleContent":"rm x"}],"behavior":"allow","destination":"localSettings"}]}""")
        assertEquals(listOf("allow", "deny", "denyAndStop"), req.decisions.map { it.id })
        val kind = req.kind as RequestKind.ToolApproval
        assertEquals("dangerous", kind.reason)
        assertTrue(kind.defaultToNo)

        val interactive = canUseTool("""{"subtype":"can_use_tool","tool_name":"Bash","input":{},"tool_use_id":"tu","requires_user_interaction":true}""")
        assertEquals("no one-tap approve", listOf("deny", "denyAndStop"), interactive.decisions.map { it.id })
    }

    @Test fun answersAreBuiltFromTheOfferedDecisionOnly() {
        val req = canUseTool("""{"subtype":"can_use_tool","tool_name":"Edit","input":{"file_path":"/workspace/a"},"tool_use_id":"tu",
            "permission_suggestions":[{"type":"setMode","mode":"acceptEdits","destination":"session"}]}""")
        val always = ClaudeRequests.answer(req, RequestResponse.Decide("allowAlways:0"))
        assertEquals("""{"behavior":"allow","updatedInput":{"file_path":"/workspace/a"},"updatedPermissions":[{"type":"setMode","mode":"acceptEdits","destination":"session"}]}""", always.payload!!.encode())
        assertEquals(DecisionKind.ALLOW_SESSION, req.decisions[1].kind)
        val stop = ClaudeRequests.answer(req, RequestResponse.Decide("denyAndStop", "no"))
        assertEquals("""{"behavior":"deny","message":"no","interrupt":true}""", stop.payload!!.encode())
        try { ClaudeRequests.answer(req, RequestResponse.Decide("allowAlways:5")); fail() } catch (_: IllegalArgumentException) {}
        try { ClaudeRequests.answer(req, RequestResponse.RawResult(obj("""{"behavior":"allow"}"""))); fail() } catch (_: IllegalArgumentException) {}
    }

    @Test fun askUserQuestionAndPlanApproval() {
        val ask = canUseTool("""{"subtype":"can_use_tool","tool_name":"AskUserQuestion","tool_use_id":"tu","input":{"questions":[
            {"question":"Which DB?","header":"DB","multiSelect":false,"options":[{"label":"Postgres","description":"p"},{"label":"SQLite","description":"s"}]},
            {"question":"Features?","header":"F","multiSelect":true,"options":[{"label":"A","description":""},{"label":"B","description":""}]}]}}""")
        val questions = (ask.kind as RequestKind.UserInput).questions
        assertEquals(listOf("Which DB?", "Features?"), questions.map { it.id })
        assertTrue(questions[1].multiSelect)
        val answer = ClaudeRequests.answer(ask, RequestResponse.Answer(mapOf("Which DB?" to listOf("SQLite"), "Features?" to listOf("A", "B"))))
        assertEquals("allow", answer.payload!!["behavior"].str)
        assertEquals("""{"Which DB?":"SQLite","Features?":"A, B"}""", answer.payload!!["updatedInput"]["answers"]!!.encode())
        assertEquals(2, answer.payload!!["updatedInput"]["questions"].arr!!.size)

        val plan = canUseTool("""{"subtype":"can_use_tool","tool_name":"ExitPlanMode","tool_use_id":"tu","input":{"plan":"1. do it"}}""")
        assertEquals("1. do it", (plan.kind as RequestKind.PlanApproval).plan)
        assertEquals(listOf("allow", "deny"), plan.decisions.map { it.id })
    }

    @Test fun elicitationAndUndeclaredDialogs() {
        val elicit = ClaudeRequests.pending("e1", "elicitation", obj("""{"subtype":"elicitation","mcp_server_name":"gh","message":"Token?","mode":"form","requested_schema":{"type":"object"}}"""), obj("{}"), "s", null, 1)!!
        assertEquals("gh", (elicit.kind as RequestKind.Elicitation).server)
        assertEquals("""{"action":"accept","content":{"token":"x"}}""", ClaudeRequests.answer(elicit, RequestResponse.Elicit("accept", obj("""{"token":"x"}"""))).payload!!.encode())
        val dialog = ClaudeRequests.pending("d1", "request_user_dialog", obj("""{"subtype":"request_user_dialog","dialog_kind":"refusal_fallback_prompt","payload":{}}"""), obj("{}"), "s", null, 1)!!
        assertTrue(dialog.decisions.isEmpty())
        try { ClaudeRequests.answer(dialog, RequestResponse.Decide("x")); fail() } catch (_: IllegalStateException) {}
        assertNull(ClaudeRequests.pending("h", "hook_callback", obj("{}"), obj("{}"), "s", null, 1))
    }

    @Test fun attachmentsMapToContentBlocks() = runBlocking {
        val reader = AttachmentReader { path, _ -> path.toByteArray() }
        val content = ClaudeRequests.content(listOf(
            UserPart.Text("look"), UserPart.Image("/workspace/a.jpg"), UserPart.File("/workspace/doc.pdf"), UserPart.File("/workspace/src/Main.kt"),
        ), reader, 1024)
        assertEquals("look\n\n@/workspace/src/Main.kt", content[0]["text"].str)
        assertEquals("image/jpeg", content[1]["source"]["media_type"].str)
        assertEquals("document", content[2]["type"].str)
        assertEquals("application/pdf", content[2]["source"]["media_type"].str)
        assertEquals(3, content.size)
        val onlyFile = ClaudeRequests.content(listOf(UserPart.File("/workspace/notes.txt")), reader, 1024)
        assertEquals("@/workspace/notes.txt", onlyFile.single()["text"].str)
    }

    @Test fun launchContractNeverWeakensPermissions() {
        val config = ClaudeConfig(configDir = "/home/work/.claude-app", env = mapOf("ANTHROPIC_API_KEY" to "leak", "CLAUDECODE" to "1", "LANG" to "C.UTF-8"),
            credentials = mapOf("CLAUDE_CODE_OAUTH_TOKEN" to "app-token"))
        val argv = ClaudeLaunch.argv(config, ClaudeSession.Resume("s1"), "sonnet", "high", PermissionPreset.AUTO_EDIT)
        assertEquals(listOf("--permission-mode", "acceptEdits"), argv.subList(argv.indexOf("--permission-mode"), argv.indexOf("--permission-mode") + 2))
        assertTrue(argv.containsAll(listOf("--resume", "s1", "--model", "sonnet", "--effort", "high", "--replay-user-messages")))
        val env = ClaudeLaunch.env(config)
        assertFalse(env.containsKey("ANTHROPIC_API_KEY"))
        assertFalse(env.containsKey("CLAUDECODE"))
        assertEquals("app-token", env["CLAUDE_CODE_OAUTH_TOKEN"])
        assertEquals("1", env["DISABLE_AUTOUPDATER"])
        for (bad in listOf("--dangerously-skip-permissions", "bypassPermissions", "auto")) {
            try { ClaudeLaunch.argv(config.copy(extraArgs = listOf(bad)), ClaudeSession.New("s"), null, null, null); fail(bad) } catch (_: IllegalArgumentException) {}
        }
        assertEquals("/home/work/.claude-app/projects/-workspace/s1.jsonl", ClaudeLaunch.transcriptPath("/home/work/.claude-app", "/workspace", "s1"))
    }

    @Test fun transcriptHydration() {
        val lines = Fixtures.lines("claude/transcript-1577088f.jsonl")
        val history = ClaudeTranscript.history("1577088f-7864-4263-aee6-9afe1b2f4a73", lines)
        assertEquals("workflow protocol research", history.title)
        assertEquals(listOf("47397f51-ee05-4bf1-912e-296866b9da74", "01b893ae-40c7-451c-b444-9f3f82ab80f2", "98c79d86-1546-45df-aeaa-07ac81e3f40a", "5eb19f10-e16a-4a65-98f0-8995aef2fac0"), history.turns.map { it.id })
        assertEquals(listOf(TurnStatus.COMPLETED, TurnStatus.COMPLETED, TurnStatus.COMPLETED, TurnStatus.INTERRUPTED), history.turns.map { it.status })
        val (t1, t2, t3, t4) = history.turns
        val user = t1.items.filterIsInstance<UserMessageItem>().single()
        assertEquals(2, user.parts.size)
        assertTrue(user.parts[1] is UserPart.InlineData)
        assertEquals(1, t1.items.filterIsInstance<ReasoningItem>().size)
        assertEquals(MessagePhase.FINAL, t1.finalMessage!!.phase)
        val write = t2.items.filterIsInstance<FileChangeItem>().single()
        assertEquals(FileChangeKind.ADD, write.changes.single().kind)
        assertEquals(ItemStatus.COMPLETED, write.status)
        assertEquals(ItemStatus.DECLINED, t3.items.filterIsInstance<CommandItem>().single().status)
        assertTrue("local /model command is between turns, not a user message", t3.items.none { it is UserMessageItem && it.text.contains("command-name") })
        assertEquals(ItemStatus.INCOMPLETE, t4.items.filterIsInstance<AgentMessageItem>().single().status)
        assertTrue(t4.items.any { it is MarkerItem && it.kind == MarkerKind.INTERRUPTED })
        // Fork anchor for "branch after turn 2".
        assertEquals("5366cf09", ClaudeTranscript.lastUuid(lines, t2.id)!!.take(8))
    }

    @Test fun modelsAndAccountFromInitialize() {
        val init = Fixtures.envelopes("claude/noauth.jsonl").first { it.msg["response"]["response"]["models"] != null }.msg["response"]["response"]
        val catalog = ClaudeMapper.models(init["models"])
        assertTrue(catalog.models.all { it.inputModalities.containsAll(setOf("image", "pdf")) })
        assertEquals("medium", catalog.model("opus")!!.defaultEffort)
        assertTrue(catalog.slider().first { it.model.id == "haiku" }.detents.isEmpty())
        val account = ClaudeMapper.account(init)
        assertEquals(top.flysoftbeta.workflow.agent.model.LoginState.LOGGED_OUT, account.state)
    }
}
