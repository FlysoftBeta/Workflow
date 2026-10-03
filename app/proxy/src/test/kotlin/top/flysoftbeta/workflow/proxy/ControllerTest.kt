package top.flysoftbeta.workflow.proxy

import java.io.IOException
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.take
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.After
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.proxy.config.ProxyControllerSettings
import top.flysoftbeta.workflow.proxy.controller.ControllerException
import top.flysoftbeta.workflow.proxy.controller.DelayResult
import top.flysoftbeta.workflow.proxy.controller.MihomoController
import top.flysoftbeta.workflow.proxy.controller.MihomoControllerClient
import top.flysoftbeta.workflow.proxy.controller.ProviderKind
import top.flysoftbeta.workflow.proxy.controller.ProxyLogLevel
import top.flysoftbeta.workflow.proxy.controller.ProxyMode

class ControllerTest {
    private val server = FakeMihomo()
    private val controller = MihomoController(server.settings)

    @After fun close() = server.close()

    @Test fun everyRequestCarriesTheBearerSecretOnlyInTheHeader() = runBlocking<Unit> {
        assertEquals("v1.19.31", controller.version())
        controller.proxies()
        assertTrue(server.calls.all { it.authorization == "Bearer ${server.secret}" })
        assertTrue(server.calls.none { (it.query ?: "").contains(server.secret) || it.path.contains(server.secret) })
    }

    @Test fun wrongSecretIsReportedWithoutEchoingIt() = runBlocking<Unit> {
        val wrong = MihomoController(ProxyControllerSettings(server.settings.endpoint, "wrong-secret-123"))
        val error = assertThrows(ControllerException::class.java) { runBlocking { wrong.version() } }
        assertEquals(401, error.status)
        assertFalse(error.message.orEmpty().contains("wrong-secret-123"))
        assertEquals("控制接口拒绝了配置中的 secret", error.message)
    }

    @Test fun modeSwitchIsVerifiedByReadback() = runBlocking<Unit> {
        for (mode in ProxyMode.entries) {
            controller.setMode(mode)
            assertEquals(mode.wire, server.mode)
            assertEquals(mode.wire, controller.mode())
        }
        assertTrue(server.calls.any { it.method == "PATCH" && it.body == """{"mode":"direct"}""" })
    }

    @Test fun groupsFollowGlobalOrderAndExposeMembers() = runBlocking<Unit> {
        val snapshot = controller.proxies()
        assertEquals(listOf("Proxy", "Auto", "Hidden"), snapshot.groups.map { it.name })
        assertEquals("GLOBAL", snapshot.global?.name)
        assertEquals(listOf("Proxy", "Auto"), snapshot.visibleGroups(ProxyMode.RULE).map { it.name })
        assertEquals(listOf("GLOBAL"), snapshot.visibleGroups(ProxyMode.GLOBAL).map { it.name })
        assertTrue(snapshot.visibleGroups(ProxyMode.DIRECT).isEmpty())
        val proxy = snapshot.group("Proxy")!!
        assertEquals("HK 01", proxy.now)
        assertEquals(listOf("Auto", "HK 01", "JP 02", "DIRECT"), proxy.members.map { it.name })
        assertEquals("HK 01", proxy.members[0].now) // nested group shows its selection
        assertEquals(120, proxy.members[1].delayMs)
        assertNull(proxy.members[2].delayMs) // last test failed (0)
        assertFalse(proxy.members[2].alive)
        val auto = snapshot.group("Auto")!!
        assertEquals("URLTest", auto.type)
        assertEquals("https://cp.example/generate_204", auto.testUrl)
    }

    @Test fun selectionIsEncodedAndVerified() = runBlocking<Unit> {
        controller.select("Proxy", "JP 02")
        assertEquals("JP 02", server.selected["Proxy"])
        assertTrue(server.calls.any { it.method == "PUT" && it.path == "/proxies/Proxy" && it.body == """{"name":"JP 02"}""" })
        server.ignoreSelection = true
        val error = assertThrows(IOException::class.java) { runBlocking { controller.select("Proxy", "HK 01") } }
        assertEquals("控制接口未确认节点切换", error.message)
    }

    @Test fun nodeDelayMapsSuccessTimeoutAndFailure() = runBlocking<Unit> {
        assertEquals(DelayResult.Success(120), controller.nodeDelay("HK 01"))
        assertTrue(controller.nodeDelay("JP 02") is DelayResult.Failed)
        server.delays["JP 02"] = -1
        assertEquals(DelayResult.Timeout, controller.nodeDelay("JP 02", timeoutMs = 2000))
        val call = server.calls.last()
        assertEquals("/proxies/JP%2002/delay", call.path)
        assertEquals("url=https%3A%2F%2Fwww.gstatic.com%2Fgenerate_204&timeout=2000", call.query)
        assertThrows(ControllerException::class.java) { runBlocking { controller.nodeDelay("missing") } }
        assertThrows(IllegalArgumentException::class.java) { runBlocking { controller.nodeDelay("HK 01", "file:///etc/passwd") } }
    }

