package top.flysoftbeta.workflow.feature.terminal

import android.content.ClipboardManager
import android.graphics.Rect
import android.os.Build
import android.os.SystemClock
import android.view.ActionMode
import android.view.MotionEvent
import android.view.WindowManager
import android.view.inputmethod.InputMethodManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.test.core.app.ActivityScenario
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.ui.design.SeamCallbacks
import top.flysoftbeta.workflow.ui.design.seamGestures
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicReference
import kotlin.math.abs

/**
 * Real API 28 touch input on the production WebView, hosted in Compose `AndroidView` under a parent that
 * consumes drags (as Workbench gestures would) in an adjustResize window with a real soft keyboard.
 * Offline xterm and the production Android bridge; no JavaScript gesture mocks.
 */
class TerminalTouchAcceptanceTest {
    private val instrument get() = InstrumentationRegistry.getInstrumentation()
    private val probe by lazy { TerminalDeviceProbe(instrument) }
    private class Events : TerminalPageListener {
        val ready = CountDownLatch(1)
        val urls = CopyOnWriteArrayList<String>()
        val paths = CopyOnWriteArrayList<String>()
        val inputs = CopyOnWriteArrayList<String>()
        val selection = AtomicReference("")
        @Volatile var columns = 0
        @Volatile var rows = 0
        var page: TerminalWebView? = null
        override fun onReady() { ready.countDown() }
        override fun onInput(data: String) { inputs += data }
        override fun onKey(data: String) {}
        override fun onResize(columns: Int, rows: Int) { this.columns = columns; this.rows = rows }
        override fun onOpenUrl(url: String) { urls += url }
        override fun onOpenPath(text: String) { paths += text }
        override fun onCheckLinks(requestId: Int, texts: List<String>) { page?.linksChecked(requestId, texts.map { it == "target.txt:3:2" }) }
        override fun onSelection(text: String, x: Int, y: Int) { selection.set(text) }
        override fun onGone() {}
    }
    private class Host(val page: TerminalWebView, val events: Events, val stolen: AtomicInteger, val activity: ComponentActivity) {
        /** Drags claimed by the Workbench seam beside the page, when [withTerminal] hosts one. */
        val seamDrags = AtomicInteger()
        val seamPosition = AtomicReference(Float.NaN)
    }

    @Test fun tapLinksAndFilesButScrollingDoesNotOpenThem() = withTerminal { host ->
        val page = host.page
        onMain { page.reset("https://example.com/terminal\r\ntarget.txt:3:2\r\n") }
        await("URL and file hit targets") { js(page, "document.querySelectorAll('.workflow-link').length >= 2") == "true" }
        tap(page, rect(page, "document.querySelectorAll('.workflow-link')[0]"))
        await("URL callback") { host.events.urls == listOf("https://example.com/terminal") }
        tap(page, rect(page, "document.querySelectorAll('.workflow-link')[1]"))
        await("File callback") { host.events.paths == listOf("target.txt:3:2") }
        val start = rect(page, "document.querySelectorAll('.workflow-link')[0]")
        Finger(page, start).apply { move(start.copy(y = start.y + 90)); up() }
        SystemClock.sleep(250)
        assertEquals(1, host.events.urls.size)
        assertEquals(0, host.stolen.get())
    }

