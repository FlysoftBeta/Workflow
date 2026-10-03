package top.flysoftbeta.workflow.feature.files

import top.flysoftbeta.workflow.core.store.ReferenceWorkspaceStore

import androidx.activity.ComponentActivity
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.test.SemanticsNodeInteraction
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.getBoundsInRoot
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performImeAction
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.performTextReplacement
import androidx.compose.ui.test.performTouchInput
import androidx.test.ext.junit.runners.AndroidJUnit4
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
import top.flysoftbeta.workflow.app.panel.ReferencePanelRegistry
import top.flysoftbeta.workflow.core.io.MemoryFileSystem
import top.flysoftbeta.workflow.core.io.readText
import top.flysoftbeta.workflow.core.io.writeAtomic
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.Workbench
import top.flysoftbeta.workflow.core.resource.FileStatus
import top.flysoftbeta.workflow.core.store.WorkspaceStore
import top.flysoftbeta.workflow.feature.editor.TextEditorController
import top.flysoftbeta.workflow.feature.workbench.WorkbenchRuntime
import top.flysoftbeta.workflow.ui.design.dnd.DragDropState

/**
 * Explorer operations and editor Working-Resource behaviour through the real Workbench UI, against an
 * in-memory workspace (the app's real `.workspace` is never touched).
 */
