package top.flysoftbeta.workflow.proxy

import java.io.File
import java.net.ServerSocket
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.take
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import top.flysoftbeta.workflow.proxy.controller.DelayResult
import top.flysoftbeta.workflow.proxy.controller.ProxyMode
import top.flysoftbeta.workflow.proxy.network.NetInterface
import top.flysoftbeta.workflow.proxy.runtime.ProxyConflictException
import top.flysoftbeta.workflow.proxy.runtime.ProxyFiles
import top.flysoftbeta.workflow.proxy.runtime.ProxyPhase
import top.flysoftbeta.workflow.proxy.runtime.ProxyPorts
import top.flysoftbeta.workflow.proxy.runtime.ProxyRuntime
import top.flysoftbeta.workflow.proxy.runtime.ProxyTiming
import top.flysoftbeta.workflow.proxy.runtime.RootStatus

class ProxyRuntimeTest {
    @get:Rule val temporary = TemporaryFolder()
    private val secret = "0123456789abcdef0123456789abcdef"
    private lateinit var directory: File
    private lateinit var kernel: File
    private lateinit var guardian: File
    private val probe = FakeNetworkProbe()
    private val root = FakeRootShell()
    private val tool = FakeKernelTool()
    private val lease = FakeLease()
    private var controllerPort = 0
    private var tunDevice: String? = null
    private lateinit var launcher: FakeGuardianLauncher
    private lateinit var runtime: ProxyRuntime

    @Before fun setUp() {
        directory = temporary.newFolder("workspace", ".workflow", "proxy")
        kernel = temporary.newFile("libmihomo.so").apply { setExecutable(true) }
        guardian = temporary.newFile("libworkflow_proxy_guard.so").apply { setExecutable(true) }
        controllerPort = ServerSocket(0).use { it.localPort }
        launcher = FakeGuardianLauncher({ controllerPort }, secret, probe, { tunDevice })
        root.tunRunning = { launcher.processes.any { it.isAlive } }
        runtime = ProxyRuntime(ProxyFiles(directory), kernel, guardian, ProxyPorts(root, launcher, tool, probe, lease),
            ProxyTiming(guardianStartMs = 3000, earlyExitMs = 300, controllerReadyMs = 3000, tunReadyMs = 1500, stopMs = 2000, pollMs = 20))
    }

    @After fun tearDown() { runBlocking { runtime.stop() }; launcher.servers.forEach { it.close() } }

    private fun config(tun: String = "tun:\n  enable: false\n", extra: String = "") =
        "mixed-port: 0\nexternal-controller: 127.0.0.1:$controllerPort\nsecret: \"$secret\"\nmode: rule\n" + tun + extra

    private fun write(text: String) = File(directory, "config.yaml").writeText(text)

    private val safeTun = "tun:\n  enable: true\n  device: workflow-tun\n  iproute2-table-index: 9500\n  iproute2-rule-index: 9500\n"

    @Test fun firstUseCreatesTheTemplateOnceAndRefreshNeedsNoRoot() = runBlocking<Unit> {
        runtime.ensureConfig().getOrThrow()
        val first = File(directory, "config.yaml").readText()
        runtime.ensureConfig().getOrThrow()
        assertEquals(first, File(directory, "config.yaml").readText())
        val state = runtime.refresh().getOrThrow()
        assertTrue(state.config.exists && state.config.inspection != null)
        assertTrue(state.capabilities.kernel && state.capabilities.guardian)
        assertEquals(RootStatus.UNKNOWN, state.capabilities.root)
        assertEquals(ProxyMode.RULE, state.mode)
        assertEquals("Mihomo Meta v1.19.31 android arm64", state.version)
        assertTrue(state.canStart)
        assertTrue("refresh must not use root", root.scripts.isEmpty())
    }

