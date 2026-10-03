package top.flysoftbeta.workflow.platform.device

import android.accessibilityservice.AccessibilityServiceInfo
import android.os.Build
import android.os.SystemClock
import android.view.accessibility.AccessibilityNodeInfo
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import kotlinx.coroutines.flow.first
import top.flysoftbeta.workflow.platform.connection.ConnectionStatus
import org.junit.*
import org.junit.Assert.*
import org.junit.Assume.assumeTrue
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.app.AppGraph
import top.flysoftbeta.workflow.platform.WorkflowOverlayService
import top.flysoftbeta.workflow.platform.DeviceCapabilities
import top.flysoftbeta.workflow.platform.RootAccess
import top.flysoftbeta.workflow.platform.connection.WorkspaceConnectionManager
import top.flysoftbeta.workflow.platform.clientservices.WorkspaceDocuments

/** Uses Android's actual overlay AppOp, service/window lifecycle and accessibility tree. Emulator only. */
@RunWith(AndroidJUnit4::class)
class OverlayEmulatorTest {
    private val instrument get() = InstrumentationRegistry.getInstrumentation()
    private val context get() = instrument.targetContext
    private val store get() = AppGraph.workspaceStore(context)
    private var admitted = false
    private var priorEnabled = false
    private var priorPermission = false
    private fun shell(command: String) { instrument.uiAutomation.executeShellCommand(command).use { android.os.ParcelFileDescriptor.AutoCloseInputStream(it).readBytes() } }
    private fun await(predicate: () -> Boolean) { val end = SystemClock.uptimeMillis() + 10_000; while (!predicate() && SystemClock.uptimeMillis() < end) SystemClock.sleep(100); assertTrue(predicate()) }
    private fun node(matches: (AccessibilityNodeInfo) -> Boolean): AccessibilityNodeInfo? {
        fun visit(current: AccessibilityNodeInfo?): AccessibilityNodeInfo? {
            current ?: return null
            if (matches(current)) return current
            for (index in 0 until current.childCount) visit(current.getChild(index))?.let { return it }
            return null
        }
        return instrument.uiAutomation.windows.firstNotNullOfOrNull { visit(it.root) }
    }
    private fun bubble() = node { it.contentDescription?.startsWith("Workflow 快捷控制") == true }
    /** Measure the WindowManager-backed window: API 28 can retain old node bounds after rotation. */
    private fun settledBubbleBounds(): android.graphics.Rect {
        instrument.uiAutomation.waitForIdle(750, 10_000)
        val end = SystemClock.uptimeMillis() + 10_000
        var previous: android.graphics.Rect? = null
        var stableSince = SystemClock.uptimeMillis()
        while (SystemClock.uptimeMillis() < end) {
            val windowId = bubble()?.windowId
            val current = instrument.uiAutomation.windows.firstOrNull { it.id == windowId }
                ?.let { window -> android.graphics.Rect().also(window::getBoundsInScreen) }
            val now = SystemClock.uptimeMillis()
            if (current == null || current != previous) {
                previous = current
                stableSince = now
            } else if (now - stableSince >= 750) return current
            SystemClock.sleep(100)
        }
        throw AssertionError("Overlay geometry did not settle: $previous; service=${WorkflowOverlayService.failure.value}")
    }
    private fun expanded() = node { it.text?.toString() == "黑白" } != null
    private fun screenshot(name: String) {
        SystemClock.sleep(350)
        val output = java.io.File(context.filesDir, "w8-acceptance/$name.png")
        output.parentFile!!.mkdirs()
        output.outputStream().use { instrument.uiAutomation.takeScreenshot().compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it) }
    }

    @Before fun setup() {
        assumeTrue(InstrumentationRegistry.getArguments().getString("settingsEmulator") == "true" && Build.HARDWARE in listOf("ranchu", "goldfish"))
        admitted = true
        instrument.uiAutomation.serviceInfo = instrument.uiAutomation.serviceInfo.apply { flags = flags or AccessibilityServiceInfo.FLAG_RETRIEVE_INTERACTIVE_WINDOWS }
        runBlocking {
            val manager = WorkspaceConnectionManager.get(context)
            withTimeout(60_000) { manager.status.first { it !is ConnectionStatus.Loading } }
            if (manager.sessionOrNull() == null) manager.useEmbedded()
            val connected = withTimeout(60_000) { manager.status.first { it is ConnectionStatus.Connected || it is ConnectionStatus.Failed } }
            check(connected is ConnectionStatus.Connected) { "Fixture workspace failed: $connected" }
            store.awaitReady(); priorEnabled = store.state.value.config.overlay.enabled
        }
        priorPermission = android.provider.Settings.canDrawOverlays(context)
        shell("input keyevent 224"); shell("wm dismiss-keyguard")
        shell("appops set ${context.packageName} SYSTEM_ALERT_WINDOW allow")
        runBlocking { store.updateConfig { it.copy(overlay = it.overlay.copy(enabled = true)) } }
        instrument.runOnMainSync { assertTrue(WorkflowOverlayService.start(context).isSuccess) }
        await { WorkflowOverlayService.isRunning && bubble() != null }
    }
    @After fun cleanup() {
        if (!admitted) return
        instrument.runOnMainSync { WorkflowOverlayService.stop(context) }
        runBlocking { store.updateConfig { it.copy(overlay = it.overlay.copy(enabled = priorEnabled)) } }
        shell("appops set ${context.packageName} SYSTEM_ALERT_WINDOW ${if (priorPermission) "allow" else "default"}")
    }

    @Test fun backHomeAndOutsideDismissExpandedPanel() {
        bubble()!!.performAction(AccessibilityNodeInfo.ACTION_CLICK)
        await(::expanded)
        screenshot("overlay-expanded")
        shell("input keyevent 4")
        await { !expanded() && bubble() != null }
        bubble()!!.performAction(AccessibilityNodeInfo.ACTION_CLICK)
        await(::expanded)
        shell("input keyevent 3")
        await { !expanded() && bubble() != null }
        bubble()!!.performAction(AccessibilityNodeInfo.ACTION_CLICK)
        await(::expanded)
        shell("input tap 80 100")
        await { !expanded() && bubble() != null }
        shell("input keyevent 223")
        await { bubble() == null }
        shell("input keyevent 224")
        shell("wm dismiss-keyguard")
        await { bubble() != null }
    }

    @Test fun permissionRevocationStopsTheRealService() {
        shell("appops set ${context.packageName} SYSTEM_ALERT_WINDOW deny")
        await { !WorkflowOverlayService.isRunning }
        assertFalse(WorkflowOverlayService.start(context).isSuccess)
    }

    @Test fun draggingSnapsPersistsAndSurvivesRotationAndServiceRestart() {
        val display = context.getSystemService(android.hardware.display.DisplayManager::class.java).getDisplay(android.view.Display.DEFAULT_DISPLAY)
        val oldRotation = display.rotation
        val oldAuto = android.provider.Settings.System.getInt(context.contentResolver, "accelerometer_rotation", 1)
        val oldUser = android.provider.Settings.System.getInt(context.contentResolver, "user_rotation", 0)
        val manager = WorkspaceConnectionManager.get(context)
        val profile = manager.requireSession().profile
        var documents = WorkspaceDocuments(manager.requireSession().rpc)
        fun position() = runBlocking { documents.read("overlay", "position").text?.let { org.json.JSONObject(it) } }
        try {
            val screen = android.graphics.Point().also(display::getRealSize)
            val bounds = android.graphics.Rect().also { bubble()!!.getBoundsInScreen(it) }
            shell("input swipe ${bounds.centerX()} ${bounds.centerY()} 80 ${screen.y * 3 / 4} 500")
            await { position()?.optBoolean("right", true) == false }
            val fraction = position()!!.getDouble("verticalFraction")
            assertTrue(fraction in 0.0..1.0)
            await { android.graphics.Rect().also { bubble()!!.getBoundsInScreen(it) }.left <= 0 }
            screenshot("overlay-dragged-left")
            shell("settings put system accelerometer_rotation 0")
            shell("settings put system user_rotation ${(oldRotation + 1) % 4}")
            await { display.rotation != oldRotation }
            val rotated = settledBubbleBounds()
            assertTrue(rotated.left <= 0)
            assertTrue(rotated.top > 0)
            assertEquals(fraction, position()!!.getDouble("verticalFraction"), .001)
            screenshot("overlay-rotation")
            instrument.runOnMainSync { WorkflowOverlayService.stop(context) }
            await { !WorkflowOverlayService.isRunning }
            instrument.runOnMainSync { assertTrue(WorkflowOverlayService.start(context).isSuccess) }
            await { WorkflowOverlayService.isRunning && bubble() != null }
            val restored = settledBubbleBounds()
            assertTrue("Restart moved overlay: before=$rotated after=$restored; saved=${position()}; service=${WorkflowOverlayService.failure.value}",
                restored.left <= 0 && kotlin.math.abs(restored.top - rotated.top) < 8)
            // The overlay belongs to this Engine connection. Reconnect reads the same persisted
            // document from a new Rust process rather than retaining Android's projection.
            manager.disconnect()
            await { !WorkflowOverlayService.isRunning }
            runBlocking {
                manager.connect(profile)
                val connected = withTimeout(60_000) { manager.status.first { it is ConnectionStatus.Connected || it is ConnectionStatus.Failed } }
                check(connected is ConnectionStatus.Connected) { "Engine reconnect failed: $connected" }
            }
            documents = WorkspaceDocuments(manager.requireSession().rpc)
            assertEquals(fraction, position()!!.getDouble("verticalFraction"), .001)
            assertFalse(position()!!.getBoolean("right"))
            instrument.runOnMainSync { assertTrue(WorkflowOverlayService.start(context).isSuccess) }
            await { WorkflowOverlayService.isRunning && bubble() != null }
        } finally {
            shell("settings put system user_rotation $oldUser")
            shell("settings put system accelerometer_rotation $oldAuto")
            instrument.runOnMainSync { WorkflowOverlayService.resetPosition(context) }
        }
    }

    @Test fun aRevokedLockGrantReopensThePanelWithAnInlineFailure() {
        val capabilities = DeviceCapabilities(context)
        val probe = java.util.concurrent.CountDownLatch(1)
        var root = RootAccess.UNCHECKED
        capabilities.probeRoot { root = it.rootAccess; probe.countDown() }
        assertTrue(probe.await(20, java.util.concurrent.TimeUnit.SECONDS))
        assumeTrue("This denial-path case needs an emulator without app su access", root == RootAccess.UNAVAILABLE)
        shell("dpm set-active-admin ${context.packageName}/.platform.WorkflowDeviceAdminReceiver")
        assertTrue(capabilities.snapshot().deviceAdminActive)
        bubble()!!.performAction(AccessibilityNodeInfo.ACTION_CLICK)
        await(::expanded)
        assertTrue(capabilities.revokeDeviceAdmin().isSuccess)
        await { !capabilities.snapshot().deviceAdminActive }
        var lock = node { it.text?.toString() == "锁屏" }
        while (lock != null && !lock.isClickable) lock = lock.parent
        assertNotNull("Lock's actionable accessibility node", lock)
        assertTrue(lock!!.performAction(AccessibilityNodeInfo.ACTION_CLICK))
        await { expanded() && node { it.contentDescription?.startsWith("操作失败：") == true } != null }
        assertTrue(context.getSystemService(android.os.PowerManager::class.java).isInteractive)
        screenshot("overlay-lock-failure-inline")
        SystemClock.sleep(2_200)
        assertNull(node { it.contentDescription?.startsWith("操作失败：") == true })
    }
}
