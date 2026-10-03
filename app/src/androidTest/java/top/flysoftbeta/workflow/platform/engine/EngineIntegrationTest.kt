package top.flysoftbeta.workflow.platform.engine

import android.os.Build
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import java.io.InputStream
import java.io.OutputStream
import java.util.UUID
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*
import org.junit.*
import org.junit.Assert.*
import org.junit.Assume.assumeTrue
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.agent.AgentStateStore
import top.flysoftbeta.workflow.agent.codex.CodexBackend
import top.flysoftbeta.workflow.agent.codex.CodexConfig
import top.flysoftbeta.workflow.core.connection.*
import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.core.json.Json
import top.flysoftbeta.workflow.core.layout.*
import top.flysoftbeta.workflow.core.store.*
import top.flysoftbeta.workflow.core.terminal.TerminalSpec
import top.flysoftbeta.workflow.platform.agent.ClaudeCodeInstaller
import top.flysoftbeta.workflow.platform.agent.GuestAgents
import top.flysoftbeta.workflow.platform.agent.runGuest
import top.flysoftbeta.workflow.platform.connection.WorkspaceConnectionSession
import top.flysoftbeta.workflow.platform.connection.WorkspaceNetworkReporter

/** Actual packaged Rust Server/runtime/loader in an Android app UID; all test data is isolated. */
@RunWith(AndroidJUnit4::class)
class EngineIntegrationTest {
    private val context get() = InstrumentationRegistry.getInstrumentation().targetContext
    private var scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private lateinit var root: File
    private lateinit var server: Process
    private lateinit var rpc: WorkspaceRpc
    private lateinit var store: RemoteWorkspaceStore
    private lateinit var engine: EngineController

    @Before fun start() = runBlocking {
        assumeTrue("Disposable emulator only", Build.HARDWARE in setOf("ranchu", "goldfish"))
        root = File(context.filesDir, "engine-acceptance/${UUID.randomUUID()}")
        startServer()
    }
    private suspend fun startServer() {
        val libs = context.applicationInfo.nativeLibraryDir
        server = withContext(Dispatchers.IO) {
            ProcessBuilder("$libs/libworkflow-engine.so", "serve", "--root", root.path,
                "--runtime", "$libs/libworkflow-runtime.so", "--loader", "$libs/libworkflow-loader.so",
                "--apk", context.applicationInfo.sourceDir, "--native-dir", libs)
                .apply { environment().keys.toList().filter { it.startsWith("OPENAI_") || it.startsWith("ANTHROPIC_") || it.startsWith("CODEX_") || it.startsWith("CLAUDE_") }.forEach { environment().remove(it) }; environment()["TMPDIR"] = context.cacheDir.path }
                .start()
        }
        // Diagnostics contain no host credentials; keep a bounded in-memory tail only.
        Thread({ runCatching { server.errorStream.use { input -> val bytes = ByteArray(4096); while (input.read(bytes) >= 0) Unit } } }, "test-engine-stderr").apply { isDaemon = true }.start()
        rpc = WorkspaceRpc(object : WorkspaceTransport {
            override val input: InputStream get() = server.inputStream
            override val output: OutputStream get() = server.outputStream
            override fun close() { server.destroy(); if (!server.waitFor(3, TimeUnit.SECONDS)) server.destroyForcibly() }
        }, scope)
        store = RemoteWorkspaceStore(rpc, scope, "android-acceptance").also { it.start() }
        assertEquals(StoreStatus.READY, withTimeout(30_000) { store.awaitReady() }.status)
        val profile = WorkspaceConnectionConfig("qa", "qa", WorkspaceEndpoint.Embedded("qa"))
        WorkspaceNetworkReporter.attach(context, WorkspaceConnectionSession("qa", profile, root, rpc, store, scope))
        engine = EngineController(context, scope) { store }.also { it.start() }
    }
    private suspend fun ready() {
        val health = withTimeout(180_000) { engine.health.first { it.usable || it is EnvironmentHealth.Failed || it is EnvironmentHealth.Unavailable } }
        assertTrue(engine.describe(health) + "\n" + engine.logTail(null), health.usable)
    }
    @After fun stop() = runBlocking {
        if (::store.isInitialized) runCatching { withTimeout(5_000) { store.close() } }
        if (::server.isInitialized) server.destroyForcibly()
        scope.cancel()
        if (::root.isInitialized) withContext(Dispatchers.IO) { root.deleteRecursively() }
    }

