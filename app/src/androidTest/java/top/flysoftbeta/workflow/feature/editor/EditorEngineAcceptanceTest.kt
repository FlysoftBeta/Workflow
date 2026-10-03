package top.flysoftbeta.workflow.feature.editor

import android.graphics.Bitmap
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import io.github.rosemoe.sora.widget.schemes.EditorColorScheme
import java.io.File
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.MainActivity
import top.flysoftbeta.workflow.app.ShellViewModel
import top.flysoftbeta.workflow.core.config.ThemeMode
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.TextCursor
import top.flysoftbeta.workflow.core.store.FileOpResult
import top.flysoftbeta.workflow.platform.connection.ConnectionStatus
import top.flysoftbeta.workflow.platform.connection.WorkspaceConnectionManager
import top.flysoftbeta.workflow.ui.design.theme.WorkflowDarkColorScheme
import top.flysoftbeta.workflow.ui.design.theme.WorkflowLightColorScheme
import top.flysoftbeta.workflow.ui.design.MenuEntry

/** Real retained Sora view and Rust-owned PanelView, on the wrapper-managed API 28 emulator. */
@RunWith(AndroidJUnit4::class)
class EditorEngineAcceptanceTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()
    private val manager get() = WorkspaceConnectionManager.get(compose.activity)
    private val store get() = manager.requireSession().store
    private val model get() = ViewModelProvider(compose.activity)[ShellViewModel::class.java]
    private val wb get() = store.state.value.activeSession!!.workbench
    private val runtime get() = model.workbench.session(store.state.value.activeSessionId!!)

    private fun editor(path: String): TextEditorController {
        compose.waitUntil(15_000) {
            wb.panelFor(PanelTarget.File(path))?.let { (runtime.existing(it.id) as? TextEditorController)?.editorView?.isEditable } == true
        }
        compose.waitForIdle()
        return runtime.existing(wb.panelFor(PanelTarget.File(path))!!.id) as TextEditorController
    }

    private fun screenshot(path: String, name: String) {
        val file = File(InstrumentationRegistry.getInstrumentation().targetContext.filesDir, "editor-qa/$name.png")
        file.parentFile!!.mkdirs()
        val bitmap = compose.onNodeWithTag("editor:$path").captureToImage().asAndroidBitmap()
        file.outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
    }

    @Test fun initialThemeScrollOnlyAndWrappedViewportSurviveControllerRecreation() {
        check(android.os.Build.HARDWARE.contains("ranchu") || android.os.Build.FINGERPRINT.contains("generic"))
        compose.waitUntil(15_000) { manager.status.value !is ConnectionStatus.Loading }
        if (manager.status.value is ConnectionStatus.Configure) compose.onNodeWithText("使用此设备").performClick()
        compose.waitUntil(60_000) { manager.status.value is ConnectionStatus.Connected || manager.status.value is ConnectionStatus.Failed }
        assertTrue("${manager.status.value}", manager.status.value is ConnectionStatus.Connected)
        runBlocking { store.updateConfig { it.copy(appearance = it.appearance.copy(theme = ThemeMode.LIGHT)) } }
        compose.onNodeWithText("工作台").performClick()
        val session = runBlocking { store.createSession("Editor viewport QA") }
        val path = "editor-qa-${System.nanoTime()}.md"
        val text = (0 until 250).joinToString("\n") {
            "# Line $it — ~~removed~~ and **strong**; " + "wrapped reading position with cursor elsewhere. ".repeat(15)
        }
        assertEquals(FileOpResult.Done, runBlocking { store.createFile(path, text.toByteArray()) })
        compose.runOnUiThread { runtime.commands.openFile(path) }
        val first = editor(path)
        compose.runOnUiThread {
            assertEquals(WorkflowLightColorScheme.surface.toArgb(), first.editorView!!.colorScheme.getColor(EditorColorScheme.WHOLE_BACKGROUND))
            first.editorView!!.setSelection(0, 0, false)
        }
        screenshot(path, "initial-light")
        compose.onNodeWithTag("editor:$path").performTouchInput { swipeUp(durationMillis = 450) }
        compose.waitUntil(10_000) { first.editorView!!.scroller.isFinished }
        // Stop within a wrapped line (rather than coincidentally on its leading row).
        compose.runOnUiThread {
            val view = first.editorView!!
            val y = view.layout.getCharLayoutOffset(20, 400)[0] - view.rowHeight + view.rowHeight / 3
            view.eventHandler.scrollBy(0f, y - view.offsetY)
            assertEquals("Sora scroll target", 20, view.firstVisibleLine)
            val bottom = view.bottom
            view.layout(view.left, view.top, view.right, bottom - 40)
            assertEquals("Resizing must keep reading position when the caret is elsewhere", 20, view.firstVisibleLine)
            view.layout(view.left, view.top, view.right, bottom)
        }
        try {
            compose.waitUntil(15_000) { wb.panelFor(PanelTarget.File(path))?.view?.scrollAnchor == "20" }
        } catch (error: Throwable) {
            val view = first.editorView!!
            throw AssertionError("Scroll persistence: shown=${view.firstVisibleLine}/${view.offsetY}; editable=${view.isEditable}; scrollerFinished=${view.scroller.isFinished}; saved=${wb.panelFor(PanelTarget.File(path))!!.view}; writeError=${store.state.value.writeError}", error)
        }
        compose.waitUntil(10_000) { first.editorView!!.scroller.isFinished }
        compose.waitForIdle()
        val saved = wb.panelFor(PanelTarget.File(path))!!.view
        assertEquals(TextCursor(0, 0), saved.cursor)
        assertTrue(saved.extras["scrollColumn"]!!.toInt() > 0)
        runBlocking { store.createSession("Editor away") }
        compose.waitUntil(10_000) { first.editorView == null }
        runBlocking { store.activateSession(session) }
        val restored = editor(path)
        assertNotSame(first, restored)
        compose.waitUntil(10_000) { restored.editorView!!.firstVisibleLine == saved.scrollAnchor!!.toInt() }
        compose.runOnUiThread {
            val view = restored.editorView!!
            assertEquals(saved.extras["scrollColumn"]!!.toInt(), view.layout.getRowAt(view.firstVisibleRow).startColumn)
            assertEquals(0, view.cursor.leftLine)
        }
        screenshot(path, "restored-light")
        runBlocking { store.updateConfig { it.copy(appearance = it.appearance.copy(theme = ThemeMode.DARK)) } }
        compose.waitUntil(10_000) { restored.editorView!!.colorScheme.getColor(EditorColorScheme.WHOLE_BACKGROUND) == WorkflowDarkColorScheme.surface.toArgb() }
        screenshot(path, "retained-dark")
        // The focus-loss flush persists a horizontal scroll even before the debounce runs.
        compose.runOnUiThread { (restored.resourceMenu.single { it.key == "wrap" } as MenuEntry.Toggle).onCheckedChange(false) }
        compose.waitUntil(10_000) { restored.editorView!!.isEditable && !restored.editorView!!.isWordwrap }
        compose.runOnUiThread {
            restored.editorView!!.eventHandler.scrollBy(240f, 0f)
            restored.onFocusChanged(false)
        }
        compose.waitUntil(10_000) { wb.panelFor(PanelTarget.File(path))!!.view.extras["scrollX"] == "240" }
        runBlocking { store.createSession("Editor horizontal away") }
        compose.waitUntil(10_000) { restored.editorView == null }
        runBlocking { store.activateSession(session) }
        val horizontal = editor(path)
        compose.waitUntil(10_000) { horizontal.editorView!!.offsetX == 240 }
        compose.runOnUiThread { horizontal.navigate(TextCursor(100, 5)) }
        compose.waitUntil(10_000) {
            val view = horizontal.editorView!!
            view.scroller.isFinished && view.cursor.leftLine == 100 && view.firstVisibleLine <= 100 && view.lastVisibleLine >= 100
        }
        compose.runOnUiThread {
            val view = horizontal.editorView!!
            val caretBottom = view.layout.getCharLayoutOffset(100, 5)[0]
            view.eventHandler.scrollBy(0f, caretBottom - view.height + view.rowHeight / 2 - view.offsetY)
            val before = view.offsetY
            assertTrue(view.firstVisibleLine <= 100 && view.lastVisibleLine >= 100)
            val bottom = view.bottom
            view.layout(view.left, view.top, view.right, view.top + view.height / 2)
            view.scroller.abortAnimation()
            assertTrue("A visible caret near the bottom must trigger resize scrolling", view.offsetY > before)
            assertTrue("Resizing must still reveal a previously visible caret", view.firstVisibleLine <= 100 && view.lastVisibleLine >= 100)
            view.layout(view.left, view.top, view.right, bottom)
        }
        println("EDITOR_QA: light first paint, scroll-only persisted $saved, wrapped viewport restored, retained dark theme, horizontal restore, focus flush, resize behavior and explicit navigation passed")
    }
}
