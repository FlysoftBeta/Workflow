package top.flysoftbeta.workflow.feature.terminal

import android.os.Build
import android.view.View
import android.view.ViewGroup
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.lifecycle.ViewModelProvider
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import top.flysoftbeta.workflow.MainActivity
import top.flysoftbeta.workflow.app.ShellViewModel
import top.flysoftbeta.workflow.app.Space
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.Workbench
import top.flysoftbeta.workflow.core.store.FileOpResult
import top.flysoftbeta.workflow.feature.editor.TextEditorController
import top.flysoftbeta.workflow.platform.connection.ConnectionStatus
import top.flysoftbeta.workflow.platform.connection.WorkspaceConnectionManager

/** Real Engine cwd resolution → production panel → Sora, with stale generations rejected. */
class TerminalWorkbenchAcceptanceTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    /**
     * The model MainActivity bound for the connection. `Connected` is published on the Engine's thread
     * before the activity recomposes and binds. A ViewModelProvider read in between creates an orphan
     * model in the activity's store; the bind then clears that store and creates the real model, so the
     * tile click navigates the real Shell while the orphan's Shell stays on the Launcher. Both read the
     * same session store, so only the order proves which one is bound: the Launcher tile is composed from
     * the bound model, and the read happens on the main thread after it appears.
     */
    private fun boundModel(): ShellViewModel {
        compose.waitUntil(15_000) { compose.onAllNodesWithText("工作台").fetchSemanticsNodes().isNotEmpty() }
        var model: ShellViewModel? = null
        compose.runOnUiThread { model = ViewModelProvider(compose.activity)[ShellViewModel::class.java] }
        return checkNotNull(model)
    }

    @Test fun engineResolvedOutputPathOpensWorkbenchEditorAtCursor() {
        check(Build.HARDWARE in setOf("ranchu", "goldfish"))
        val manager = WorkspaceConnectionManager.get(compose.activity)
        compose.waitUntil(10_000) { manager.status.value !is ConnectionStatus.Loading }
        if (manager.status.value is ConnectionStatus.Configure) compose.onNodeWithText("使用此设备").performClick()
        compose.waitUntil(60_000) { manager.status.value is ConnectionStatus.Connected || manager.status.value is ConnectionStatus.Failed }
        assertTrue(manager.status.value.toString(), manager.status.value is ConnectionStatus.Connected)
        val model = boundModel()
        val session = manager.requireSession()
        val store = session.store
        compose.onNodeWithText("工作台").performClick()
        compose.waitUntil(15_000) { model.shell.space == Space.WORKBENCH && store.state.value.activeSessionId != null }
        runBlocking {
            assertEquals(FileOpResult.Done, store.createDirectory("terminal-paths"))
            assertEquals(FileOpResult.Done, store.createFile("terminal-paths/target.txt", "one\ntwo\nthree\nfour\n".toByteArray()))
        }
        val runtime = model.workbench.session(store.state.value.activeSessionId!!)
        compose.runOnUiThread { runtime.commands.newTerminal() }
        compose.waitUntil(180_000) {
            val panels = store.state.value.activeSession!!.workbench.panelsIn(Workbench.BOTTOM)
            panels.lastOrNull()?.let { (runtime.existing(it.id) as? TerminalController)?.terminalSession?.status?.value is TerminalStatus.Running } == true
        }
        val panel = store.state.value.activeSession!!.workbench.panelsIn(Workbench.BOTTOM).last()
        val controller = runtime.existing(panel.id) as TerminalController
        val terminal = controller.terminalSession!!
        terminal.write("cd /workspace/terminal-paths; printf '\\033]7;file://localhost/workspace/terminal-paths\\007'; printf 'target.txt:3:2\\n'\n")
        compose.waitUntil(20_000) { runBlocking { terminal.currentDirectory() } == "/workspace/terminal-paths" }
        val resolved = runBlocking { terminal.resolvePaths(listOf("target.txt:3:2", "/etc/passwd", "/workspace/.workspace/state/workspace.json")) }
        assertEquals("terminal-paths/target.txt", resolved[0].path)
        assertEquals(2, resolved[0].line)
        assertEquals(1, resolved[0].column)
        assertNull(resolved[1].path); assertNull(resolved[2].path)
        compose.runOnUiThread { controller.onOpenPath("target.txt:3:2") }
        compose.waitUntil(15_000) {
            store.state.value.activeSession!!.workbench.panelFor(PanelTarget.File("terminal-paths/target.txt"))?.let {
                val editor = (runtime.existing(it.id) as? TextEditorController)?.editorView
                editor?.cursor?.leftLine == 2 && editor.cursor.leftColumn == 1
            } == true
        }
        val stale = runBlocking { runCatching { session.rpc.request("terminal.resolvePaths", mapOf("terminalId" to terminal.id,
            "generation" to terminal.generation.value - 1, "candidates" to listOf("target.txt"))) }.exceptionOrNull() }
        assertNotNull("Stale terminal generation must be refused", stale)
        // Copy uses the platform floating toolbar of the actual panel and the system clipboard.
        fun web(view: View): TerminalWebView? = when (view) {
            is TerminalWebView -> view
            is ViewGroup -> (0 until view.childCount).firstNotNullOfOrNull { web(view.getChildAt(it)) }
            else -> null
        }
        var page: TerminalWebView? = null
        compose.runOnUiThread { page = web(compose.activity.window.decorView); page?.selectAll() }
        assertNotNull(page)
        compose.waitUntil(10_000) { page!!.selectionChrome.actionMode?.type == android.view.ActionMode.TYPE_FLOATING }
        TerminalDeviceProbe(InstrumentationRegistry.getInstrumentation()).tapToolbarItem(compose.activity.getString(android.R.string.copy))
        val clipboard = compose.activity.getSystemService(android.content.ClipboardManager::class.java)
        compose.waitUntil(10_000) { clipboard.primaryClip?.getItemAt(0)?.text?.contains("target.txt:3:2") == true }
    }
}