    @Test fun startRunsThroughTheGuardianAndStopConfirmsExit() = runBlocking<Unit> {
        write(config())
        runtime.start().getOrThrow()
        val running = runtime.state.value
        assertEquals(ProxyPhase.RUNNING, running.phase)
        assertEquals(RootStatus.GRANTED, running.capabilities.root)
        assertTrue(running.controllerReachable)
        assertEquals(listOf("Proxy", "Auto", "Hidden"), running.groups.groups.map { it.name })
        assertEquals(1, lease.held.size)
        assertTrue(File(directory, ".process.json").isFile)
        assertEquals("only the root check; no previous record to verify", listOf("id -u"), root.scripts.toList())

        // Traffic flows into the state while running.
        withTimeout(3000) { runtime.state.first { it.traffic != null } }

        runtime.setMode(ProxyMode.GLOBAL).getOrThrow()
        assertEquals(ProxyMode.GLOBAL, runtime.state.value.mode)
        runtime.select("Proxy", "JP 02").getOrThrow()
        assertEquals("JP 02", runtime.state.value.groups.group("Proxy")?.now)
        val delays = runtime.testGroup("Auto").getOrThrow()
        assertEquals(DelayResult.Success(120), delays["HK 01"])
        assertEquals(DelayResult.Success(120), runtime.state.value.delays["HK 01"])
        assertTrue(runtime.state.value.testing.isEmpty())
        assertEquals(DelayResult.Success(120), runtime.testNode("HK 01").getOrThrow())
        assertEquals(1, runtime.connections().getOrThrow().connections.size)
        assertEquals(3, runtime.loadProviders().getOrThrow().size)
        val logs = withTimeout(3000) { runtime.logs().take(1).toList() }
        assertFalse(logs.single().payload.contains(secret))

        runtime.stop().getOrThrow()
        val stopped = runtime.state.value
        assertEquals(ProxyPhase.STOPPED, stopped.phase)
        assertFalse(stopped.controllerReachable)
        assertNull(stopped.traffic)
        assertTrue(lease.held.isEmpty())
        assertFalse(File(directory, ".process.json").exists())
        assertNotNull("guardian received the exact stop record", launcher.processes.single().stopLine)
        assertTrue(runtime.setMode(ProxyMode.RULE).isFailure)
    }

    @Test fun disconnectClosesOwnedGuardianWithoutAnotherRootProbe() = runBlocking<Unit> {
        write(config())
        runtime.start().getOrThrow()
        val roots = root.scripts.size
        runtime.closeOwnedChannel()
        withTimeout(3000) { while (launcher.processes.single().isAlive) delay(20) }
        assertTrue(launcher.processes.single().eof)
        assertEquals(roots, root.scripts.size)
        assertTrue(runtime.start().isFailure)
        assertEquals(roots, root.scripts.size)
    }

    @Test fun closedConnectionCannotPromptRootOrLaunch() = runBlocking<Unit> {
        write(config())
        runtime.closeOwnedChannel()
        assertTrue(runtime.start().isFailure)
        assertTrue(root.scripts.isEmpty())
        assertTrue(launcher.processes.isEmpty())
    }

    @Test fun kernelOutputIsLoggedWithTheSecretRedacted() = runBlocking<Unit> {
        write(config())
        runtime.start().getOrThrow()
        withTimeout(3000) { while (!runtime.logTail().contains("starting")) delay(20) }
        runtime.stop().getOrThrow()
        val log = File(directory, "runtime.log").readText()
        assertTrue(log.contains("INFO starting with secret [已隐藏]"))
        assertFalse(log.contains(secret))
        assertFalse(runtime.state.value.toString().contains(secret))
    }

    @Test fun blockingConfigIssuesAndInvalidConfigsNeverReachRoot() = runBlocking<Unit> {
        write("external-controller: 127.0.0.1:$controllerPort\nmode: rule\n")
        val error = runtime.start().exceptionOrNull()!!
        assertTrue(error.message!!.contains("secret"))
        assertEquals(ProxyPhase.ERROR, runtime.state.value.phase)
        write(config())
        tool.valid = false
        assertTrue(runtime.start().exceptionOrNull()!!.message!!.contains("bad indentation"))
        assertTrue(root.scripts.isEmpty())
        assertTrue(launcher.processes.isEmpty())
        assertEquals(0, lease.retained.get())
    }

