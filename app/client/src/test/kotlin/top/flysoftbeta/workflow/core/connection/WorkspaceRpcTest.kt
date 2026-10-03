package top.flysoftbeta.workflow.core.connection

import java.io.PipedInputStream
import java.io.PipedOutputStream
import java.util.concurrent.atomic.AtomicInteger
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.first
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.core.json.Json

class WorkspaceRpcTest {
    @Test fun rejectsProtocolMismatchAndClosesTransport() = runBlocking<Unit> {
        RpcPeer { request -> response(request, mapOf("protocol" to "workflow.workspace/2")) }.use { peer ->
            assertTrue(runCatching { peer.rpc.hello("test") }.isFailure)
            assertNotNull(peer.rpc.failure.value)
            assertEquals(1, peer.closes.get())
        }
    }

    @Test fun concurrentHelloIsOneHandshakeAndCannotChangeClientIdentity() = runBlocking<Unit> {
        val calls = AtomicInteger()
        RpcPeer { request -> calls.incrementAndGet(); response(request, greeting) }.use { peer ->
            coroutineScope { List(8) { async { peer.rpc.hello("test") } }.awaitAll() }
            assertEquals(1, calls.get())
            assertTrue(runCatching { peer.rpc.hello("other") }.isFailure)
            assertNull(peer.rpc.failure.value)
        }
    }

    @Test fun malformedResponseFailsAllWaitersAndRetiresTransport() = runBlocking<Unit> {
        RpcPeer { request ->
            if (request["method"] == "slow") { delay(60_000); null }
            else mapOf("jsonrpc" to "2.0", "id" to request["id"], "result" to null, "error" to mapOf("code" to -1, "message" to "invalid"))
        }.use { peer ->
            val slow = async { runCatching { peer.rpc.request("slow") } }
            delay(20)
            assertTrue(runCatching { peer.rpc.request("malformed") }.isFailure)
            assertTrue(withTimeout(2_000) { slow.await() }.isFailure)
            withTimeout(2_000) { peer.rpc.failure.first { it != null } }
            assertEquals(1, peer.closes.get())
        }
    }

    @Test fun timedOutRequestDoesNotConsumeLateResponseOrPoisonTheNextRequest() = runBlocking<Unit> {
        RpcPeer { request ->
            if (request["method"] == "slow") delay(100)
            response(request, request["method"])
        }.use { peer ->
            assertTrue(runCatching { peer.rpc.request("slow", timeoutMs = 10) }.exceptionOrNull() is TimeoutCancellationException)
            delay(150)
            assertEquals("next", peer.rpc.request("next"))
            assertNull(peer.rpc.failure.value)
        }
    }

    @Test fun responseMustUseIntegerErrorCodeAndExactStringRequestId() = runBlocking<Unit> {
        for (frame in listOf(
            mapOf("jsonrpc" to "2.0", "id" to "r1", "error" to mapOf("code" to -1.5, "message" to "bad")),
            mapOf("jsonrpc" to "2.0", "id" to 1, "result" to "wrong"),
        )) RpcPeer { frame }.use { peer -> assertTrue(runCatching { peer.rpc.request("test") }.isFailure) }
    }

    companion object {
        val greeting = mapOf("protocol" to WorkspaceRpc.PROTOCOL, "engineVersion" to "1.0.0", "workspaceRoot" to "/test",
            "capabilities" to mapOf("workspace" to true, "files" to true, "documents" to true, "environment" to true,
                "processes" to true, "pty" to true, "services" to true, "maxFrameBytes" to WorkspaceRpc.MAX_FRAME_BYTES, "maxBlobChunkBytes" to 65_536))
        fun response(request: Map<String, Any?>, result: Any?) = mapOf("jsonrpc" to "2.0", "id" to request["id"], "result" to result)
    }
}

internal class RpcPeer(private val handler: suspend (Map<String, Any?>) -> Map<String, Any?>?) : AutoCloseable {
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val clientInput = PipedInputStream(131072)
    private val serverOutput = PipedOutputStream(clientInput)
    private val serverInput = PipedInputStream(131072)
    private val clientOutput = PipedOutputStream(serverInput)
    val closes = AtomicInteger()
    val rpc = WorkspaceRpc(object : WorkspaceTransport {
        override val input = clientInput
        override val output = clientOutput
        override fun close() {
            closes.incrementAndGet()
            clientInput.close(); clientOutput.close(); serverInput.close(); serverOutput.close()
        }
    }, scope)
    init {
        scope.launch {
            runCatching {
                serverInput.bufferedReader().useLines { lines ->
                    lines.forEach { line ->
                        val request = WorkspaceRpc.obj(Json.parse(line))
                        launch {
                            val response = handler(request) ?: return@launch
                            synchronized(serverOutput) {
                                runCatching { serverOutput.write((Json.stringify(response) + "\n").toByteArray()); serverOutput.flush() }
                            }
                        }
                    }
                }
            }
        }
    }
    override fun close() { rpc.close(); scope.cancel() }
}
