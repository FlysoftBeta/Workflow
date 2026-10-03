package top.flysoftbeta.workflow.feature.settings

import top.flysoftbeta.workflow.core.store.ReferenceWorkspaceStore

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
import kotlinx.coroutines.*
import org.junit.*
import org.junit.Assert.*
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.app.panel.*
import top.flysoftbeta.workflow.core.config.Density
import top.flysoftbeta.workflow.core.config.ThemeMode
import top.flysoftbeta.workflow.core.io.MemoryFileSystem
import top.flysoftbeta.workflow.core.layout.*
import top.flysoftbeta.workflow.core.store.WorkspaceStore
import top.flysoftbeta.workflow.ui.design.theme.WorkflowDesignTheme

@RunWith(AndroidJUnit4::class)
class SettingsFeatureTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private lateinit var store: WorkspaceStore
    private lateinit var controller: SettingsController
    private var width by mutableIntStateOf(390)
    private val messages = mutableListOf<String>()

    @Before fun setup() {
        store = ReferenceWorkspaceStore(MemoryFileSystem(), scope, Dispatchers.IO).also { it.start() }
        val session = runBlocking { store.awaitReady(); store.openInSeparateSession(PanelTarget.Settings) }
        val panel = store.state.value.activeSession!!.workbench.panels.values.single()
        val context = object : PanelContext {
            override val appContext = compose.activity.applicationContext
            override val store = this@SettingsFeatureTest.store
            override val sessionId = session
            override val panelId = panel.id
            override val target = panel.target
            override val initialView = PanelView()
            override val scope = this@SettingsFeatureTest.scope
            override fun layout(op: LayoutOp) { scope.launch { store.applyLayout(session, op) } }
            override val commands = object : WorkbenchCommands {
                override fun open(target: PanelTarget, placement: Placement) = Unit
                override fun openFile(path: String, cursor: TextCursor?) = Unit
                override fun attachToConversation(paths: List<String>) = Unit
                override fun pasteIntoTerminal(paths: List<String>) = Unit
                override fun newTerminal(directory: String?) = Unit
                override fun newConversation() = Unit
                override fun revealInExplorer(path: String) = Unit
                override fun snackbar(message: String, actionLabel: String?, onAction: (() -> Unit)?) { messages += message }
                override suspend fun decide(request: DecisionRequest): String? = null
            }
        }
        val services = object : SettingsFeatureServices {
            override val hub = top.flysoftbeta.workflow.platform.agent.AgentHub(context.appContext,
                store, scope, Dispatchers.IO)
            override val environment = kotlinx.coroutines.flow.MutableStateFlow<top.flysoftbeta.workflow.platform.engine.EnvironmentHealth>(
                top.flysoftbeta.workflow.platform.engine.EnvironmentHealth.NotInstalled)
            override fun retryEnvironment() = Unit
            override suspend fun restartEnvironment() = Unit
        }
        controller = SettingsController(context, services)
        compose.setContent { WorkflowDesignTheme { Surface(Modifier.width(width.dp)) { controller.Content(PanelFrame(true, PanelPlacement.SOLO, false, false), Modifier) } } }
    }
    @After fun cleanup() { runBlocking { store.close() }; scope.cancel() }

    private fun screenshot(name: String) {
        compose.waitForIdle()
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val output = java.io.File(instrumentation.targetContext.filesDir, "w8-acceptance/$name.png")
        output.parentFile!!.mkdirs()
        // Capture the settled Compose root, excluding Android's separately animated test Activity windows.
        val bitmap = compose.onRoot().captureToImage().asAndroidBitmap()
        output.outputStream().use { bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it) }
    }

    @Test fun narrowSettingsWriteThroughStoreAndBackStaysInsideSettings() {
        compose.onNodeWithText("外观").performClick()
        compose.onNodeWithText("主题").performClick()
        compose.onNodeWithText("深色").performClick()
        compose.waitUntil { store.state.value.config.appearance.theme == ThemeMode.DARK }
        assertEquals("外观", controller.tab.title)
        assertEquals("返回设置", controller.actions.single().label)
        compose.runOnUiThread { compose.activity.onBackPressedDispatcher.onBackPressed() }
        compose.onNodeWithText("关于").assertIsDisplayed()
        assertEquals("设置", controller.tab.title)
        assertTrue(controller.actions.isEmpty())
        screenshot("settings-narrow-categories")
    }

    @Test fun wideSettingsKeepCategoriesAndUpdateDensity() {
        compose.runOnUiThread { width = 820 }
        compose.onNodeWithText("密度").performClick()
        compose.onNodeWithText("标准").performClick()
        compose.waitUntil { store.state.value.config.appearance.density == Density.STANDARD }
        compose.onNodeWithText("关于").assertIsDisplayed()
        compose.onNodeWithText("主题").assertIsDisplayed()
        assertEquals("设置", controller.tab.title)
        assertTrue(controller.actions.isEmpty())
        screenshot("settings-wide-appearance")
    }

    @Test fun externalOverlayEntryOpensExtraApplications() {
        compose.runOnUiThread { top.flysoftbeta.workflow.platform.SettingsEntryPoint.destination.value = top.flysoftbeta.workflow.platform.SettingsEntryPoint.Destination.OVERLAY_APPS }
        compose.onNodeWithText("搜索应用").assertIsDisplayed()
        compose.onNodeWithText("完成").performClick()
        compose.onNodeWithText("重置位置").assertIsDisplayed()
        assertEquals(Category.OVERLAY, controller.selected)
    }

    @Test fun fontSizeCommitsAndAccountsStayDisabledBeforeEnvironmentIsReady() {
        compose.onNodeWithText("外观").performClick()
        compose.onNode(SemanticsMatcher.keyIsDefined(androidx.compose.ui.semantics.SemanticsActions.SetProgress))
            .performSemanticsAction(androidx.compose.ui.semantics.SemanticsActions.SetProgress) { it(18f) }
        compose.waitUntil { store.state.value.config.appearance.monoFontSize == 18.0 }
        compose.onNodeWithText("18 sp").assertIsDisplayed()
        compose.runOnUiThread { compose.activity.onBackPressedDispatcher.onBackPressed() }
        compose.onNodeWithText("账户").performClick()
        compose.onAllNodesWithText("等待环境").assertCountEquals(2)
        compose.onAllNodesWithText("登录").assertCountEquals(2)
        compose.onAllNodesWithText("登录")[0].assertIsNotEnabled()
        compose.onAllNodesWithText("登录")[1].assertIsNotEnabled()
        screenshot("settings-accounts-environment-gate")
        compose.runOnUiThread { compose.activity.onBackPressedDispatcher.onBackPressed() }
        compose.onNodeWithText("环境").performClick()
        compose.onNodeWithText("环境状态").assertIsDisplayed()
        assertFalse(controller.services.environment.value.usable)
        compose.onNodeWithText("重启环境").assertDoesNotExist()
        screenshot("settings-environment-not-ready")
    }

    @Test fun permissionsHomeAndAboutExposeMeasuredStateAndBundledLicenses() {
        compose.onNodeWithText("权限").performClick()
        listOf("悬浮窗", "修改系统设置", "黑白", "锁屏", "Root", "通知").forEach {
            compose.onNodeWithText(it).assertIsDisplayed()
        }
        screenshot("settings-permissions")
        compose.runOnUiThread { compose.activity.onBackPressedDispatcher.onBackPressed() }
        compose.onNodeWithText("默认桌面").performClick()
        val context = compose.activity
        val homeIntent = android.content.Intent(android.content.Intent.ACTION_MAIN).addCategory(android.content.Intent.CATEGORY_HOME)
        val isDefault = context.packageManager.resolveActivity(homeIntent, android.content.pm.PackageManager.MATCH_DEFAULT_ONLY)?.activityInfo?.packageName == context.packageName
        compose.onNodeWithText(if (isDefault) "Workflow" else "其他桌面").assertIsDisplayed()
        compose.runOnUiThread { compose.activity.onBackPressedDispatcher.onBackPressed() }
        compose.onNodeWithText("关于").performClick()
        compose.waitUntil { compose.onAllNodesWithText("1.0.0").fetchSemanticsNodes().size == 1 }
        screenshot("settings-about")
        compose.onNodeWithText("开源许可").performClick()
        compose.waitUntil { compose.onAllNodesWithText("Apache", substring = true).fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithText("关闭").performClick()
    }
}