    @Test fun rootDenialIsReportedHonestly() = runBlocking<Unit> {
        write(config())
        root.granted = false
        assertTrue(runtime.start().isFailure)
        val state = runtime.state.value
        assertEquals(ProxyPhase.ERROR, state.phase)
        assertEquals(RootStatus.DENIED, state.capabilities.root)
        assertTrue(launcher.processes.isEmpty())
        assertTrue(lease.held.isEmpty())
    }

    @Test fun anotherTunIsAConflictBeforeRootAndCannotBeOverridden() = runBlocking<Unit> {
        write(config(safeTun))
        probe.interfaces += NetInterface("Meta", true, tun = true)
        val refused = runtime.start().exceptionOrNull()
        assertTrue(refused is ProxyConflictException)
        val state = runtime.state.value
        assertEquals(ProxyPhase.CONFLICT, state.phase)
        assertEquals(listOf("Meta"), state.conflict.foreignInterfaces)
        assertTrue(root.scripts.isEmpty())
        assertTrue(launcher.processes.isEmpty())
        runtime.dismiss()
        assertEquals(ProxyPhase.STOPPED, runtime.state.value.phase)

        // A VPN is a conflict too.
        probe.interfaces.removeIf { it.name == "Meta" }
        probe.vpn = true
        assertTrue(runtime.start().exceptionOrNull() is ProxyConflictException)
        assertTrue(runtime.state.value.conflict.vpnActive)

        // Repeated start requests must not take over another VPN.
        assertTrue(runtime.start().exceptionOrNull() is ProxyConflictException)
        assertTrue(root.scripts.isEmpty())
        probe.vpn = false
        // Once the competing VPN is gone, an explicit start verifies the TUN.
        tunDevice = "workflow-tun"
        root.tunRules = "9500:\tfrom all iif workflow-tun goto 9510\n9501:\tnot from all iif lo lookup 9500\n9510:\tfrom all nop\n"
        runtime.start().getOrThrow()
        val running = runtime.state.value
        assertEquals(ProxyPhase.RUNNING, running.phase)
        assertEquals("workflow-tun", running.tun?.device)
        assertTrue(running.tun!!.up)
        assertEquals(true, running.tun.routed)
        // Our own interface is not reported as a conflict while we run it.
        assertFalse(runtime.refresh().getOrThrow().conflict.foreignInterfaces.contains("workflow-tun"))
        runtime.stop().getOrThrow()
        assertFalse(probe.interfaces.any { it.name == "workflow-tun" })
    }

    @Test fun foreignRulesInOurRangeCannotBeOverridden() = runBlocking<Unit> {
        write(config(safeTun))
        root.rules += "9505:\tfrom all lookup 3000\n"
        val refused = runtime.start().exceptionOrNull()
        assertTrue(refused is ProxyConflictException)
        assertTrue((refused as ProxyConflictException).conflict.hard)
        assertEquals(listOf("9505: from all lookup 3000"), runtime.state.value.conflict.ruleCollisions)
        assertTrue(launcher.processes.isEmpty())
        assertTrue(lease.held.isEmpty())
    }

    @Test fun rootLinkCheckFindsCarrierlessTunMissingFromJavaEnumeration() = runBlocking<Unit> {
        write(config(safeTun))
        root.links += "12: Meta: <NO-CARRIER,POINTOPOINT,MULTICAST,NOARP,UP> mtu 1500 state DOWN\n"
        val refused = runtime.start().exceptionOrNull()
        assertTrue(refused is ProxyConflictException)
        assertEquals(listOf("Meta"), runtime.state.value.conflict.foreignInterfaces)
        assertTrue(launcher.processes.isEmpty())
        assertTrue(lease.held.isEmpty())
    }

