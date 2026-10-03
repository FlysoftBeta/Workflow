package top.flysoftbeta.workflow.feature.proxy

import android.graphics.Bitmap
import android.os.Build
import androidx.activity.ComponentActivity
import androidx.compose.ui.test.SemanticsNodeInteraction
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.getBoundsInRoot
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithContentDescription
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import java.io.InputStream
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.Proxy
import java.net.ServerSocket
import java.net.Socket
import java.util.UUID
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.app.Shell
import top.flysoftbeta.workflow.app.Space
import top.flysoftbeta.workflow.app.WorkflowApp
import top.flysoftbeta.workflow.app.panel.PanelWiring
import top.flysoftbeta.workflow.core.config.ThemeMode
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.Placement
import top.flysoftbeta.workflow.core.layout.ProxyPage
import top.flysoftbeta.workflow.core.store.WorkspaceStore
import top.flysoftbeta.workflow.feature.workbench.WorkbenchRuntime
import top.flysoftbeta.workflow.platform.proxy.ProxyService
import top.flysoftbeta.workflow.platform.proxy.EngineProxyFixture
import top.flysoftbeta.workflow.platform.proxy.ShellRoot
import top.flysoftbeta.workflow.proxy.controller.DelayResult
import top.flysoftbeta.workflow.proxy.controller.ProxyMode
import top.flysoftbeta.workflow.proxy.runtime.ProxyApi
import top.flysoftbeta.workflow.proxy.runtime.ProxyPhase
import top.flysoftbeta.workflow.proxy.runtime.ProxyTiming
import top.flysoftbeta.workflow.proxy.runtime.RootShell
import top.flysoftbeta.workflow.proxy.runtime.RootStatus
import top.flysoftbeta.workflow.proxy.runtime.ShellResult
import top.flysoftbeta.workflow.ui.design.dnd.DragDropState

/**
 * The proxy panels through the real Workbench UI, driving a **real Mihomo** (TUN off) through the real
 * guardian. Emulator only (root comes from `adb root`, see [ShellRoot]); gated like the root TUN test so
 * it can never run on the user's tablet: `-e proxyEmulator true`, ranchu/goldfish hardware, uid 0.
 *
 * Nodes are local fake HTTP proxies with fixed latencies (80 / 420 / 950 ms / never), so latency colours
 * and timeouts are deterministic and no traffic leaves the device. Screenshots go to
 * `files/screenshots/proxy/` in the app's external files dir (pulled to artifacts/ui/proxy/).
 */
