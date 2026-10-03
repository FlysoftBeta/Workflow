package top.flysoftbeta.workflow.agent.claude

import java.util.Base64
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import top.flysoftbeta.workflow.agent.AgentStateStore
import top.flysoftbeta.workflow.agent.ThreadOptions
import top.flysoftbeta.workflow.agent.json.arr
import top.flysoftbeta.workflow.agent.json.get
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.model.AgentMessageItem
import top.flysoftbeta.workflow.agent.model.AgentState
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.CommandItem
import top.flysoftbeta.workflow.agent.model.DecisionKind
import top.flysoftbeta.workflow.agent.model.FileChangeItem
import top.flysoftbeta.workflow.agent.model.FileChangeKind
import top.flysoftbeta.workflow.agent.model.ItemStatus
import top.flysoftbeta.workflow.agent.model.LoginState
import top.flysoftbeta.workflow.agent.model.MarkerItem
import top.flysoftbeta.workflow.agent.model.MarkerKind
import top.flysoftbeta.workflow.agent.model.MessagePhase
import top.flysoftbeta.workflow.agent.model.NoticeItem
import top.flysoftbeta.workflow.agent.model.PermissionPreset
import top.flysoftbeta.workflow.agent.model.ReasoningItem
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
import top.flysoftbeta.workflow.agent.testing.FakeLauncher
import top.flysoftbeta.workflow.agent.testing.FakeProcess
import top.flysoftbeta.workflow.agent.testing.Fixtures
import top.flysoftbeta.workflow.agent.testing.ScriptedServer
import top.flysoftbeta.workflow.agent.testing.await

/**
 * Drives the real [ClaudeBackend] against the recorded Claude Code 2.1.283 frames
 * (fixtures/claude/session.jsonl + fork.jsonl: markdown/math/image, hook callback, can_use_tool
 * allow and deny, model/effort/mode switch, rename, interrupt, fork) and the not-logged-in run.
 */
