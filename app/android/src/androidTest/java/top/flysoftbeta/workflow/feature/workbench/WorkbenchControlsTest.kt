package top.flysoftbeta.workflow.feature.workbench

import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.SemanticsNodeInteraction
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.ui.design.RegionHeader
import top.flysoftbeta.workflow.ui.design.ToolAction
import top.flysoftbeta.workflow.ui.design.WfIconButton
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.ThemeMode
import top.flysoftbeta.workflow.ui.design.theme.UiDensity
import top.flysoftbeta.workflow.ui.design.theme.WorkflowDesignTheme
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** Isolated UI checks: the callbacks stand in for authoritative layout commands, without an engine. */
@RunWith(AndroidJUnit4::class)
class WorkbenchControlsTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    private fun SemanticsNodeInteraction.customAction(label: String, handled: Boolean = true) {
        val action = fetchSemanticsNode().config[SemanticsActions.CustomActions].single { it.label == label }
        compose.runOnIdle { assertEquals("Action: $label", handled, action.action()) }
        compose.waitForIdle()
    }

    @Test fun regionActionsResizeWithinLimitsCollapseAndResetTheCorrectRegion() {
        var sideWidth by mutableStateOf<Dp?>(240.dp)
        var auxWidth by mutableStateOf<Dp?>(300.dp)
        val resized = mutableListOf<Pair<Boolean, Float>>()
        val collapsed = mutableListOf<Boolean>()
        val reset = mutableListOf<Boolean>()
        val dragging = mutableListOf<Boolean>()
        compose.setContent {
            WorkflowDesignTheme(ThemeMode.Light) {
                RegionRow(
                    sideWidth = sideWidth,
                    auxWidth = auxWidth,
                    sideLimits = 200f..250f,
                    auxLimits = 200f..360f,
                    onResize = { side, width ->
                        resized += side to width
                        if (side) sideWidth = width.dp else auxWidth = width.dp
                    },
                    onCollapse = { side ->
                        collapsed += side
                        if (side) sideWidth = null else auxWidth = null
                    },
                    onReset = { side ->
                        reset += side
                        if (side) sideWidth = 240.dp else auxWidth = 300.dp
                    },
                    onResizing = { dragging += it },
                    side = { Box(Modifier.fillMaxSize().testTag("side-content")) },
                    center = {},
                    aux = {},
                    modifier = Modifier.size(900.dp, 320.dp),
                )
            }
        }

        val controls = compose.onNodeWithContentDescription("调整区域布局")
        controls.customAction("增大侧栏")
        controls.customAction("增大侧栏", handled = false)
        controls.customAction("减小辅助区")
        controls.customAction("恢复辅助区默认宽度")
        controls.customAction("收起侧栏")
        compose.runOnIdle {
            assertEquals(listOf(true to 250f, false to 276f), resized)
            assertEquals(listOf(false), reset)
            assertEquals(listOf(true), collapsed)
            assertEquals(null, sideWidth)
            assertEquals(300.dp, auxWidth)
            assertTrue("Accessibility adjustments must not enter pointer-drag mode", dragging.isEmpty())
        }
        val labels = controls.fetchSemanticsNode().config[SemanticsActions.CustomActions].map { it.label }
        assertFalse("A collapsed side has no visible seam to adjust", labels.any { "侧栏" in it })
    }

    @Test fun splitActionsChangeAdjacentSizesAndResetWithoutDragging() {
        var weights by mutableStateOf(listOf(0.5, 0.5))
        val committed = mutableListOf<List<Double>>()
        var resets = 0
        compose.setContent {
            WorkflowDesignTheme(ThemeMode.Light) {
                WeightedSplit(
                    horizontal = true,
                    weights = weights,
                    fixed = emptyMap(),
                    minSize = 80.dp,
                    onCommit = { committed += it; weights = it },
                    onReset = { resets++; weights = listOf(0.5, 0.5) },
                    onResizing = {},
                    children = listOf(
                        { Box(Modifier.fillMaxSize().testTag("first-pane")) },
                        { Box(Modifier.fillMaxSize().testTag("second-pane")) },
                    ),
                    modifier = Modifier.size(600.dp, 240.dp),
                )
            }
        }

        val firstBefore = compose.onNodeWithTag("first-pane").fetchSemanticsNode().boundsInRoot.width
        val secondBefore = compose.onNodeWithTag("second-pane").fetchSemanticsNode().boundsInRoot.width
        val controls = compose.onNodeWithContentDescription("调整横向拆分")
        controls.customAction("增大第 1 个窗格")
        val firstAfter = compose.onNodeWithTag("first-pane").fetchSemanticsNode().boundsInRoot.width
        val secondAfter = compose.onNodeWithTag("second-pane").fetchSemanticsNode().boundsInRoot.width
        assertTrue(firstAfter > firstBefore)
        assertTrue(secondAfter < secondBefore)
        assertEquals(firstBefore + secondBefore, firstAfter + secondAfter, 1f)
        compose.runOnIdle {
            assertEquals(1, committed.size)
            assertTrue(committed.single()[0] > 0.5)
            assertEquals(1.0, committed.single().sum(), 0.000001)
        }
        controls.customAction("减小第 1 个窗格")
        controls.customAction("恢复拆分比例")
        compose.runOnIdle {
            assertEquals(2, committed.size)
            assertTrue(committed[1][0] < committed[0][0])
            assertEquals(1, resets)
            assertEquals(listOf(0.5, 0.5), weights)
        }
    }

    @Test fun bottomActionsResizeCollapseExpandAndReset() {
        var fraction by mutableStateOf(0.35f)
        var collapsed by mutableStateOf(false)
        val resized = mutableListOf<Float>()
        val collapsedChanges = mutableListOf<Boolean>()
        var resets = 0
        compose.setContent {
            WorkflowDesignTheme(ThemeMode.Light) {
                CenterColumn(
                    bottomFraction = fraction,
                    bottomCollapsed = collapsed,
                    collapsedHeight = 36.dp,
                    editorMin = 120.dp,
                    imeFocus = null,
                    onResize = { resized += it; fraction = it },
                    onCollapse = { collapsedChanges += it; collapsed = it },
                    onReset = { resets++; fraction = 0.35f; collapsed = false },
                    onResizing = {},
                    editor = {},
                    bottom = { Box(Modifier.fillMaxSize().testTag("bottom-content")) },
                    modifier = Modifier.size(320.dp, 420.dp),
                )
            }
        }

        val initialHeight = compose.onNodeWithTag("bottom-content").fetchSemanticsNode().boundsInRoot.height
        val controls = compose.onNodeWithContentDescription("调整编辑器与底部区域")
        controls.customAction("增大底部区域")
        val enlargedHeight = compose.onNodeWithTag("bottom-content").fetchSemanticsNode().boundsInRoot.height
        assertTrue(enlargedHeight > initialHeight)
        controls.customAction("收起底部区域")
        assertTrue(compose.onNodeWithTag("bottom-content").fetchSemanticsNode().boundsInRoot.height < initialHeight)
        controls.customAction("展开底部区域")
        assertEquals(enlargedHeight, compose.onNodeWithTag("bottom-content").fetchSemanticsNode().boundsInRoot.height, 1f)
        controls.customAction("恢复底部区域默认大小")
        compose.runOnIdle {
            assertEquals(1, resized.size)
            assertTrue(resized.single() > 0.35f)
            assertEquals(listOf(true, false), collapsedChanges)
            assertEquals(1, resets)
            assertEquals(0.35f, fraction, 0.0001f)
            assertFalse(collapsed)
        }
    }

    @Test fun imeForcedSizesExposeNoPersistedResizeActions() {
        var imeFocus by mutableStateOf<ImeFocus?>(ImeFocus.EDITOR)
        var callbacks = 0
        compose.setContent {
            WorkflowDesignTheme(ThemeMode.Light) {
                CenterColumn(
                    bottomFraction = 0.35f,
                    bottomCollapsed = false,
                    collapsedHeight = 36.dp,
                    editorMin = 120.dp,
                    imeFocus = imeFocus,
                    onResize = { callbacks++ },
                    onCollapse = { callbacks++ },
                    onReset = { callbacks++ },
                    onResizing = {},
                    editor = {
                        WeightedSplit(
                            horizontal = false,
                            weights = listOf(0.5, 0.5),
                            fixed = if (imeFocus != null) mapOf(1 to 36.dp) else emptyMap(),
                            minSize = 80.dp,
                            onCommit = { callbacks++ },
                            onReset = { callbacks++ },
                            onResizing = {},
                            children = listOf({}, {}),
                            modifier = Modifier.fillMaxSize().testTag("ime-split"),
                        )
                    },
                    bottom = {},
                    modifier = Modifier.size(320.dp, 420.dp).testTag("ime-column"),
                )
            }
        }

        for (focus in listOf(ImeFocus.EDITOR, ImeFocus.BOTTOM)) {
            compose.runOnIdle { imeFocus = focus }
            assertFalse(compose.onNodeWithTag("ime-column").fetchSemanticsNode().config.contains(SemanticsActions.CustomActions))
            assertFalse(compose.onNodeWithTag("ime-split").fetchSemanticsNode().config.contains(SemanticsActions.CustomActions))
        }
        compose.runOnIdle { assertEquals(0, callbacks); imeFocus = null }
        compose.onNodeWithContentDescription("调整编辑器与底部区域").assertExists()
        compose.onNodeWithContentDescription("调整纵向拆分").assertExists()
    }

    @Test fun narrowHeaderKeepsScaledTitleReadableAndOverflowActionsReachable() {
        var titleLayout: TextLayoutResult? = null
        val invoked = mutableListOf<Int>()
        compose.setContent {
            WorkflowDesignTheme(ThemeMode.Light, UiDensity.Standard) {
                val density = LocalDensity.current
                CompositionLocalProvider(LocalDensity provides Density(density.density, 1.3f)) {
                    Box(Modifier.width(320.dp).height(64.dp)) {
                        RegionHeader(
                            leading = { WfIconButton(Sym.Home, "测试主页", {}) },
                            title = {
                                Text(
                                    "当前工作区",
                                    style = WorkflowTheme.text.titleSm,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                    onTextLayout = { titleLayout = it },
                                )
                            },
                            actions = (1..8).map { index ->
                                ToolAction("action:$index", Sym.Add, "操作 $index", onClick = { invoked += index })
                            },
                            trailing = { WfIconButton(Sym.Settings, "测试设置", {}) },
                        )
                    }
                }
            }
        }

        compose.onNodeWithText("当前工作区").assertIsDisplayed()
        compose.onNodeWithContentDescription("测试主页").assertIsDisplayed()
        compose.onNodeWithContentDescription("测试设置").assertIsDisplayed()
        compose.runOnIdle {
            assertNotNull(titleLayout)
            assertFalse("The title must remain readable at 320dp and 1.3× font scale", titleLayout!!.hasVisualOverflow)
        }
        compose.onNodeWithContentDescription("操作 1").assertIsDisplayed().performClick()
        compose.onNodeWithContentDescription("操作 8").assertDoesNotExist()
        compose.onNodeWithText("操作 8").assertDoesNotExist()
        compose.onNodeWithContentDescription("更多").assertIsDisplayed().performClick()
        compose.onNodeWithText("操作 8").performScrollTo().assertIsDisplayed().performClick()
        compose.runOnIdle { assertEquals(listOf(1, 8), invoked) }
    }
}