    @Test fun swipeScrollsOneToOneThenFlingsAndParentGesturesCannotStealIt() = withTerminal { host ->
        val page = host.page
        onMain { page.reset((1..800).joinToString("\r\n", postfix = "\r\n") { "row $it https://example.com/$it" }) }
        await("scrollback") { max(page) > 500 }
        val cell = cellHeight(page)
        val bottom = value(page)
        // Slow, held swipe: content follows the finger row for row, without momentum.
        val from = rect(page, "document.querySelector('.xterm-screen')", yFraction = 0.25)
        Finger(page, from).apply { move(from.copy(y = from.y + 10 + cell * 12.4), steps = 20, stepMs = 25); SystemClock.sleep(200); up() }
        SystemClock.sleep(300)
        val held = value(page)
        assertTrue("1:1 swipe moved ${bottom - held} rows", abs((bottom - held) - 12) <= 1)
        // Fast swipe released while moving: momentum continues after the finger lifts.
        Finger(page, from).apply { move(from.copy(y = from.y + 300), steps = 6, stepMs = 10); up() }
        val released = value(page)
        SystemClock.sleep(700)
        val coasted = value(page)
        assertTrue("fling continued after release ($released -> $coasted)", coasted < released - 5)
        assertEquals("a Compose ancestor drag detector must not steal the swipe", 0, host.stolen.get())
        assertTrue("swipes never open links: ${host.events.urls}", host.events.urls.isEmpty())
        assertTrue("a swipe never focuses xterm", js(page, "document.activeElement === document.querySelector('.xterm-helper-textarea')") == "false")
    }

    @Test fun scrollbarThumbTracksTheFingerAcrossTheTrackWhileTheSoftKeyboardOpensAndCloses() = withTerminal { host ->
        val page = host.page
        onMain { page.reset((1..800).joinToString("\r\n", postfix = "\r\n") { "scroll row $it" }) }
        await("scrollback scrollbar") { max(page) > 500 }
        openKeyboard(host)
        val track = rect(page, "document.querySelector('.workflow-scrollbar')", yFraction = 0.0)
        val thumb = rect(page, "document.querySelector('.workflow-scroll-thumb')")
        val thumbHeight = JSONArray(js(page, "[document.querySelector('.workflow-scroll-thumb').getBoundingClientRect().height]")).getDouble(0)
        val trackHeight = JSONArray(js(page, "[document.querySelector('.workflow-scrollbar').getBoundingClientRect().height]")).getDouble(0)
        val finger = Finger(page, thumb)
        // Finger at the middle of the travel maps to the middle of the scrollback.
        finger.move(thumb.copy(y = track.y + thumbHeight / 2 + (trackHeight - thumbHeight) / 2), steps = 16)
        SystemClock.sleep(150)
        val middle = value(page).toDouble() / max(page)
        assertTrue("thumb tracks the finger 1:1 (fraction $middle)", abs(middle - 0.5) < 0.03)
        // The keyboard closes under the finger: the page grows and remaps, but the drag continues.
        val shrunk = height(page)
        closeKeyboard(host, shrunk)
        SystemClock.sleep(300)
        finger.move(track.copy(y = track.y - 40), steps = 16)
        await("thumb reached the first rows") { value(page) < max(page) * 0.02 }
        // The track is taller now; dragging past its new end reaches the newest rows.
        val grown = rect(page, "document.querySelector('.workflow-scrollbar')", yFraction = 1.0)
        finger.move(grown.copy(y = grown.y + 200), steps = 24)
        await("thumb reached the last rows") { max(page) - value(page) < max(page) * 0.02 }
        finger.up()
        assertEquals(0, host.stolen.get())
        // Streaming output does not move a reader who scrolled back.
        val top = rect(page, "document.querySelector('.workflow-scrollbar')", yFraction = 0.01)
        Finger(page, rect(page, "document.querySelector('.workflow-scroll-thumb')")).apply { move(top); up() }
        await("scrolled back") { value(page) < max(page) * 0.05 }
        val before = value(page)
        val maximum = max(page)
        onMain { page.write("new streamed output\r\n") }
        await("stream parsed") { max(page) > maximum }
        assertEquals(before, value(page))
    }

