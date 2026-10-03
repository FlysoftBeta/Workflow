package top.flysoftbeta.workflow.proxy

import java.net.ServerSocket
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.proxy.config.LocalProxyConfig
import top.flysoftbeta.workflow.proxy.config.MihomoConfigTemplate
import top.flysoftbeta.workflow.proxy.controller.MihomoControllerClient
import top.flysoftbeta.workflow.proxy.guardian.GuardianCommand

/** Covers the parts moved out of the Android proxy manager in W1a. */
class ProxyExtractionTest {
    @Test fun templateIsInactiveAndItsLoopbackControllerParses() {
        val secret = MihomoConfigTemplate.newSecret()
        assertTrue(secret.matches(Regex("[0-9a-f]{64}")))
        assertNotEquals(secret, MihomoConfigTemplate.newSecret())
        val text = MihomoConfigTemplate.render(secret)
        assertTrue(text.contains("\ntun:\n  # 改为 true 后以 TUN 接管本机流量。\n  enable: false\n"))
        assertTrue(text.contains("mixed-port: 17890\n"))
        assertTrue(text.endsWith("rules:\n  - MATCH,DIRECT\n"))
        val controller = LocalProxyConfig.controller(text)!!
        assertEquals("http://127.0.0.1:19090", controller.endpoint)
        assertEquals(secret, controller.secret)
        assertFalse("toString must not reveal the secret", controller.toString().contains(secret))
        assertThrows(IllegalArgumentException::class.java) { MihomoConfigTemplate.render("\"injected") }
    }

    @Test fun guardianCommandSingleQuotesEveryArgument() {
        assertEquals("'a'\"'\"'b'", GuardianCommand.quote("a'b"))
        assertEquals("exec '/g' supervise '/k k' '/d' '/c;x' 'id'", GuardianCommand.supervise("/g", "/k k", "/d", "/c;x", "id"))
    }

    @Test fun controllerPathsAreEncodedAsSingleSegments() {
        assertEquals("/proxies/%E8%8A%82%E7%82%B9%20A%2FB%3F", MihomoControllerClient.proxyPath("节点 A/B?"))
    }

    @Test fun occupiedControllerPortIsDetected() {
        ServerSocket(0).use { server -> assertTrue(MihomoControllerClient.acceptsConnections("http://127.0.0.1:${server.localPort}")) }
        val free = ServerSocket(0).use { it.localPort }
        assertFalse(MihomoControllerClient.acceptsConnections("http://127.0.0.1:$free"))
    }
}
