package top.flysoftbeta.workflow.engine.chat

import java.io.*
import kotlinx.coroutines.*
import kotlinx.serialization.json.*
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.rpc.*
import top.flysoftbeta.workflow.core.config.*
import top.flysoftbeta.workflow.core.connection.*

class ChatServiceTest {
    @Test fun servicePersistsThroughEngineWhileWorkspaceWatchIsWaiting() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        val input = ResponseInputStream()
        val documents = java.util.concurrent.ConcurrentHashMap<String, String>()
        val frames = ByteArrayOutputStream()
        val state = buildJsonObject {
            put("status", "ready"); put("sessions", JsonArray(emptyList())); put("pinned", JsonArray(emptyList()))
            listOf("drafts", "composers", "disk").forEach { put(it, JsonObject(emptyMap())) }
            put("config", ChatWire.json.parseToJsonElement(ConfigCodec.encode(AppConfig())))
        }
        val transport = object : WorkspaceTransport {
            override val input: InputStream = input
            override val output = object : OutputStream() {
                override fun write(b: Int) {
                    if (b != 10) { frames.write(b); return }
                    val request = ChatWire.json.parseToJsonElement(frames.toString(Charsets.UTF_8)).jsonObject
                    frames.reset()
                    val method = request.string("method")
                    if (method == "workspace.watch") return // Intentionally keep outstanding while documents RPC proceeds.
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
                        "environment.tools.status" -> buildJsonObject { put("tools", JsonArray(emptyList())) }
                        "documents.read" -> buildJsonObject { put("document", documents[args.string("key")]?.let(::JsonPrimitive) ?: JsonNull); put("revision", 0) }
                        "documents.write" -> { documents[args.string("key")] = args.string("document"); buildJsonObject { put("revision", 1) } }
                        else -> error("Unexpected callback: $method")
                    }
                    input.deliver((buildJsonObject { put("jsonrpc", "2.0"); put("id", request.getValue("id")); put("result", result) }.toString() + "\n").toByteArray())
                }
            }
            override fun close() = input.close()
        }
        val rpc = WorkspaceRpc(transport, scope)
        val service = ChatService(rpc, scope)
        try {
            withTimeout(5000) {
                service.start()
                val created = service.request("chat.command", buildJsonObject { put("name", "newConversation"); putJsonObject("args") { put("backend", "claude"); put("id", "conversation") } })
                assertEquals("conversation", created.jsonPrimitive.content)
                val snapshot = ChatWire.snapshot(service.request("chat.snapshot", JsonObject(emptyMap())))
                assertEquals(BackendKind.CLAUDE, snapshot.metadata.conversations.single().backend)
                assertTrue(snapshot.metadata.available.isEmpty())
                assertEquals("conversation", ConversationIndexCodec.decode(documents.getValue("conversations")).single().id)
            }
        } finally { service.close(); rpc.close(); scope.cancel() }
    }
    @Test fun frameReaderRejectsTruncatedOrInvalidUtf8() {
        assertTrue(runCatching { readFrame(ByteArrayInputStream("{}".toByteArray())) }.isFailure)
        assertTrue(runCatching { readFrame(ByteArrayInputStream(byteArrayOf(0xff.toByte(), 10))) }.isFailure)
        assertEquals("{}", readFrame(ByteArrayInputStream("{}\n".toByteArray())))
    }
}
