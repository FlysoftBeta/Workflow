package top.flysoftbeta.workflow.platform.proxy

import java.io.PipedInputStream
import java.io.PipedOutputStream
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.core.connection.WorkspaceRpc
import top.flysoftbeta.workflow.core.connection.WorkspaceTransport
import top.flysoftbeta.workflow.core.json.Json
import top.flysoftbeta.workflow.proxy.config.MihomoConfigInspector
import top.flysoftbeta.workflow.proxy.controller.TrafficSample
import top.flysoftbeta.workflow.proxy.runtime.ProxyConfigState
import top.flysoftbeta.workflow.proxy.runtime.ProxyPhase
import top.flysoftbeta.workflow.proxy.runtime.ProxyState

class EngineProxyWorkspaceTest {
    @Test fun reportExcludesConfigCredentialsAndSubscriptionUrls() {
        val inspection = MihomoConfigInspector.inspect("""
            external-controller: 127.0.0.1:29090
            secret: fixture-controller-secret
            proxy-providers:
              fixture:
                type: http
                url: https://example.invalid/subscription?token=fixture-subscription-token
                path: providers/fixture.yaml
            proxy-groups:
              - name: fixture-group
                type: url-test
                proxies: [DIRECT]
                url: https://example.invalid/ping?token=fixture-test-token
        """.trimIndent())
        val report = Json.stringify(proxyReport(ProxyState(config = ProxyConfigState(true, inspection), groups = inspection.groups)))
        assertFalse(report.contains("fixture-controller-secret"))
        assertFalse(report.contains("fixture-subscription-token"))
        assertFalse(report.contains("fixture-test-token"))
        assertFalse(report.contains("example.invalid"))
        assertTrue(report.contains("fixture-group"))
    }

    @Test fun acknowledgedReportAcceptsEngineFieldOrderAndIntegerWidths() = runBlocking<Unit> {
        val fixture = RpcFixture { method, params ->
            if (method == "services.executor.register") return@RpcFixture mapOf("epoch" to "test-epoch")
            assertEquals("services.report", method)
            assertEquals("test-epoch", params["epoch"])
            // Engine objects are sorted; its parsed JSON integers are Long even when Kotlin's report used Int.
            mapOf("state" to (params["state"] as Map<*, *>).entries.sortedBy { it.key.toString() }.associate { it.key to it.value },
                "serviceId" to "proxy", "revision" to 19L)
        }
        try {
            EngineProxyWorkspace(fixture.rpc) { _, _ -> error("Unexpected asset read") }.report(ProxyState(phase = ProxyPhase.RUNNING, pid = 42, traffic = TrafficSample(1, 2, 3, 4)))
        } finally { fixture.close() }
    }

    @Test fun mismatchedAcknowledgementFailsInsteadOfPublishingSuccess() = runBlocking<Unit> {
        val fixture = RpcFixture { method, _ -> if (method == "services.executor.register") mapOf("epoch" to "test-epoch") else mapOf("serviceId" to "proxy", "state" to mapOf("phase" to "running"), "revision" to 1) }
        try { assertTrue(runCatching { EngineProxyWorkspace(fixture.rpc) { _, _ -> error("Unexpected asset read") }.report(ProxyState()) }.isFailure) }
        finally { fixture.close() }
    }

    @Test fun configImportUsesEngineCommandWithLeaseAndExpectedRevision() = runBlocking<Unit> {
        val fixture = RpcFixture { method, params ->
            when (method) {
                "services.executor.register" -> mapOf("epoch" to "epoch")
                "documents.read" -> { assertEquals("services.proxy", params["namespace"]); mapOf("document" to "mode: direct\n", "revision" to 7) }
                "services.command" -> {
                    assertEquals("epoch", params["epoch"])
                    assertEquals("importConfig", params["name"])
                    val args = WorkspaceRpc.obj(params["args"])
                    assertEquals(7L, args["expectedRevision"])
                    assertEquals("mode: rule\n", args["text"])
                    mapOf("operation" to null, "config" to mapOf("text" to args["text"], "revision" to 8))
                }
                else -> error("Unexpected method")
            }
        }
        try {
            val workspace = EngineProxyWorkspace(fixture.rpc) { _, _ -> error("Unexpected asset read") }
            val config = workspace.readConfig()
            workspace.writeConfig("mode: rule\n", config.revision)
        } finally { fixture.close() }
    }

    @Test fun commandsAndReceiptsRemainBoundToOneExecutorEpoch() = runBlocking<Unit> {
        var registrations = 0
        val fixture = RpcFixture { method, params ->
            when (method) {
                "services.executor.register" -> { registrations++; mapOf("epoch" to "epoch") }
                "services.command" -> {
                    assertEquals("epoch", params["epoch"])
                    mapOf("operation" to mapOf("id" to "ticket", "name" to "setMode", "args" to mapOf("mode" to "direct")))
                }
                "services.complete" -> {
                    assertEquals("epoch", params["epoch"]); assertEquals("ticket", params["operationId"])
                    mapOf("serviceId" to "proxy", "state" to params["state"])
                }
                else -> error("Unexpected method")
            }
        }
        try {
            val workspace = EngineProxyWorkspace(fixture.rpc) { _, _ -> error("Unexpected asset read") }
            val ticket = workspace.command("setMode", mapOf("mode" to "rule"))
            assertEquals("direct", ticket.args["mode"])
            workspace.complete(ticket, true, ProxyState())
            assertEquals(1, registrations)
        } finally { fixture.close() }
    }

    private class RpcFixture(handler: (String, Map<String, Any?>) -> Any?) {
        private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
        private val clientInput = PipedInputStream(65536)
        private val serverOutput = PipedOutputStream(clientInput)
        private val serverInput = PipedInputStream(65536)
        private val clientOutput = PipedOutputStream(serverInput)
        val rpc = WorkspaceRpc(object : WorkspaceTransport {
            override val input = clientInput
            override val output = clientOutput
            override fun close() { clientInput.close(); clientOutput.close() }
        }, scope)
        init {
            scope.launch {
                serverInput.bufferedReader().use { reader ->
                    val writer = serverOutput.bufferedWriter()
                    while (true) {
                        val line = reader.readLine() ?: break
                        val request = WorkspaceRpc.obj(Json.parse(line))
                        val response = handler(request["method"] as String, WorkspaceRpc.obj(request["params"]))
                        writer.write(Json.stringify(mapOf("jsonrpc" to "2.0", "id" to request["id"], "result" to response)))
                        writer.newLine(); writer.flush()
                    }
                }
            }
        }
        fun close() { rpc.close(); scope.cancel(); serverInput.close(); serverOutput.close() }
    }
}
