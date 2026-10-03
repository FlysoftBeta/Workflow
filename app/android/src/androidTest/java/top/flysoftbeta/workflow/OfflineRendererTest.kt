package top.flysoftbeta.workflow

import android.view.ViewGroup
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.Text
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.test.core.app.ActivityScenario
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.feature.terminal.TerminalPageListener
import top.flysoftbeta.workflow.feature.terminal.TerminalWebView
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference

/** Real production xterm adapter on Android 9; chat math/Markdown is covered by NativeTranscriptTest. */
@RunWith(AndroidJUnit4::class)
class OfflineRendererTest {
    private class Events : TerminalPageListener {
        val ready = CountDownLatch(1)
        val inputs = CopyOnWriteArrayList<String>()
        val selection = AtomicReference("")
        @Volatile var rows = 0
        override fun onReady() { ready.countDown() }
        override fun onInput(data: String) { inputs += data }
        override fun onKey(data: String) {}
        override fun onResize(columns: Int, rows: Int) { this.rows = rows }
        override fun onOpenUrl(url: String) {}
        override fun onOpenPath(text: String) {}
        override fun onCheckLinks(requestId: Int, texts: List<String>) {}
        override fun onSelection(text: String, x: Int, y: Int) { selection.set(text) }
        override fun onGone() {}
    }

    @Test fun xtermReceivesIncrementalPtyOutput() = withTerminal { view, events ->
        onMain { view.write("hello PTY\r\n") }
        awaitSelection(view, events) { it.contains("hello PTY") }
        onMain { view.write("second line\r\n") }
        awaitSelection(view, events) { it.contains("hello PTY\nsecond line") }
    }

    @Test fun androidImeBurstPreservesRepeatedLettersAndEnterOrdering() = withTerminal { view, events ->
        awaitJs(view, "!!document.querySelector('.xterm-helper-textarea')")
        onMain { view.evaluateJavascript("""
            (() => {
              const target = document.querySelector('.xterm-helper-textarea');
              target.value = '';
              for (const character of 'bookkeeper') {
                target.dispatchEvent(new KeyboardEvent('keydown', {keyCode:229, which:229, bubbles:true, composed:true}));
                target.value += character;
                target.dispatchEvent(new InputEvent('input', {data:character, inputType:'insertText', bubbles:true, composed:true}));
                target.dispatchEvent(new KeyboardEvent('keyup', {keyCode:229, which:229, bubbles:true, composed:true}));
              }
              target.dispatchEvent(new KeyboardEvent('keydown', {key:'Enter', code:'Enter', keyCode:13, which:13, bubbles:true, composed:true}));
              setTimeout(() => { window.imeBurstFinished = true; }, 50);
            })()
        """.trimIndent(), null) }
        awaitJs(view, "window.imeBurstFinished === true")
        await("IME input not delivered") { events.inputs.joinToString("").endsWith("\r") }
        assertEquals("bookkeeper\r", events.inputs.joinToString(""))
    }

    @Test fun embeddedTerminalResizesAndKeepsAlreadyRenderedOutput() {
        val panelHeight = mutableStateOf(230.dp)
        val events = Events()
        val reference = AtomicReference<TerminalWebView>()
        ActivityScenario.launch(ComponentActivity::class.java).use { scenario ->
            scenario.onActivity { activity ->
                activity.setContent {
                    Column(Modifier.fillMaxSize()) {
                        Text("Editor above terminal")
                        AndroidView(
                            factory = { TerminalWebView(it, events).also(reference::set) },
                            modifier = Modifier.fillMaxWidth().height(panelHeight.value),
                        )
                        Text("Content below terminal")
                    }
                }
            }
            assertTrue("Terminal page did not start", events.ready.await(30, TimeUnit.SECONDS))
            val view = reference.get()
            try {
                onMain { view.write("first\r\nsecond\r\n") }
                awaitSelection(view, events) { it.contains("first\nsecond") }
                await("Terminal has no usable viewport") { events.rows > 1 }
                val height = AtomicReference(0)
                onMain { height.set(view.height); panelHeight.value = 130.dp }
                await("Embedded terminal did not shrink") {
                    val smaller = AtomicReference(false)
                    onMain { smaller.set(view.height in 1 until height.get()) }
                    smaller.get()
                }
                awaitJs(view, "document.querySelector('#terminal').getBoundingClientRect().height > 0 && Math.abs(document.documentElement.getBoundingClientRect().height - window.innerHeight) < 2")
                onMain { view.write("third\r\n") }
                awaitSelection(view, events) { it.contains("first\nsecond\nthird") }
            } finally { onMain { view.release() } }
        }
    }

    private fun withTerminal(action: (TerminalWebView, Events) -> Unit) {
        val events = Events()
        ActivityScenario.launch(ComponentActivity::class.java).use { scenario ->
            lateinit var view: TerminalWebView
            scenario.onActivity { activity ->
                view = TerminalWebView(activity, events)
                activity.setContentView(view, ViewGroup.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT))
            }
            assertTrue("Terminal page did not start", events.ready.await(30, TimeUnit.SECONDS))
            try { action(view, events) } finally { onMain { view.release() } }
        }
    }

    private fun awaitSelection(view: TerminalWebView, events: Events, predicate: (String) -> Boolean) =
        await("Expected output missing: ${events.selection.get()}") {
            onMain { view.selectAll() }
            predicate(events.selection.get().replace("\r\n", "\n"))
        }

    private fun awaitJs(view: TerminalWebView, expression: String) = await("Renderer failed: $expression") {
        val result = AtomicReference<String>()
        val latch = CountDownLatch(1)
        onMain { view.evaluateJavascript("Boolean($expression)") { result.set(it); latch.countDown() } }
        latch.await(1, TimeUnit.SECONDS)
        result.get() == "true"
    }

    private fun await(message: String, condition: () -> Boolean) {
        repeat(100) { if (condition()) return; Thread.sleep(100) }
        fail(message)
    }

    private fun onMain(action: () -> Unit) = InstrumentationRegistry.getInstrumentation().runOnMainSync(action)
}
