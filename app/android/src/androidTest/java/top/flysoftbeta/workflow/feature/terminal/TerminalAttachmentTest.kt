package top.flysoftbeta.workflow.feature.terminal

import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.coroutines.*
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.core.terminal.*

@RunWith(AndroidJUnit4::class)
class TerminalAttachmentTest {
    @Test fun endedAttachmentObservesAnotherClientRestartAndNeverStopsOnCancellation() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        val remote = FakeAttachment()
        val backend = FakeBackend(remote)
        val session = TerminalSession("engine-id", 0, backend, scope)
        try {
            session.attach()
            remote.emit(1, "old", eof = true)
            await { session.status.value is TerminalStatus.Ended }
            remote.emit(1, "", eof = true) // Repeated EOF must not finalize the decoder twice.
            remote.emit(2, "new", eof = false)
            await { session.generation.value == 2L }
            val (generation, slice) = session.outputSince(3, 1)
            assertEquals(2L, generation)
            assertTrue(slice.reset)
            assertEquals("new", slice.text)
            assertTrue(session.status.value is TerminalStatus.Running)
            assertEquals(0, remote.restarts)
            assertEquals(0, remote.stops)
            assertEquals(1, backend.attachments)
        } finally { scope.cancel() }
        assertEquals(0, remote.stops)
    }

    @Test fun renameAndClearWaitForAuthoritativeFrames() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        val remote = FakeAttachment()
        val session = TerminalSession("engine-id", 0, FakeBackend(remote), scope)
        try {
            session.bind(remote)
            remote.emit(1, "retained", eof = false)
            await { session.status.value is TerminalStatus.Running }
            session.rename("requested")
            session.clearScrollback()
            await { remote.renames == 1 && remote.clears == 1 }
            assertNull(session.customTitle.value)
            assertEquals("retained", session.outputSince(-1, -1).second.text)
            remote.emit(2, "", eof = false, title = "accepted")
            await { session.generation.value == 2L }
            assertEquals("accepted", session.customTitle.value)
            assertEquals("", session.outputSince(-1, -1).second.text)
        } finally { scope.cancel() }
    }

    private suspend fun await(check: () -> Boolean) = withTimeout(5000) { while (!check()) delay(10) }

    private class FakeBackend(private val remote: FakeAttachment) : ManagedTerminalBackend {
        override val paths = ShellPaths("/workspace", "/home/work")
        @Volatile var attachments = 0
        override suspend fun create(spec: TerminalSpec) = remote
        override suspend fun attach(id: String, rows: Int?, columns: Int?): ManagedTerminalProcess {
            attachments++
            return remote
        }
    }

    private class FakeAttachment : ManagedTerminalProcess {
        override val initial = TerminalMetadata("engine-id", 9, 1, "/workspace/sub", null, null, "running", null, null, 24, 80)
        private val changes = Channel<TerminalFrame>(Channel.UNLIMITED)
        override val frames = changes.receiveAsFlow()
        override val output = emptyFlow<ByteArray>()
        @Volatile var stops = 0
        @Volatile var restarts = 0
        @Volatile var renames = 0
        @Volatile var clears = 0
        suspend fun emit(generation: Long, text: String, eof: Boolean, title: String? = null) {
            changes.send(TerminalFrame(initial.copy(generation = generation, customTitle = title,
                status = if (eof) "ended" else "running", exitCode = if (eof) 0 else null),
                text.toByteArray(), reset = generation != 1L, eof = eof))
        }
        override suspend fun restart(rows: Int, columns: Int): TerminalMetadata { restarts++; return initial }
        override suspend fun rename(title: String?): TerminalMetadata { renames++; return initial }
        override suspend fun clear(): TerminalMetadata { clears++; return initial }
        override suspend fun write(bytes: ByteArray) {}
        override suspend fun resize(rows: Int, columns: Int) {}
        override suspend fun awaitExit() = 0
        override fun terminate(force: Boolean) { stops++ }
        override suspend fun currentDirectory() = initial.cwd
    }
}
