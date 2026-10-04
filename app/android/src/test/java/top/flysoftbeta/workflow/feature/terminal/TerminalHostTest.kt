package top.flysoftbeta.workflow.feature.terminal

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.emptyFlow
import kotlinx.coroutines.flow.onStart
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test
import top.flysoftbeta.workflow.core.terminal.ManagedTerminalBackend
import top.flysoftbeta.workflow.core.terminal.ManagedTerminalProcess
import top.flysoftbeta.workflow.core.terminal.ShellPaths
import top.flysoftbeta.workflow.core.terminal.TerminalFrame
import top.flysoftbeta.workflow.core.terminal.TerminalMetadata
import top.flysoftbeta.workflow.core.terminal.TerminalSpec

class TerminalHostTest {
    @Test fun terminalCreatedBeforeTheEnvironmentIsUsableStartsThroughAttach() = runBlocking {
        val created = FakeProcess("starting")
        val attached = FakeProcess("running")
        val backend = FakeBackend(created, attached)
        val host = TerminalHost(backend)

        val id = host.create("src")
        val session = host.session(id)!!
        assertEquals("engine-id", id)
        assertEquals(4, session.ordinal)
        assertEquals(TerminalStatus.Starting, session.status.value)
        // The panel can open now; the attachment waits for the environment and nothing reads the created resource.
        await { backend.attachRequests == 1 }
        delay(50)
        assertEquals(TerminalStatus.Starting, session.status.value)
        assertFalse(created.collected)

        backend.environmentUsable.complete(Unit)
        await { attached.collected }
        attached.emit("running", "$ ")
        await { session.status.value == TerminalStatus.Running }
        assertEquals("$ ", session.outputSince(-1, -1).second.text)
        assertFalse(created.collected)
    }

    @Test fun terminalCreatedWithAUsableEnvironmentIsReadWithoutAttaching() = runBlocking {
        val created = FakeProcess("running")
        val backend = FakeBackend(created, FakeProcess("running"))
        val session = TerminalHost(backend).let { it.session(it.create(null))!! }

        await { created.collected }
        created.emit("running", "ready")
        await { session.status.value == TerminalStatus.Running }
        assertEquals(0, backend.attachRequests)
    }

    private suspend fun await(check: () -> Boolean) = withTimeout(5000) { while (!check()) delay(10) }

    private class FakeBackend(private val created: FakeProcess, private val attached: FakeProcess) : ManagedTerminalBackend {
        override val paths = ShellPaths("/workspace", "/home/work")
        /** Stands in for EngineController.awaitUsable in the production attach. */
        val environmentUsable = CompletableDeferred<Unit>()
        @Volatile var attachRequests = 0
        override suspend fun create(spec: TerminalSpec) = created
        override suspend fun attach(id: String, rows: Int?, columns: Int?): ManagedTerminalProcess {
            attachRequests++
            environmentUsable.await()
            return attached
        }
    }

    private class FakeProcess(status: String) : ManagedTerminalProcess {
        override val initial = TerminalMetadata("engine-id", 4, 1, "/workspace/src", null, null, status, null, null, 24, 80)
        private val changes = Channel<TerminalFrame>(Channel.UNLIMITED)
        @Volatile var collected = false
        override val frames = changes.receiveAsFlow().onStart { collected = true }
        override val output = emptyFlow<ByteArray>()
        suspend fun emit(status: String, text: String) {
            changes.send(TerminalFrame(initial.copy(status = status), text.toByteArray(), reset = false, eof = false))
        }
        override suspend fun write(bytes: ByteArray) {}
        override suspend fun resize(rows: Int, columns: Int) {}
        override suspend fun awaitExit() = 0
        override fun terminate(force: Boolean) {}
        override suspend fun currentDirectory() = initial.cwd
        override suspend fun restart(rows: Int, columns: Int) = initial
        override suspend fun rename(title: String?) = initial
        override suspend fun clear() = initial
    }
}
