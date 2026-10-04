package top.flysoftbeta.workflow.core.connection

import java.io.File
import java.io.InputStream
import java.nio.file.Files
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.first
import org.junit.Assert.*
import org.junit.Assume.assumeTrue
import org.junit.Test
import top.flysoftbeta.workflow.core.config.ThemeMode
import top.flysoftbeta.workflow.core.json.Json
import top.flysoftbeta.workflow.core.layout.*
import top.flysoftbeta.workflow.core.session.ArchiveDecision
import top.flysoftbeta.workflow.core.store.*

/** Real Kotlin client -> host Rust Engine, with isolated roots and no host process fallback. */
class RemoteWorkspaceStoreEngineTest {
    @Test fun engineAllocatesImportNamesAtCommitWithoutOverwritingConcurrentFile() = runBlocking<Unit> {
        withEngine { engine ->
            val first = engine.store.importUnique("inbox", "archive.tar.gz", 3) {
                File(engine.root, "inbox").mkdirs()
                File(engine.root, "inbox/archive.tar.gz").writeText("keep")
                "new".byteInputStream()
            }
            assertEquals("inbox/archive (1).tar.gz", first)
            assertEquals("keep", File(engine.root, "inbox/archive.tar.gz").readText())
            assertEquals("new", File(engine.root, first).readText())
            assertEquals("inbox/archive (2).tar.gz", engine.store.importUnique("inbox", "archive.tar.gz", 0) { byteArrayOf().inputStream() })
            assertTrue(engine.uploads().isEmpty())
        }
    }

    @Test fun sizeDeclaredChunkedUploadIsAtomicAndNeverOverwrites() = runBlocking<Unit> {
        withEngine { engine ->
            val bytes = ByteArray(3 * 65_536 + 713) { (it % 251).toByte() }
            val closed = AtomicBoolean()
            assertEquals(FileOpResult.Done, engine.store.importFile("media/binary.dat", bytes.size.toLong()) {
                object : java.io.ByteArrayInputStream(bytes) { override fun close() { closed.set(true); super.close() } }
            })
            assertTrue(closed.get())
            assertArrayEquals(bytes, engine.store.readBytes("media/binary.dat", bytes.size))
            assertTrue(engine.store.importFile("media/binary.dat", 1) { error("Existing destination must fail before source is opened") } is FileOpResult.Failed)
            assertArrayEquals(bytes, File(engine.root, "media/binary.dat").readBytes())
            assertEquals(FileOpResult.Done, engine.store.createFile("empty"))
            assertEquals(0L, File(engine.root, "empty").length())
            assertTrue(engine.store.importFile("racing", 1) {
                File(engine.root, "racing").writeText("keep")
                byteArrayOf(9).inputStream()
            } is FileOpResult.Failed)
            assertEquals("keep", File(engine.root, "racing").readText())
            assertEquals(emptyList<String>(), engine.uploads())
        }
    }

    @Test fun shortLongFailedAndCancelledSourcesLeaveNoPublishedFileOrStaging() = runBlocking<Unit> {
        withEngine { engine ->
            assertTrue(engine.store.importFile("short", 2) { byteArrayOf(1).inputStream() } is FileOpResult.Failed)
            assertTrue(engine.store.importFile("long", 1) { byteArrayOf(1, 2).inputStream() } is FileOpResult.Failed)
            assertTrue(engine.store.importFile("broken", 4) { throw java.io.IOException("source missing") } is FileOpResult.Failed)
            val closed = AtomicBoolean()
            assertTrue(runCatching { engine.store.importFile("cancelled", 2) {
                object : InputStream() {
                    override fun read(): Int = throw CancellationException("cancel source")
                    override fun close() { closed.set(true) }
                }
            } }.exceptionOrNull() is CancellationException)
            assertTrue(closed.get())
            assertEquals(emptyList<String>(), engine.uploads())
            for (path in listOf("short", "long", "broken", "cancelled")) assertFalse(File(engine.root, path).exists())
            assertEquals(FileOpResult.Done, engine.store.createFile("after-cancel", "ok".toByteArray()))
        }
    }