    @Test fun unreadableRootLinkStateDoesNotPermitTunStart() = runBlocking<Unit> {
        write(config(safeTun))
        root.linksCode = 1
        assertTrue(runtime.start().isFailure)
        assertTrue(launcher.processes.isEmpty())
        assertTrue(lease.held.isEmpty())
        root.linksCode = 0
        root.links = ""
        assertTrue(runtime.start().isFailure)
        assertTrue(launcher.processes.isEmpty())
        assertTrue(lease.held.isEmpty())
    }

    @Test fun missingTunInterfaceIsReportedNotFaked() = runBlocking<Unit> {
        write(config(safeTun))
        tunDevice = null
        runtime.start().getOrThrow()
        val state = runtime.state.value
        assertEquals(ProxyPhase.RUNNING, state.phase)
        assertFalse(state.tun!!.up)
        assertTrue(state.error!!.contains("TUN"))
    }

    @Test fun guardianStartupFailureCleansUp() = runBlocking<Unit> {
        write(config())
        launcher.behavior = GuardianBehavior.FAIL_START
        val error = runtime.start().exceptionOrNull()!!
        assertTrue(error.message!!.contains("identity verification failed"))
        val state = runtime.state.value
        assertEquals(ProxyPhase.ERROR, state.phase)
        assertFalse(state.stopUnconfirmed)
        withTimeout(3000) { while (lease.held.isNotEmpty()) delay(20) }
    }

    @Test fun unexpectedKernelExitBecomesAnError() = runBlocking<Unit> {
        write(config())
        launcher.behavior = GuardianBehavior.CRASH_AFTER_START
        runtime.start().getOrThrow()
        assertEquals(ProxyPhase.RUNNING, runtime.state.value.phase)
        val failed = withTimeout(5000) { runtime.state.first { it.phase == ProxyPhase.ERROR } }
        assertTrue(failed.error!!, failed.error.contains("代理内核已退出（2）"))
        assertFalse(failed.stopUnconfirmed)
        assertNull(failed.pid)
        assertTrue(lease.held.isEmpty())
        runtime.dismiss()
        assertEquals(ProxyPhase.STOPPED, runtime.state.value.phase)
    }

    @Test fun lostGuardianChannelIsResolvedOnlyByAReadOnlyRootCheck() = runBlocking<Unit> {
        write(config())
        launcher.behavior = GuardianBehavior.LOSE_CHANNEL
        root.procCheck = 4 // identity cannot be read
        runtime.start().getOrThrow()
        assertTrue(runtime.stop().isFailure)
        val state = runtime.state.value
        assertEquals(ProxyPhase.ERROR, state.phase)
        assertTrue(state.stopUnconfirmed)
        assertTrue("record kept for diagnosis", File(directory, ".process.json").exists())
        // A new start refuses until the old run is resolved.
        assertTrue(runtime.start().isFailure)
        assertEquals(1, launcher.processes.size)
        // Once /proc shows the recorded kernel gone, stop confirms it without sending any signal.
        root.procCheck = 3
        runtime.stop().getOrThrow()
        assertEquals(ProxyPhase.STOPPED, runtime.state.value.phase)
        assertFalse(runtime.state.value.stopUnconfirmed)
        assertFalse(File(directory, ".process.json").exists())
        launcher.behavior = GuardianBehavior.NORMAL
        runtime.start().getOrThrow()
        assertEquals(ProxyPhase.RUNNING, runtime.state.value.phase)
    }

    @Test fun stopTimeoutIsNotReportedAsStopped() = runBlocking<Unit> {
        write(config())
        launcher.behavior = GuardianBehavior.IGNORE_STOP
        runtime.start().getOrThrow()
        val error = runtime.stop().exceptionOrNull()!!
        assertTrue(error.message!!.contains("超时"))
        assertTrue(runtime.state.value.stopUnconfirmed)
        assertTrue(launcher.processes.single().stopLine != null)
        launcher.processes.single().destroy()
    }

