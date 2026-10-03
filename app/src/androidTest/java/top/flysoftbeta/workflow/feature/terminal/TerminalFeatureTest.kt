package top.flysoftbeta.workflow.feature.terminal

import top.flysoftbeta.workflow.core.store.ReferenceWorkspaceStore

import androidx.activity.ComponentActivity
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithContentDescription
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.test.ext.junit.runners.AndroidJUnit4
import java.io.File
import java.util.UUID
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.app.Shell
import top.flysoftbeta.workflow.app.Space
import top.flysoftbeta.workflow.app.WorkflowApp
import top.flysoftbeta.workflow.app.panel.PanelRegistry
import top.flysoftbeta.workflow.app.panel.placeholder.PlaceholderConversationProvider
import top.flysoftbeta.workflow.app.panel.placeholder.PlaceholderPanelProvider
import top.flysoftbeta.workflow.app.panel.placeholder.PlaceholderRailProvider
import top.flysoftbeta.workflow.core.layout.PanelKind
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.Workbench
import top.flysoftbeta.workflow.core.store.WorkspaceStore
import top.flysoftbeta.workflow.feature.editor.EditorPanelProvider
import top.flysoftbeta.workflow.feature.editor.TextEditorController
import top.flysoftbeta.workflow.feature.files.FilesExplorerProvider
import top.flysoftbeta.workflow.feature.workbench.WorkbenchRuntime
import top.flysoftbeta.workflow.platform.pty.AndroidShellBackend
import top.flysoftbeta.workflow.platform.workspace.AndroidFileSystem
import top.flysoftbeta.workflow.ui.design.dnd.DragDropState
import top.flysoftbeta.workflow.ui.design.dnd.FilesDragPayload

/**
 * The terminal panel on the real JNI PTY (Android shell), in a throwaway workspace under the app cache:
 * links open files at path:line:col (relative to the shell's cwd), dropped files paste shell-escaped
 * paths, the extra-keys row sends ^C / arrows and latches Ctrl, OSC titles name the tab, and the process
 * outlives its panel controller.
 */
