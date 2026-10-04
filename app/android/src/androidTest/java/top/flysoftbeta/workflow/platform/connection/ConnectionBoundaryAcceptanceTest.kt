package top.flysoftbeta.workflow.platform.connection

import android.content.Context
import android.os.Build
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import java.io.File
import java.util.UUID
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.first
import org.junit.Assert.*
import org.junit.Assume.assumeTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.MainActivity
import top.flysoftbeta.workflow.core.config.ThemeMode
import top.flysoftbeta.workflow.core.connection.*
import top.flysoftbeta.workflow.core.json.Json
import top.flysoftbeta.workflow.core.store.*
import top.flysoftbeta.workflow.platform.UnhandledFailures
import top.flysoftbeta.workflow.platform.service.LocalRuntimeService

/** Real app UID, packaged Engine and manager lifecycle. Disposable wrapper-managed emulator only. */
@RunWith(AndroidJUnit4::class)
class ConnectionBoundaryAcceptanceTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    @Test fun connectionFailureRetiresScopeLeaseAndProjectionThenReconnectsFromServer() = runBlocking<Unit> {
        assumeTrue(Build.HARDWARE.contains("goldfish") || Build.HARDWARE.contains("ranchu"))
        val context = ApplicationProvider.getApplicationContext<Context>()
        val manager = WorkspaceConnectionManager.get(context)
        val defects = UnhandledFailures.defects
        run {
            withTimeout(50_000) { manager.status.first { it !is ConnectionStatus.Loading && it !is ConnectionStatus.Connecting } }
            manager.disconnect()
            await { LocalRuntimeService.state.value.environmentCount == 0 }
            val id = "client-qa-${UUID.randomUUID()}"
            val profile = WorkspaceConnectionConfig(id, "Connection acceptance", WorkspaceEndpoint.Embedded(id))
            manager.connect(profile)
            val first = withTimeout(50_000) { manager.status.first { it is ConnectionStatus.Connected || it is ConnectionStatus.Failed } }
            assertTrue(first.toString(), first is ConnectionStatus.Connected)
            // Compose produces frames only when the test synchronizes: let the activity bind this session's Workbench.
            compose.waitForIdle()
            val session = manager.requireSession()
            val root = File(context.filesDir, "workspaces/${session.profile.id}")
            assertTrue(File(context.filesDir, "workspace-connections.json").readText().contains(id))
            val payload = ByteArray(131_073) { (it % 239).toByte() }
            assertEquals(FileOpResult.Done, session.store.createFile("binary.dat", payload))
            assertArrayEquals(payload, session.store.readBytes("binary.dat", payload.size))
            assertEquals(FileOpResult.Done, session.store.createFile("draft.md", "disk".toByteArray()))
            val snapshot = session.store.openFile("draft.md")
            session.store.editFile("draft.md", "draft survives", snapshot.disk)
            assertTrue(session.store.flush())
            assertTrue(session.store.updateConfig { it.copy(appearance = it.appearance.copy(theme = ThemeMode.DARK)) } is ConfigUpdate.Updated)
            await { manager.localConfig.value.appearance.theme == ThemeMode.DARK }
            val cached = WorkspaceWire.obj(Json.parse(File(context.filesDir, "workspace-local-config.json").readText()))
            assertEquals(id, cached["connection"])
            assertEquals(setOf("appearance", "launcher", "overlay", "terminal"), WorkspaceWire.obj(cached["config"]).keys)
            assertFalse(cached.containsKey("sessions"))
            compose.waitForIdle()
            withContext(Dispatchers.IO) { session.rpc.close() }
            // A connection lost after it was online reconnects by itself while the Workbench stays inert.
            val lost = withTimeout(10_000) { manager.status.first { it is ConnectionStatus.Reconnecting } } as ConnectionStatus.Reconnecting
            assertEquals(1, lost.attempt)
            // The card's progress animates indefinitely, so Compose never idles by itself while it is shown.
            compose.mainClock.autoAdvance = false
            val reconnected = try {
                try { compose.waitUntil(10_000) { overlayShown() } }
                catch (missing: androidx.compose.ui.test.ComposeTimeoutException) {
                    throw AssertionError("No reconnect card while ${manager.status.value::class.simpleName}", missing)
                }
                await { !session.scope.isActive }
                assertNull(manager.sessionOrNull())
                assertEquals(StoreStatus.FAILED, session.store.state.value.status)
                assertTrue(session.store.createFile("must-not-exist") is FileOpResult.Failed)
                assertFalse(File(root, "must-not-exist").exists())
                val connected = withTimeout(50_000) { manager.status.first { it is ConnectionStatus.Connected } } as ConnectionStatus.Connected
                compose.waitUntil(10_000) { !overlayShown() }
                connected
            } finally { compose.mainClock.autoAdvance = true }
            assertNotEquals(session.token, reconnected.session.token)
            assertEquals("draft survives", reconnected.session.store.state.value.drafts["draft.md"]?.text)
            assertEquals(ThemeMode.DARK, reconnected.session.store.state.value.config.appearance.theme)
            assertEquals(session.profile.id, reconnected.session.profile.id)
            manager.disconnect()
            await { !reconnected.session.scope.isActive && LocalRuntimeService.state.value.environmentCount == 0 }
            assertNull(manager.sessionOrNull())
            manager.connect(WorkspaceConnectionConfig("remote", "Unsupported", WorkspaceEndpoint.Remote(RemoteBootstrapSpec("ssh", "example.invalid", "/work"))))
            val unsupported = withTimeout(10_000) { manager.status.first { it is ConnectionStatus.Failed } }
            assertTrue((unsupported as ConnectionStatus.Failed).message.contains("尚未实现"))
            assertNull(manager.sessionOrNull())
            assertEquals(0, LocalRuntimeService.state.value.environmentCount)
            // Leave the process-wide manager unselected for the next test, not on the unsupported profile.
            manager.disconnect()
        }
        assertEquals("A failure reached the last-resort handler", defects, UnhandledFailures.defects)
    }

    /** Samples the reconnect card on the manually advanced clock. */
    private fun overlayShown(): Boolean {
        compose.mainClock.advanceTimeByFrame()
        return compose.onAllNodesWithTag("connection:overlay").fetchSemanticsNodes().isNotEmpty()
    }

    private suspend fun await(predicate: () -> Boolean) = withTimeout(15_000) { while (!predicate()) delay(50) }
}