    @Test fun guardianThatNeverConfirmsIsCleanedUpByEof() = runBlocking<Unit> {
        write(config())
        launcher.behavior = GuardianBehavior.NEVER_START
        assertTrue(runtime.start().isFailure)
        assertTrue(runtime.state.value.stopUnconfirmed)
        launcher.processes.single().destroy()
    }

    @Test fun previousRecordIsCheckedReadOnlyWithRoot() = runBlocking<Unit> {
        write(config())
        runtime.start().getOrThrow()
        val record = File(directory, ".process.json").readText()
        runtime.stop().getOrThrow()
        // Simulate a record left by a previous app process whose guardian cleaned up on EOF.
        File(directory, ".process.json").writeText(record)
        val fresh = ProxyRuntime(ProxyFiles(directory), kernel, guardian, ProxyPorts(root, launcher, tool, probe, lease))
        // Nothing answers on our controller port: the earlier guardian cleaned up, so nothing is flagged.
        assertFalse(fresh.refresh().getOrThrow().stopUnconfirmed)
        // Something answers there: the earlier kernel may still run.
        ServerSocket(controllerPort, 1, java.net.InetAddress.getByName("127.0.0.1")).use {
            assertTrue(fresh.refresh().getOrThrow().stopUnconfirmed)
        }
        root.scripts.clear()
        fresh.stop().getOrThrow()
        assertTrue(root.scripts.any { it.startsWith("if [ ! -e /proc/") })
        assertFalse(fresh.state.value.stopUnconfirmed)
        assertFalse(File(directory, ".process.json").exists())
    }

    @Test fun stoppedStateShowsConfiguredGroupsThenTheLastLiveOnes() = runBlocking<Unit> {
        write(config(extra = """
            proxies:
              - {name: HK 01, type: vmess, server: a.example, port: 443, uuid: 00000000-0000-0000-0000-000000000000}
              - {name: JP 02, type: trojan, server: b.example, port: 443, password: x}
            proxy-groups:
              - {name: Proxy, type: select, proxies: [Auto, HK 01, DIRECT]}
              - {name: Auto, type: url-test, include-all: true, filter: "JP", url: "https://cp.example/generate_204"}
        """.trimIndent() + "\n"))
        val stopped = runtime.refresh().getOrThrow()
        assertFalse(stopped.live)
        assertEquals(listOf("Proxy", "Auto"), stopped.groups.groups.map { it.name })
        assertEquals(listOf("Auto", "HK 01", "DIRECT"), stopped.groups.group("Proxy")!!.members.map { it.name })
        assertEquals(listOf("URLTest", "Vmess", "Direct"), stopped.groups.group("Proxy")!!.members.map { it.type })
        assertEquals(listOf("JP 02"), stopped.groups.group("Auto")!!.members.map { it.name })
        assertNull("selection is unknown until the kernel runs", stopped.groups.group("Proxy")!!.now)
        assertEquals(listOf("DIRECT", "REJECT", "HK 01", "JP 02", "Proxy", "Auto"), stopped.groups.global!!.members.map { it.name })

        runtime.start().getOrThrow()
        assertTrue(runtime.state.value.live)
        runtime.select("Proxy", "JP 02").getOrThrow()
        runtime.stop().getOrThrow()
        // Same file: the kernel's own groups (with the selection Mihomo will restore) stay visible.
        assertEquals("JP 02", runtime.refresh().getOrThrow().groups.group("Proxy")?.now)
        // Edited file: back to what the file says.
        File(directory, "config.yaml").appendText("# edited\n")
        File(directory, "config.yaml").setLastModified(System.currentTimeMillis() + 5000)
        assertNull(runtime.refresh().getOrThrow().groups.group("Proxy")?.now)
    }

    @Test fun occupiedControllerPortRefusesADuplicateKernel() = runBlocking<Unit> {
        write(config())
        ServerSocket(controllerPort, 1, java.net.InetAddress.getByName("127.0.0.1")).use {
            assertTrue(runtime.start().exceptionOrNull()!!.message!!.contains("控制接口端口已被其他进程占用"))
        }
        assertTrue(root.scripts.isEmpty())
    }
}