@RunWith(AndroidJUnit4::class)
class FilesFeatureTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private val fs = MemoryFileSystem(System::currentTimeMillis)
    private lateinit var store: WorkspaceStore
    private lateinit var shell: Shell
    private lateinit var runtime: WorkbenchRuntime

    @Before fun setUp() {
        val context = compose.activity.applicationContext
        fs.writeAtomic("docs/a.md", "# A\n")
        fs.writeAtomic("docs/sub/b.md", "B\n")
        fs.writeAtomic(".hidden.txt", "h")
        fs.writeAtomic("notes.txt", "one\ntwo\n")
        store = ReferenceWorkspaceStore(fs, scope, Dispatchers.IO.limitedParallelism(1)).also { it.start() }
        runBlocking { store.awaitReady() }
        shell = Shell(context, store, ReferencePanelRegistry.create(context), scope)
        runtime = WorkbenchRuntime(context, store, shell.registry, shell)
        compose.setContent { WorkflowApp(shell, runtime, DragDropState()) }
        compose.onNodeWithText("工作台").performClick()
        compose.waitUntil(5_000) { shell.space == Space.WORKBENCH && store.state.value.activeSession != null }
        compose.waitUntil(5_000) { compose.onAllNodesWithText("docs").fetchSemanticsNodes().isNotEmpty() }
    }

    @After fun tearDown() {
        compose.runOnUiThread { if (::runtime.isInitialized) runtime.dispose() }
        scope.cancel()
    }

    private fun workbench(): Workbench = store.state.value.activeSession!!.workbench
    private fun explorer(): FilesExplorer = runtime.session(store.state.value.activeSessionId!!).explorerController() as FilesExplorer
    private fun exists(path: String) = fs.stat(path) != null
    private fun shown(text: String) = compose.onAllNodesWithText(text).fetchSemanticsNodes().isNotEmpty()

    @Test fun treeNestsFoldersAndHidesInternalFiles() {
        assertTrue(shown("docs"))
        assertTrue(shown("notes.txt"))
        assertTrue(!shown(".workspace"))
        assertTrue(!shown(".hidden.txt"))
        compose.onNodeWithText("docs").performClick()
        compose.waitUntil(5_000) { shown("a.md") }
        // A tree, not a flat list: children are indented under their folder.
        val folder = compose.onNodeWithText("docs", useUnmergedTree = true).getBoundsInRoot()
        val child = compose.onNodeWithText("a.md", useUnmergedTree = true).getBoundsInRoot()
        assertTrue(child.left > folder.left)
        assertEquals(listOf("docs"), workbench().explorer.expanded)
        compose.runOnUiThread { explorer().changeShowHidden(true) }
        compose.waitUntil(5_000) { shown(".hidden.txt") }
        assertTrue(!shown(".workspace"))
    }

    @Test fun inlineCreateRenameDeleteAndUndo() {
        compose.runOnUiThread { explorer().beginCreate("", false) }
        compose.onNodeWithTag("explorer:edit").performTextInput("a/b")
        compose.onNodeWithTag("explorer:edit").performImeAction()
        compose.waitUntil(5_000) { shown("不能包含 /") }
        compose.onNodeWithTag("explorer:edit").performTextReplacement("new.txt")
        compose.onNodeWithTag("explorer:edit").performImeAction()
        compose.waitUntil(5_000) { exists("new.txt") && workbench().panelFor(PanelTarget.File("new.txt")) != null }

        compose.runOnUiThread { explorer().beginRename("new.txt") }
        compose.onNodeWithTag("explorer:edit").performTextReplacement("renamed.txt")
        compose.onNodeWithTag("explorer:edit").performImeAction()
        compose.waitUntil(5_000) { exists("renamed.txt") && !exists("new.txt") }
        // The open editor follows the rename.
        compose.waitUntil(5_000) { workbench().panelFor(PanelTarget.File("renamed.txt")) != null }

        compose.runOnUiThread { explorer().delete("renamed.txt") }
        compose.waitUntil(5_000) { !exists("renamed.txt") && shown("撤销") }
        assertTrue(fs.list(".workspace/trash").isNotEmpty())
        compose.onNodeWithText("撤销").performClick()
        compose.waitUntil(5_000) { exists("renamed.txt") }
    }

    @Test fun draggingAFileOntoAFolderMovesItAfterConfirmation() {
        runBlocking { store.createDirectory("dest") }
        // The in-memory workspace has no file watcher: refresh like the watcher would.
        compose.runOnUiThread { explorer().refresh() }
        compose.waitUntil(5_000) { shown("dest") }
        // Index 0 is the row; once picked up, the drag shadow chip shows the name too.
        val source = compose.onAllNodesWithText("notes.txt")[0]
        val from = source.getBoundsInRoot()
        val to = compose.onNodeWithText("dest").getBoundsInRoot()
        val target = with(compose.density) { Offset((to.left - from.left).toPx() + 8f, ((to.top + to.bottom) / 2 - from.top).toPx()) }
        dragFrom(source, target)
        compose.waitUntil(5_000) { shown("移动到「dest」？") }
        compose.onNodeWithText("移动").performClick()
        compose.waitUntil(5_000) { exists("dest/notes.txt") && !exists("notes.txt") }
    }

    @Test fun editorKeepsDraftsAndNeverOverwritesExternalChanges() {
        val id = store.state.value.activeSessionId!!
        runBlocking { store.applyLayout(id, LayoutOp.Open(PanelTarget.File("notes.txt"))) }
        val editor = editorFor("notes.txt")
        compose.runOnUiThread { editor.editorView!!.setSelection(0, 3); editor.editorView!!.commitText("!") }
        compose.waitUntil(5_000) { store.state.value.drafts["notes.txt"]?.text == "one!\ntwo\n" }
        assertEquals(FileStatus.DIRTY, store.state.value.fileStatus("notes.txt"))
        assertEquals("one\ntwo\n", fs.readText("notes.txt"))

        // External change while a draft exists: conflict bar, save refused, disk untouched.
        fs.writeAtomic("notes.txt", "external\n")
        runBlocking { store.runMaintenance() }
        compose.waitUntil(5_000) { shown("磁盘版本已更改") }
        compose.onNodeWithText("对比").performClick()
        compose.waitUntil(5_000) { workbench().panelFor(PanelTarget.Diff("notes.txt")) != null }
        compose.runOnUiThread { editor.actions.first { it.key == "save" }.onClick() }
        compose.waitForIdle()
        assertEquals("external\n", fs.readText("notes.txt"))

        // Keep mine, then save writes the draft.
        runBlocking { store.applyLayout(id, LayoutOp.Open(PanelTarget.File("notes.txt"))) }
        compose.waitUntil(5_000) { shown("保留我的") }
        compose.onAllNodesWithText("保留我的")[0].performClick()
        compose.waitUntil(5_000) { store.state.value.fileStatus("notes.txt") == FileStatus.DIRTY }
        compose.runOnUiThread { editor.actions.first { it.key == "save" }.onClick() }
        compose.waitUntil(5_000) { fs.readText("notes.txt") == "one!\ntwo\n" && store.state.value.drafts["notes.txt"] == null }

        // A clean buffer follows the disk silently.
        fs.writeAtomic("notes.txt", "reloaded\n")
        runBlocking { store.runMaintenance() }
        compose.waitUntil(5_000) { editor.editorView!!.text.toString() == "reloaded\n" }
        assertEquals(FileStatus.CLEAN, store.state.value.fileStatus("notes.txt"))
    }

    @Test fun navigateMovesTheCursorToALinkTarget() {
        val id = store.state.value.activeSessionId!!
        runBlocking { store.applyLayout(id, LayoutOp.Open(PanelTarget.File("notes.txt"))) }
        val editor = editorFor("notes.txt")
        compose.runOnUiThread { runtime.session(id).commands.openFile("notes.txt", top.flysoftbeta.workflow.core.layout.TextCursor(1, 2)) }
        compose.waitUntil(5_000) { editor.editorView!!.cursor.leftLine == 1 && editor.editorView!!.cursor.leftColumn == 2 }
    }

    private fun editorFor(path: String): TextEditorController {
        compose.waitUntil(5_000) { workbench().panelFor(PanelTarget.File(path)) != null }
        val panel = workbench().panelFor(PanelTarget.File(path))!!
        compose.waitUntil(5_000) {
            val controller = runtime.session(store.state.value.activeSessionId!!).existing(panel.id) as? TextEditorController
            controller?.editorView != null
        }
        compose.onNodeWithTag("editor:$path").assertIsDisplayed()
        return runtime.session(store.state.value.activeSessionId!!).existing(panel.id) as TextEditorController
    }

    /** Long press (pickup after 300ms of virtual time), move in steps to [target] (node-local px), release. */
    private fun dragFrom(node: SemanticsNodeInteraction, target: Offset) {
        var start = Offset.Zero
        compose.mainClock.autoAdvance = false
        try {
            node.performTouchInput { start = center; down(center) }
            compose.mainClock.advanceTimeBy(500)
            node.performTouchInput {
                for (i in 1..12) {
                    advanceEventTime(16)
                    moveTo(start + (target - start) * (i / 12f))
                }
            }
            compose.mainClock.advanceTimeBy(100)
            node.performTouchInput { up() }
            compose.mainClock.advanceTimeBy(500)
        } finally {
            compose.mainClock.autoAdvance = true
        }
    }
}
