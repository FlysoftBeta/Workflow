package top.flysoftbeta.workflow.feature.terminal

import android.os.Build
import android.os.SystemClock
import android.view.MotionEvent
import android.view.ViewGroup
import androidx.activity.ComponentActivity
import androidx.test.core.app.ActivityScenario
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONArray
import org.junit.Assert.*
import org.junit.Test
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference

/** Real API 28 WebView touch dispatch, offline xterm, and production Android bridge. No JS gesture mocks. */
class TerminalTouchAcceptanceTest {
    private val instrument get() = InstrumentationRegistry.getInstrumentation()
    private class Events : TerminalPageListener {
        val ready = CountDownLatch(1)
        val urls = CopyOnWriteArrayList<String>()
        val paths = CopyOnWriteArrayList<String>()
        val selection = AtomicReference("")
        @Volatile var columns = 0
        @Volatile var rows = 0
        var page: TerminalWebView? = null
        override fun onReady() { ready.countDown() }
        override fun onInput(data: String) {}
        override fun onKey(data: String) {}
        override fun onResize(columns: Int, rows: Int) { this.columns = columns; this.rows = rows }
        override fun onOpenUrl(url: String) { urls += url }
        override fun onOpenPath(text: String) { paths += text }
        override fun onCheckLinks(requestId: Int, texts: List<String>) { page?.linksChecked(requestId, texts.map { it == "target.txt:3:2" }) }
        override fun onSelection(text: String, x: Int, y: Int) { selection.set(text) }
        override fun onGone() {}
    }
    @Test fun tapLinksAndFilesButScrollingDoesNotOpenThem() = withTerminal { page, events ->
        onMain { page.reset("https://example.com/terminal\r\ntarget.txt:3:2\r\n") }
        await("URL and file hit targets") { js(page, "document.querySelectorAll('.workflow-link').length >= 2") == "true" }
        touch(page, rect(page, "document.querySelectorAll('.workflow-link')[0]"))
        await("URL callback") { events.urls == listOf("https://example.com/terminal") }
        touch(page, rect(page, "document.querySelectorAll('.workflow-link')[1]"))
        await("File callback") { events.paths == listOf("target.txt:3:2") }
        val start = rect(page, "document.querySelectorAll('.workflow-link')[0]")
        touch(page, start, start.copy(y = start.y + 90), holdMs = 80)
        SystemClock.sleep(250)
        assertEquals(1, events.urls.size)
    }
    @Test fun largeScrollbarDragsFastAndPreservesViewportWhileOutputStreams() = withTerminal { page, _ ->
        onMain { page.reset((1..800).joinToString("\r\n", postfix = "\r\n") { "scroll row $it" }) }
        await("scrollback scrollbar") { js(page, "Number(document.querySelector('.workflow-scrollbar').getAttribute('aria-valuemax')) > 500") == "true" }
        val thumb = rect(page, "document.querySelector('.workflow-scroll-thumb')")
        val top = rect(page, "document.querySelector('.workflow-scrollbar')", yFraction = 0.02)
        touch(page, thumb, top, holdMs = 100)
        await("thumb reached first rows") { js(page, "Number(document.querySelector('.workflow-scrollbar').getAttribute('aria-valuenow')) < 40") == "true" }
        val before = js(page, "Number(document.querySelector('.workflow-scrollbar').getAttribute('aria-valuenow'))").toInt()
        onMain { page.write("new streamed output\r\n") }
        SystemClock.sleep(200)
        assertEquals(before, js(page, "Number(document.querySelector('.workflow-scrollbar').getAttribute('aria-valuenow'))").toInt())
        val end = rect(page, "document.querySelector('.workflow-scrollbar')", yFraction = 0.98)
        touch(page, rect(page, "document.querySelector('.workflow-scroll-thumb')"), end, holdMs = 100)
        await("thumb reached final rows") { js(page, "Number(document.querySelector('.workflow-scrollbar').getAttribute('aria-valuemax')) - Number(document.querySelector('.workflow-scrollbar').getAttribute('aria-valuenow')) < 40") == "true" }
    }
    @Test fun longPressShowsTwoHandlesAndEachCanChangeTheCopiedSelection() = withTerminal { page, events ->
        onMain { page.reset("alpha beta gamma delta\r\n") }
        await("terminal geometry") { events.columns > 10 && events.rows > 1 }
        SystemClock.sleep(150)
        val origin = rect(page, "document.querySelector('.xterm-screen')", xFraction = 2.5 / events.columns, yFraction = 0.5 / events.rows)
        touch(page, origin, holdMs = 650)
        await("long press selection") { events.selection.get().contains("alpha") }
        await("two visible handles") { js(page, "Array.from(document.querySelectorAll('.workflow-selection-handle')).filter(x => !x.hidden).length === 2") == "true" }
        val initial = events.selection.get()
        val screen = JSONArray(js(page, "[document.querySelector('.xterm-screen').getBoundingClientRect().width]"))
        val cells = screen.getDouble(0) / events.columns
        val end = rect(page, "document.querySelector('.workflow-selection-handle.end')")
        touch(page, end, end.copy(x = end.x + cells * 11), holdMs = 100)
        await("end handle expanded selection") { events.selection.get().length > initial.length }
        val expanded = events.selection.get()
        val start = rect(page, "document.querySelector('.workflow-selection-handle.start')")
        touch(page, start, start.copy(x = start.x + cells * 6), holdMs = 100)
        await("start handle changed selection") { events.selection.get().isNotEmpty() && events.selection.get() != expanded }
        assertFalse(events.selection.get().startsWith("alpha"))
        onMain { page.clearSelection() }
        await("clear hides handles") { js(page, "Array.from(document.querySelectorAll('.workflow-selection-handle')).every(x => x.hidden)") == "true" }
    }
    private data class Point(val x: Double, val y: Double, val width: Double)
    private fun rect(page: TerminalWebView, element: String, xFraction: Double = .5, yFraction: Double = .5): Point {
        val a = JSONArray(js(page, "(() => { const r=($element).getBoundingClientRect(); return [r.left+r.width*$xFraction,r.top+r.height*$yFraction,window.innerWidth]; })()"))
        return Point(a.getDouble(0), a.getDouble(1), a.getDouble(2))
    }
    private fun touch(page: TerminalWebView, from: Point, to: Point = from, holdMs: Long = 60) {
        val location = IntArray(2)
        var scale = 1.0
        onMain { page.getLocationOnScreen(location); scale = page.width / from.width }
        val down = SystemClock.uptimeMillis()
        fun send(action: Int, point: Point) {
            val event = MotionEvent.obtain(down, SystemClock.uptimeMillis(), action,
                (location[0] + point.x * scale).toFloat(), (location[1] + point.y * scale).toFloat(), 0)
            instrument.sendPointerSync(event); event.recycle()
        }
        send(MotionEvent.ACTION_DOWN, from)
        SystemClock.sleep(holdMs)
        if (from != to) for (step in 1..12) {
            send(MotionEvent.ACTION_MOVE, from.copy(x = from.x + (to.x - from.x) * step / 12, y = from.y + (to.y - from.y) * step / 12))
            SystemClock.sleep(16)
        }
        send(MotionEvent.ACTION_UP, to)
    }
    private fun withTerminal(test: (TerminalWebView, Events) -> Unit) {
        check(Build.HARDWARE in setOf("ranchu", "goldfish"))
        val events = Events()
        ActivityScenario.launch(ComponentActivity::class.java).use { scenario ->
            lateinit var page: TerminalWebView
            scenario.onActivity { activity ->
                page = TerminalWebView(activity, events)
                events.page = page
                activity.setContentView(page, ViewGroup.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT))
            }
            assertTrue(events.ready.await(30, TimeUnit.SECONDS))
            try { test(page, events) } finally { onMain { page.release() } }
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
}
