package top.flysoftbeta.workflow.app

import android.graphics.Bitmap
import android.view.inputmethod.InputMethodManager
import android.content.Context
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.MainActivity
import top.flysoftbeta.workflow.core.layout.*
import top.flysoftbeta.workflow.core.resource.FileStatus
import top.flysoftbeta.workflow.core.store.SaveResult
import top.flysoftbeta.workflow.feature.editor.TextEditorController
import top.flysoftbeta.workflow.feature.files.FilesExplorer
import top.flysoftbeta.workflow.platform.connection.ConnectionStatus
import top.flysoftbeta.workflow.platform.connection.WorkspaceConnectionManager

/** Actual MainActivity + packaged Rust Engine. Run only on the disposable wrapper-managed emulator. */
@RunWith(AndroidJUnit4::class)
class WorkbenchEngineAcceptanceTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()
    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val manager get() = WorkspaceConnectionManager.get(compose.activity)
    private val model get() = ViewModelProvider(compose.activity)[ShellViewModel::class.java]
    private val store get() = manager.requireSession().store
    private val shell get() = model.shell
    private val wb get() = store.state.value.activeSession!!.workbench
    private val runtime get() = model.workbench.session(store.state.value.activeSessionId!!)
    private val label get() = InstrumentationRegistry.getArguments().getString("workbenchQaLabel") ?: "default"
    private fun shown(text: String) = compose.onAllNodesWithText(text).fetchSemanticsNodes().isNotEmpty()
    private fun settle() { compose.waitForIdle(); instrumentation.waitForIdleSync() }
    private fun back() = compose.runOnUiThread { compose.activity.onBackPressedDispatcher.onBackPressed() }
    private fun screenshot(name: String) {
        settle()
        val file = File(instrumentation.targetContext.filesDir, "workbench-qa/$label-$name.png")
        file.parentFile!!.mkdirs()
        // PixelCopy of the Compose window avoids API 28 UiAutomation's framebuffer crop after wm size.
        android.os.SystemClock.sleep(350)
        val roots = compose.onAllNodes(isRoot())
        val count = roots.fetchSemanticsNodes().size
        for (index in 0 until count) {
            val image = roots[index].captureToImage().asAndroidBitmap()
            val target = if (index == 0) file else File(file.parentFile, "${file.nameWithoutExtension}-window$index.png")
            target.outputStream().use { image.compress(Bitmap.CompressFormat.PNG, 100, it) }
        }
    }
    private fun note(text: String) = println("WORKBENCH_QA[$label] $text")
    private fun editor(path: String): TextEditorController {
        compose.waitUntil(15_000) {
            wb.panelFor(PanelTarget.File(path))?.let { (runtime.existing(it.id) as? TextEditorController)?.editorView } != null
        }
        return runtime.existing(wb.panelFor(PanelTarget.File(path))!!.id) as TextEditorController
    }
    private fun hideIme() = compose.runOnUiThread {
        (compose.activity.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager)
            .hideSoftInputFromWindow(compose.activity.window.decorView.windowToken, 0)
    }

    @Test fun integratedWorkbenchFileSessionsAndNavigation() {
        check(android.os.Build.FINGERPRINT.contains("generic") || android.os.Build.HARDWARE.contains("ranchu"))
        compose.waitUntil(10_000) { manager.status.value !is ConnectionStatus.Loading }
        if (manager.status.value is ConnectionStatus.Configure) {
            compose.onNodeWithText("连接工作区").assertIsDisplayed()
            assertNull(manager.sessionOrNull())
            screenshot("connection")
            compose.onNodeWithText("使用此设备").performClick()
            note("Explicit first connection selected; no workspace before selection")
        }
        compose.waitUntil(60_000) { manager.status.value is ConnectionStatus.Connected || manager.status.value is ConnectionStatus.Failed }
        assertTrue("Connection status: ${manager.status.value}", manager.status.value is ConnectionStatus.Connected)
        compose.waitUntil(10_000) { shown("工作台") }
        screenshot("launcher")
        compose.onNodeWithText("工作台").performClick()
        compose.waitUntil(15_000) { store.state.value.activeSession != null && shell.space == Space.WORKBENCH }
        runBlocking { store.createSession("UI $label") }
        compose.waitForIdle()
        val session = store.state.value.activeSessionId!!
        val path = "qa-${System.nanoTime()}.md"
        // The default route verifies inline creation. Diagnostic fixture mode isolates later UI
        // acceptance if the separately owned client upload boundary is under repair.
        val fixtureMode = InstrumentationRegistry.getArguments().getString("workbenchSeedViaRpc") == "true"
        fun seed(name: String, text: String) = runBlocking {
            if (fixtureMode) {
                val response = manager.requireSession().rpc.request("workspace.command", mapOf(
                    "name" to "createFile", "args" to mapOf("path" to name, "data" to java.util.Base64.getEncoder().encodeToString(text.toByteArray())),
                )) as Map<*, *>
                val value = response["value"] as Map<*, *>
                if (value["kind"] != "done") {
                    note("Engine createFile failed: $value; diagnostic seed via saveFile")
                    assertTrue(store.saveFile(name, text) is SaveResult.Saved)
                }
                assertEquals(text, store.openFile(name).text)
            } else assertEquals(top.flysoftbeta.workflow.core.store.FileOpResult.Done, store.createFile(name, text.toByteArray()))
        }
        if (fixtureMode) {
            seed(path, "")
            compose.runOnUiThread { runtime.commands.openFile(path) }
            note("Diagnostic fixture mode: inline-create/upload boundary excluded")
        } else {
            compose.runOnUiThread { runtime.beginCreateFile() }
            compose.waitUntil(5_000) { compose.onAllNodesWithTag("explorer:edit").fetchSemanticsNodes().isNotEmpty() }
            compose.onNodeWithTag("explorer:edit").performTextInput(path)
            compose.onNodeWithTag("explorer:edit").performImeAction()
        }
        val editor = editor(path)
        assertFalse(runBlocking { store.listDirectory("", true) }.any { it.path == ".workspace" })
        val text = "# Workspace notes\n\nA durable draft.\n" + (1..60).joinToString("\n") { "Line $it: edit, resize and restore." }
        compose.runOnUiThread { editor.editorView!!.commitText(text) }
        compose.waitUntil(10_000) { store.state.value.drafts[path]?.text == text }
        compose.runOnUiThread {
            editor.editorView!!.requestFocus()
            (compose.activity.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager)
                .showSoftInput(editor.editorView!!, InputMethodManager.SHOW_FORCED)
        }
        compose.waitUntil(5_000) {
            androidx.core.view.ViewCompat.getRootWindowInsets(compose.activity.window.decorView)
                ?.isVisible(androidx.core.view.WindowInsetsCompat.Type.ime()) == true
        }
        screenshot("editor-ime")
        hideIme()
        compose.runOnUiThread { editor.save() }
        compose.waitUntil(10_000) { store.state.value.drafts[path] == null }
        assertEquals(text, runBlocking { store.openFile(path).text })
        note("Typed and saved through actual Sora + Rust store; inline creation tested=${!fixtureMode}")
        // Test-only external actor, outside app commands, to exercise the true on-disk conflict boundary.
        compose.runOnUiThread { editor.editorView!!.setSelection(0, 0); editor.editorView!!.commitText("Draft ") }
        compose.waitUntil(10_000) { store.state.value.drafts[path] != null }
        File(manager.requireSession().root, path).writeText("External version\n")
        runBlocking { store.runMaintenance() }
        compose.waitUntil(10_000) { shown("磁盘版本已更改") }
        assertTrue(runBlocking { store.saveFile(path) } is SaveResult.Conflict)
        screenshot("conflict")
        compose.onAllNodesWithText("保留我的")[0].performClick()
        compose.waitUntil(10_000) { store.state.value.fileStatus(path) == FileStatus.DIRTY }
        compose.runOnUiThread { editor.save() }
        compose.waitUntil(10_000) { store.state.value.drafts[path] == null }
        assertTrue(runBlocking { store.openFile(path).text }.startsWith("Draft #"))
        note("External change refused; explicit keep-mine/save resolved conflict")
        // Archive must offer a decision, keep the durable draft, then restore it.
        compose.runOnUiThread { editor.editorView!!.commitText("Kept draft "); hideIme() }
        compose.waitUntil(10_000) { store.state.value.drafts[path] != null }
        val kept = store.state.value.drafts[path]!!.text
        compose.runOnUiThread { shell.archive(session) }
        compose.waitUntil(10_000) { shown("保留草稿") }
        screenshot("archive-decision")
        compose.onNodeWithText("保留草稿").performClick()
        compose.waitUntil(10_000) { store.state.value.session(session)?.isArchived == true }
        assertEquals(kept, store.state.value.drafts[path]?.text)
        compose.runOnUiThread { shell.switchTo(session) }
        compose.waitUntil(10_000) { store.state.value.activeSessionId == session && store.state.value.session(session)?.isArchived == false }
        assertEquals(kept, editor(path).editorView!!.text.toString())
        note("Archive required explicit decision, retained draft and restored session")
        // Close/switch/recreate keeps the editor view state and draft in the Engine.
        val restoredEditor = editor(path)
        compose.runOnUiThread { restoredEditor.editorView!!.setSelection(45, 8); restoredEditor.editorView!!.ensureSelectionVisible() }
        compose.waitUntil(10_000) { wb.panelFor(PanelTarget.File(path))?.view?.cursor?.line == 45 }
        val viewBefore = wb.panelFor(PanelTarget.File(path))!!.view
        compose.activityRule.scenario.recreate()
        compose.waitUntil(15_000) { shell.space == Space.WORKBENCH && wb.panelFor(PanelTarget.File(path)) != null }
        val recreated = editor(path)
        compose.waitUntil(10_000) { recreated.editorView!!.cursor.leftLine == 45 }
        assertEquals(kept, recreated.editorView!!.text.toString())
        assertEquals(viewBefore.cursor, wb.panelFor(PanelTarget.File(path))!!.view.cursor)
        screenshot("restored")
        note("Activity recreation retained editor cursor and draft; scroll-only restoration audited separately")
        // Tab move menu and real drag each commit to the Rust layout reducer.
        val second = path.replace(".md", "-two.txt")
        seed(second, "second\n")
        runBlocking { store.applyLayout(session, LayoutOp.Open(PanelTarget.File(second))) }
        editor(second)
        runBlocking { store.applyLayout(session, LayoutOp.SetRegionCollapsed(Region.EXPLORER, true)) }
        compose.runOnUiThread { runtime.presence.sideOverlay = false }
        settle()
        val stack = wb.stackOf(wb.panelFor(PanelTarget.File(second))!!.id)!!
        val before = wb.stacks.getValue(stack).panels.toList()
        compose.onAllNodesWithText(second)[0].performTouchInput { longClick() }
        compose.waitUntil(5_000) { shown("向前移动标签") }
        compose.onNodeWithText("向前移动标签").performClick()
        compose.waitUntil(10_000) { wb.stacks.getValue(stack).panels != before }
        assertEquals(second, (wb.panelsIn(stack).first().target as PanelTarget.File).path)
        val region = compose.onNodeWithTag("stack:$stack").getBoundsInRoot()
        val tab = compose.onAllNodesWithText(second)[0].getBoundsInRoot()
        dragFrom(compose.onAllNodesWithText(second)[0], with(compose.density) {
            Offset((region.right - 24.dp - tab.left).toPx(), ((region.top + region.bottom) / 2 - tab.top).toPx())
        })
        compose.waitUntil(10_000) { wb.editorStacks.size >= 2 }
        assertTrue(wb.violations().isEmpty())
        screenshot("split")
        note("Tab reorder menu and touch split completed with valid Engine layout")
        // Maximized editor still opens explorer as overlay and closes it before leaving workbench.
        runBlocking { store.applyLayout(session, LayoutOp.SetMaximized(wb.focusedStack)) }
        compose.onNodeWithContentDescription("文件浏览器").performClick()
        compose.waitUntil(5_000) { runtime.presence.sideOverlay }
        compose.onNodeWithTag("explorer").assertIsDisplayed()
        screenshot("maximized-explorer")
        back(); compose.waitUntil(5_000) { !runtime.presence.sideOverlay }
        runBlocking { store.applyLayout(session, LayoutOp.SetMaximized(null)) }
        // Trash/undo uses the actual explorer action and authoritative restored file.
        val explorer = runtime.explorerController() as FilesExplorer
        compose.runOnUiThread { explorer.delete(second) }
        compose.waitUntil(10_000) { shell.snackbar.currentSnackbarData != null }
        note("Trash snackbar: ${shell.snackbar.currentSnackbarData?.visuals?.message}")
        screenshot("trash")
        compose.onNodeWithText("撤销").performClick()
        compose.waitUntil(10_000) { runBlocking { store.openFile(second).disk.exists } }
        note("Trash/undo restored file through Engine")
        compose.onNodeWithContentDescription("设置").performClick()
        compose.waitUntil(10_000) { wb.panelFor(PanelTarget.Settings) != null }
        screenshot("settings")
        compose.runOnUiThread { shell.goHome() }
        compose.onNodeWithText("代理").performClick()
        compose.waitUntil(10_000) { wb.panelFor(PanelTarget.Proxy()) != null }
        screenshot("proxy")
        compose.runOnUiThread { runtime.commands.newConversation() }
        compose.waitUntil(15_000) { wb.panels.values.any { it.target is PanelTarget.Conversation } }
        screenshot("native-chat-empty")
        compose.runOnUiThread { runtime.commands.attachToConversation(listOf(path)) }
        compose.waitUntil(10_000) { store.state.value.composers.values.any { draft -> draft.attachments.any { it.path == path } } }
        screenshot("native-chat-attachment")
        note("Settings, proxy, native chat and attach-file alternative rendered; no authenticated model turn claimed")
        runBlocking { store.flush() }
    }

    @Test fun terminalRunsInsideEnvironmentAndAcceptsFilePathAlternative() {
        check(android.os.Build.FINGERPRINT.contains("generic") || android.os.Build.HARDWARE.contains("ranchu"))
        compose.waitUntil(10_000) { manager.status.value !is ConnectionStatus.Loading }
        if (manager.status.value is ConnectionStatus.Configure) compose.onNodeWithText("使用此设备").performClick()
        compose.waitUntil(60_000) { manager.status.value is ConnectionStatus.Connected || manager.status.value is ConnectionStatus.Failed }
        assertTrue("Connection: ${manager.status.value}", manager.status.value is ConnectionStatus.Connected)
        compose.waitUntil(10_000) { shown("工作台") }
        compose.onNodeWithText("工作台").performClick()
        compose.waitUntil(10_000) { shell.space == Space.WORKBENCH }
        runBlocking { store.createSession("Terminal UI") }
        val path = "path with spaces.txt"
        runBlocking { store.saveFile(path, "terminal fixture") }
        compose.runOnUiThread { runtime.commands.newTerminal() }
        compose.waitUntil(15_000) { wb.panels.values.any { it.target is PanelTarget.Terminal } }
        val panel = wb.panels.values.first { it.target is PanelTarget.Terminal }
        compose.waitUntil(15_000) { (runtime.existing(panel.id) as? top.flysoftbeta.workflow.feature.terminal.TerminalController)?.terminalSession != null }
        val controller = runtime.existing(panel.id) as top.flysoftbeta.workflow.feature.terminal.TerminalController
        val terminal = controller.terminalSession!!
        compose.waitUntil(180_000) {
            terminal.status.value is top.flysoftbeta.workflow.feature.terminal.TerminalStatus.Running ||
                terminal.status.value is top.flysoftbeta.workflow.feature.terminal.TerminalStatus.Failed
        }
        compose.waitUntil(15_000) {
            var ready = false
            compose.runOnUiThread {
                fun visit(view: android.view.View) {
                    if (view is top.flysoftbeta.workflow.feature.terminal.TerminalWebView && view.isReady) ready = true
                    if (view is android.view.ViewGroup) for (index in 0 until view.childCount) visit(view.getChildAt(index))
                }
                visit(compose.activity.window.decorView)
            }
            ready || terminal.status.value is top.flysoftbeta.workflow.feature.terminal.TerminalStatus.Failed
        }
        screenshot("terminal")
        note("Terminal status: ${terminal.status.value}")
        if (terminal.status.value !is top.flysoftbeta.workflow.feature.terminal.TerminalStatus.Running) {
            note("Environment status: ${runBlocking { manager.requireSession().rpc.request("environment.status") }}")
        }
        assertTrue("Terminal: ${terminal.status.value}", terminal.status.value is top.flysoftbeta.workflow.feature.terminal.TerminalStatus.Running)
        terminal.write("printf '%s' ")
        compose.runOnUiThread { runtime.commands.pasteIntoTerminal(listOf(path)) }
        // xterm's paste bridge is asynchronous; raw echoed bytes may also wrap mid-path.
        settle()
        android.os.SystemClock.sleep(500)
        terminal.write("> qa-terminal-path-output.txt\n")
        try {
            compose.waitUntil(10_000) { runBlocking { store.openFile("qa-terminal-path-output.txt").text } == path }
        } finally {
            note("Terminal captured output: ${terminal.scrollback.since(0).text}")
            screenshot("terminal-path")
        }
        note("xterm + Engine PTY wrote the menu-delivered, correctly quoted relative path into a workspace file")
    }

    @Test fun savedEnvironmentConfigBuildsAndConfirmedRestartReconnectsTerminal() {
        check(android.os.Build.HARDWARE in setOf("ranchu", "goldfish"))
        compose.waitUntil(10_000) { manager.status.value !is ConnectionStatus.Loading }
        if (manager.status.value is ConnectionStatus.Configure) compose.onNodeWithText("使用此设备").performClick()
        compose.waitUntil(60_000) { manager.status.value is ConnectionStatus.Connected }
        compose.waitUntil(10_000) { shown("工作台") }
        compose.onNodeWithText("工作台").performClick()
        compose.waitUntil(10_000) { shell.space == Space.WORKBENCH }
        val session = runBlocking { store.createSession("Environment rebuild UI") }
        compose.waitForIdle()
        runBlocking {
            store.createDirectory("rebuild-cwd")
            val disk = store.openFile("rebuild-draft.txt").shownVersion
            store.editFile("rebuild-draft.txt", "retained draft", disk)
            store.flush()
        }
        compose.runOnUiThread { runtime.commands.newTerminal("rebuild-cwd") }
        compose.waitUntil(15_000) { wb.panels.values.any { it.target is PanelTarget.Terminal } }
        val panel = wb.panels.values.first { it.target is PanelTarget.Terminal }
        compose.waitUntil(15_000) { (runtime.existing(panel.id) as? top.flysoftbeta.workflow.feature.terminal.TerminalController)?.terminalSession != null }
        val terminal = (runtime.existing(panel.id) as top.flysoftbeta.workflow.feature.terminal.TerminalController).terminalSession!!
        compose.waitUntil(180_000) { terminal.status.value is top.flysoftbeta.workflow.feature.terminal.TerminalStatus.Running }
        terminal.write("pwd > before-cwd.txt\n")
        compose.waitUntil(10_000) { runBlocking { store.openFile("rebuild-cwd/before-cwd.txt").text.trim() } == "/workspace/rebuild-cwd" }
        val generation = terminal.generation.value
        val spec = """{"version":1,"env":{"WF_REBUILD_UI":"confirmed"}}"""
        assertTrue(runBlocking { store.saveFile(top.flysoftbeta.workflow.core.io.WorkspacePaths.ENVIRONMENT, spec) } is SaveResult.Saved)
        // No environment.reconcile RPC: saving the declaration must be sufficient.
        val engine = AppGraph.engine(compose.activity)
        compose.waitUntil(120_000) {
            assertTrue("Prior environment must remain usable throughout construction", engine.health.value.usable)
            engine.health.value is top.flysoftbeta.workflow.platform.engine.EnvironmentHealth.NeedsRestart
        }
        assertEquals(generation, terminal.generation.value)
        screenshot("rebuild-pending")
        compose.onNodeWithText("重启环境").performClick()
        compose.onNodeWithText("正在运行的终端和助手进程将停止并重新连接。").assertIsDisplayed()
        compose.onNodeWithText("重启", useUnmergedTree = true).performClick()
        compose.waitUntil(30_000) { terminal.generation.value > generation && terminal.status.value is top.flysoftbeta.workflow.feature.terminal.TerminalStatus.Running }
        terminal.write("printf '%s:%s' \"\$WF_REBUILD_UI\" \"\$PWD\" > after-rebuild.txt\n")
        compose.waitUntil(10_000) { runBlocking { store.openFile("rebuild-cwd/after-rebuild.txt").text } == "confirmed:/workspace/rebuild-cwd" }
        assertEquals(session, store.state.value.activeSessionId)
        assertNotNull(wb.panels[panel.id])
        assertEquals("retained draft", store.state.value.drafts["rebuild-draft.txt"]?.text)
        screenshot("rebuild-ready")
        note("Saved env.json automatically built; confirmed UI restart retained terminal cwd/panel and unsaved draft")
    }

    private fun dragFrom(node: SemanticsNodeInteraction, target: Offset) {
        var start = Offset.Zero
        compose.mainClock.autoAdvance = false
        try {
            node.performTouchInput { start = center; down(center) }
            compose.mainClock.advanceTimeBy(500)
            node.performTouchInput { for (i in 1..12) { advanceEventTime(16); moveTo(start + (target - start) * (i / 12f)) } }
            compose.mainClock.advanceTimeBy(100)
            node.performTouchInput { up() }
            compose.mainClock.advanceTimeBy(500)
        } finally { compose.mainClock.autoAdvance = true }
    }
}
