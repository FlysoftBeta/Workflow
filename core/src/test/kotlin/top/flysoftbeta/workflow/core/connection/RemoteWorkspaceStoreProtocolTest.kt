package top.flysoftbeta.workflow.core.connection

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.first
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.core.config.AppConfig
import top.flysoftbeta.workflow.core.config.ConfigCodec
import top.flysoftbeta.workflow.core.json.Json
import top.flysoftbeta.workflow.core.layout.Workbench
import top.flysoftbeta.workflow.core.session.Session
import top.flysoftbeta.workflow.core.store.*

class RemoteWorkspaceStoreProtocolTest {
    @Test fun staleWatchCannotReplaceNewerCommandProjection() = runBlocking<Unit> {
        val release = CompletableDeferred<Unit>()
        RpcPeer { request ->
            val result = when (request["method"]) {
                "hello" -> WorkspaceRpcTest.greeting
                "workspace.snapshot" -> snapshot(1)
                "workspace.watch" -> { release.await(); delay(25); snapshot(1) }
                "workspace.command" -> snapshot(2, "newer") + ("value" to "session")
                else -> error("unexpected request")
            }
            WorkspaceRpcTest.response(request, result)
        }.use { peer ->
            val store = RemoteWorkspaceStore(peer.rpc, peer.scope, "test").also { it.start() }
            assertEquals(StoreStatus.READY, withTimeout(2_000) { store.awaitReady() }.status)
            assertEquals("session", store.createSession("newer"))
            release.complete(Unit)
            delay(100)
            assertEquals("newer", store.state.value.sessions.single().name)
            store.disconnect()
        }
    }

    @Test fun cancellingAQueuedRequestDoesNotRunItOrStopLaterCommands() = runBlocking<Unit> {
        val begun = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        val received = java.util.concurrent.CopyOnWriteArrayList<String>()
        RpcPeer { request ->
            val result = when (request["method"]) {
                "hello" -> WorkspaceRpcTest.greeting
                "workspace.snapshot" -> snapshot(1)
                "workspace.watch" -> { delay(60_000); snapshot(1) }
                "workspace.command" -> {
                    val args = WorkspaceWire.obj(WorkspaceWire.obj(request["params"])["args"])
                    val name = WorkspaceWire.string(args, "name")
                    received += name
                    if (name == "first") { begun.complete(Unit); release.await() }
                    snapshot(2) + ("value" to name)
                }
                else -> error("unexpected request")
            }
            WorkspaceRpcTest.response(request, result)
        }.use { peer ->
            val store = RemoteWorkspaceStore(peer.rpc, peer.scope, "test").also { it.start() }
            store.awaitReady()
            val first = async { store.createSession("first") }
            begun.await()
            val cancelled = async { store.createSession("cancelled") }
            delay(20)
            cancelled.cancelAndJoin()
            release.complete(Unit)
            assertEquals("first", first.await())
            assertEquals("next", withTimeout(2_000) { store.createSession("next") })
            assertEquals(listOf("first", "next"), received.toList())
            store.disconnect()
        }
    }

    @Test fun rejectsMalformedAuthoritativeLayoutInsteadOfRepairingOrInventingEmptyLayout() {
        val state = WorkspaceWire.obj(snapshot(1, "session")["state"]).toMutableMap()
        val sessions = (state["sessions"] as List<*>).map { WorkspaceWire.obj(it).toMutableMap() }
        sessions.single()["workbench"] = mapOf("invalid" to true)
        state["sessions"] = sessions
        assertTrue(runCatching { WorkspaceWire.state(state) }.isFailure)
        assertTrue(runCatching { WorkspaceWire.long(mapOf("revision" to 1.5), "revision") }.isFailure)
    }

    @Test fun disconnectCancelsPendingCommandsAndRejectsFutureMutations() = runBlocking<Unit> {
        val sent = CompletableDeferred<Unit>()
        RpcPeer { request ->
            val result = when (request["method"]) {
                "hello" -> WorkspaceRpcTest.greeting
                "workspace.snapshot" -> snapshot(1)
                else -> { if (request["method"] == "workspace.command") sent.complete(Unit); delay(60_000); null }
            }
            WorkspaceRpcTest.response(request, result)
        }.use { peer ->
            val store = RemoteWorkspaceStore(peer.rpc, peer.scope, "test").also { it.start() }
            store.awaitReady()
            val pending = async { runCatching { store.createSession("pending") } }
            sent.await()
            store.disconnect("lost")
            assertTrue(withTimeout(2_000) { pending.await() }.isFailure)
            assertEquals(StoreStatus.FAILED, store.state.value.status)
            assertTrue(runCatching { store.createSession("after disconnect") }.isFailure)
        }
    }

    private fun snapshot(revision: Long, name: String? = null): Map<String, Any?> {
        val sessions = StateCodec.encodeSessions(if (name == null) emptyList() else listOf(Session("session", name, 1, workbench = Workbench.empty())), if (name == null) null else "session", emptyList())
        return mapOf("revision" to revision, "state" to (WorkspaceWire.obj(Json.parse(sessions)) + mapOf(
            "status" to "ready", "failure" to null, "config" to Json.parse(ConfigCodec.encode(AppConfig())),
            "drafts" to emptyMap<String, Any?>(), "composers" to emptyMap<String, Any?>(), "disk" to emptyMap<String, Any?>(),
            "notices" to emptyList<Any>(), "configProblem" to null, "writeError" to null,
        )))
    }
}