@RunWith(AndroidJUnit4::class)
class ProxyPanelEmulatorTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private val secret = UUID.randomUUID().toString().replace("-", "")
    private val servers = mutableListOf<ServerSocket>()
    private lateinit var root: ShellRoot
    private lateinit var fixture: File
    private lateinit var engine: EngineProxyFixture
    private lateinit var api: ProxyApi
    private lateinit var store: WorkspaceStore
    private lateinit var shell: Shell
    private lateinit var runtime: WorkbenchRuntime
    private val shots by lazy { File(compose.activity.getExternalFilesDir(null), "screenshots/proxy").apply { mkdirs() } }

    @Before fun gate() {
        assumeTrue("Explicit -e proxyEmulator true is required", InstrumentationRegistry.getArguments().getString("proxyEmulator") == "true")
        assumeTrue("Emulator only", Build.HARDWARE == "ranchu" || Build.HARDWARE == "goldfish")
        val context = instrumentation.targetContext
        root = ShellRoot(instrumentation, File(context.cacheDir, "proxy-ui-root"))
        assumeTrue("adb root is required", root.uid() == "0")
        root.prepareTunTool()
        engine = runBlocking { EngineProxyFixture.connect(context) }
        fixture = engine.staging
    }

    @After fun tearDown() {
        if (::api.isInitialized) runBlocking { withTimeout(20_000) { api.stop() } }
        if (::runtime.isInitialized) compose.runOnUiThread { runtime.dispose() }
        servers.forEach { runCatching { it.close() } }
        if (::root.isInitialized) { root.run("ip link delete Meta 2>/dev/null; true"); root.restoreTunTool() }
        scope.cancel()
        if (::engine.isInitialized) engine.close()
    }

    // ------------------------------------------------------------------ fixture

    private fun startUi(rootShell: RootShell = root.rootShell, theme: ThemeMode = ThemeMode.LIGHT) {
        val context = compose.activity.applicationContext
        api = engine.bind(ProxyService.ports(context, rootShell, root.Launcher()),
            ProxyTiming(controllerReadyMs = 20_000))
        store = engine.session.store
        runBlocking {
            store.awaitReady()
            store.updateConfig { it.copy(appearance = it.appearance.copy(theme = theme)) }
        }
        shell = Shell(context, store, PanelWiring.create(context, proxy = ProxyPanelProvider { api }), scope)
        runtime = WorkbenchRuntime(context, store, shell.registry, shell)
        val dnd = DragDropState()
        compose.setContent { WorkflowApp(shell, runtime, dnd) }
    }

    private fun openProxy(solo: Boolean = false) {
        compose.runOnUiThread { if (solo) shell.openInSeparateSession(PanelTarget.Proxy()) else shell.openInCurrentSession(PanelTarget.Proxy()) }
        compose.waitUntil(10_000) { shell.space == Space.WORKBENCH && compose.onAllNodesWithTag("proxy:overview").fetchSemanticsNodes().isNotEmpty() }
    }

    private fun openPage(page: ProxyPage) {
        runBlocking {
            val id = store.state.value.activeSessionId!!
            val wb = store.state.value.activeSession!!.workbench
            val stack = wb.stackOf(wb.panelFor(PanelTarget.Proxy())!!.id)!!
            store.applyLayout(id, LayoutOp.Open(PanelTarget.Proxy(page), Placement.InStack(stack)))
        }
        compose.waitForIdle()
    }

    private fun shot(name: String) {
        compose.waitForIdle()
        Thread.sleep(700) // animations (≤ 300ms) and the 300ms loading-indicator delay
        val bitmap: Bitmap = instrumentation.uiAutomation.takeScreenshot() ?: error("screenshot failed")
        File(shots, "$name.png").outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
    }

    private fun shown(text: String) = compose.onAllNodesWithText(text).fetchSemanticsNodes().isNotEmpty()

    private fun waitPhase(phase: ProxyPhase, timeout: Long = 30_000) = compose.waitUntil(timeout) { api.state.value.phase == phase }

    /** A local server; [handler] runs per connection on its own thread. */
    private fun serve(handler: (Socket) -> Unit): Int {
        val server = ServerSocket(0, 16, InetAddress.getByName("127.0.0.1")).also { servers += it }
        Thread({
            while (!server.isClosed) {
                val client = runCatching { server.accept() }.getOrNull() ?: break
                Thread({ runCatching { client.use(handler) } }, "proxy-ui-conn").apply { isDaemon = true; start() }
            }
        }, "proxy-ui-server").apply { isDaemon = true; start() }
        return server.localPort
    }

    private fun readHead(input: InputStream): String? {
        val seen = StringBuilder()
        while (!seen.endsWith("\r\n\r\n")) { val next = input.read(); if (next < 0) return null; seen.append(next.toChar()) }
        return seen.toString()
    }

    /** An HTTP CONNECT proxy that answers the tunneled request with 204 after [delayMs]; null never answers. */
    private fun fakeNode(delayMs: Long?): Int = serve { client ->
        client.soTimeout = 30_000
        val input = client.getInputStream()
        readHead(input) ?: return@serve
        client.getOutputStream().write("HTTP/1.1 200 Connection established\r\n\r\n".toByteArray())
        readHead(input) ?: return@serve
        if (delayMs == null) { Thread.sleep(30_000); return@serve }
        Thread.sleep(delayMs)
        client.getOutputStream().write("HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n".toByteArray())
    }

    /** DIRECT target: 204 for tests, or a slow endless body (traffic + a visible connection). */
    private fun echo(): Int = serve { client ->
        client.soTimeout = 30_000
        val head = readHead(client.getInputStream()) ?: return@serve
        val out = client.getOutputStream()
        if (head.startsWith("GET /bulk")) {
            out.write("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\n\r\n".toByteArray())
            val chunk = ByteArray(24 * 1024)
            repeat(600) { out.write(chunk); out.flush(); Thread.sleep(100) }
        } else out.write("HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n".toByteArray())
    }

    private data class Ports(val mixed: Int, val control: Int, val echo: Int, val bulk: Int)

    private fun writeConfig(tun: Boolean = false, mixedPort: Int? = null): Ports {
        val ports = Ports(mixedPort ?: ServerSocket(0).use { it.localPort }, ServerSocket(0).use { it.localPort }, echo(), echo())
        val hk = fakeNode(80); val jp = fakeNode(420); val us = fakeNode(950); val sg = fakeNode(null)
        val url = "http://127.0.0.1:${ports.echo}/generate_204"
        runBlocking { engine.config("""
            mixed-port: ${ports.mixed}
            bind-address: "127.0.0.1"
            allow-lan: false
            mode: rule
            log-level: info
            external-controller: 127.0.0.1:${ports.control}
            secret: "$secret"
            tun:
              enable: $tun
              device: wf-ui-tun
              stack: gvisor
              auto-route: true
              dns-hijack: []
              iproute2-table-index: 9500
              iproute2-rule-index: 9500
              include-uid: [${android.os.Process.myUid()}]
            dns:
              enable: false
            proxies:
              - {name: 香港 01, type: http, server: 127.0.0.1, port: $hk}
              - {name: 日本 02, type: http, server: 127.0.0.1, port: $jp}
              - {name: 美国 03, type: http, server: 127.0.0.1, port: $us}
              - {name: 新加坡 04, type: http, server: 127.0.0.1, port: $sg}
            proxy-groups:
              - {name: 节点选择, type: select, proxies: [自动选择, 香港 01, 日本 02, 美国 03, 新加坡 04, DIRECT], url: "$url"}
              - {name: 自动选择, type: url-test, proxies: [香港 01, 日本 02, 美国 03, 新加坡 04], url: "$url", interval: 600}
              - {name: 流媒体, type: select, proxies: [节点选择, 日本 02, 美国 03], url: "$url"}
            rules:
              - DST-PORT,${ports.echo},节点选择
              - MATCH,DIRECT
        """.trimIndent() + "\n") }
        return ports
    }

    private fun node(group: String, name: String): SemanticsNodeInteraction = compose.onNodeWithTag("proxy:node:$group:$name")

    /** The More button of the proxy panel's own stack (the one on the same row, right of the 代理 tab). */
    private fun proxyMore(): SemanticsNodeInteraction {
        val tab = compose.onAllNodesWithText("代理", useUnmergedTree = true)[0].getBoundsInRoot()
        val candidates = compose.onAllNodesWithContentDescription("更多")
        val count = candidates.fetchSemanticsNodes().size
        val index = (0 until count).first { i ->
            val bounds = candidates[i].getBoundsInRoot()
            bounds.left > tab.left && kotlin.math.abs((bounds.top - tab.top).value) < 12f
        }
        return candidates[index]
    }

    // ------------------------------------------------------------------ tests

    @Test fun overviewDrivesARealKernel() {
        val ports = writeConfig()
        startUi()
        openProxy()
        // Stopped: groups come from config.yaml, disabled; the mode is the configured one.
        compose.waitUntil(10_000) { shown("节点选择") && shown("自动选择") }
        assertFalse(api.state.value.live)
        assertEquals(ProxyMode.RULE, api.state.value.mode)
        shot("01-stopped-from-config")

        compose.onNodeWithTag("proxy:power").performClick()
        waitPhase(ProxyPhase.RUNNING)
        compose.waitUntil(10_000) { api.state.value.live && api.state.value.groups.group("节点选择")?.now != null }
        assertEquals(RootStatus.GRANTED, api.state.value.capabilities.root)
        assertEquals("running", runBlocking { engine.serviceState()["phase"] })
        shot("02-running")

        // Group test from the tool row: colours by tier, the silent node times out.
        compose.onNodeWithContentDescription("延迟测试").performClick()
        compose.waitUntil(30_000) { api.state.value.delays["新加坡 04"] != null && api.state.value.testing.isEmpty() }
        val delays = api.state.value.delays
        assertTrue(delays.toString(), (delays["香港 01"] as DelayResult.Success).ms < 300)
        assertTrue(delays.toString(), (delays["日本 02"] as DelayResult.Success).ms in 300..799)
        assertTrue(delays.toString(), (delays["美国 03"] as DelayResult.Success).ms >= 800)
        assertEquals(DelayResult.Timeout, delays["新加坡 04"])
        compose.waitUntil(5_000) { shown("超时") }
        shot("03-latency")

        // Select a node: the kernel confirms, the chip moves.
        node("节点选择", "日本 02").performClick()
        compose.waitUntil(10_000) { api.state.value.groups.group("节点选择")?.now == "日本 02" }

        // Traffic through the mixed port (DIRECT, slow body) shows in the strip and as a connection.
        val load = Thread({
            runCatching {
                val proxy = Proxy(Proxy.Type.HTTP, InetSocketAddress("127.0.0.1", ports.mixed))
                val connection = java.net.URL("http://127.0.0.1:${ports.bulk}/bulk").openConnection(proxy)
                connection.readTimeout = 60_000
                connection.getInputStream().use { input -> val buffer = ByteArray(65536); while (input.read(buffer) >= 0) Unit }
            }
        }, "proxy-ui-load").apply { isDaemon = true; start() }
        compose.waitUntil(15_000) { (api.state.value.traffic?.down ?: 0) > 1024 }
        compose.onNodeWithTag("proxy:traffic").fetchSemanticsNode()
        shot("04-selected-traffic")

        // Mode switch: global lists GLOBAL only.
        compose.onNodeWithText("全局").performClick()
        compose.waitUntil(10_000) { api.state.value.mode == ProxyMode.GLOBAL && shown("GLOBAL") }
        shot("05-global")
        compose.onNodeWithText("规则").performClick()
        compose.waitUntil(10_000) { api.state.value.mode == ProxyMode.RULE }

        proxyMore().performClick()
        compose.waitUntil(5_000) { shown("编辑配置") && shown("日志") && shown("连接") }
        shot("06-more-menu")
        compose.onNodeWithText("连接").performClick()
        compose.waitUntil(5_000) { !shown("导入配置…") }

        compose.waitUntil(10_000) { compose.onAllNodesWithTag("proxy:connections").fetchSemanticsNodes().isNotEmpty() }
        runBlocking { api.connections().getOrThrow() }
        shot("07-connections-loading")
        compose.waitUntil(15_000) { compose.onAllNodesWithText("个连接", substring = true).fetchSemanticsNodes().isNotEmpty() }
        compose.waitUntil(15_000) { compose.onAllNodesWithText("127.0.0.1:${ports.bulk}", substring = true).fetchSemanticsNodes().isNotEmpty() }
        shot("07-connections")

        openPage(ProxyPage.LOGS)
        compose.waitUntil(10_000) { compose.onAllNodesWithTag("proxy:logs").fetchSemanticsNodes().isNotEmpty() }
        compose.waitUntil(10_000) { compose.onAllNodesWithText("INFO", substring = true).fetchSemanticsNodes().isNotEmpty() }
        runBlocking {
            val tail = api.logTail(65_536)
            assertEquals(tail, engine.read(ProxyService.LOG_PATH))
            assertFalse("secret must never reach the Engine log", tail.contains(secret))
        }
        shot("08-logs")
        load.interrupt()

        // Back to the overview and stop from the status strip.
        compose.onAllNodesWithText("代理", useUnmergedTree = true)[0].performClick()
        compose.waitUntil(5_000) { compose.onAllNodesWithTag("proxy:overview").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithTag("proxy:power").performClick()
        waitPhase(ProxyPhase.STOPPED)
        assertFalse(File(fixture, ".process.json").exists())
        // The kernel's last groups (with the selection Mihomo restores) stay visible while stopped.
        compose.waitUntil(10_000) { api.state.value.groups.group("节点选择")?.now == "日本 02" }
        shot("09-stopped-after-run")
    }

    @Test fun conflictErrorAndEmptyStates() {
        startUi()
        openProxy()
        // No config.yaml yet.
        compose.waitUntil(10_000) { shown("没有代理配置") }
        shot("10-no-config")

        // Another TUN exists and our config wants a TUN: refused before kernel launch, explained on request.
        writeConfig(tun = true)
        root.run("ip tuntap add dev Meta mode tun && ip link set Meta up").also { assertEquals(it.output, 0, it.code) }
        compose.waitUntil(10_000) { shown("节点选择") }
        runBlocking { api.refresh() }
        assertTrue("fixture must enable TUN", api.state.value.config.inspection!!.tun.enable)
        compose.onNodeWithTag("proxy:power").assertIsEnabled().performClick()
        try {
            compose.waitUntil(10_000) { api.state.value.phase == ProxyPhase.CONFLICT }
        } catch (failure: Throwable) {
            shot("11-conflict-failure")
            val current = api.state.value
            throw AssertionError("Expected CONFLICT, phase=${current.phase}, tun=${current.tun}, configuredTun=${current.config.inspection?.tun}, progress=${current.progress}, error=${current.error}", failure)
        }
        assertEquals(listOf("Meta"), api.state.value.conflict.foreignInterfaces)
        assertTrue("nothing was started", root.run("ip link show wf-ui-tun").code != 0)
        compose.waitUntil(10_000) { shown("编辑配置") && shown("取消") }
        assertFalse(shown("仍然启动"))
        shot("11-conflict-dialog")
        compose.onNodeWithText("取消").performClick()
        compose.waitUntil(5_000) { !shown("取消") }
        shot("12-conflict-banner")
        root.run("ip link delete Meta")

        // A start failure is shown where it happened, with the log one tap away.
        val busy = ServerSocket(0, 1, InetAddress.getByName("127.0.0.1")).also { servers += it }
        writeConfig(mixedPort = busy.localPort)
        runBlocking { api.refresh() }
        compose.onNodeWithTag("proxy:power").performClick()
        compose.waitUntil(15_000) { api.state.value.phase == ProxyPhase.ERROR }
        compose.waitUntil(5_000) { compose.onAllNodesWithTag("proxy:error").fetchSemanticsNodes().isNotEmpty() }
        shot("13-start-error")
    }

    @Test fun rootDeniedOffersARequest() {
        writeConfig()
        val denied = object : RootShell {
            override suspend fun run(script: String, timeoutMs: Long) = ShellResult(1, "Permission denied")
        }
        startUi(rootShell = denied)
        openProxy()
        compose.waitUntil(10_000) { shown("节点选择") }
        compose.onNodeWithTag("proxy:power").performClick()
        compose.waitUntil(10_000) { api.state.value.capabilities.root == RootStatus.DENIED }
        compose.waitUntil(5_000) { compose.onAllNodesWithTag("proxy:root").fetchSemanticsNodes().isNotEmpty() }
        assertEquals(ProxyPhase.ERROR, api.state.value.phase)
        shot("14-root-denied")
    }

    @Test fun darkSoloSession() {
        writeConfig()
        startUi(theme = ThemeMode.DARK)
        openProxy(solo = true)
        compose.waitUntil(10_000) { shown("节点选择") }
        compose.onNodeWithTag("proxy:power").performClick()
        waitPhase(ProxyPhase.RUNNING)
        compose.waitUntil(10_000) { api.state.value.live }
        compose.onNodeWithContentDescription("延迟测试").performClick()
        compose.waitUntil(30_000) { api.state.value.delays["新加坡 04"] != null && api.state.value.testing.isEmpty() }
        assertNotNull(api.state.value.groups.group("自动选择"))
        shot("15-dark-solo")
    }
}
