package top.flysoftbeta.workflow.platform.proxy

import android.os.Build
import android.os.Process
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import java.net.InetSocketAddress
import java.net.ServerSocket
import java.net.Socket
import java.util.UUID
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import org.json.JSONArray
import org.json.JSONObject
import org.junit.After
import org.junit.Assert.*
import org.junit.Assume.assumeTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.platform.service.LocalRuntimeService
import top.flysoftbeta.workflow.proxy.controller.DelayResult
import top.flysoftbeta.workflow.proxy.controller.ProxyLogLevel
import top.flysoftbeta.workflow.proxy.controller.ProxyMode
import top.flysoftbeta.workflow.proxy.runtime.ProxyApi
import top.flysoftbeta.workflow.proxy.runtime.ProxyConflictException
import top.flysoftbeta.workflow.proxy.runtime.ProxyPhase
import top.flysoftbeta.workflow.proxy.runtime.ProxyTiming

/**
 * Real root TUN lifecycle on the **emulator only**: real guardian, real Mihomo, real policy routing.
 *
 * Triple-gated so it can never run on the user's tablet (which runs ClashMetaForAndroid's TUN):
 * `-e proxyEmulatorRoot true`, an emulator build (ranchu/goldfish), and uid 0 for shell commands (`adb root`).
 * The TUN only captures this test app's uid (`include-uid`), DNS is untouched, and every run checks that
 * the interface and policy rules are gone afterwards. Results: files/proxy-emulator-result.json.
 */
