package top.flysoftbeta.workflow.platform.device

import android.content.Context
import android.os.Build
import android.provider.Settings
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.*
import org.junit.Assert.*
import org.junit.Assume.assumeTrue
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.platform.DeviceCapabilities
import top.flysoftbeta.workflow.platform.RootAccess
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import top.flysoftbeta.workflow.platform.connection.ConnectionStatus
import top.flysoftbeta.workflow.platform.connection.WorkspaceConnectionManager

/** May mutate brightness only on the disposable emulator. Never executes on a daily device. */
@RunWith(AndroidJUnit4::class)
class DeviceControlsEmulatorTest {
    private val instrument get() = InstrumentationRegistry.getInstrumentation()
    private val context: Context get() = instrument.targetContext
    private fun command(command: String) { instrument.uiAutomation.executeShellCommand(command).use { android.os.ParcelFileDescriptor.AutoCloseInputStream(it).readBytes() } }
    private fun record(key: String, value: Any) {
        val file = java.io.File(context.filesDir, "w8-acceptance/device-results.json")
        file.parentFile!!.mkdirs()
        val results = if (file.isFile) org.json.JSONObject(file.readText()) else org.json.JSONObject()
        file.writeText(results.put(key, value).toString(2))
    }
    @Before fun emulatorOnly() {
        assumeTrue(InstrumentationRegistry.getArguments().getString("settingsEmulator") == "true" && Build.HARDWARE in listOf("ranchu", "goldfish"))
        runBlocking {
            val manager = WorkspaceConnectionManager.get(context)
            withTimeout(60_000) { manager.status.first { it !is ConnectionStatus.Loading && it !is ConnectionStatus.Connecting } }
            if (manager.sessionOrNull() == null) manager.useEmbedded()
            val connected = withTimeout(60_000) { manager.status.first { it is ConnectionStatus.Connected || it is ConnectionStatus.Failed } }
            check(connected is ConnectionStatus.Connected) { "Packaged Engine fixture failed: $connected" }
        }
    }

    @Test fun brightnessDenialAndReadbackUseActualPermissionAndSystemValues() {
        val capabilities = DeviceCapabilities(context)
        val before = Settings.System.getInt(context.contentResolver, Settings.System.SCREEN_BRIGHTNESS, 128)
        val mode = Settings.System.getInt(context.contentResolver, Settings.System.SCREEN_BRIGHTNESS_MODE, 0)
        val permission = Settings.System.canWrite(context)
        try {
            command("appops set ${context.packageName} WRITE_SETTINGS deny")
            assertFalse(capabilities.snapshot().writeSettingsGranted)
            assertTrue(capabilities.setBrightness(91).isFailure)
            command("appops set ${context.packageName} WRITE_SETTINGS allow")
            assertTrue(capabilities.snapshot().writeSettingsGranted)
            assertTrue(capabilities.setBrightness(91).isSuccess)
            assertEquals(91, capabilities.snapshot().brightness)
            assertEquals(0, Settings.System.getInt(context.contentResolver, Settings.System.SCREEN_BRIGHTNESS_MODE))
            record("brightness", "denied before grant; manual mode and value 91 read back after grant")
        } finally {
            command("settings put system screen_brightness $before")
            command("settings put system screen_brightness_mode $mode")
            command("appops set ${context.packageName} WRITE_SETTINGS ${if (permission) "allow" else "default"}")
        }
    }

    @Test fun rootProbeCompletesAndReportsARealResult() {
        val latch = CountDownLatch(1)
        var access = RootAccess.UNCHECKED
        DeviceCapabilities(context).probeRoot { access = it.rootAccess; latch.countDown() }
        assertTrue(latch.await(20, TimeUnit.SECONDS))
        assertNotEquals(RootAccess.UNCHECKED, access)
        record("appRootAccess", access.name)
    }

    @Test fun deviceAdminLockIsConfirmedByPowerStateAndThenRemoved() {
        val capabilities = DeviceCapabilities(context)
        val alreadyActive = capabilities.snapshot().deviceAdminActive
        try {
            command("dpm set-active-admin ${context.packageName}/.platform.WorkflowDeviceAdminReceiver")
            assertTrue(capabilities.snapshot().deviceAdminActive)
            val latch = CountDownLatch(1)
            var result: Result<Unit>? = null
            instrument.runOnMainSync { capabilities.lockScreen { result = it; latch.countDown() } }
            assertTrue(latch.await(5, TimeUnit.SECONDS))
            assertTrue(result?.isSuccess == true)
            assertFalse(context.getSystemService(android.os.PowerManager::class.java).isInteractive)
            record("deviceAdminLock", "granted; screen noninteractive after lockNow; grant revoked after test")
        } finally {
            command("input keyevent 224")
            command("wm dismiss-keyguard")
            if (!alreadyActive) capabilities.revokeDeviceAdmin()
        }
    }

    @Test fun grayscaleOnlyReportsSuccessWhenSystemReadbackMatches() {
        val capabilities = DeviceCapabilities(context)
        val before = capabilities.snapshot().grayscaleEnabled
        val latch = CountDownLatch(1)
        var success = false
        capabilities.setGrayscale(!before) { result -> success = result.isSuccess; latch.countDown() }
        assertTrue(latch.await(20, TimeUnit.SECONDS))
        if (success) {
            assertEquals(!before, capabilities.snapshot().grayscaleEnabled)
            val restore = CountDownLatch(1)
            capabilities.setGrayscale(before) { restore.countDown() }
            assertTrue(restore.await(20, TimeUnit.SECONDS))
            assertEquals(before, capabilities.snapshot().grayscaleEnabled)
            record("grayscale", "enabled value read back and previous value restored")
        } else {
            assertEquals(before, capabilities.snapshot().grayscaleEnabled)
            record("grayscale", "root unavailable; operation failed and system value unchanged")
        }
    }

}
