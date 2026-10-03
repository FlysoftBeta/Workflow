package top.flysoftbeta.workflow.ui.design

import android.graphics.Bitmap
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.key
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.ThemeMode
import top.flysoftbeta.workflow.ui.design.theme.WorkflowDesignTheme
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** Run on the narrow API 28 AVD; full-window captures include the popup shadow. */
@RunWith(AndroidJUnit4::class)
class MenuSurfaceRegressionTest {
    @get:Rule val compose = createComposeRule()

    @Test fun launcherDragKeepsCaptionAndBadgeVisible() {
        var moved: Pair<Int, Int>? = null
        compose.setContent {
            WorkflowDesignTheme(themeMode = ThemeMode.Light) {
                Column(Modifier.fillMaxSize().background(WorkflowTheme.colors.surface).padding(16.dp)) {
                    SessionChip("周六 15:11", {}, {}, Modifier.heightIn(min = 48.dp))
                    ReorderableAppGrid(listOf("工作台", "代理", "设置"), { it },
                        { from, to -> moved = from to to }, {}) { label ->
                        AppGridCell(label, {}, badge = label == "工作台") { BuiltInAppIcon(Sym.SpaceDashboard) }
                    }
                }
            }
        }
        compose.onNodeWithText("工作台").performTouchInput { down(center) }
        compose.mainClock.advanceTimeBy(400)
        compose.waitForIdle()
        compose.onNodeWithText("工作台").performTouchInput { moveBy(Offset(280f, 0f)) }
        compose.waitForIdle()
        compose.onNodeWithText("工作台").assertIsDisplayed()
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val output = File(instrumentation.targetContext.getExternalFilesDir(null), "screenshots/menu-regression").apply { mkdirs() }
        File(output, "launcher-drag.png").outputStream().use {
            instrumentation.uiAutomation.takeScreenshot()!!.compress(Bitmap.CompressFormat.PNG, 100, it)
        }
        compose.onNodeWithText("工作台").performTouchInput { up() }
        compose.runOnIdle { assertTrue("drag should reorder the first tile", moved == (0 to 1)) }
    }

    @Test fun supportingDescriptionsRemainVisibleAndUntruncated() {
        val descriptions = listOf("运行命令或修改文件前询问", "可直接修改工作区文件，命令仍需确认", "只讨论与规划，不执行", "不询问，未允许的操作一律拒绝")
        val titles = listOf("需确认", "自动编辑", "仅规划", "仅限已允许")
        var theme by mutableStateOf(ThemeMode.Light)
        var permissions by mutableStateOf(true)
        compose.setContent {
            key(theme, permissions) {
            WorkflowDesignTheme(themeMode = theme) {
                Box(Modifier.fillMaxSize().background(WorkflowTheme.colors.surface), contentAlignment = Alignment.Center) {
                    Box(Modifier.padding(24.dp)) {
                        Text("其他方式", color = WorkflowTheme.colors.onSurface)
                        CompositeMenu(true, {}, listOf(MenuGroup("choices", if (permissions) {
                            titles.mapIndexed { index, title ->
                                MenuEntry.Action("$index", title, if (index == 0) Sym.Check else null,
                                    supportingText = descriptions[index]) {}
                            }
                        } else listOf(
                            MenuEntry.Action("browser", "在浏览器中登录", Sym.OpenInNew) {},
                            MenuEntry.Action("key", "使用 API Key", Sym.Key) {},
                        ))))
                    }
                }
            }
            }
        }
        for (mode in listOf(ThemeMode.Light, ThemeMode.Dark)) {
            for (showPermissions in listOf(true, false)) {
                compose.runOnIdle { theme = mode; permissions = showPermissions }
                compose.waitForIdle()
                val instrumentation = InstrumentationRegistry.getInstrumentation()
                val output = File(instrumentation.targetContext.getExternalFilesDir(null), "screenshots/menu-regression").apply { mkdirs() }
                val bitmap = instrumentation.uiAutomation.takeScreenshot()
                assertTrue(bitmap != null)
                File(output, "${mode.name}-$showPermissions.png").outputStream().use { bitmap!!.compress(Bitmap.CompressFormat.PNG, 100, it) }
                if (showPermissions) descriptions.forEach { description ->
                    val layouts = mutableListOf<TextLayoutResult>()
                    compose.onNodeWithText(description, useUnmergedTree = true).assertIsDisplayed().performSemanticsAction(SemanticsActions.GetTextLayoutResult) { it(layouts) }
                    assertTrue(layouts.isNotEmpty())
                    layouts.forEach { layout ->
                        // Text widths are rounded to integer pixels; allow that rounding, not truncation.
                        assertTrue(layout.getLineEnd(layout.lineCount - 1) == description.length)
                        for (line in 0 until layout.lineCount) {
                            assertFalse(layout.isLineEllipsized(line))
                            assertTrue(layout.getLineRight(line) <= layout.size.width + 1f)
                            assertTrue(layout.getLineBottom(line) <= layout.size.height + 1f)
                        }
                    }
                } else compose.onNodeWithText("使用 API Key").assertIsDisplayed()

            }
        }
    }
}
