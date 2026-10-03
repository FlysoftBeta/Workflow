package top.flysoftbeta.workflow.engine.chat

import java.io.*
import java.util.concurrent.CopyOnWriteArrayList
import kotlinx.coroutines.*
import kotlinx.serialization.json.*
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.agent.model.*
import top.flysoftbeta.workflow.agent.rpc.*
import top.flysoftbeta.workflow.core.config.*
import top.flysoftbeta.workflow.core.connection.*

class ChatServiceTest {
    @Test fun servicePersistsThroughEngineWhileWorkspaceWatchIsWaiting() = runBlocking {
        Fixture().useService {
            service.start()
            val created = command("newConversation", "backend" to "claude", "id" to "conversation")
            assertEquals("conversation", created.jsonPrimitive.content)
            val snapshot = ChatWire.snapshot(service.request("chat.snapshot", JsonObject(emptyMap())))
            assertEquals(BackendKind.CLAUDE, snapshot.metadata.conversations.single().backend)
            assertTrue(snapshot.metadata.available.isEmpty())
            assertEquals("conversation", ConversationIndexCodec.decode(documents.getValue("conversations")).single().id)
        }
    }

    @Test fun selectedClaudeStartsOneInstallAndConcurrentRequestsDoNotRepeatIt() = runBlocking {
        Fixture(selectedBackend = "claude").useService {
            service.start()
            assertEquals("installing", phase)
            // A lagging status read must not cause another submission of the same install intent.
            phase = "not_installed"
            coroutineScope { (1..12).map { async { command("newConversation", "backend" to "claude", "id" to "conversation-$it") } }.awaitAll() }
            assertEquals(1, installs.size)
            assertEquals(false, installs.single()["retry"]?.jsonPrimitive?.boolean)
            assertEquals("claude", installs.single().string("toolId"))
        }
    }

    @Test fun openingExistingClaudeConversationRequestsItsToolWithCodexDefault() = runBlocking {
        Fixture().useService {
            documents["conversations"] = ConversationIndexCodec.encode(listOf(ConversationEntry("existing", BackendKind.CLAUDE, "thread", "History", "/workspace", 0, 0)))
            service.start()
            assertTrue("An unopened historical conversation does not install optional tools", installs.isEmpty())
            command("open", "id" to "existing")
            assertEquals("installing", phase)
            assertEquals(1, installs.size)
        }
    }

    @Test fun codexOnlyUseNeverInstallsClaude() = runBlocking {
        Fixture().useService {
            service.start()
            repeat(3) { command("newConversation", "backend" to "codex", "id" to "codex-$it") }
            assertEquals("not_installed", phase)
            assertTrue(installs.isEmpty())
        }
    }

    @Test fun failedClaudeRemainsFailedUntilExplicitToolsRetry() = runBlocking {
        Fixture(selectedBackend = "claude", initialPhase = "failed").useService {
            service.start()
            repeat(3) { command("newConversation", "backend" to "claude", "id" to "claude-$it") }
            assertEquals("failed", phase)
            assertTrue(installs.isEmpty())
            // This is the separate explicit UI tools action, not chat's automatic policy.
            rpc.request("environment.tools.install", mapOf("toolId" to "claude", "retry" to true))
            assertEquals("installing", phase)
            command("open", "id" to "claude-0")
            assertEquals(1, installs.size)
            assertEquals(true, installs.single()["retry"]?.jsonPrimitive?.boolean)
        }
    }

    @Test fun frameReaderRejectsTruncatedOrInvalidUtf8() {
        assertTrue(runCatching { readFrame(ByteArrayInputStream("{}".toByteArray())) }.isFailure)
        assertTrue(runCatching { readFrame(ByteArrayInputStream(byteArrayOf(0xff.toByte(), 10))) }.isFailure)
        assertEquals("{}", readFrame(ByteArrayInputStream("{}\n".toByteArray())))
    }

    private class Fixture(selectedBackend: String = "codex", initialPhase: String = "not_installed") {
        private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        private val input = ResponseInputStream()
        val documents = java.util.concurrent.ConcurrentHashMap<String, String>()
        val installs = CopyOnWriteArrayList<JsonObject>()
        @Volatile var phase = initialPhase
        private val frames = ByteArrayOutputStream()
        private val state = buildJsonObject {
            put("status", "ready"); put("sessions", JsonArray(emptyList())); put("pinned", JsonArray(emptyList()))
            listOf("drafts", "composers", "disk").forEach { put(it, JsonObject(emptyMap())) }
            val config = AppConfig().let { it.copy(agent = it.agent.copy(backend = selectedBackend)) }
            put("config", ChatWire.json.parseToJsonElement(ConfigCodec.encode(config)))
        }
        private fun tools() = buildJsonObject {
            putJsonArray("tools") { add(buildJsonObject { put("id", "claude"); put("phase", phase); put("binary", "/opt/workflow/tools/claude/bin/claude") }) }
        }
        private val transport = object : WorkspaceTransport {
            override val input: InputStream = this@Fixture.input
            override val output = object : OutputStream() {
                override fun write(b: Int) {
                    if (b != 10) { frames.write(b); return }
                    val request = ChatWire.json.parseToJsonElement(frames.toString(Charsets.UTF_8)).jsonObject
                    frames.reset()
                    val method = request.string("method")
                    if (method == "workspace.watch") return // Keep outstanding while tools/document RPCs proceed.
                    val args = request["params"]!!.jsonObject
                    val result = when (method) {
                        "hello" -> buildJsonObject {
                            put("protocol", WorkspaceRpc.PROTOCOL); put("engineVersion", "1.0.0"); put("workspaceRoot", "/host/workspace")
                            putJsonObject("capabilities") {
                                listOf("workspace", "files", "documents", "environment", "processes", "pty", "services").forEach { put(it, true) }
                                put("maxFrameBytes", WorkspaceRpc.MAX_FRAME_BYTES); put("maxBlobChunkBytes", 65_536)
                            }
                        }
                        "workspace.snapshot" -> buildJsonObject { put("revision", 0); put("state", state) }
                        "environment.tools.status" -> tools()
                        "environment.tools.install" -> {
                            installs += args
                            check(args.string("toolId") == "claude")
                            if (phase == "not_installed" || args["retry"]?.jsonPrimitive?.boolean == true) phase = "installing"
                            tools()
                        }
                        "documents.read" -> buildJsonObject { put("document", documents[args.string("key")]?.let(::JsonPrimitive) ?: JsonNull); put("revision", 0) }
                        "documents.write" -> { documents[args.string("key")] = args.string("document"); buildJsonObject { put("revision", 1) } }
                        else -> error("Unexpected callback: $method")
                    }
                    this@Fixture.input.deliver((buildJsonObject { put("jsonrpc", "2.0"); put("id", request.getValue("id")); put("result", result) }.toString() + "\n").toByteArray())
                }
            }
            override fun close() = input.close()
        }
        val rpc = WorkspaceRpc(transport, scope)
        val service = ChatService(rpc, scope)
        suspend fun command(name: String, vararg args: Pair<String, String>): JsonElement = service.request("chat.command", buildJsonObject {
            put("name", name); put("args", JsonObject(args.associate { it.first to JsonPrimitive(it.second) }))
        })
        suspend fun useService(block: suspend Fixture.() -> Unit) {
            try { withTimeout(10_000) { block() } }
            finally { service.close(); rpc.close(); scope.cancel() }
        }
    }
}