    @Test fun installedEnvironmentRunsTerminalNetworkAndCodexAndReconcilesRestart() = runBlocking {
        ready()
        println("ENGINE_ACCEPTANCE verified custom environment ready")
        val upload = WorkspaceWire.obj(rpc.request("files.upload.begin", mapOf("path" to "atomic.txt", "size" to 3)))
        val uploadId = WorkspaceWire.string(upload, "uploadId")
        val created = WorkspaceWire.obj(rpc.request("workspace.command", mapOf("name" to "createFile",
            "args" to mapOf("path" to "atomic.txt", "data" to "b3JpZ2luYWw="))))
        assertEquals("done", WorkspaceWire.obj(created["value"])["kind"])
        rpc.request("files.upload.chunk", mapOf("uploadId" to uploadId, "offset" to 0, "data" to "bmV3"))
        assertEquals("failed", WorkspaceWire.obj(rpc.request("files.upload.commit", mapOf("uploadId" to uploadId)))["kind"])
        rpc.request("files.upload.cancel", mapOf("uploadId" to uploadId))
        assertEquals("original", store.openFile("atomic.txt").text)
        println("ENGINE_ACCEPTANCE atomic create and upload collision preserve existing file")
        val session = store.createSession("Rust workspace acceptance")
        store.applyLayout(session, LayoutOp.Open(PanelTarget.File("draft.txt")))
        val shown = store.openFile("draft.txt").shownVersion
        store.editFile("draft.txt", "unsaved survives restart", shown)
        assertTrue(store.flush())
        assertEquals("unsaved survives restart", store.state.value.drafts["draft.txt"]?.text)
        val probe = runGuest(engine, listOf("/bin/bash", "-lc", "set -ex; test \"\$(id -u)\" = 1000; test \"\$PWD\" = /home/work; test ! -e /workspace/.workspace/state/workspace.json; test ! -e /workspace/.workspace/environment; test -f /workspace/.workspace/env.json; python3 --version; node --version; sudo -n id -u; printf shared > /workspace/from-guest.txt"))
        assertEquals(probe.error, 0, probe.exitCode)
        assertEquals("shared", store.openFile("from-guest.txt").text)
        val net = runGuest(engine, listOf("/usr/bin/curl", "-sS", "--max-time", "30", "-o", "/dev/null", "-w", "%{http_code}", "https://api.openai.com"))
        assertEquals(net.error, 0, net.exitCode)
        assertTrue(net.output.toString(Charsets.UTF_8).matches(Regex("[1-5][0-9]{2}")))
        println("ENGINE_ACCEPTANCE guest identity, language tools, files and HTTPS verified")

        val setup = GuestAgents.codex(engine)
        val state = AgentStateStore()
        val codex = CodexBackend(setup.launcher, CodexConfig(setup.executable, setup.homeDir, cwd = setup.workspaceRoot, env = setup.env), state, scope)
        try { codex.start(); assertTrue(codex.refreshModels().models.isNotEmpty()); codex.refreshAccount() }
        finally { codex.stop() }
        println("ENGINE_ACCEPTANCE Codex app-server, models and account RPC verified")

        val terminal = EngineTerminalBackend(engine).start(TerminalSpec())
        val output = StringBuffer()
        val reader = launch { terminal.output.collect { output.append(it.toString(Charsets.UTF_8)) } }
        terminal.write("echo PTY-\$((6*7))\n".toByteArray())
        withTimeout(15_000) { while (!output.contains("PTY-42")) delay(50) }
        val spec = Json.stringify(mapOf("version" to 1, "python" to listOf("3.14"), "node" to emptyList<String>(),
            "packages" to emptyList<String>(), "env" to mapOf("WF_ACCEPTANCE" to "reconciled"),
            "post_scripts" to listOf(mapOf("id" to "home-marker", "run" to "printf home-kept > \"\$HOME/qa-marker\"", "user" to "work"))))
        assertTrue(store.saveFile(WorkspacePaths.ENVIRONMENT, spec) is SaveResult.Saved)
        rpc.request("environment.reconcile")
        val pending = withTimeout(120_000) { engine.health.first { it is EnvironmentHealth.NeedsRestart || it is EnvironmentHealth.Failed } }
        assertTrue(engine.describe(pending), pending is EnvironmentHealth.NeedsRestart)
        println("ENGINE_ACCEPTANCE verified generation pending while terminal remains active")
        terminal.write("node --version\n".toByteArray())
        withTimeout(15_000) { while (!output.contains("v24.")) delay(50) }
        engine.restartEnvironment()
        withTimeout(15_000) { terminal.awaitExit() }
        reader.join()
        val changed = runGuest(engine, listOf("/bin/bash", "-lc", "test \"\$WF_ACCEPTANCE\" = reconciled && test \"\$(cat ~/qa-marker)\" = home-kept && node --version; code=\$?; test \"\$code\" = 127"))
        assertEquals(changed.error, 0, changed.exitCode)
        println("ENGINE_ACCEPTANCE explicit restart activated config, retained home and stopped old PTY")
        assertEquals("shared", store.openFile("from-guest.txt").text)
        assertEquals("unsaved survives restart", store.state.value.drafts["draft.txt"]?.text)
        assertTrue(store.flush())
        // Data is physically owned by the Server under the new metadata directory only.
        assertTrue(File(root, ".workspace/env.json").isFile)
        assertFalse(File(root, ".workflow").exists())
        assertFalse(File(root, "container.json").exists())

        val failedSpec = Json.stringify(mapOf("version" to 1, "node" to emptyList<String>(),
            "post_scripts" to listOf(mapOf("id" to "rollback", "run" to "printf bad > \"\$HOME/qa-marker\"; exit 23", "user" to "work"))))
        assertTrue(store.saveFile(WorkspacePaths.ENVIRONMENT, failedSpec) is SaveResult.Saved)
        rpc.request("environment.reconcile")
        val failed = withTimeout(120_000) { engine.health.first { it is EnvironmentHealth.Failed } }
        assertTrue("Failed rebuild must retain the verified environment", failed.usable)
        assertEquals("home-kept", runGuest(engine, listOf("/usr/bin/cat", "/home/work/qa-marker")).output.toString(Charsets.UTF_8))
        println("ENGINE_ACCEPTANCE failed post-script rolled back home, prior environment usable")

        store.close()
        scope.cancel()
        scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        startServer()
        ready()
        assertEquals("unsaved survives restart", store.state.value.drafts["draft.txt"]?.text)
        assertEquals(session, store.state.value.activeSessionId)
        assertEquals("shared", store.openFile("from-guest.txt").text)
        assertEquals("home-kept", runGuest(engine, listOf("/usr/bin/cat", "/home/work/qa-marker")).output.toString(Charsets.UTF_8))
        println("ENGINE_ACCEPTANCE Server restart restored session, unsaved draft, file and home")
    }

    @Test fun claudeInstallsOnDemandAndRunsInsideTheEnvironment() = runBlocking {
        ready()
        val installer = ClaudeCodeInstaller(engine, scope)
        installer.install()
        val result = withTimeout(660_000) { installer.state.first { it is ClaudeCodeInstaller.State.Installed || it is ClaudeCodeInstaller.State.Failed } }
        assertTrue(result.toString(), result is ClaudeCodeInstaller.State.Installed)
        val probe = runGuest(engine, listOf(ClaudeCodeInstaller.GUEST_BINARY, "--version"))
        assertEquals(probe.error, 0, probe.exitCode)
        assertTrue(probe.output.toString(Charsets.UTF_8).contains(ClaudeCodeInstaller.VERSION))
    }
}
