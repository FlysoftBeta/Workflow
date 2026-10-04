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
import top.flysoftbeta.workflow.agent.model.*
import top.flysoftbeta.workflow.agent.rpc.*
import kotlinx.serialization.json.*
import top.flysoftbeta.workflow.core.connection.*
import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.core.json.Json
import top.flysoftbeta.workflow.core.layout.*
import top.flysoftbeta.workflow.core.store.*
import top.flysoftbeta.workflow.core.terminal.TerminalSpec
import top.flysoftbeta.workflow.core.terminal.ManagedTerminalProcess
import top.flysoftbeta.workflow.platform.agent.AgentHub
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
                "--apk", context.applicationInfo.sourceDir)
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
        WorkspaceNetworkReporter.attach(context, WorkspaceConnectionSession("qa", profile, rpc, store, scope))
        engine = EngineController(context, scope) { store }.also { it.start() }
    }
    private suspend fun ready() {
        rpc.request("environment.reconcile")
        val health = withTimeout(180_000) { engine.health.first { it.usable || it is EnvironmentHealth.Failed || it is EnvironmentHealth.Unavailable } }
        assertTrue(engine.describe(health) + "\n" + engine.logTail(null), health.usable)
    }
    @After fun stop() = runBlocking {
        if (::store.isInitialized) runCatching { withTimeout(5_000) { store.close() } }
        if (::server.isInitialized) server.destroyForcibly()
        scope.cancel()
        if (::root.isInitialized) withContext(Dispatchers.IO) { root.deleteRecursively() }
    }

    @Test fun bundledCodexStartsFromLoginShellWithoutDaemonPackage() = runBlocking<Unit> {
        ready()
        val terminal = createTerminal(TerminalSpec(rows = 30, columns = 100))
        val output = StringBuffer()
        val reader = launch {
            var answeredCursor = false
            terminal.output.collect {
                output.append(it.toString(Charsets.UTF_8))
                if (!answeredCursor && output.contains("\u001b[6n")) {
                    answeredCursor = true
                    terminal.write("\u001b[1;1R".toByteArray())
                }
            }
        }
        try {
            terminal.write("codex --no-alt-screen\n".toByteArray())
            withTimeout(30_000) {
                while (!output.contains("Welcome to Codex") && !output.contains("Sign in")) {
                    assertFalse("Standalone CLI requested a daemon package", output.contains("no complete local package"))
                    delay(50)
                }
            }
            println("ENGINE_ACCEPTANCE bundled Codex interactive login screen reached through terminal shell")
        } finally {
            terminal.terminate(force = true)
            withTimeout(15_000) { terminal.awaitExit() }
            reader.cancelAndJoin()
        }
    }

    @Test fun installedEnvironmentRunsTerminalNetworkAndReconcilesRestart() = runBlocking {
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

        // Chat startup writes plugin/cache/database files. Its generation restart is tested
        // separately; post-script preparation must keep rejecting a file changing during copy.
        val terminal = createTerminal(TerminalSpec())
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
        reader.cancelAndJoin()
        val restored = EngineTerminalBackend(engine).attach(terminal.initial.id)
        assertTrue("Engine restarted the terminal resource in the new environment", restored.initial.generation > terminal.initial.generation)
        assertEquals(true, WorkspaceWire.obj(rpc.request("terminal.wait", mapOf("terminalId" to terminal.initial.id, "timeoutMs" to 0)))["running"])
        val changed = runGuest(engine, listOf("/bin/bash", "-lc", "test \"\$WF_ACCEPTANCE\" = reconciled && test \"\$(cat ~/qa-marker)\" = home-kept && node --version; code=\$?; test \"\$code\" = 127"))
        assertEquals(changed.error, 0, changed.exitCode)
        println("ENGINE_ACCEPTANCE explicit restart activated config, retained home and restored terminal resource")
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

    private suspend fun createTerminal(spec: TerminalSpec): ManagedTerminalProcess {
        val terminal = EngineTerminalBackend(engine).create(spec)
        // Engine retires terminals without layout references; the fixture acts like a real panel.
        val session = store.state.value.activeSessionId ?: store.createSession("Terminal acceptance")
        store.applyLayout(session, LayoutOp.Open(PanelTarget.Terminal(terminal.initial.id)))
        return terminal
    }

    private fun wire(value: Any?): JsonElement = ChatWire.json.parseToJsonElement(Json.stringify(value))

    private suspend fun chatSnapshot(): ChatSnapshot = ChatWire.snapshot(wire(rpc.request("chat.snapshot", timeoutMs = 90_000)))

    private suspend fun chatCommand(name: String, args: Map<String, Any?> = emptyMap()): JsonElement =
        wire(rpc.request("chat.command", mapOf("name" to name, "args" to args), 120_000))

    private suspend fun tool(id: String): Map<String, Any?> {
        val status = WorkspaceWire.obj(rpc.request("environment.tools.status", timeoutMs = 90_000))
        return (status["tools"] as? List<*>)?.map(WorkspaceWire::obj)?.single { it["id"] == id }
            ?: error("Engine omitted tool $id")
    }

    /** No credentials or model turn: actual vendor initialize and account/read over Engine RPC. */
    private suspend fun startCodexThroughChat(): ChatSnapshot {
        val before = chatSnapshot()
        assertTrue("Guest JVM must publish a service epoch", before.epoch.isNotBlank())
        withTimeout(30_000) {
            while (BackendKind.CODEX !in chatSnapshot().metadata.available) delay(100)
        }
        chatCommand("warmUp", mapOf("kind" to "codex"))
        chatCommand("refreshAccount", mapOf("kind" to "codex"))
        return withTimeout(30_000) {
            var snapshot = chatSnapshot()
            while (snapshot.state.backend(BackendKind.CODEX).process !is ProcessState.Ready ||
                snapshot.state.backend(BackendKind.CODEX).account.state == LoginState.UNKNOWN) {
                delay(100)
                snapshot = chatSnapshot()
            }
            assertEquals(before.epoch, snapshot.epoch)
            assertEquals("Fresh isolated guest has no vendor credentials", LoginState.LOGGED_OUT,
                snapshot.state.backend(BackendKind.CODEX).account.state)
            assertTrue(snapshot.metadata.loginMethods[BackendKind.CODEX].orEmpty().isNotEmpty())
            assertTrue(snapshot.metadata.processEpochs[BackendKind.CODEX]?.isNotBlank() == true)
            snapshot
        }
    }

    @Test fun engineGuestChatPublishesStateAndNewClientReattachesToSameConversation() = runBlocking<Unit> {
        ready()
        for (id in listOf("codex")) {
            val measured = tool(id)
            assertEquals("Required Engine tool $id: $measured", "ready", measured["phase"])
            assertTrue((measured["version"] as? String).orEmpty().isNotBlank())
        }
        val before = chatSnapshot()
        val connected = startCodexThroughChat()
        val update = ChatWire.update(wire(rpc.request("chat.watch", mapOf("epoch" to before.epoch,
            "afterRevision" to before.revision, "timeoutMs" to 1000))))
        assertFalse("Small startup journal must remain replayable", update.resnapshot)
        assertEquals(before.epoch, update.epoch)
        assertTrue(update.revision > before.revision)
        val reduced = AgentReducer.reduceAll(before.state, update.events)
        assertTrue(reduced.backend(BackendKind.CODEX).process is ProcessState.Ready)
        assertEquals(LoginState.LOGGED_OUT, reduced.backend(BackendKind.CODEX).account.state)
        assertEquals(connected.metadata.processEpochs[BackendKind.CODEX], update.metadata.processEpochs[BackendKind.CODEX])

        val id = "engine-owned-${UUID.randomUUID()}"
        assertEquals(id, chatCommand("newConversation", mapOf("id" to id, "backend" to "codex")).jsonPrimitive.content)
        chatCommand("rename", mapOf("id" to id, "title" to "Engine-owned conversation"))
        chatCommand("flush")
        val firstScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        val secondScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        try {
            val first = AgentHub(context, store, firstScope, Dispatchers.IO).also { it.start() }
            val entry = withTimeout(30_000) { first.conversations.first { entries -> entries.any { it.id == id && it.title == "Engine-owned conversation" } }.single { it.id == id } }
            firstScope.cancel()
            val second = AgentHub(context, store, secondScope, Dispatchers.IO).also { it.start() }
            val reattached = withTimeout(30_000) { second.conversations.first { entries -> entries.any { it.id == id && it.title == "Engine-owned conversation" } }.single { it.id == id } }
            assertEquals(entry, reattached)
            assertEquals("Engine-owned conversation", reattached.title)
            assertEquals(connected.epoch, chatSnapshot().epoch)
            assertEquals(1, chatSnapshot().metadata.conversations.count { it.id == id })
        } finally { firstScope.cancel(); secondScope.cancel() }
        // Changing only declared environment variables needs no home snapshot. Verify Engine
        // owns stopping/rehydrating chat independently of the transactional post-script tests.
        val declaration = Json.stringify(mapOf("version" to 1, "env" to mapOf("WF_CHAT_RESTART" to "yes")))
        assertTrue(store.saveFile(WorkspacePaths.ENVIRONMENT, declaration) is SaveResult.Saved)
        rpc.request("environment.reconcile")
        val pending = withTimeout(120_000) { engine.health.first { it is EnvironmentHealth.NeedsRestart || it is EnvironmentHealth.Failed } }
        assertTrue(engine.describe(pending), pending is EnvironmentHealth.NeedsRestart)
        engine.restartEnvironment()
        val restored = chatSnapshot()
        assertNotEquals("A generation restart retires the old chat service", connected.epoch, restored.epoch)
        assertEquals("Engine-owned conversation", restored.metadata.conversations.single { it.id == id }.title)
        val restarted = startCodexThroughChat()
        assertNotEquals(connected.metadata.processEpochs[BackendKind.CODEX], restarted.metadata.processEpochs[BackendKind.CODEX])
        println("ENGINE_ACCEPTANCE guest JVM chat, measured Codex account, event replay, client reattachment and generation restart verified")
    }

    @Test fun terminalAttachmentsReplaySameShellAndNoOpRestartPreservesProcess() = runBlocking<Unit> {
        ready()
        val terminal = createTerminal(TerminalSpec())
        val firstOutput = StringBuffer()
        val firstReader = launch { terminal.output.collect { firstOutput.append(it.toString(Charsets.UTF_8)) } }
        var secondReader: Job? = null
        try {
            terminal.write("WF_ATTACH_KEEP=kept; echo BEFORE-\$((6*7))\n".toByteArray())
            withTimeout(15_000) { while (!firstOutput.contains("BEFORE-42")) delay(50) }
            firstReader.cancelAndJoin()
            val attached = EngineTerminalBackend(engine).attach(terminal.initial.id)
            assertEquals(terminal.initial.id, attached.initial.id)
            assertEquals(terminal.initial.generation, attached.initial.generation)
            val replay = StringBuffer()
            secondReader = launch { attached.output.collect { replay.append(it.toString(Charsets.UTF_8)) } }
            withTimeout(15_000) { while (!replay.contains("BEFORE-42")) delay(50) }
            val before = WorkspaceWire.obj(rpc.request("environment.status"))
            assertEquals("ready", before["phase"])
            engine.restartEnvironment()
            val after = WorkspaceWire.obj(rpc.request("environment.status"))
            assertEquals(WorkspaceWire.obj(before["active"])["generation"], WorkspaceWire.obj(after["active"])["generation"])
            val running = WorkspaceWire.obj(rpc.request("terminal.wait", mapOf("terminalId" to terminal.initial.id, "timeoutMs" to 0)))
            assertEquals(true, running["running"])
            val afterRestart = EngineTerminalBackend(engine).attach(terminal.initial.id)
            assertEquals(terminal.initial.generation, afterRestart.initial.generation)
            attached.write("printf 'AFTER-%s\\n' \"\$WF_ATTACH_KEEP\"\n".toByteArray())
            withTimeout(15_000) { while (!replay.contains("AFTER-kept")) delay(50) }
            println("ENGINE_ACCEPTANCE retained terminal bytes, shell state and no-op environment restart verified")
        } finally {
            terminal.terminate(force = true)
            withTimeout(15_000) { terminal.awaitExit() }
            firstReader.cancelAndJoin()
            secondReader?.cancelAndJoin()
        }
    }

    @Test fun selectingClaudeInstallsAndStartsItThroughEngine() = runBlocking<Unit> {
        ready()
        val id = chatCommand("newConversation", mapOf("backend" to "claude")).jsonPrimitive.content
        assertTrue(chatSnapshot().metadata.conversations.any { it.id == id && it.backend == BackendKind.CLAUDE })
        val result = withTimeout(660_000) {
            var state = tool("claude")
            while (state["phase"] !in setOf("ready", "failed")) { delay(1000); state = tool("claude") }
            state
        }
        assertEquals(result.toString(), "ready", result["phase"])
        val binary = WorkspaceWire.string(result, "binary")
        val version = WorkspaceWire.string(result, "version")
        assertTrue("Engine supplied a guest tool path", binary.startsWith("/opt/workflow/tools/"))
        val probe = runGuest(engine, listOf(binary, "--version"))
        assertEquals(probe.error, 0, probe.exitCode)
        assertTrue(probe.output.toString(Charsets.UTF_8).contains(version))
        withTimeout(30_000) { while (BackendKind.CLAUDE !in chatSnapshot().metadata.available) delay(100) }
        chatCommand("warmUp", mapOf("kind" to "claude"))
        assertTrue(chatSnapshot().state.backend(BackendKind.CLAUDE).process is ProcessState.Ready)
    }
}
