package top.flysoftbeta.workflow.proxy

import java.io.File
import java.io.IOException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import top.flysoftbeta.workflow.proxy.runtime.*

class WorkspaceProxyApiTest {
    @get:Rule val temporary = TemporaryFolder()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private val root = FakeRootShell()
    private val workspace = MemoryWorkspace()
    private lateinit var runtime: ProxyRuntime
    private lateinit var api: WorkspaceProxyApi

    @Before fun prepare() {
        val probe = FakeNetworkProbe()
        runtime = ProxyRuntime(ProxyFiles(temporary.newFolder("cache")),
            temporary.newFile("libmihomo.so").apply { setExecutable(true) },
            temporary.newFile("libworkflow_proxy_guard.so").apply { setExecutable(true) },
            ProxyPorts(root, FakeGuardianLauncher({ null }, "", probe, { null }), FakeKernelTool(), probe, FakeLease()))
        api = WorkspaceProxyApi(runtime, workspace, scope)
    }
    @After fun cleanup() { scope.cancel(); runtime.closeOwnedChannel() }

    @Test fun templateIsEngineOwnedAndOnlyReturnedBytesAreStaged() = runBlocking<Unit> {
        workspace.acceptedText = "mode: direct\ntun: {enable: false}\n"
        val cache = api.ensureConfig().getOrThrow()
        assertEquals(workspace.acceptedText, cache.readText())
        assertEquals(1, workspace.writes)
        api.ensureConfig().getOrThrow()
        assertEquals("existing canonical config must not be recreated", 1, workspace.writes)
        assertTrue(root.scripts.isEmpty())
    }

    @Test fun missingCanonicalConfigCannotUseCachedConfig() = runBlocking<Unit> {
        runtime.importConfig("mode: rule\n".toByteArray()).getOrThrow()
        assertTrue(api.start().isFailure)
        assertTrue("no root prompt from cache fallback", root.scripts.isEmpty())
    }

    @Test fun unavailableEngineCannotLaunchEvenWhenStagingSucceeded() = runBlocking<Unit> {
        workspace.text = "mode: rule\ntun: {enable: false}\n"
        workspace.reportFailure = IOException("disconnected")
        assertTrue(api.start().isFailure)
        assertTrue(root.scripts.isEmpty())
        assertFalse(api.state.value.canStart)
        assertEquals(ProxyPhase.ERROR, api.state.value.phase)
    }

    @Test fun stateWaitsForEngineAcknowledgement() = runBlocking<Unit> {
        val gate = CompletableDeferred<Unit>()
        workspace.reportGate = gate
        runtime.refresh().getOrThrow()
        withTimeout(2000) { workspace.reportEntered.await() }
        assertFalse("unacknowledged executor capabilities must stay hidden", api.state.value.capabilities.kernel)
        gate.complete(Unit)
        withTimeout(2000) { api.state.first { it.capabilities.kernel } }
        assertTrue(workspace.reports.any { it.capabilities.kernel })
    }

    @Test fun rejectedCanonicalWriteCannotAlterStagedConfiguration() = runBlocking<Unit> {
        workspace.text = "mode: rule\n"
        api.ensureConfig().getOrThrow()
        val original = runtime.configFile.readText()
        workspace.writeFailure = IOException("revision conflict")
        assertTrue(api.importConfig("mode: direct\n".toByteArray()).isFailure)
        assertEquals(original, runtime.configFile.readText())
        assertEquals(original, workspace.text)
    }

    @Test fun repeatedRefreshDoesNotRewriteKernelStagingOrInvalidateItsStamp() = runBlocking<Unit> {
        workspace.text = "mode: rule\n"
        api.refresh().getOrThrow()
        runtime.configFile.setLastModified(1234000)
        api.refresh().getOrThrow()
        assertEquals(1234000, runtime.configFile.lastModified())
    }

    @Test fun logCopiesArePublishedBeforeReturningToTheUi() = runBlocking<Unit> {
        runtime.files.appendLog("redacted log\n")
        assertEquals("redacted log\n", api.logTail())
        assertEquals("redacted log\n", workspace.log)
    }

    @Test fun fileProvidersAreStagedFromEngineBeforeValidation() = runBlocking<Unit> {
        workspace.text = "proxy-providers:\n  local:\n    type: file\n    path: providers/local.yaml\n"
        workspace.assets["providers/local.yaml"] = "proxies: []\n".toByteArray()
        api.checkConfig().getOrThrow()
        val staged = File(runtime.configFile.parentFile, "providers/local.yaml")
        assertEquals("proxies: []\n", staged.readText())
        workspace.assets.clear()
        assertTrue("missing canonical asset cannot fall back to its cached copy", api.checkConfig().isFailure)
        assertTrue(root.scripts.isEmpty())
    }

    private class MemoryWorkspace : ProxyWorkspace {
        @Volatile var text: String? = null
        var revision = 0L
        var writes = 0
        var acceptedText: String? = null
        var writeFailure: Exception? = null
        @Volatile var reportFailure: Exception? = null
        @Volatile var reportGate: CompletableDeferred<Unit>? = null
        val reportEntered = CompletableDeferred<Unit>()
        val reports = java.util.concurrent.CopyOnWriteArrayList<ProxyState>()
        var log: String? = null
        val assets = mutableMapOf<String, ByteArray>()
        override suspend fun readConfig() = ProxyWorkspace.Config(text, revision)
        override suspend fun writeConfig(text: String, expectedRevision: Long) {
            writeFailure?.let { throw it }
            check(expectedRevision == revision)
            this.text = acceptedText ?: text; revision++; writes++
        }
        override suspend fun report(measured: ProxyState) {
            reportEntered.complete(Unit)
            reportGate?.await()
            reportFailure?.let { throw it }
            reports += measured
        }
        override suspend fun writeLog(text: String) { log = text }
        override suspend fun readAsset(path: String) = assets[path] ?: error("Missing asset")
    }
}
