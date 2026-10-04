package top.flysoftbeta.workflow.feature.terminal

import android.accessibilityservice.AccessibilityServiceInfo
import android.app.Instrumentation
import android.graphics.Bitmap
import android.graphics.Color
import android.graphics.Rect
import android.os.ParcelFileDescriptor
import android.os.SystemClock
import android.view.MotionEvent
import org.junit.Assert.fail
import kotlin.math.abs

/** Real platform surfaces for terminal acceptance: the floating toolbar window, shell settings and screen pixels. */
internal class TerminalDeviceProbe(private val instrument: Instrumentation) {
    private val automation by lazy {
        instrument.uiAutomation.also {
            it.serviceInfo = it.serviceInfo.apply { flags = flags or AccessibilityServiceInfo.FLAG_RETRIEVE_INTERACTIVE_WINDOWS }
        }
    }

    fun shell(command: String): String =
        ParcelFileDescriptor.AutoCloseInputStream(automation.executeShellCommand(command)).bufferedReader().use { it.readText().trim() }

    /** Screen bounds of a visible floating-toolbar item with [label], or null. */
    fun toolbarItem(label: String): Rect? = automation.windows.asSequence()
        .mapNotNull { it.root }
        .flatMap { it.findAccessibilityNodeInfosByText(label).asSequence() }
        .firstOrNull { it.isVisibleToUser && it.text?.toString() == label }
        ?.let { node -> Rect().also(node::getBoundsInScreen) }

    /** Taps the item with real injected input, as a finger on the platform toolbar would. */
    fun tapToolbarItem(label: String) {
        val deadline = SystemClock.uptimeMillis() + 10_000
        var bounds = toolbarItem(label)
        while (bounds == null && SystemClock.uptimeMillis() < deadline) {
            SystemClock.sleep(100)
            bounds = toolbarItem(label)
        }
        if (bounds == null) fail("Floating toolbar item '$label' is not visible")
        else tapScreen(bounds.exactCenterX(), bounds.exactCenterY())
    }

    fun tapScreen(x: Float, y: Float) {
        val down = SystemClock.uptimeMillis()
        listOf(MotionEvent.ACTION_DOWN, MotionEvent.ACTION_UP).forEach { action ->
            val event = MotionEvent.obtain(down, SystemClock.uptimeMillis(), action, x, y, 0)
            instrument.sendPointerSync(event)
            event.recycle()
            SystemClock.sleep(40)
        }
    }

    fun screenshot(): Bitmap = automation.takeScreenshot()

    /** True when some sampled pixel inside [screen] (screen coordinates) is close to [color]. */
    fun hasColor(bitmap: Bitmap, screen: Rect, color: Int, tolerance: Int = 40): Boolean {
        for (fy in listOf(0.35f, 0.5f, 0.65f, 0.8f)) for (fx in listOf(0.25f, 0.4f, 0.5f, 0.6f, 0.75f)) {
            val x = (screen.left + screen.width() * fx).toInt().coerceIn(0, bitmap.width - 1)
            val y = (screen.top + screen.height() * fy).toInt().coerceIn(0, bitmap.height - 1)
            val pixel = bitmap.getPixel(x, y)
            if (abs(Color.red(pixel) - Color.red(color)) <= tolerance && abs(Color.green(pixel) - Color.green(color)) <= tolerance &&
                abs(Color.blue(pixel) - Color.blue(color)) <= tolerance) return true
        }
        return false
    }
}