    @Test fun scrollbarDragsBesideAVerticalSeamReachTheTerminalWhileSidewaysDragsMoveTheSeam() = withTerminal(seam = true) { host ->
        val page = host.page
        onMain { page.reset((1..800).joinToString("\r\n", postfix = "\r\n") { "seam row $it" }) }
        await("scrollback scrollbar") { max(page) > 500 }
        val width = JSONArray(js(page, "[window.innerWidth]")).getDouble(0)
        // The seam lies 2dp beyond the page's right edge with a 20dp band, so the thumb's outer 4dp are inside it.
        val thumb = rect(page, "document.querySelector('.workflow-scroll-thumb')", xFraction = 40.0 / 44)
        assertTrue("thumb point ${thumb.x} of $width lies in the seam band", width + 2 - thumb.x < 10)
        val top = rect(page, "document.querySelector('.workflow-scrollbar')", yFraction = 0.0)
        val finger = Finger(page, thumb)
        // One coarse, steep first move that exceeds touch slop along both axes at once.
        finger.move(thumb.copy(x = thumb.x - 10, y = thumb.y - 30), steps = 1)
        finger.move(thumb.copy(x = thumb.x - 8, y = top.y - 40), steps = 16)
        await("scrollbar drag beside the seam reached the first rows") { value(page) < max(page) * 0.05 }
        finger.up()
        assertEquals("the seam must not claim a vertical drag", 0, host.seamDrags.get())
        // A sideways drag from the gap between the page and the container edge, at mid height, moves the seam.
        val height = JSONArray(js(page, "[window.innerHeight]")).getDouble(0)
        val gap = Point(width + 2, height / 2, width)
        Finger(page, gap).apply { move(gap.copy(x = gap.x + 60), steps = 12); up() }
        assertEquals(1, host.seamDrags.get())
        // The page starts at the seam container's origin, so container px are page CSS px times the scale.
        val scale = onMainValue { page.width } / width
        assertEquals("seam follows the finger", ((gap.x + 60) * scale).toFloat(), host.seamPosition.get(), (2 * scale).toFloat())
        // A sideways drag that starts on the thumb inside the band still resizes.
        val before = value(page)
        val outer = rect(page, "document.querySelector('.workflow-scroll-thumb')", xFraction = 40.0 / 44)
        Finger(page, outer).apply { move(outer.copy(x = outer.x - 60), steps = 12); up() }
        assertEquals(2, host.seamDrags.get())
        assertEquals(before, value(page))
        assertEquals(0, host.stolen.get())
    }

    @Test fun linkTapsOpenWhileOutputStreamsIncludingHardWrappedAndOsc8Links() = withTerminal { host ->
        val page = host.page
        val events = host.events
        awaitStableGeometry(events)
        val head = "https://example.com/wrapped/"
        val wrapped = head + "x".repeat(events.columns - head.length) + "/continued?q=1"
        val first = wrapped.substring(0, events.columns)
        val rest = wrapped.substring(events.columns)
        onMain {
            page.reset((1..60).joinToString("\r\n", postfix = "\r\n") { "line $it" } +
                "\u001b]8;;https://example.com/osc\u0007open docs\u001b]8;;\u0007\r\n$first\r\n$rest tail\r\nhttps://example.com/streaming\r\nprompt")
        }
        await("links drawn") { js(page, "document.querySelectorAll('button.workflow-link').length >= 3") == "true" }
        // Output arrives between press and release; the viewport follows it and the link moves under the finger.
        val streaming = rect(page, "document.querySelector('button.workflow-link[aria-label=\"Open https://example.com/streaming\"]')")
        val finger = Finger(page, streaming)
        val before = max(page)
        onMain { page.write("\r\nstreamed while pressed") }
        await("viewport followed the stream") { value(page) > before - 1 && max(page) > before }
        SystemClock.sleep(60)
        finger.up()
        await("streaming tap opened") { events.urls.contains("https://example.com/streaming") }
        tap(page, rect(page, "document.querySelector('button.workflow-link[aria-label=\"Open https://example.com/osc\"]')"))
        await("OSC 8 target opened") { events.urls.contains("https://example.com/osc") }
        val continuation = "Array.from(document.querySelectorAll('span.workflow-link')).find(n => n._link && n._link.text === ${JSONObject.quote(wrapped)})"
        await("continuation row is part of the link") { js(page, "!!($continuation)") == "true" }
        tap(page, rect(page, continuation))
        await("wrapped URL opened whole") { events.urls.contains(wrapped) }
        assertEquals(3, events.urls.size)
    }