    @Test fun serverOwnsDraftsLayoutsArchiveAndReconnectProjection() = runBlocking<Unit> {
        val root = Files.createTempDirectory("workflow-client-reconnect").toFile()
        try {
            var id = ""
            Engine(root).use { engine ->
                engine.ready()
                id = engine.store.createSession("client session")
                assertEquals(FileOpResult.Done, engine.store.createFile("notes.md", "disk".toByteArray()))
                val opened = engine.store.openFile("notes.md")
                engine.store.editFile("notes.md", "draft", opened.disk)
                engine.store.applyLayout(id, LayoutOp.Open(PanelTarget.File("notes.md")))
                assertTrue(engine.store.archiveSession(id) is ArchiveOutcome.NeedsDecision)
                assertTrue(engine.store.archiveSession(id, ArchiveDecision.KEEP_DRAFTS) is ArchiveOutcome.Archived)
                assertTrue(engine.store.restoreSession(id))
                assertTrue(engine.store.flush())
                engine.rpc.close()
                withTimeout(3_000) { engine.store.state.first { it.status == StoreStatus.FAILED } }
                assertTrue(runCatching { engine.store.createSession("disconnected") }.isFailure)
            }
            Engine(root).use { engine ->
                engine.ready()
                assertEquals("draft", engine.store.state.value.drafts["notes.md"]?.text)
                assertEquals(id, engine.store.state.value.activeSessionId)
                assertTrue(engine.store.state.value.sessions.single().workbench.panels.values.any { it.target == PanelTarget.File("notes.md") })
                assertEquals("disk", File(root, "notes.md").readText())
            }
        } finally { root.deleteRecursively() }
    }

    @Test fun configConflictReappliesTransformToAuthoritativeConfigAndPreservesUnknownFields() = runBlocking<Unit> {
        withEngine { engine ->
            var attempts = 0
            val result = engine.store.updateConfig { config ->
                if (attempts++ == 0) {
                    val path = File(engine.root, ".workspace/config.json")
                    val raw = WorkspaceWire.obj(Json.parse(path.readText())).toMutableMap()
                    raw["terminal"] = mapOf("extraKeysPinned" to true)
                    raw["extension"] = mapOf("value" to "preserve")
                    path.writeText(Json.stringify(raw))
                }
                config.copy(appearance = config.appearance.copy(theme = ThemeMode.LIGHT))
            }
            assertTrue(result is ConfigUpdate.Updated)
            assertEquals(2, attempts)
            assertTrue(engine.store.state.value.config.terminal.extraKeysPinned)
            assertEquals(ThemeMode.LIGHT, engine.store.state.value.config.appearance.theme)
            val raw = WorkspaceWire.obj(Json.parse(File(engine.root, ".workspace/config.json").readText()))
            assertEquals(mapOf("value" to "preserve"), raw["extension"])
            val local = WorkspaceWire.obj(engine.rpc.request("client.config", mapOf("clientId" to "client-test")))
            assertFalse(WorkspaceWire.obj(local["config"]).containsKey("agent"))
        }
    }

    private suspend fun withEngine(block: suspend (Engine) -> Unit) {
        val root = Files.createTempDirectory("workflow-client-engine").toFile()
        try { Engine(root).use { it.ready(); block(it) } } finally { root.deleteRecursively() }
    }

    private class Engine(val root: File) : AutoCloseable {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
        private val executable = System.getenv("WORKFLOW_ENGINE")?.let(::File) ?: File("../../engine/target/debug/workflow-engine")
        init { assumeTrue("Build host Rust Engine or set WORKFLOW_ENGINE", executable.canExecute()) }
        private val process = ProcessBuilder(executable.absolutePath, "serve", "--root", root.path)
            .redirectError(File(root, "engine-diagnostic.log")).start()
        val rpc = WorkspaceRpc(object : WorkspaceTransport {
            override val input = process.inputStream
            override val output = process.outputStream
            override fun close() { process.destroy(); if (!process.waitFor(2, TimeUnit.SECONDS)) process.destroyForcibly(); input.close(); output.close() }
        }, scope)
        val store = RemoteWorkspaceStore(rpc, scope, "client-test").also { it.start() }
        suspend fun ready() { assertEquals(StoreStatus.READY, withTimeout(5_000) { store.awaitReady() }.status) }
        fun uploads() = File(root, ".workspace/cache/uploads").list().orEmpty().toList()
        override fun close() { store.disconnect(); scope.cancel(); assertTrue(process.waitFor(3, TimeUnit.SECONDS)) }
    }
}
