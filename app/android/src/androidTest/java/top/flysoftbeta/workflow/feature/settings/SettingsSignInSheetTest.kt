package top.flysoftbeta.workflow.feature.settings

import androidx.activity.ComponentActivity
import androidx.compose.material3.Surface
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.hasSetTextAction
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.LifecycleRegistry
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.agent.model.AccountState
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.LoginFlow
import top.flysoftbeta.workflow.agent.model.LoginMethod
import top.flysoftbeta.workflow.agent.model.LoginState
import top.flysoftbeta.workflow.ui.design.AccountCommands
import top.flysoftbeta.workflow.ui.design.SignInContent
import top.flysoftbeta.workflow.ui.design.SignInState
import top.flysoftbeta.workflow.ui.design.theme.WorkflowDesignTheme
import java.util.concurrent.CopyOnWriteArrayList

/**
 * The sign-in content of the Settings sheet, which is the conversation's shared [SignInContent] over
 * [SignInState]: it shows the LoginView phases and issues commands only on a tap.
 */
@RunWith(AndroidJUnit4::class)
class SettingsSignInSheetTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private var account by mutableStateOf(AccountState())
    private var ready by mutableStateOf(true)
    private val calls = CopyOnWriteArrayList<String>()
    @Volatile private var gate: CompletableDeferred<Unit>? = null
    private lateinit var signIn: SignInState
    /** The sheet's lifecycle, moved explicitly: ON_RESUME means the app came back, for example from the browser. */
    private val owner = object : LifecycleOwner {
        val registry = LifecycleRegistry(this)
        override val lifecycle: Lifecycle get() = registry
    }
    private val commands = object : AccountCommands {
        override suspend fun login(kind: BackendKind, method: LoginMethod, secret: String?) {
            calls += "login:${method.name}:${if (secret == null) "interactive" else "secret"}"
            gate?.await()
        }
        override suspend fun cancelLogin(kind: BackendKind, loginId: String) { calls += "cancel:$loginId" }
        override suspend fun refreshAccount(kind: BackendKind) { calls += "refresh" }
    }

    @Before fun setUp() {
        signIn = SignInState(BackendKind.CODEX, commands, scope) { account }
        compose.runOnUiThread { owner.registry.currentState = Lifecycle.State.STARTED }
        compose.setContent {
            CompositionLocalProvider(LocalLifecycleOwner provides owner) {
                WorkflowDesignTheme {
                    Surface {
                        SignInContent(
                            signIn, account, listOf(LoginMethod.CODEX_DEVICE_CODE, LoginMethod.CODEX_BROWSER, LoginMethod.CODEX_API_KEY),
                            ready, openUrl = { true }, copy = { _, _ -> }, openTerminal = {},
                        )
                    }
                }
            }
        }
    }

    @After fun tearDown() { gate?.complete(Unit); scope.cancel() }

    private fun set(next: AccountState) { compose.runOnUiThread { account = next }; compose.waitForIdle() }
    private fun shown(text: String) = compose.onAllNodes(androidx.compose.ui.test.hasText(text)).fetchSemanticsNodes().isNotEmpty()

    @Test fun phasesFollowTheSharedAccountProjectionAndNothingStartsWithoutATap() {
        // Checking: only an indicator, no choices.
        compose.waitForIdle()
        assertTrue(!shown("登录 Codex"))
        set(AccountState(state = LoginState.LOGGED_OUT))
        compose.onNodeWithText("登录 Codex").assertIsEnabled()
        compose.onNodeWithText("其他方式").performClick()
        compose.onNodeWithText("在浏览器中登录").assertIsDisplayed()
        compose.onNodeWithText("使用 API Key").assertIsDisplayed()
        compose.runOnUiThread { ready = false }
        compose.onNodeWithText("登录 Codex").assertIsNotEnabled()
        compose.onNodeWithText("其他方式").assertIsNotEnabled()
        compose.runOnUiThread { ready = true }
        assertEquals("rendering issues no command", emptyList<String>(), calls.toList())

        // The login command is in flight before the Engine reports the attempt: busy without Cancel.
        gate = CompletableDeferred()
        compose.onNodeWithText("登录 Codex").performClick()
        compose.waitUntil { calls.contains("login:CODEX_DEVICE_CODE:interactive") }
        compose.onNodeWithText("正在开始登录…").assertIsDisplayed()
        assertTrue(!shown("取消"))
        // Starting: the Engine's pending attempt can be cancelled.
        set(AccountState(state = LoginState.LOGGING_IN, login = LoginFlow.Progress("attempt-1", emptyList())))
        compose.onNodeWithText("正在开始登录…").assertIsDisplayed()
        compose.onNodeWithText("取消").performClick()
        compose.waitUntil { calls.contains("cancel:attempt-1") }
        gate!!.complete(Unit)
        // Waiting: the device code stays with Cancel; a failing background check is a caption, not a failure.
        set(AccountState(state = LoginState.LOGGING_IN, login = LoginFlow.DeviceCode("attempt-1", "https://auth.example.test/device", "ABCD-1234"),
            checkError = "account read timed out"))
        compose.onNodeWithText("ABCD-1234").assertIsDisplayed()
        compose.onNodeWithText("等待确认…").assertIsDisplayed()
        compose.onNodeWithText("暂时无法确认登录状态，正在重试：account read timed out").assertIsDisplayed()
        assertTrue(!shown("登录没有完成"))
        compose.onNodeWithText("取消").performClick()
        compose.waitUntil { calls.count { it == "cancel:attempt-1" } == 2 }

        // Failed: the vendor reason with Retry (the last method) and Check again.
        set(AccountState(state = LoginState.LOGGED_OUT, login = LoginFlow.Completed("attempt-1", false, "workspace routing discovery timed out")))
        compose.onNodeWithText("登录没有完成").assertIsDisplayed()
        compose.onNodeWithText("workspace routing discovery timed out").assertIsDisplayed()
        compose.onNodeWithText("重试").performClick()
        compose.waitUntil { calls.count { it == "login:CODEX_DEVICE_CODE:interactive" } == 2 }
        compose.onNodeWithText("重新检查").performClick()
        compose.waitUntil { calls.contains("refresh") }

        // A cancelled attempt is never shown as a failure.
        set(AccountState(state = LoginState.LOGGED_OUT, login = LoginFlow.Completed("attempt-2", false, "cancelled")))
        assertTrue(!shown("登录没有完成"))
        compose.onNodeWithText("登录 Codex").assertIsEnabled()

        // Account unreadable: the reason and Retry above the choices.
        set(AccountState(state = LoginState.UNKNOWN, checkError = "account/read failed"))
        compose.onNodeWithText("无法读取 Codex 账户状态").assertIsDisplayed()
        compose.onNodeWithText("account/read failed").assertIsDisplayed()
        compose.onNodeWithText("登录 Codex").assertIsDisplayed()
        compose.onNodeWithText("重试").performClick()
        compose.waitUntil { calls.count { it == "refresh" } == 2 }

        // Browser flow.
        set(AccountState(state = LoginState.LOGGING_IN, login = LoginFlow.Browser("attempt-3", "https://auth.example.test/authorize")))
        compose.onNodeWithText("在浏览器中继续").assertIsDisplayed()
        compose.onNodeWithText("取消").performClick()
        compose.waitUntil { calls.contains("cancel:attempt-3") }

        // A secret method submits once and is never offered again by Retry.
        set(AccountState(state = LoginState.LOGGED_OUT))
        compose.onNodeWithText("其他方式").performClick()
        compose.onNodeWithText("使用 API Key").performClick()
        compose.onNode(hasSetTextAction()).performTextInput("fixture-not-a-key")
        compose.onNodeWithText("登录").performClick()
        compose.waitUntil { calls.contains("login:CODEX_API_KEY:secret") }
        assertEquals(LoginMethod.CODEX_DEVICE_CODE, signIn.lastMethod)
        assertTrue(calls.none { it.contains("fixture-not-a-key") })
    }

    @Test fun returningToTheAppRereadsTheAccountUntilSignedIn() {
        set(AccountState(state = LoginState.LOGGING_IN, login = LoginFlow.DeviceCode("attempt-1", "https://auth.example.test/device", "ABCD-1234")))
        assertEquals(emptyList<String>(), calls.toList())
        // The browser was in front; the app returns. A second return during the re-reads starts no second series.
        compose.runOnUiThread { owner.registry.currentState = Lifecycle.State.RESUMED }
        compose.waitUntil(2_000) { calls.count { it == "refresh" } == 1 }
        compose.runOnUiThread { owner.registry.currentState = Lifecycle.State.STARTED; owner.registry.currentState = Lifecycle.State.RESUMED }
        compose.waitUntil(4_000) { calls.count { it == "refresh" } == 2 }
        // Signed in before the last re-read at five seconds: it is skipped.
        set(AccountState(state = LoginState.LOGGED_IN, email = "user@example.test"))
        Thread.sleep(6_000)
        assertEquals(listOf("refresh", "refresh"), calls.toList())
    }
}