    @Test fun longPressSelectsWithPlatformHandlesAndFloatingToolbarWithoutOpeningTheKeyboard() = withTerminal { host ->
        val page = host.page
        val events = host.events
        val handleColor = android.graphics.Color.parseColor("#C2185B")
        onMain { page.theme(JSONObject(), "#C2185B"); page.reset("alpha beta gamma delta\r\nsecond line words\r\n") }
        awaitStableGeometry(events)
        val initialHeight = height(page)
        val origin = cellPoint(page, events, 2.5, 0.5)
        Finger(page, origin).apply { SystemClock.sleep(700); up() }
        await("long press selection") { events.selection.get() == "alpha" }
        SystemClock.sleep(1000)
        assertEquals("a long press must not open the soft keyboard", initialHeight, height(page))
        assertEquals("false", js(page, "document.activeElement === document.querySelector('.xterm-helper-textarea')"))
        assertPlatformChrome(page, handleColor)
        // Drag the native end handle to the right: the selection grows by whole cells.
        val cells = cellWidth(page, events)
        val end = handleCentre(page, 1)
        Finger(page, end).apply { move(end.copy(x = end.x + cells * 11)); SystemClock.sleep(100); up() }
        await("end handle expanded selection") { events.selection.get() == "alpha beta gamma" }
        val start = handleCentre(page, 0)
        Finger(page, start).apply { move(start.copy(x = start.x + cells * 6)); SystemClock.sleep(100); up() }
        await("start handle shrank selection") { events.selection.get() == "beta gamma" }
        await("toolbar returns after a handle drag") { onMainValue { page.selectionChrome.actionMode } != null }
        probe.tapToolbarItem(page.context.getString(android.R.string.copy))
        await("copied to the system clipboard") {
            onMainValue { page.context.getSystemService(ClipboardManager::class.java).primaryClip?.getItemAt(0)?.text?.toString() } == "beta gamma"
        }
        await("copy clears the selection and its chrome") {
            onMainValue { page.selectionChrome.actionMode == null && page.selectionChrome.handleBounds().isEmpty() }
        }
        // With the keyboard open, select, then close the keyboard: a rows-only resize keeps the selection.
        openKeyboard(host)
        Finger(page, cellPoint(page, events, 8.5, 1.5)).apply { SystemClock.sleep(700); up() }
        await("selection with keyboard open") { events.selection.get() == "line" }
        closeKeyboard(host, height(page))
        SystemClock.sleep(500)
        assertEquals("line", events.selection.get())
        assertEquals("true", js(page, "Array.from(document.querySelectorAll('.workflow-selection-handle')).every(x => !x.hidden)"))
        assertPlatformChrome(page, handleColor)
        probe.tapToolbarItem(page.context.getString(android.R.string.selectAll))
        await("select all") { events.selection.get().contains("alpha beta gamma delta") && events.selection.get().contains("second line words") }
    }

    private fun assertPlatformChrome(page: TerminalWebView, color: Int) {
        await("floating toolbar") { onMainValue { page.selectionChrome.actionMode?.type } == ActionMode.TYPE_FLOATING }
        await("two platform handles") { onMainValue { page.selectionChrome.handleBounds().size } == 2 }
        SystemClock.sleep(300)
        val location = IntArray(2)
        val bounds = onMainValue { page.getLocationOnScreen(location); page.selectionChrome.handleBounds() }
        val screenshot = probe.screenshot()
        bounds.forEach { local ->
            val screen = Rect(local).apply { offset(location[0], location[1]) }
            assertTrue("teardrop handle drawn at $screen", probe.hasColor(screenshot, screen, color))
        }
        assertNotNull("platform Copy item", probe.toolbarItem(page.context.getString(android.R.string.copy)))
    }

