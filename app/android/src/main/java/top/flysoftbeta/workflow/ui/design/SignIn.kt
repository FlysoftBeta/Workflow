package top.flysoftbeta.workflow.ui.design

import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.MenuAnchorPosition
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import top.flysoftbeta.workflow.agent.model.AccountState
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.LoginFlow
import top.flysoftbeta.workflow.agent.model.LoginMethod
import top.flysoftbeta.workflow.agent.model.LoginPhase
import top.flysoftbeta.workflow.agent.model.LoginView
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/**
 * The account commands a sign-in surface issues. None of them answers an agent request; sign-in never
 * approves anything on the user's behalf.
 */
interface AccountCommands {
    suspend fun login(kind: BackendKind, method: LoginMethod, secret: String?)
    suspend fun cancelLogin(kind: BackendKind, loginId: String)
    suspend fun refreshAccount(kind: BackendKind)
}

/**
 * Sign-in for one backend, shared by the conversation and Settings (docs/ux/conversations.md "Signing in").
 * What a surface shows is [LoginView.phase] of the Engine's [account] projection; this holder only adds a
 * login command still in flight before the Engine reports its attempt ([starting]), a command that failed
 * in the Engine or the connection ([problem]), and the method Retry offers again. Vendor failures arrive as
 * the account's failed flow instead.
 */
@Stable
class SignInState(
    val kind: BackendKind,
    private val commands: AccountCommands,
    private val scope: CoroutineScope,
    private val account: () -> AccountState?,
) {
    /** The login command is in flight; the Engine's `Progress` flow may not have arrived yet. */
    var starting by mutableStateOf(false)
        private set
    /** A login, cancel or account check command that failed before the Engine recorded a flow. */
    var problem by mutableStateOf<String?>(null)
        private set
    /** The method of the last interactive attempt, offered again by Retry. Secrets are never kept. */
    var lastMethod by mutableStateOf<LoginMethod?>(null)
        private set
    private var rechecks: Job? = null

    fun login(method: LoginMethod, secret: String? = null) {
        if (starting) return
        starting = true
        problem = null
        if (secret == null) lastMethod = method
        scope.launch {
            try { commands.login(kind, method, secret) }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { problem = error.message ?: "无法开始登录" }
            finally { starting = false }
        }
    }

    fun cancel(loginId: String) {
        scope.launch {
            try { commands.cancelLogin(kind, loginId) }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { problem = error.message ?: "无法取消登录" }
        }
    }

    /** Reads the account again; a failed read is shown from the account state, not here. */
    fun recheck() {
        problem = null
        scope.launch {
            try { commands.refreshAccount(kind) }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { problem = error.message ?: "无法检查账户状态" }
        }
    }

    /**
     * The app returned to the foreground while a sign-in surface is visible (for example from the browser):
     * re-read the account at [LoginView.RESUME_RECHECK_DELAYS_MS] instead of relying on a notification alone.
     */
    fun recheckAfterResume() {
        if (rechecks?.isActive == true || !LoginView.shouldRecheck(account())) return
        rechecks = scope.launch {
            for (wait in LoginView.RESUME_RECHECK_DELAYS_MS) {
                delay(wait)
                if (!LoginView.shouldRecheck(account())) return@launch
                try { commands.refreshAccount(kind) }
                catch (cancelled: CancellationException) { throw cancelled }
                catch (_: Exception) { /* The watch recovers the projection; the next re-read retries. */ }
            }
        }
    }

    fun stopRechecks() { rechecks?.cancel(); rechecks = null }
}

/** Display text shared by every sign-in surface. */
object SignInText {
    fun backendName(kind: BackendKind) = when (kind) {
        BackendKind.CODEX -> "Codex"
        BackendKind.CLAUDE -> "Claude Code"
    }

    fun methodLabel(method: LoginMethod) = when (method) {
        LoginMethod.CODEX_DEVICE_CODE -> "使用设备码登录"
        LoginMethod.CODEX_BROWSER -> "在浏览器中登录"
        LoginMethod.CODEX_API_KEY -> "使用 API Key"
        LoginMethod.CLAUDE_TERMINAL_LOGIN -> "在终端中登录"
        LoginMethod.CLAUDE_SETUP_TOKEN -> "使用长期令牌"
        LoginMethod.CLAUDE_API_KEY -> "使用 API Key"
    }

    /** Methods that take a typed secret instead of a browser or terminal flow. */
    fun takesSecret(method: LoginMethod) =
        method == LoginMethod.CODEX_API_KEY || method == LoginMethod.CLAUDE_API_KEY || method == LoginMethod.CLAUDE_SETUP_TOKEN