@RunWith(AndroidJUnit4::class)
class TerminalFeatureTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private lateinit var root: File
    private lateinit var host: TerminalHost
    private lateinit var store: WorkspaceStore
    private lateinit var shell: Shell
    private lateinit var runtime: WorkbenchRuntime

    @Before fun setUp() {
        val context = compose.activity.applicationContext
        root = File(context.cacheDir, "w6-terminal-${UUID.randomUUID()}").apply { mkdirs() }.canonicalFile
        File(root, "sub").mkdirs()
        File(root, "sub/x.txt").writeText("l1\nl2\nl3 target\nl4\n")
        File(root, "sub/a b.txt").writeText("space")
        host = TerminalHost(AndroidShellBackend(context, root))
        val placeholder = PlaceholderPanelProvider()
        val editor = EditorPanelProvider()
        val registry = PanelRegistry(
            providers = mapOf(
                PanelKind.FILE to editor, PanelKind.IMAGE to editor, PanelKind.DIFF to editor,
                PanelKind.TERMINAL to TerminalPanelProvider { host },
                PanelKind.CONVERSATION to PlaceholderConversationProvider(),
            ),
            explorer = FilesExplorerProvider(context),
            rail = PlaceholderRailProvider(),
            fallback = placeholder,
        )
        store = ReferenceWorkspaceStore(AndroidFileSystem(root), scope, Dispatchers.IO.limitedParallelism(1)).also { it.start() }
        runBlocking {
            store.awaitReady()
            store.updateConfig { it.copy(terminal = it.terminal.copy(extraKeysPinned = true)) }
        }
        shell = Shell(context, store, registry, scope)
        runtime = WorkbenchRuntime(context, store, registry, shell)
        compose.setContent { WorkflowApp(shell, runtime, DragDropState()) }
        compose.onNodeWithText("工作台").performClick()
        compose.waitUntil(5_000) { shell.space == Space.WORKBENCH && store.state.value.activeSession != null }
    }

    @After fun tearDown() {
        compose.runOnUiThread { runtime.dispose() }
        runBlocking { store.close() }
        scope.cancel()
        root.deleteRecursively()
    }

    private fun workbench(): Workbench = store.state.value.activeSession!!.workbench
    private fun sessionRuntime() = runtime.session(store.state.value.activeSessionId!!)

    private fun openTerminal(): TerminalController {
        compose.runOnUiThread { sessionRuntime().commands.newTerminal() }
        compose.waitUntil(10_000) { workbench().panelsIn(Workbench.BOTTOM).isNotEmpty() }
        val panel = workbench().panelsIn(Workbench.BOTTOM).last()
        compose.waitUntil(10_000) {
            (sessionRuntime().existing(panel.id) as? TerminalController)?.terminalSession?.status?.value is TerminalStatus.Running
        }
        val controller = sessionRuntime().existing(panel.id) as TerminalController
        // Wait for the prompt, so input is not raced by shell start-up.
        compose.waitUntil(10_000) { controller.terminalSession!!.scrollback.since(0).text.contains("$") }
        return controller
    }

    private fun output(controller: TerminalController) = controller.terminalSession!!.scrollback.since(0).text

    private fun awaitFile(name: String, content: String) {
        compose.waitUntil(10_000) { runCatching { File(root, name).readText() == content }.getOrDefault(false) }
    }

    @Test fun linksOpenFilesAtLineAndColumnRelativeToTheShellDirectory() {
        val terminal = openTerminal()
        compose.runOnUiThread { terminal.onOpenPath("./sub/x.txt:3:4") }
        compose.waitUntil(10_000) { editorCursor("sub/x.txt") == 2 to 3 }
        terminal.terminalSession!!.write("cd sub\n")
        compose.waitUntil(10_000) { runBlocking { terminal.terminalSession!!.currentDirectory() } == File(root, "sub").path }
        compose.runOnUiThread { terminal.onOpenPath("x.txt:2") }
        compose.waitUntil(10_000) { editorCursor("sub/x.txt") == 1 to 0 }
        compose.runOnUiThread { terminal.onOpenPath("missing.txt:1") }
        compose.waitUntil(5_000) { compose.onAllNodesWithText("找不到 missing.txt:1").fetchSemanticsNodes().isNotEmpty() }
    }

    @Test fun droppedFilesPasteShellEscapedPaths() {
        val terminal = openTerminal()
        val session = terminal.terminalSession!!
        session.write("printf '%s|' ")
        compose.waitUntil(5_000) { output(terminal).contains("printf") }
        // The shell is in the workspace root: paths are pasted relative to it, quoted when needed.
        compose.runOnUiThread { terminal.onDrop(FilesDragPayload(listOf("sub/a b.txt", "sub/x.txt"))) }
        compose.waitUntil(10_000) { output(terminal).contains("'sub/a b.txt' sub/x.txt") }
        session.write("> pasted.txt\n")
        awaitFile("pasted.txt", "sub/a b.txt|sub/x.txt|")
    }

    @Test fun extraKeysSendControlArrowsAndLatchedCtrl() {
        val terminal = openTerminal()
        val session = terminal.terminalSession!!
        // ^C key interrupts a foreground job.
        session.write("sleep 30\n")
        compose.waitUntil(5_000) { output(terminal).contains("sleep 30") }
        Thread.sleep(300)
        // The row scrolls horizontally; ^C sits in the fifth group.
        compose.onAllNodesWithContentDescription("^C")[0].performScrollTo().performClick()
        session.write("echo INT1 > int1.txt\n")
        awaitFile("int1.txt", "INT1\n")
        // Latched Ctrl applies to the next typed character (IME input goes through onInput).
        session.write("sleep 30\n")
        Thread.sleep(500)
        compose.onAllNodesWithContentDescription("Ctrl")[0].performScrollTo().performClick()
        compose.runOnUiThread { terminal.onInput("c") }
        session.write("echo INT2 > int2.txt\n")
        awaitFile("int2.txt", "INT2\n")
        // ↑ recalls the previous command from history.
        File(root, "int2.txt").delete()
        compose.onAllNodesWithContentDescription("↑")[0].performScrollTo().performClick()
        Thread.sleep(300)
        session.write("\r")
        awaitFile("int2.txt", "INT2\n")
    }

    @Test fun oscTitleNamesTheTabAndTheProcessOutlivesItsPanelController() {
        val terminal = openTerminal()
        val session = terminal.terminalSession!!
        session.write("printf '\\033]0;build-watch\\007'; echo MARK-\$((40+2))\n")
        compose.waitUntil(10_000) { compose.onAllNodesWithText("build-watch").fetchSemanticsNodes().isNotEmpty() }
        compose.waitUntil(10_000) { output(terminal).contains("MARK-42") }
        // Leaving the session disposes the controllers; the terminal keeps running in the host.
        val first = store.state.value.activeSessionId!!
        runBlocking { store.createSession() }
        compose.waitForIdle()
        assertTrue(session.status.value is TerminalStatus.Running)
        runBlocking { store.activateSession(first) }
        compose.waitForIdle()
        val panel = workbench().panelsIn(Workbench.BOTTOM).single()
        val reattached = sessionRuntime().controller(panel) as TerminalController
        assertTrue(reattached !== terminal)
        assertEquals(session, reattached.terminalSession)
        assertTrue(output(reattached).contains("MARK-42"))
    }

    private fun editorCursor(path: String): Pair<Int, Int>? {
        val panel = workbench().panelFor(PanelTarget.File(path)) ?: return null
        val editor = (sessionRuntime().existing(panel.id) as? TextEditorController)?.editorView ?: return null
        return editor.cursor.leftLine to editor.cursor.leftColumn
    }
}