    /** Asks the IME to hide until the adjustResize window grows back; a single request can race the IME. */
    private fun closeKeyboard(host: Host, withKeyboard: Int) = await("soft keyboard closed") {
        onMain { host.activity.getSystemService(InputMethodManager::class.java).hideSoftInputFromWindow(host.page.windowToken, 0) }
        SystemClock.sleep(300)
        height(host.page) > withKeyboard
    }

    /** The page reports its fitted size asynchronously; wait until it stops changing. */
    private fun awaitStableGeometry(events: Events) {
        await("terminal geometry") { events.columns > 30 && events.rows > 2 }
        var size = events.columns to events.rows
        await("stable terminal geometry") {
            SystemClock.sleep(400)
            val next = events.columns to events.rows
            (next == size).also { size = next }
        }
    }

    private fun openKeyboard(host: Host) {
        val page = host.page
        val before = height(page)
        // A real tap on the terminal focuses xterm; the soft keyboard resizes the adjustResize window.
        tap(page, rect(page, "document.querySelector('.xterm-screen')", xFraction = 0.6, yFraction = 0.9))
        await("soft keyboard resized the terminal") { height(page) in 1 until before }
        SystemClock.sleep(400)
    }

    private data class Point(val x: Double, val y: Double, val width: Double)
    /** One finger of real injected input, in CSS px of the page. */
    private inner class Finger(private val page: TerminalWebView, start: Point) {
        private val down = SystemClock.uptimeMillis()
        private val location = IntArray(2)
        private var scale = 1.0
        private var last = start
        init {
            onMain { page.getLocationOnScreen(location); scale = page.width / start.width }
            send(MotionEvent.ACTION_DOWN, start)
        }
        fun move(to: Point, steps: Int = 12, stepMs: Long = 16) {
            val from = last
            for (step in 1..steps) {
                send(MotionEvent.ACTION_MOVE, from.copy(x = from.x + (to.x - from.x) * step / steps, y = from.y + (to.y - from.y) * step / steps))
                SystemClock.sleep(stepMs)
            }
        }
        fun up() = send(MotionEvent.ACTION_UP, last)
        private fun send(action: Int, point: Point) {
            last = point
            val event = MotionEvent.obtain(down, SystemClock.uptimeMillis(), action,
                (location[0] + point.x * scale).toFloat(), (location[1] + point.y * scale).toFloat(), 0)
            instrument.sendPointerSync(event)
            event.recycle()
        }
    }
    private fun tap(page: TerminalWebView, at: Point) = Finger(page, at).apply { SystemClock.sleep(60); up() }

    private fun rect(page: TerminalWebView, element: String, xFraction: Double = .5, yFraction: Double = .5): Point {
        val a = JSONArray(js(page, "(() => { const r=($element).getBoundingClientRect(); return [r.left+r.width*$xFraction,r.top+r.height*$yFraction,window.innerWidth]; })()"))
        return Point(a.getDouble(0), a.getDouble(1), a.getDouble(2))
    }
    private fun cellPoint(page: TerminalWebView, events: Events, column: Double, row: Double) =
        rect(page, "document.querySelector('.xterm-screen')", xFraction = column / events.columns, yFraction = row / events.rows)
    private fun cellWidth(page: TerminalWebView, events: Events) =
        JSONArray(js(page, "[document.querySelector('.xterm-screen').getBoundingClientRect().width]")).getDouble(0) / events.columns
    private fun cellHeight(page: TerminalWebView) =
        JSONArray(js(page, "[document.querySelector('.xterm-screen').getBoundingClientRect().height]")).getDouble(0) / events(page).rows
    private fun events(page: TerminalWebView) = hosts.getValue(page)
    private val hosts = HashMap<TerminalWebView, Events>()
    /** The centre of a platform handle's drawable, in CSS px. */
    private fun handleCentre(page: TerminalWebView, index: Int): Point {
        val width = JSONArray(js(page, "[window.innerWidth]")).getDouble(0)
        val (bounds, viewWidth) = onMainValue { page.selectionChrome.handleBounds()[index] to page.width }
        val scale = viewWidth / width
        return Point(bounds.exactCenterX() / scale, bounds.exactCenterY() / scale, width)
    }
    private fun value(page: TerminalWebView) = js(page, "Number(document.querySelector('.workflow-scrollbar').getAttribute('aria-valuenow'))").toDouble().toInt()
    private fun max(page: TerminalWebView) = js(page, "Number(document.querySelector('.workflow-scrollbar').getAttribute('aria-valuemax'))").toDouble().toInt()
    private fun height(page: TerminalWebView) = onMainValue { page.height }