class ClaudeBackendReplayTest {
    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())
    private val store = AgentStateStore()

    @After fun tearDown() = scope.cancel()

    private val session = "1577088f-7864-4263-aee6-9afe1b2f4a73"
    private val fork = "6810f1e3-226d-4f67-8c39-1a4e00ebab88"
    private val u1 = "47397f51-ee05-4bf1-912e-296866b9da74"
    private val u2 = "01b893ae-40c7-451c-b444-9f3f82ab80f2"
    private val u3 = "98c79d86-1546-45df-aeaa-07ac81e3f40a"
    private val u4 = "5eb19f10-e16a-4a65-98f0-8995aef2fac0"
    private val u5 = "284b1a4e-b70b-4a8d-b7e2-ca43830b8844"
    private val png = "iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAIAAACQkWg2AAAAF0lEQVR4nGP4z8BAEiJN9aiGUQ1DSgMAkPn/Afnh+ngAAAAASUVORK5CYII="

    private val hookCalls = mutableListOf<JsonObject>()

    private fun backend(scripts: List<String>, ids: List<String>): Pair<ClaudeBackend, FakeLauncher> {
        val servers = scripts.map { ScriptedServer(Fixtures.envelopes(it), ScriptedServer.Protocol.CLAUDE_CONTROL) }
        val launcher = FakeLauncher { n, spec -> FakeProcess(spec) { p, f -> servers[n].react(p, f) }.also(servers[n]::start) }
        val next = ids.iterator()
        val config = ClaudeConfig(
            executable = "claude", configDir = "/home/work/.claude-app", cwd = "/workspace",
            env = mapOf("PATH" to "/usr/bin", "ANTHROPIC_API_KEY" to "host-secret", "CLAUDE_CODE_ENTRYPOINT" to "desktop"),
            hooks = listOf(ClaudeHook("PreToolUse", "Write|Bash", "hook_pre_1") { input -> hookCalls += input; buildJsonObject { put("continue", true) } }),
        )
        val reader = AttachmentReader { path, _ -> check(path == "/workspace/red.png"); Base64.getDecoder().decode(png) }
        return ClaudeBackend(launcher, config, store, scope, reader, newId = { next.next() }) to launcher
    }

    private fun AgentState.t(id: String = session): ThreadState = threads.getValue(ThreadKey(BackendKind.CLAUDE, id))

    @Test fun fullSessionReplay() = runBlocking {
        val (claude, launcher) = backend(listOf("claude/session.jsonl", "claude/fork.jsonl"), listOf(session, u1, u2, u3, u4, fork, u5))
        claude.start()
        val spec = launcher.launched[0].spec
        val argv = spec.argv.joinToString(" ")
        assertTrue(argv.startsWith("claude -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages"))
        assertTrue(argv.contains("--permission-prompt-tool stdio --permission-mode default"))
        assertTrue(argv.endsWith("--session-id $session"))
        assertFalse(argv.contains("dangerously") || argv.contains("bypassPermissions"))
        assertEquals("/home/work/.claude-app", spec.env["CLAUDE_CONFIG_DIR"])
        assertFalse("host credentials are not forwarded", spec.env.containsKey("ANTHROPIC_API_KEY"))
        assertFalse(spec.env.containsKey("CLAUDE_CODE_ENTRYPOINT"))

        val backendStatus = store.state.value.backend(BackendKind.CLAUDE)
        assertEquals(LoginState.LOGGED_IN, backendStatus.account.state) // research run used an API key
        val models = backendStatus.models!!
        assertEquals("default", models.defaultModel!!.id)
        assertEquals(emptyList<String>(), models.model("haiku")!!.efforts.map { it.id })
        assertEquals(listOf("low", "medium", "high", "xhigh", "max"), models.model("sonnet")!!.efforts.map { it.id })

        assertEquals(session, claude.startThread(ThreadOptions("/workspace")))
        claude.rawRequest("mcp_status", null)
        claude.refreshModels()
        claude.rawRequest("get_settings", null)
        claude.rawRequest("get_context_usage", buildJsonObject { put("detail", "summary") })

        // ---- turn 1: markdown + math + image attachment
        claude.send(session, listOf(UserPart.Text("MATH: answer in markdown with a formula; what colour is the image?"), UserPart.Image("/workspace/red.png", "image/png")))
        store.await(what = "turn 1") { it.t().turn(u1)?.status == TurnStatus.COMPLETED }
        val written = launcher.launched[0].written.first { it["type"].str == "user" }
        assertEquals(png, written["message"]["content"].arr!![1]["source"]["data"].str)
        val turn1 = store.state.value.t().turn(u1)!!
        val user = turn1.items.filterIsInstance<UserMessageItem>().single()
        assertEquals(u1, user.id)
        assertFalse(user.local)
        assertEquals(UserPart.Image("/workspace/red.png", "image/png"), user.parts[1]) // composed parts kept over base64 echo
        assertEquals("The user wants markdown with a formula; the image is a red square.", turn1.items.filterIsInstance<ReasoningItem>().single().content.single().toString())
        val final1 = turn1.finalMessage!!
        assertTrue(final1.text.toString().contains("\$\$x=\\frac{-b\\pm\\sqrt{b^2-4ac}}{2a}\$\$"))
        assertEquals(0.00128, turn1.usage!!.costUsd!!, 1e-9)
        assertEquals("claude-opus-5-5[1m]", store.state.value.t().settings.model)

        // ---- turn 2: hook callback (app-registered) + permission allowed by the user
        claude.send(session, listOf(UserPart.Text("WRITE: create hello.txt")))
        val allowKey = ClaudeRequests.key("c071187f-d192-476b-942a-e990906bfdd6")
        store.await(what = "write permission") { it.requests[allowKey]?.status == RequestStatus.PENDING }
        assertEquals("PreToolUse", hookCalls.single()["hook_event_name"].str)
        val write = store.state.value.requests.getValue(allowKey)
        assertEquals(listOf("allow", "allowAlways:0", "deny", "denyAndStop"), write.decisions.map { it.id })
        assertEquals(DecisionKind.ALLOW_SESSION, write.decisions[1].kind)
        assertEquals("mode acceptEdits → session", write.decisions[1].detail)
        assertEquals("Write", (write.kind as RequestKind.ToolApproval).tool)
        assertEquals(RunState.WAITING_APPROVAL, store.state.value.t().runState)
        Thread.sleep(200)
        val p0 = launcher.launched[0]
        assertTrue("no permission answer before the user", p0.written.none { it["response"]["request_id"].str == allowKey.rawId.str })
        claude.respond(allowKey, RequestResponse.Decide("allow"))
        store.await(what = "turn 2") { it.t().turn(u2)?.status == TurnStatus.COMPLETED }
        val turn2 = store.state.value.t().turn(u2)!!
        val file = turn2.items.filterIsInstance<FileChangeItem>().single()
        assertEquals(ItemStatus.COMPLETED, file.status)
        assertEquals(FileChangeKind.ADD, file.changes.single().kind)
        assertEquals("/workspace/hello.txt", file.changes.single().path)
        assertEquals(listOf(MessagePhase.COMMENTARY, MessagePhase.FINAL), turn2.items.filterIsInstance<AgentMessageItem>().map { it.phase })
        assertTrue(turn2.items.any { it is MarkerItem && it.kind == MarkerKind.HOOK })
        assertEquals(RequestStatus.ANSWERED, store.state.value.requests.getValue(allowKey).status)

        // ---- turn 3: deny
        claude.send(session, listOf(UserPart.Text("BASH: write bash.txt")))
        val denyKey = ClaudeRequests.key("3d376a32-71e6-44e8-bc27-a708d72b6482")
        store.await(what = "bash permission") { it.requests[denyKey]?.status == RequestStatus.PENDING }
        val bash = store.state.value.requests.getValue(denyKey)
        assertEquals(listOf("allow", "allowAlways:0", "allowAlways:1", "deny", "denyAndStop"), bash.decisions.map { it.id })
        assertEquals(DecisionKind.ALLOW_PERSISTENT, bash.decisions[1].kind) // localSettings rule: written to disk
        assertEquals(DecisionKind.ALLOW_SESSION, bash.decisions[2].kind)
        assertEquals("/workspace/bash.txt", (bash.kind as RequestKind.ToolApproval).blockedPath)
        claude.respond(denyKey, RequestResponse.Decide("deny", "Declined by research client"))
        val denial = p0.written.single { it["response"]["request_id"].str == denyKey.rawId.str }
        assertEquals("deny", denial["response"]["response"]["behavior"].str)
        store.await(what = "turn 3") { it.t().turn(u3)?.status == TurnStatus.COMPLETED }
        assertEquals(ItemStatus.DECLINED, store.state.value.t().turn(u3)!!.items.filterIsInstance<CommandItem>().single().status)

        // ---- mid-session controls, then turn 4 with model/effort switch, interrupted
        claude.rawRequest("set_max_thinking_tokens", buildJsonObject { put("max_thinking_tokens", 4096) })
        claude.setPermissions(session, PermissionPreset.PLAN)
        claude.setPermissions(session, PermissionPreset.ASK)
        claude.rename(session, "workflow protocol research")
        claude.send(session, listOf(UserPart.Text("SLOW: count to 300")), TurnSettings(model = "sonnet", effort = "high"))
        store.await(what = "turn 4 streaming") { s -> s.t().turn(u4)?.items?.any { it is AgentMessageItem && it.text.length > 10 } == true }
        claude.interrupt(session)
        store.await(what = "turn 4 interrupted") { it.t().turn(u4)?.status == TurnStatus.INTERRUPTED }
        val turn4 = store.state.value.t().turn(u4)!!
        assertEquals(ItemStatus.INCOMPLETE, turn4.items.filterIsInstance<AgentMessageItem>().single().status)
        assertTrue(turn4.items.any { it is MarkerItem && it.kind == MarkerKind.INTERRUPTED })
        val thread = store.state.value.t()
        assertEquals("workflow protocol research", thread.title)
        assertEquals("claude-sonnet-5", thread.settings.model) // resolved by the CLI in system/init
        assertEquals("high", thread.settings.effort)
        assertEquals("default", thread.settings.approvalPolicy)
        val subtypes = p0.written.filter { it["type"].str == "control_request" }.map { it["request"]["subtype"].str }
        assertTrue(subtypes.containsAll(listOf("set_model", "apply_flag_settings", "set_permission_mode", "rename_session", "interrupt")))

        // ---- fork into a new session id we chose
        assertEquals(fork, claude.forkThread(session, null, ThreadOptions("/workspace")))
        assertTrue(launcher.launched[1].spec.argv.joinToString(" ").contains("--resume $session --fork-session --session-id $fork"))
        claude.send(fork, listOf(UserPart.Text("MATH again after fork")))
        store.await(what = "fork turn") { it.t(fork).turn(u5)?.status == TurnStatus.COMPLETED }
        assertEquals(session, store.state.value.t(fork).forkedFrom)
        assertEquals("claude-sonnet-5", store.state.value.t(fork).settings.model)

        // exactly the two answers the user gave, plus the app hook responses
        val answers = p0.written.filter { it["type"].str == "control_response" }
        assertEquals(2 + 2, answers.size)
        assertEquals(2, answers.count { it["response"]["response"]["behavior"] != null })
    }

    @Test fun notLoggedIn() = runBlocking {
        val noauth = "c595e3cd-450f-43ab-99a8-3df521adf5d1"
        val turn = "ffd7f76f-d373-4734-a221-69cb322c89a8"
        val (claude, _) = backend(listOf("claude/noauth.jsonl"), listOf(noauth, turn))
        claude.start()
        assertEquals(LoginState.LOGGED_OUT, store.state.value.backend(BackendKind.CLAUDE).account.state)
        claude.startThread(ThreadOptions("/workspace"))
        claude.send(noauth, listOf(UserPart.Text("MATH: answer in markdown with a formula; what colour is the image?"), UserPart.Image("/workspace/red.png", "image/png")))
        store.await(what = "failed turn") { it.t(noauth).turn(turn)?.status == TurnStatus.FAILED }
        val failed = store.state.value.t(noauth).turn(turn)!!
        assertEquals("api_error", failed.error!!.code)
        assertEquals("Not logged in · Please run /login", failed.error.message)
        assertEquals("authentication_failed", failed.items.filterIsInstance<NoticeItem>().single().notice.code)
        assertTrue(failed.items.none { it is AgentMessageItem })
    }
}

