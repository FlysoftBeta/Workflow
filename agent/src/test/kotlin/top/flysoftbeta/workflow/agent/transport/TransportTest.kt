package top.flysoftbeta.workflow.agent.transport

import java.io.ByteArrayInputStream
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.async
import kotlinx.coroutines.cancel
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import top.flysoftbeta.workflow.agent.json.encode
import top.flysoftbeta.workflow.agent.json.get
import top.flysoftbeta.workflow.agent.json.parseJson
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.process.LaunchSpec
import top.flysoftbeta.workflow.agent.testing.FakeProcess

class TransportTest {
    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())
    @After fun tearDown() = scope.cancel()

    private fun obj(text: String) = parseJson(text) as JsonObject
    private fun process(react: (FakeProcess, JsonObject) -> Unit = { _, _ -> }) = FakeProcess(LaunchSpec(listOf("x"), emptyMap(), "/", "t"), react)

    @Test fun lineReaderHandlesCrlfOversizeInvalidUtf8AndMissingFinalNewline() {
        val bytes = "{\"a\":1}\r\n\n".toByteArray() + "x".repeat(50).toByteArray() + "\n".toByteArray() +
            byteArrayOf(0xC3.toByte(), 0x28, '\n'.code.toByte()) + "{\"b\":\"é\"}".toByteArray()
        val reader = LineReader(ByteArrayInputStream(bytes), maxLineBytes = 20)
        assertEquals(LineReader.Result.Line("{\"a\":1}"), reader.next())
        assertEquals(LineReader.Result.TooLarge(50), reader.next())
        assertEquals(LineReader.Result.BadEncoding(2), reader.next())
        assertEquals(LineReader.Result.Line("{\"b\":\"é\"}"), reader.next())
        assertEquals(LineReader.Result.Eof, reader.next())
    }

    @Test fun jsonRpcKeepsDirectionsApartAndEchoesServerIdsVerbatim() = runBlocking {
        lateinit var fake: FakeProcess
        fake = process { p, frame ->
            // Before answering our request id 1, the server sends its own request with the same id 1.
            if (frame["method"].str == "thread/start") {
                p.emit(obj("""{"id":1,"method":"item/commandExecution/requestApproval","params":{"threadId":"t"}}"""))
                p.emit(obj("""{"id":"srv-7","method":"item/tool/requestUserInput","params":{}}"""))
                p.emit(obj("""{"id":99,"result":{}}""")) // response for an id we never used
                p.emit(obj("""{"method":"future/notification","params":{"x":[1,{"y":null}]},"extra":true}"""))
                p.emit(obj("""{"id":1,"result":{"thread":{"id":"t"}}}"""))
            }
        }
        val lines = JsonLineChannel(fake, scope)
        val rpc = JsonRpcConnection(lines, scope)
        val response = scope.async { rpc.request("thread/start", buildJsonObject { put("cwd", "/workspace") }) }
        val inbound = withTimeout(5000) { List(4) { rpc.inbound.receive() } }
        assertEquals("t", withTimeout(5000) { response.await() }["thread"]["id"].str)
        val serverRequest = inbound[0] as JsonRpcConnection.Inbound.Request
        assertEquals(JsonPrimitive(1), serverRequest.id)
        val stringId = inbound[1] as JsonRpcConnection.Inbound.Request
        assertEquals(JsonPrimitive("srv-7"), stringId.id)
        assertTrue(inbound[2] is JsonRpcConnection.Inbound.Stray)
        val unknown = inbound[3] as JsonRpcConnection.Inbound.Notification
        assertEquals("""{"method":"future/notification","params":{"x":[1,{"y":null}]},"extra":true}""", unknown.raw.encode())
        assertEquals(setOf("1", "\"srv-7\""), rpc.openServerRequests)

        rpc.respond(stringId.id, buildJsonObject { put("answers", JsonObject(emptyMap())) })
        rpc.respondError(serverRequest.id, JsonRpcConnection.METHOD_NOT_FOUND, "no")
        val written = fake.written.map { it.encode() }
        assertTrue(written.contains("""{"id":"srv-7","result":{"answers":{}}}"""))
        assertTrue(written.contains("""{"id":1,"error":{"code":-32601,"message":"no"}}"""))
        try { rpc.respond(stringId.id, JsonObject(emptyMap())); fail("double answer") } catch (_: IllegalStateException) {}
        assertEquals(emptySet<String>(), rpc.openServerRequests)
    }

    @Test fun pendingRequestsFailWhenTheProcessExits() = runBlocking {
        val fake = process { p, frame -> if (frame["method"].str == "slow") p.exit(1) }
        val rpc = JsonRpcConnection(JsonLineChannel(fake, scope), scope)
        try {
            withTimeout(5000) { rpc.request("slow", null) }
            fail("expected failure")
        } catch (_: ConnectionClosedException) {}
    }

    @Test fun rpcErrorsSurfaceAsExceptions() = runBlocking {
        val fake = process { p, frame -> p.emit(obj("""{"id":${frame["id"]!!.encode()},"error":{"code":-32602,"message":"bad params","data":{"f":1}}}""")) }
        val rpc = JsonRpcConnection(JsonLineChannel(fake, scope), scope)
        try {
            withTimeout(5000) { rpc.request("x", null) }
            fail()
        } catch (e: RpcException) {
            assertEquals(-32602, e.code)
            assertEquals("bad params", e.message)
        }
    }

    @Test fun controlProtocolIgnoresEchoesAndCorrelatesById() = runBlocking {
        val fake = process { p, frame ->
            if (frame["type"].str == "control_request") {
                val id = frame["request_id"]!!.encode()
                p.emit(obj("""{"type":"keep_alive"}"""))
                p.emit(obj("""{"type":"control_response","response":{"subtype":"success","request_id":"someone-else","response":{}}}"""))
                p.emit(obj("""{"type":"control_request","request_id":"cli-1","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{}}}"""))
                p.emit(obj("""{"type":"control_cancel_request","request_id":"cli-1"}"""))
                p.emit(obj("""{"type":"system","subtype":"brand_new","payload":1}"""))
                p.emit(obj("""{"type":"control_response","response":{"subtype":"success","request_id":$id,"response":{"still_queued":[]}}}"""))
            }
            if (frame["type"].str == "control_response") p.emit(frame) // the CLI echoes our responses
        }
        val control = ControlConnection(JsonLineChannel(fake, scope), scope) { "ours-1" }
        val result = withTimeout(5000) { control.control("interrupt") }
        assertEquals("""{"still_queued":[]}""", result.encode())
        val first = withTimeout(5000) { control.inbound.receive() } as ControlConnection.Inbound.ControlRequest
        assertEquals("cli-1", first.requestId)
        assertEquals("can_use_tool", first.subtype)
        val cancel = withTimeout(5000) { control.inbound.receive() } as ControlConnection.Inbound.ControlCancel
        assertEquals("cli-1", cancel.requestId)
        val unknown = withTimeout(5000) { control.inbound.receive() } as ControlConnection.Inbound.Message
        assertEquals("system", unknown.type)
        assertEquals(1L, control.ignoredResponses)
        // Cancelled requests can no longer be answered.
        try { control.respond("cli-1", null); fail() } catch (_: IllegalStateException) {}
        assertEquals("""{"type":"control_request","request_id":"ours-1","request":{"subtype":"interrupt"}}""", fake.written.first().encode())
    }

    @Test fun controlErrorsSurfaceAsExceptions() = runBlocking {
        val fake = process { p, frame ->
            p.emit(obj("""{"type":"control_response","response":{"subtype":"error","request_id":${frame["request_id"]!!.encode()},"error":"nope"}}"""))
        }
        val control = ControlConnection(JsonLineChannel(fake, scope), scope)
        try { withTimeout(5000) { control.control("set_model", buildJsonObject { put("model", "x") }) }; fail() } catch (e: ControlException) { assertEquals("nope", e.message) }
    }
}