    @Test fun groupDelayCoversEveryMember() = runBlocking<Unit> {
        val results = controller.groupDelay("Auto", "https://cp.example/generate_204")
        assertEquals(mapOf("HK 01" to DelayResult.Success(120), "JP 02" to DelayResult.Timeout), results)
        server.delays["HK 01"] = 0
        assertEquals(setOf(DelayResult.Timeout), controller.groupDelay("Auto").values.toSet())
    }

    @Test fun trafficStreamDeliversSamplesAndClosesOnCancel() = runBlocking<Unit> {
        val samples = withTimeout(5000) { controller.traffic().take(3).toList() }
        assertEquals(listOf(10L, 20L, 30L), samples.map { it.up })
        assertEquals(30000L, samples.last().downTotal)
        withTimeout(5000) { while (server.openStreams.get() != 0) delay(20) }
    }

    @Test fun cancellingACollectorStopsTheStream() = runBlocking<Unit> {
        val job = launch { controller.traffic().collect { } }
        withTimeout(5000) { while (server.trafficLines.get() < 2) delay(20) }
        job.cancel(); job.join()
        withTimeout(5000) { while (server.openStreams.get() != 0) delay(20) }
    }

    @Test fun logStreamRedactsTheSecret() = runBlocking<Unit> {
        val lines = withTimeout(5000) { controller.logs(ProxyLogLevel.DEBUG).take(2).toList() }
        assertEquals("line 1 auth=[已隐藏]", lines[0].payload)
        assertEquals("info", lines[0].level)
        assertEquals("level=debug", server.calls.last { it.path == "/logs" }.query)
    }

    @Test fun connectionsSnapshotIsReadOnly() = runBlocking<Unit> {
        val snapshot = controller.connections()
        assertEquals(20L, snapshot.uploadTotal)
        val connection = snapshot.connections.single()
        assertEquals("example.com", connection.host)
        assertEquals("93.184.216.34:443", connection.destination)
        assertEquals("198.18.0.1:40000", connection.source)
        assertEquals(listOf("HK 01", "Proxy"), connection.chains)
        assertTrue(server.calls.none { it.method == "DELETE" })
    }

    @Test fun providerHealthcheckFailureIsNotReportedAsSuccess() = runBlocking<Unit> {
        server.healthcheckStatus = 503
        val result = controller.refreshProviders().first { it.provider.name == "sub" }
        assertNotNull(result.error)
        assertTrue(result.error!!.contains("healthcheck failed"))
        assertFalse(result.error!!.contains(server.secret))
    }

    @Test fun providersSkipCompatibleAndRefreshReportsPerProviderErrors() = runBlocking<Unit> {
        val providers = controller.providers()
        assertEquals(listOf("bad" to ProviderKind.PROXY, "sub" to ProviderKind.PROXY, "ads" to ProviderKind.RULE), providers.map { it.name to it.kind })
        assertEquals(42, providers.last().size)
        val results = controller.refreshProviders()
        assertEquals(listOf("proxies/bad", "proxies/sub", "rules/ads"), server.providerUpdates.toList())
        val failure = results.first { it.provider.name == "bad" }.error!!
        assertTrue(failure, failure.contains("fetch failed token=[已隐藏]"))
        assertTrue(results.filter { it.provider.name != "bad" }.all { it.error == null })
    }

    @Test fun malformedAndErrorBodiesNeverLeakTheSecret() = runBlocking<Unit> {
        server.errorBody = """{"message":"boom ${server.secret}"}"""
        val error = assertThrows(ControllerException::class.java) { runBlocking { controller.mode() } }
        assertEquals("代理控制请求失败（HTTP 500）：boom [已隐藏]", error.message)
        server.errorBody = null
        val broken = FakeMihomo(secret = "x")
        broken.close()
        val unreachable = assertThrows(IOException::class.java) { runBlocking { MihomoController(broken.settings).version() } }
        assertEquals("代理控制接口不可达", unreachable.message)
    }

    @Test fun blockingTransportStillWorks() {
        val text = MihomoControllerClient().request(server.settings, "GET", "/version")
        assertTrue(text.contains("v1.19.31"))
    }
}