    /** The row value for a backend's account in Settings and similar summaries. */
    fun summary(account: AccountState): String = when (val phase = LoginView.phase(account)) {
        LoginPhase.SignedIn -> account.email ?: account.organization ?: "已登录"
        LoginPhase.Checking -> "检查中"
        is LoginPhase.Starting, is LoginPhase.Waiting, is LoginPhase.Terminal -> "登录中"
        is LoginPhase.Idle -> when {
            phase.failure != null -> "登录未完成"
            phase.checkError != null -> "无法读取账户状态"
            else -> "未登录"
        }
    }

    /** True while an attempt runs, so entry points offer Continue instead of Sign in. */
    fun inProgress(account: AccountState): Boolean = when (LoginView.phase(account)) {
        is LoginPhase.Starting, is LoginPhase.Waiting, is LoginPhase.Terminal -> true
        else -> false
    }
}

/** Opens a sign-in link in the browser; false when the link is not http(s) or no browser opened. */
fun openSignInLink(context: Context, url: String): Boolean = runCatching {
    require(Uri.parse(url).scheme in listOf("https", "http"))
    context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
}.isSuccess

/**
 * The sign-in phases for [account] (docs/ux/conversations.md "Signing in"): starting with Cancel, waiting for
 * the browser with Cancel, failure with Retry and Check again, an unreadable account with Retry, and the
 * method choice. While visible, returning to the app re-reads the account. [ready] is false until the
 * backend process can take a login. [openUrl] returns false when no browser opened. [openTerminal], when
 * given, copies a terminal login command and opens a terminal.
 */