@RunWith(AndroidJUnit4::class)
class ProxyEmulatorRootTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val context = instrumentation.targetContext
    private lateinit var root: ShellRoot
    private lateinit var launcher: ShellRoot.Launcher
    private lateinit var fixture: File
    private lateinit var engine: EngineProxyFixture
    private lateinit var proxy: ProxyApi
    private val secret = UUID.randomUUID().toString().replace("-", "")
    private val device = "wf-qa-tun"
    private val report = JSONObject()
    private val steps = JSONArray()

    @Before fun gate() {
        assumeTrue("Explicit -e proxyEmulatorRoot true is required", InstrumentationRegistry.getArguments().getString("proxyEmulatorRoot") == "true")
        assumeTrue("Emulator only", Build.HARDWARE == "ranchu" || Build.HARDWARE == "goldfish")
        root = ShellRoot(instrumentation, File(context.cacheDir, "proxy-root-test"))
        assumeTrue("adb root is required", root.uid() == "0")
        root.prepareTunTool()
        launcher = root.Launcher()
        engine = runBlocking { EngineProxyFixture.connect(context) }
        fixture = engine.staging
        proxy = engine.bind(ProxyService.ports(context, root.rootShell, launcher),
            ProxyTiming(controllerReadyMs = 20_000, tunReadyMs = 15_000))
    }

    @After fun cleanUp() {
        if (!::app:proxy.isInitialized) return
        runBlocking { withTimeout(20_000) { proxy.stop() } }
        root.run("ip link delete Meta 2>/dev/null; true")
        root.run("ip rule del pref 9505 from all lookup 9700 2>/dev/null; ip rule del pref 9700 from all lookup 9700 2>/dev/null; ip route del unreachable 198.51.100.0/24 table 9500 2>/dev/null; true")
        root.restoreTunTool()
        report.put("steps", steps).put("finalRules", root.run("ip rule show").output)
        File(context.filesDir, "proxy-emulator-result.json").writeText(report.toString(2).replace(secret, "[secret]"))
        engine.close()
    }

    private fun step(name: String, detail: Any? = null) { steps.put(JSONObject().put("step", name).put("detail", detail ?: JSONObject.NULL)); android.util.Log.i(TAG, "$name ${detail ?: ""}") }

    private fun config(mixedPort: Int, controllerPort: Int, echoPort: Int) = """
        mixed-port: $mixedPort
        bind-address: "127.0.0.1"
        allow-lan: false
        mode: rule
        log-level: info
        external-controller: 127.0.0.1:$controllerPort
        secret: "$secret"
        tun:
          enable: true
          device: $device
          stack: gvisor
          auto-route: true
          auto-detect-interface: true
          dns-hijack: []
          iproute2-table-index: 9500
          iproute2-rule-index: 9500
          include-uid:
            - ${Process.myUid()}
        dns:
          enable: false
        proxies: []
        proxy-providers:
          fixture-nodes:
            type: file
            path: nodes.yaml
            health-check:
              enable: false
              url: http://127.0.0.1:$echoPort/
        rule-providers:
          fixture-rules:
            type: file
            behavior: classical
            path: rules.yaml
        proxy-groups:
          - name: QA
            type: select
            proxies: [DIRECT, REJECT]
            use: [fixture-nodes]
        rules:
          - IP-CIDR,198.51.100.0/24,REJECT
          - DST-PORT,$echoPort,QA
          - MATCH,DIRECT
    """.trimIndent() + "\n"

    private fun rules(): String = root.run("ip rule show; ip -6 rule show").output
    private fun ourRules(text: String = rules()) = text.lineSequence().filter { line -> line.substringBefore(':').trim().toIntOrNull()?.let { it in 9500..9510 } == true }.toList()
    private fun linkExists(): Boolean = root.run("ip link show $device").code == 0

    private suspend fun awaitGone(label: String) {
        withTimeout(15_000) { while (linkExists()) delay(100) }
        val left = ourRules()
        step("$label-clean", JSONObject().put("link", linkExists()).put("rules", JSONArray(left)))
        assertEquals("policy rules left behind after $label", emptyList<String>(), left)
    }

    @Test fun tunLifecycleCaptureConflictAndCleanup() = runBlocking<Unit> {
        val before = rules()
        report.put("rulesBefore", before)
        assertEquals("fixture needs a clean 9500 range", emptyList<String>(), ourRules(before))
        val mixed = ServerSocket(0).use { it.localPort }
        val control = ServerSocket(0).use { it.localPort }
        val echo = ServerSocket(0, 8, java.net.InetAddress.getByName("127.0.0.1"))
        // Minimal HTTP responder so the kernel's URL test through DIRECT gets a 204.
        Thread({
            while (!echo.isClosed) runCatching { echo.accept().use { client ->
                client.soTimeout = 2000
                val input = client.getInputStream(); val seen = StringBuilder()
                while (!seen.endsWith("\r\n\r\n")) { val next = input.read(); if (next < 0) break; seen.append(next.toChar()) }
                Thread.sleep(20) // Mihomo represents a sub-millisecond (zero) delay as a failed test.
                client.getOutputStream().write("HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n".toByteArray())
            } }
        }, "proxy-qa-echo").apply { isDaemon = true; start() }
        fun nodeProvider(name: String) = "proxies:\n  - {name: $name, type: http, server: 127.0.0.1, port: ${echo.localPort}}\n"
        val canonicalConfig = config(mixed, control, echo.localPort)
        engine.config(canonicalConfig)
        engine.asset("nodes.yaml", nodeProvider("fixture-node"))
        engine.asset("rules.yaml", "payload:\n  - DOMAIN,example.invalid\n")
        assertTrue(proxy.checkConfig().getOrThrow().valid)
        assertTrue("Staged config must equal Engine canonical bytes", canonicalConfig == proxy.configFile.readText())
        step("engine-canonical-config-and-provider-upload", engine.documents.read("services.proxy", "config.yaml").revision)

        // A table with no policy rules can still belong to somebody else. Guardian preflight must
        // preserve that route and refuse launch, rather than letting sing-tun replace the table.
        root.run("ip route add unreachable 198.51.100.0/24 table 9500").also { assertEquals(it.output, 0, it.code) }
        assertTrue("occupied route table must block start", proxy.start().isFailure)
        assertFalse(linkExists())
        assertTrue(root.run("ip route show table 9500").output.contains("198.51.100.0/24"))
        root.run("ip route del unreachable 198.51.100.0/24 table 9500").also { assertEquals(0, it.code) }
        proxy.dismiss()
        step("occupied-table-refused")

        // 1. Start: real guardian, real kernel, TUN verified through NetworkInterface and read-only `ip rule`.
        val baselineLeases = LocalRuntimeService.state.value.proxyCount
        val started = System.nanoTime()
        proxy.start().getOrThrow()
        var state = proxy.state.value
        step("started", JSONObject().put("ms", (System.nanoTime() - started) / 1_000_000).put("tun", state.tun.toString()).put("pid", state.pid))
        assertEquals(ProxyPhase.RUNNING, state.phase)
        assertTrue(state.controllerReachable)
        assertEquals(device, state.tun?.device)
        assertTrue(state.tun!!.up)
        assertEquals(true, state.tun!!.routed)
        assertEquals(baselineLeases + 1, LocalRuntimeService.state.value.proxyCount)
        val measured = engine.serviceState()
        assertEquals("running", measured["phase"])
        assertEquals(true, measured["controllerReachable"])
        assertEquals(true, (measured["tun"] as Map<*, *>)["routed"])
        assertFalse(JSONObject(measured).toString().contains(secret))
        step("engine-acknowledged-running", measured["phase"])
        val running = ourRules()
        step("rules-running", JSONArray(running))
        assertTrue(running.any { it.contains("lookup 9500") })

        // A host observer can kill the actual Android app process after all startup checks pass.
        // This opt-in deliberately never completes: am force-stop must close the real app-owned
        // control FIFO, and the observer verifies guardian/kernel death and exact network cleanup.
        if (InstrumentationRegistry.getArguments().getString("proxyHostKillAtRunning") == "true") {
            File(context.filesDir, "proxy-host-kill-ready.json").writeText(JSONObject()
                .put("appPid", Process.myPid()).put("kernelPid", state.pid).put("device", device)
                .put("rulesBefore", before).put("rulesRunning", JSONArray(running)).toString(2))
            delay(120_000)
            error("Host did not terminate the app during the app-death acceptance window")
        }

        // 2. Traffic from this uid is captured by the TUN: a REJECT rule for TEST-NET-2 shows in the live log.
        val logged = CompletableDeferred<String>()
        val collector = async(Dispatchers.IO) { proxy.logs(ProxyLogLevel.INFO).first { it.payload.contains("198.51.100.7") }.payload.also { logged.complete(it) } }
        delay(500)
        withContext(Dispatchers.IO) { runCatching { Socket().use { it.connect(InetSocketAddress("198.51.100.7", 80), 3000) } } }
        val line = withTimeout(10_000) { logged.await() }
        collector.cancel()
        step("captured", line)
        assertTrue(line, line.contains("REJECT"))
        assertFalse(line.contains(secret))

        // 3. Controller surface on the real kernel.
        for (mode in listOf(ProxyMode.GLOBAL, ProxyMode.DIRECT, ProxyMode.RULE)) {
            proxy.setMode(mode).getOrThrow()
            withTimeout(5000) { proxy.state.first { it.mode == mode } }
        }
        proxy.select("QA", "DIRECT").getOrThrow()
        withTimeout(5000) { proxy.state.first { it.groups.group("QA")?.now == "DIRECT" } }
        val delayResult = proxy.testNode("DIRECT", "http://127.0.0.1:${echo.localPort}/", 3000).getOrThrow()
        step("delay", delayResult.toString())
        assertTrue(delayResult.toString(), delayResult is DelayResult.Success)
        withTimeout(5000) { proxy.state.first { it.traffic != null } }
        assertNotNull(proxy.connections().getOrThrow())
        // Editor saves change the canonical provider; refresh must fetch it from Engine again.
        assertTrue(engine.session.store.saveFile(".workspace/proxy/nodes.yaml", nodeProvider("updated-node")) is top.flysoftbeta.workflow.core.store.SaveResult.Saved)
        val refreshed = proxy.refreshProviders().getOrThrow()
        step("providers-refreshed", refreshed.map { "${it.provider.name}: ${it.error ?: "ok"}" })
        assertEquals(2, refreshed.size)
        assertTrue(refreshed.toString(), refreshed.all { it.error == null })
        assertTrue(proxy.state.value.groups.group("QA")!!.members.any { it.name == "updated-node" })
        assertEquals(nodeProvider("updated-node"), File(fixture, "nodes.yaml").readText())
        val tail = proxy.logTail(65_536)
        assertEquals(tail, engine.read(ProxyService.LOG_PATH))
        assertFalse(tail.contains(secret))

        // 4. Ordinary stop: exact stop record → SIGTERM → Mihomo removes its TUN and rules.
        // A foreign rule created after startup must survive even if the kernel's own graceful
        // teardown would sweep that priority. Guardian chooses its exact cleanup in this case.
        root.run("ip rule add pref 9505 from all lookup 9700").also { assertEquals(0, it.code) }
        proxy.stop().getOrThrow()
        assertEquals(ProxyPhase.STOPPED, proxy.state.value.phase)
        assertEquals("stopped", engine.serviceState()["phase"])
        assertEquals("unrelated rule survives explicit stop", 1, ourRules().size)
        assertTrue(ourRules().single().contains("lookup 9700"))
        root.run("ip rule del pref 9505 from all lookup 9700").also { assertEquals(0, it.code) }
        step("stop-foreign-preserved")
        awaitGone("stop")
        assertFalse(File(fixture, ".process.json").exists())
        withTimeout(5000) { while (LocalRuntimeService.state.value.proxyCount != baselineLeases) delay(50) }

        // 5. App death: the control pipe closes without a stop record; the guardian stops the kernel (EOF path).
        proxy.start().getOrThrow()
        assertEquals(ProxyPhase.RUNNING, proxy.state.value.phase)
        launcher.processes.last().closeControlPipe()
        val afterEof = withTimeout(15_000) { proxy.state.first { it.phase == ProxyPhase.ERROR } }
        step("eof", afterEof.error)
        assertFalse(afterEof.stopUnconfirmed)
        awaitGone("eof")
        proxy.dismiss()

        // 6. Kernel SIGKILL: the guardian must remove only this run's policy rules, even though
        // Mihomo cannot run its own graceful teardown. A foreign rule outside our scope survives.
        root.run("ip rule add pref 9700 from all lookup 9700").also { assertEquals(0, it.code) }
        proxy.start().getOrThrow()
        root.run("ip rule add pref 9505 from all lookup 9700").also { assertEquals(0, it.code) }
        root.run("kill -9 ${proxy.state.value.pid}")
        withTimeout(15_000) { proxy.state.first { it.phase == ProxyPhase.ERROR } }
        assertEquals("unrelated rule inside our range survives", 1, ourRules().size)
        assertTrue(ourRules().single().contains("lookup 9700"))
        root.run("ip rule del pref 9505 from all lookup 9700")
        awaitGone("kernel-killed")
        assertTrue("foreign rule preserved", rules().contains("9700:"))
        root.run("ip rule del pref 9700 from all lookup 9700")
        proxy.dismiss()
        proxy.start().getOrThrow()
        proxy.stop().getOrThrow()
        awaitGone("restart-after-kill")

        // 7. Another TUN present: conflict before root is used for anything else; nothing started.
        root.run("ip tuntap add dev Meta mode tun && ip link set Meta up").also { assertEquals(it.output, 0, it.code) }
        val refused = proxy.start().exceptionOrNull()
        step("conflict", refused?.message)
        assertTrue(refused is ProxyConflictException)
        assertEquals(ProxyPhase.CONFLICT, proxy.state.value.phase)
        assertEquals(listOf("Meta"), proxy.state.value.conflict.foreignInterfaces)
        assertFalse(linkExists())
        proxy.dismiss()
        root.run("ip link delete Meta")

        // Disconnect the actual Engine session while the guardian is alive. Its owned channel must
        // close, remove this run's TUN/rules, and release the lease without another root start/stop.
        proxy.start().getOrThrow()
        val ownedPid = proxy.state.value.pid!!
        engine.disconnect()
        awaitGone("engine-disconnect")
        withTimeout(15_000) { while (root.run("kill -0 $ownedPid").code == 0) delay(100) }
        withTimeout(5000) { while (LocalRuntimeService.state.value.proxyCount != baselineLeases) delay(50) }
        assertTrue("a disconnected Engine cannot start from staged configuration", proxy.start().isFailure)
        step("engine-disconnect-owned-process-gone")

        echo.close()
        assertEquals("routing restored", before.trim(), rules().trim())
        report.put("passed", true)
    }

    companion object { private const val TAG = "WorkflowProxyRootTest" }
}