    /** With [seam], a vertical Workbench seam (the production detector) lies 2dp beyond the page's right edge. */
    private fun withTerminal(seam: Boolean = false, test: (Host) -> Unit) {
        check(Build.HARDWARE in setOf("ranchu", "goldfish"))
        // The disposable AVD has a hardware keyboard; show the soft keyboard anyway so IME resizes are real.
        val imeSetting = probe.shell("settings get secure show_ime_with_hard_keyboard")
        probe.shell("settings put secure show_ime_with_hard_keyboard 1")
        val events = Events()
        val stolen = AtomicInteger()
        try {
            ActivityScenario.launch(ComponentActivity::class.java).use { scenario ->
                lateinit var host: Host
                scenario.onActivity { activity ->
                    activity.window.setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE)
                    val page = TerminalWebView(activity, events)
                    events.page = page
                    hosts[page] = events
                    host = Host(page, events, stolen, activity)
                    activity.setContent {
                        Box(Modifier.fillMaxSize().pointerInput(Unit) {
                            detectDragGestures { change, _ -> change.consume(); stolen.incrementAndGet() }
                        }) {
                            if (!seam) AndroidView(factory = { page }, modifier = Modifier.fillMaxSize())
                            else {
                                val density = LocalDensity.current
                                val width = IntArray(1)
                                Box(Modifier.fillMaxSize().onSizeChanged { width[0] = it.width }
                                    .seamGestures(horizontal = true, bandPx = with(density) { 20.dp.toPx() }, callbacks = object : SeamCallbacks {
                                        override fun seams() = listOf(width[0] - with(density) { 22.dp.toPx() })
                                        override fun onDragStart(index: Int) { host.seamDrags.incrementAndGet() }
                                        override fun onDrag(index: Int, position: Float) { host.seamPosition.set(position) }
                                    })) {
                                    AndroidView(factory = { page }, modifier = Modifier.fillMaxSize().padding(end = 24.dp))
                                }
                            }
                        }
                    }
                }
                assertTrue(events.ready.await(30, TimeUnit.SECONDS))
                try { test(host) } finally { onMain { host.page.release() } }
            }
        } finally {
            probe.shell("settings put secure show_ime_with_hard_keyboard ${imeSetting.takeIf { it == "1" } ?: "0"}")
        }
    }
    private fun js(page: TerminalWebView, expression: String): String {
        val latch = CountDownLatch(1)
        val result = AtomicReference<String>()
        onMain { page.evaluateJavascript(expression) { result.set(it); latch.countDown() } }
        check(latch.await(5, TimeUnit.SECONDS))
        return result.get()
    }
    private fun await(message: String, condition: () -> Boolean) {
        repeat(150) { if (condition()) return; SystemClock.sleep(100) }
        fail(message)
    }
    private fun onMain(block: () -> Unit) = instrument.runOnMainSync(block)
    private fun <T> onMainValue(block: () -> T): T {
        val result = AtomicReference<T>()
        instrument.runOnMainSync { result.set(block()) }
        return result.get()
    }
}
