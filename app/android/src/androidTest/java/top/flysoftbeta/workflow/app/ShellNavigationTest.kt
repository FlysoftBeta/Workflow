package top.flysoftbeta.workflow.app

import top.flysoftbeta.workflow.core.store.ReferenceWorkspaceStore

import androidx.activity.ComponentActivity
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.test.SemanticsNodeInteraction
import androidx.compose.ui.test.getBoundsInRoot
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.app.panel.ReferencePanelRegistry
import top.flysoftbeta.workflow.core.io.MemoryFileSystem
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.Region
import top.flysoftbeta.workflow.core.layout.Workbench
import top.flysoftbeta.workflow.core.store.WorkspaceStore
import top.flysoftbeta.workflow.feature.workbench.WorkbenchRuntime
import top.flysoftbeta.workflow.ui.design.dnd.DragDropState

/**
 * Shell navigation (product.md §2, ui.md §2.3) and drag-and-drop commits (ui.md §4.10) against an
 * in-memory workspace: nothing touches the app's real `.workspace`.
 */
@RunWith(AndroidJUnit4::class)
class ShellNavigationTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private lateinit var store: WorkspaceStore
    private lateinit var shell: Shell
    private lateinit var runtime: WorkbenchRuntime

    @Before fun setUp() {
        val context = compose.activity.applicationContext
        store = ReferenceWorkspaceStore(MemoryFileSystem(System::currentTimeMillis), scope, Dispatchers.IO.limitedParallelism(1)).also { it.start() }
        runBlocking { store.awaitReady() }
        shell = Shell(context, store, ReferencePanelRegistry.create(context), scope)
        runtime = WorkbenchRuntime(context, store, shell.registry, shell)
        val dnd = DragDropState()
        compose.setContent { WorkflowApp(shell, runtime, dnd) }
    }

    @After fun tearDown() {
        compose.runOnUiThread { if (::runtime.isInitialized) runtime.dispose() }
        scope.cancel()
    }

    private fun back() = compose.runOnUiThread { compose.activity.onBackPressedDispatcher.onBackPressed() }

    private fun workbench(): Workbench = store.state.value.activeSession!!.workbench

    private fun enterWorkbench() {
        compose.onNodeWithText("工作台").performClick()
        compose.waitUntil(5_000) { shell.space == Space.WORKBENCH && store.state.value.activeSession != null }
        compose.waitForIdle()
    }

    @Test fun backOnLauncherDoesNothing() {
        assertEquals(Space.LAUNCHER, shell.space)
        back()
        compose.waitForIdle()
        assertEquals(Space.LAUNCHER, shell.space)
        assertFalse(compose.activity.isFinishing)
    }

    @Test fun backInWorkbenchReturnsToLauncherAndKeepsTheSession() {
        enterWorkbench()
        val session = store.state.value.activeSessionId
        runBlocking { store.applyLayout(session!!, LayoutOp.Open(PanelTarget.Settings)) }
        compose.waitForIdle()
        val before = workbench()
        back()
        compose.waitForIdle()
        assertEquals(Space.LAUNCHER, shell.space)
        assertEquals(session, store.state.value.activeSessionId)
        assertEquals(before, workbench())
        // The Workbench tile returns to the same session, layout untouched.
        enterWorkbench()
        assertEquals(session, store.state.value.activeSessionId)
        assertEquals(before, workbench())
    }

    @Test fun backClosesTheSessionSheetBeforeLeavingTheWorkbench() {
        enterWorkbench()
        compose.runOnUiThread { shell.sessionsOpen = true }
        compose.waitForIdle()
        back()
        compose.waitForIdle()
        assertFalse(shell.sessionsOpen)
        assertEquals(Space.WORKBENCH, shell.space)
        back()
        compose.waitForIdle()
        assertEquals(Space.LAUNCHER, shell.space)
    }

    @Test fun homeReturnsToLauncherAndProxyOpensInTheCurrentSession() {
        enterWorkbench()
        compose.runOnUiThread { shell.goHome() }
        compose.waitForIdle()
        assertEquals(Space.LAUNCHER, shell.space)
        val session = store.state.value.activeSessionId
        compose.onNodeWithText("代理").performClick()
        compose.waitUntil(5_000) { shell.space == Space.WORKBENCH }
        compose.waitUntil(5_000) { workbench().panelFor(PanelTarget.Proxy()) != null }
        assertEquals(session, store.state.value.activeSessionId)
    }

    @Test fun draggingATabToAStackEdgeSplitsInOneCommit() {
        enterWorkbench()
        val id = store.state.value.activeSessionId!!
        runBlocking {
            store.createFile("a.txt", "alpha".toByteArray())
            store.createFile("b.txt", "beta".toByteArray())
            // Only the tabs show the file names (the explorer would list them too).
            store.applyLayout(id, LayoutOp.SetRegionCollapsed(Region.EXPLORER, true))
            store.applyLayout(id, LayoutOp.Open(PanelTarget.File("a.txt")))
            store.applyLayout(id, LayoutOp.Open(PanelTarget.File("b.txt")))
        }
        compose.waitForIdle()
        assertEquals(1, workbench().editorStacks.size)
        val stack = workbench().editorStacks.single()
        val content = compose.onNodeWithTag("stack:$stack").getBoundsInRoot()
        val tab = compose.onNodeWithText("b.txt").getBoundsInRoot()
        // Target: inside the stack content, 12dp from its right edge (the RIGHT zone), in the tab's local space.
        val target = with(compose.density) {
            Offset((content.right - 12.dp - tab.left).toPx(), ((content.top + content.bottom) / 2 - tab.top).toPx())
        }
        // Index 0 is the tab; once picked up, the drag shadow chip shows the name too.
        dragFrom(compose.onAllNodesWithText("b.txt")[0], target)
        compose.waitUntil(5_000) { workbench().editorStacks.size == 2 }
        val wb = workbench()
        assertEquals(listOf("b.txt"), wb.panelsIn(wb.editorStacks.last()).map { (it.target as PanelTarget.File).path })
        assertEquals(listOf("a.txt"), wb.panelsIn(wb.editorStacks.first()).map { (it.target as PanelTarget.File).path })
        assertTrue(wb.violations().isEmpty())
    }

    @Test fun draggingATabWithinItsRowReorders() {
        enterWorkbench()
        val id = store.state.value.activeSessionId!!
        runBlocking {
            store.applyLayout(id, LayoutOp.Open(PanelTarget.Settings))
            store.applyLayout(id, LayoutOp.Open(PanelTarget.Proxy()))
        }
        compose.waitForIdle()
        val stack = workbench().editorStacks.single()
        fun keys() = workbench().panelsIn(stack).map { it.target.key }
        assertEquals(listOf("settings", "proxy:overview"), keys())
        // The tab row precedes the content, so the first "代理" node is the tab.
        val proxyTab = compose.onAllNodesWithText("代理", useUnmergedTree = true)[0]
        val from = proxyTab.getBoundsInRoot()
        val settings = compose.onNodeWithText("设置", useUnmergedTree = true).getBoundsInRoot()
        val target = with(compose.density) { Offset((settings.left + 4.dp - from.left).toPx(), ((from.bottom - from.top) / 2).toPx()) }
        dragFrom(proxyTab, target)
        compose.waitUntil(5_000) { keys() == listOf("proxy:overview", "settings") }
    }

    /** Long press (pickup after 300ms of virtual time), move in steps to [target] (node-local px), release. */
    private fun dragFrom(node: SemanticsNodeInteraction, target: Offset) {
        var start = Offset.Zero
        // While a drag is active the host runs a frame loop (edge auto-scroll): drive the clock by hand.
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