@Composable
fun SignInContent(
    state: SignInState,
    account: AccountState,
    methods: List<LoginMethod>,
    ready: Boolean,
    openUrl: (String) -> Boolean,
    copy: (label: String, text: String) -> Unit,
    modifier: Modifier = Modifier,
    openTerminal: ((command: String) -> Unit)? = null,
) {
    val name = SignInText.backendName(state.kind)
    val colors = WorkflowTheme.colors
    val phase = LoginView.phase(account)
    var secretMethod by remember(state) { mutableStateOf<LoginMethod?>(null) }
    var secret by remember(state) { mutableStateOf("") }
    var browserFailed by remember(state) { mutableStateOf(false) }
    fun browse(url: String) { browserFailed = !openUrl(url) }
    fun choose(method: LoginMethod) {
        if (SignInText.takesSecret(method)) { secretMethod = method; secret = "" } else state.login(method)
    }
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    DisposableEffect(lifecycle, state) {
        val observer = LifecycleEventObserver { _, event -> if (event == Lifecycle.Event.ON_RESUME) state.recheckAfterResume() }
        lifecycle.addObserver(observer)
        onDispose { lifecycle.removeObserver(observer); state.stopRechecks() }
    }
    Column(modifier.widthIn(max = 420.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        state.problem?.let { problem ->
            InlineError("登录操作没有完成", Modifier.fillMaxWidth(), listOf(TextAction("重新检查") { state.recheck() }))
            SignInDetail(problem)
            Spacer(Modifier.height(8.dp))
        }
        if (browserFailed) InlineError("无法打开浏览器", Modifier.fillMaxWidth())
        when (phase) {
            is LoginPhase.Starting -> Busy("正在开始登录…") { state.cancel(phase.attemptId) }
            is LoginPhase.Waiting -> when (val flow = phase.flow) {
                is LoginFlow.DeviceCode -> {
                    Text("在浏览器中打开下面的链接，并输入代码", style = WorkflowTheme.text.body, color = colors.onSurfaceVariant, textAlign = TextAlign.Center)
                    Spacer(Modifier.height(8.dp))
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(
                            flow.verificationUrl, Modifier.weight(1f, fill = false).clip(WorkflowShapes.sm).clickable { browse(flow.verificationUrl) }.padding(4.dp),
                            style = WorkflowTheme.text.body, color = colors.primary, maxLines = 1, overflow = TextOverflow.Ellipsis,
                        )
                        WfIconButton(Sym.OpenInNew, "打开链接", { browse(flow.verificationUrl) })
                    }
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(flow.userCode, style = WorkflowTheme.text.mono.copy(fontSize = 26.sp, lineHeight = 32.sp, letterSpacing = 2.sp), color = colors.onSurface)
                        WfIconButton(Sym.ContentCopy, "复制代码", { copy("设备码", flow.userCode) })
                    }
                    Spacer(Modifier.height(8.dp))
                    Busy("等待确认…", flow.loginId?.let { id -> { state.cancel(id) } })
                    CheckNote(phase.checkError)
                }
                is LoginFlow.Browser -> {
                    Button(onClick = { browse(flow.authUrl) }) { Text("在浏览器中继续") }
                    Spacer(Modifier.height(8.dp))
                    Busy("等待确认…", flow.loginId?.let { id -> { state.cancel(id) } })
                    CheckNote(phase.checkError)
                }
                else -> Unit
            }
            is LoginPhase.Terminal -> {
                val command = phase.flow.argv.joinToString(" ")
                Text("在终端中运行下面的命令完成登录", style = WorkflowTheme.text.body, color = colors.onSurfaceVariant, textAlign = TextAlign.Center)
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(command, style = WorkflowTheme.text.mono, color = colors.onSurface)
                    WfIconButton(Sym.ContentCopy, "复制命令", { copy("登录命令", command) })
                }
                Row {
                    if (openTerminal != null) TextButton(onClick = { openTerminal(command) }) { Text("复制并打开终端") }
                    TextButton(onClick = state::recheck) { Text("我已完成登录") }
                }
            }
            LoginPhase.SignedIn -> Unit
            // The account has not been read yet: nothing to choose from.
            LoginPhase.Checking -> InlineLoading(true)
            is LoginPhase.Idle -> {
                val primary = methods.firstOrNull()
                when {
                    phase.failure != null -> {
                        val retry = state.lastMethod ?: primary
                        InlineError("登录没有完成", Modifier.fillMaxWidth(), listOfNotNull(
                            retry?.takeIf { ready && !state.starting }?.let { method -> TextAction("重试") { choose(method) } },
                            TextAction("重新检查") { state.recheck() },
                        ))
                        SignInDetail(phase.failure)
                        SignInDetail(phase.checkError?.let { "账户状态：$it" })
                        Spacer(Modifier.height(8.dp))
                    }
                    phase.checkError != null -> {
                        InlineError("无法读取 $name 账户状态", Modifier.fillMaxWidth(), listOf(TextAction("重试") { state.recheck() }))
                        SignInDetail(phase.checkError)
                        Spacer(Modifier.height(8.dp))
                    }
                }
                val method = secretMethod
                when {
                    state.starting -> Busy("正在开始登录…", onCancel = null)
                    method != null -> {
                        OutlinedTextField(
                            value = secret, onValueChange = { secret = it }, singleLine = true,
                            label = { Text(if (method == LoginMethod.CLAUDE_SETUP_TOKEN) "长期令牌" else "API Key") },
                            visualTransformation = PasswordVisualTransformation(),
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                            modifier = Modifier.fillMaxWidth(),
                        )
                        Row(Modifier.fillMaxWidth().padding(top = 8.dp), horizontalArrangement = Arrangement.End) {
                            TextButton(onClick = { secret = ""; secretMethod = null }) { Text("取消") }
                            Spacer(Modifier.width(8.dp))
                            Button(onClick = {
                                val value = secret
                                secret = ""; secretMethod = null
                                state.login(method, value)
                            }, enabled = secret.isNotBlank() && ready) { Text("登录") }
                        }
                    }
                    else -> {
                        var more by remember { mutableStateOf(false) }
                        Button(onClick = { primary?.let(::choose) }, enabled = primary != null && ready) { Text("登录 $name") }
                        val others = methods.drop(1)
                        if (others.isNotEmpty()) Box {
                            TextButton(onClick = { more = true }, enabled = ready) { Text("其他方式") }
                            CompositeMenu(
                                expanded = more, onDismissRequest = { more = false },
                                anchorPosition = MenuAnchorPosition.Below,
                                groups = listOf(MenuGroup("methods", others.map { other ->
                                    MenuEntry.Action(other.name, SignInText.methodLabel(other), if (SignInText.takesSecret(other)) Sym.Key else Sym.OpenInNew) { choose(other) }
                                })),
                            )
                        }
                        InlineLoading(!ready)
                    }
                }
            }
        }
    }
}

/** A secondary line under a sign-in error or waiting row; vendor text verbatim, never credentials. */
@Composable
private fun SignInDetail(text: String?) {
    if (text.isNullOrBlank()) return
    Text(text, Modifier.fillMaxWidth().padding(top = 4.dp), style = WorkflowTheme.text.caption, color = WorkflowTheme.colors.onSurfaceVariant, textAlign = TextAlign.Center)
}

/** A failing background account check while waiting; the attempt continues. */
@Composable
private fun CheckNote(checkError: String?) {
    checkError ?: return
    SignInDetail("暂时无法确认登录状态，正在重试：$checkError")
}

/** A running sign-in step with an indicator and, when the step can be cancelled, Cancel. */
@Composable
private fun Busy(label: String, onCancel: (() -> Unit)?) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        InlineLoading(true)
        Spacer(Modifier.width(8.dp))
        Text(label, style = WorkflowTheme.text.label, color = WorkflowTheme.colors.onSurfaceVariant)
        if (onCancel != null) {
            Spacer(Modifier.width(8.dp))
            TextButton(onClick = onCancel) { Text("取消") }
        }
    }
}
