package top.flysoftbeta.workflow.feature.chat

import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Surface
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.core.layout.PanelView
import top.flysoftbeta.workflow.feature.chat.transcript.*
import top.flysoftbeta.workflow.ui.design.theme.ThemeMode
import top.flysoftbeta.workflow.ui.design.theme.WorkflowDesignTheme

@RunWith(AndroidJUnit4::class)
class NativeTranscriptTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val copies = mutableListOf<String>()
    private var saved = PanelView()
    private val callbacks = object : TranscriptCallbacks {
        override fun openUrl(url: String) = Unit
        override fun openPath(path: String, line: Int?, column: Int?) = Unit
        override fun copyText(text: String) { copies += text }
        override fun copyTurn(turn: String, what: String) { copies += "$turn/$what" }
        override fun toggle(turn: String, expanded: Boolean) = Unit
        override fun earlier() = Unit
        override fun layout(atBottom: Boolean) = Unit
        override fun action(turn: String, name: String) = Unit
        override fun viewport(anchor: String?, offset: Int, atBottom: Boolean) { saved = PanelView(anchor, offset, extras = mapOf("chat.follow" to atBottom.toString())) }
    }
    private fun turn(id: String, text: String) = TurnView(id, id, "completed", 1, 4000, null, UserView("Explain native rendering $id", emptyList()), listOf(ItemView("m", "message", "done", text)), "m", null, null, true, false)
    private fun screenshot(name: String) {
        compose.waitForIdle()
        val out = java.io.File(InstrumentationRegistry.getInstrumentation().targetContext.filesDir, "native-chat/$name.png")
        out.parentFile!!.mkdirs()
        out.outputStream().use { compose.onRoot().captureToImage().asAndroidBitmap().compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it) }
    }
    @Test fun nativeMathMarkdownTablesCodeLightDarkAndNarrow() {
        val source = "# Native conversation\n\n**Bold** and *emphasis*, \\(x^2+y^2=z^2\\) and \$\\alpha+\\beta\$.\n\n\$\$\\frac{1}{2}+\\sqrt{x^2+1}\$\$\n\n| Item | Value |\n|:---|---:|\n| Native | 28 |\n\n```kotlin\nval price = \"\$5\"\n```\n\n> A quote with a [local file](README.md:3)."
        val doc = NativeTranscriptProjector().project(listOf(turn("demo", source)), false)
        val state = NativeTranscriptState(PanelView(extras = mapOf("chat.follow" to "false")))
        var dark by mutableStateOf(false)
        var narrow by mutableStateOf(false)
        compose.setContent { WorkflowDesignTheme(if (dark) ThemeMode.Dark else ThemeMode.Light) { Surface(Modifier.width(if (narrow) 360.dp else 720.dp)) { NativeTranscript(doc, state, callbacks) } } }
        compose.waitUntil(20_000) { compose.onAllNodesWithTag("native-math", useUnmergedTree = true).fetchSemanticsNodes().size >= 2 }
        compose.onNodeWithTag("native-table").assertExists()
        screenshot("light-wide")
        compose.runOnUiThread { dark = true; narrow = true }
        compose.waitForIdle()
        screenshot("dark-narrow")
        compose.onNodeWithTag("native-transcript").performScrollToNode(hasTestTag("native-code"))
        compose.onAllNodesWithText("复制").onFirst().performClick()
        assertTrue(copies.any { "val price" in it })
    }
    @Test fun longTranscriptAnchorsRestoreAndStreamingDoesNotPullReaderDown() {
        val projector = NativeTranscriptProjector()
        val turns = (0..80).map { turn("t$it", "## Answer $it\n\n" + "Readable long text. ".repeat(40)) }
        var document by mutableStateOf(projector.project(turns, false))
        var state by mutableStateOf(NativeTranscriptState())
        compose.setContent { WorkflowDesignTheme { Surface(Modifier.width(360.dp)) { NativeTranscript(document, state, callbacks) } } }
        compose.waitUntil(20_000) { state.restored && state.list.firstVisibleItemIndex > 100 }
        compose.onNodeWithTag("native-transcript").performScrollToIndex(70)
        compose.onNodeWithTag("native-transcript").performTouchInput { swipeDown() }
        compose.waitUntil { !state.follow }
        compose.waitForIdle()
        val anchor = state.list.layoutInfo.visibleItemsInfo.first { it.index == state.list.firstVisibleItemIndex }.key
        compose.runOnUiThread { document = projector.project(turns + turn("new", "Streaming \\(x^"), false) }
        compose.waitForIdle()
        assertEquals(anchor, state.list.layoutInfo.visibleItemsInfo.first { it.index == state.list.firstVisibleItemIndex }.key)
        compose.mainClock.advanceTimeBy(600)
        compose.waitUntil { saved.extras["chat.follow"] == "false" }
        assertEquals("false", saved.extras["chat.follow"])
        val persisted = saved
        compose.runOnUiThread { state = NativeTranscriptState(persisted) }
        compose.waitUntil { state.restored }
        assertEquals(persisted.scrollAnchor, state.list.layoutInfo.visibleItemsInfo.first { it.index == state.list.firstVisibleItemIndex }.key)
        assertEquals(persisted.scrollOffset, state.list.firstVisibleItemScrollOffset)
        screenshot("long-restored")
        compose.runOnUiThread { state.bottom() }
        compose.waitUntil { !state.list.canScrollForward }
        assertTrue(state.follow)
    }
    @Test fun unsafeMathFallsBackWithoutDrawingOrSideEffects() = runBlocking {
        assertNull(NativeMath.render("\\includegraphics{/sdcard/private.png}", 20f, 0xff000000.toInt()))
        assertNull(NativeMath.render("\\newcommand{\\a}{\\a}\\a", 20f, 0xff000000.toInt()))
        assertNotNull(NativeMath.render("\\frac{1}{2}", 20f, 0xff000000.toInt()))
    }
}
