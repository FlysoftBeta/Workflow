package top.flysoftbeta.workflow.feature.settings

import android.content.ClipData
import android.content.ClipboardManager
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import top.flysoftbeta.workflow.agent.model.*
import top.flysoftbeta.workflow.platform.agent.AgentHub
import top.flysoftbeta.workflow.platform.engine.EnvironmentHealth
import top.flysoftbeta.workflow.ui.design.AccountCommands
import top.flysoftbeta.workflow.ui.design.InlineError
import top.flysoftbeta.workflow.ui.design.SignInContent
import top.flysoftbeta.workflow.ui.design.SignInState
import top.flysoftbeta.workflow.ui.design.SignInText
import top.flysoftbeta.workflow.ui.design.openSignInLink
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** The hub's account commands for the shared sign-in. The returned flow is unused: the hub's projection shows it. */
internal class HubAccountCommands(private val hub: AgentHub) : AccountCommands {
    override suspend fun login(kind: BackendKind, method: LoginMethod, secret: String?) { hub.login(kind, method, secret) }
    override suspend fun cancelLogin(kind: BackendKind, loginId: String) = hub.cancelLogin(kind, loginId)
    override suspend fun refreshAccount(kind: BackendKind) = hub.refreshAccount(kind)
}

/**
 * Accounts (docs/ux/launcher-and-services.md "Settings"): a row per backend and a sign-in sheet. The sheet is
 * the conversation's sign-in ([SignInContent] over [SignInState]), showing [LoginView] phases of the hub's
 * account projection, so both surfaces have the same phases, actions and resume re-checks.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable internal fun AccountSettings(c: SettingsController) {
    val hub = remember(c.services) { c.services.hub }
    val state by hub.state.collectAsState()
    val available by hub.available.collectAsState()
    val chatConfiguration by hub.configuration.collectAsState()
    val health by c.services.environment.collectAsState()
    val signIns = remember(hub) {
        val commands = HubAccountCommands(hub)
        BackendKind.entries.associateWith { kind -> SignInState(kind, commands, c.context.scope) { hub.state.value.backend(kind).account } }
    }
    var selected by remember { mutableStateOf<BackendKind?>(null) }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(available, health.usable) { if (health.usable) available.forEach(hub::warmUp) }
    BackendKind.entries.forEach { kind ->
        val account = state.backend(kind).account
        val signedIn = LoginView.phase(account) == LoginPhase.SignedIn
        val enabled = kind in available && health.usable
        SettingRow(SignInText.backendName(kind), when {
            !health.usable -> "等待环境"
            !enabled -> "尚未就绪"
            else -> SignInText.summary(account)
        }, trailing = {
            TextButton(enabled = enabled && !busy, onClick = {
                error = null
                if (signedIn) c.context.scope.launch {
                    busy = true
                    try { hub.logout(kind) }
                    catch (e: Exception) { if (e is kotlinx.coroutines.CancellationException) throw e; error = "退出未完成，请重试" }
                    finally { busy = false }
                } else selected = kind
            }) { Text(when {
                signedIn -> "退出"
                SignInText.inProgress(account) -> "继续"
                else -> "登录"
            }) }
        })
    }
    error?.let { InlineError(it) }
    val kind = selected ?: return
    val backend = state.backend(kind)
    val signedIn = LoginView.phase(backend.account) == LoginPhase.SignedIn
    LaunchedEffect(signedIn) { if (signedIn) selected = null }
    if (!signedIn) ModalBottomSheet(onDismissRequest = { selected = null }, sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)) {
        val appContext = c.context.appContext
        fun clip(label: String, text: String) =
            appContext.getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText(label, text))
        Text("登录 ${SignInText.backendName(kind)}", Modifier.padding(horizontal = 24.dp), style = WorkflowTheme.text.titleMd)
        Box(Modifier.fillMaxWidth().padding(24.dp), contentAlignment = Alignment.TopCenter) {
            SignInContent(
                state = signIns.getValue(kind),
                account = backend.account,
                methods = chatConfiguration.loginMethods[kind].orEmpty(),
                ready = kind in available && health.usable && backend.process is ProcessState.Ready,
                openUrl = { url -> openSignInLink(appContext, url) },
                copy = ::clip,
                openTerminal = { command -> clip("登录命令", command); c.context.commands.newTerminal(); selected = null },
            )
        }
    }
}

@Composable internal fun EnvironmentSettings(c: SettingsController) {
    val health by c.services.environment.collectAsState()
    var busy by remember { mutableStateOf(false) }
    val status = when (val h = health) {
        EnvironmentHealth.NotInstalled -> "准备中"
        is EnvironmentHealth.Unavailable -> "不可用"
        is EnvironmentHealth.Installing -> h.progress?.let { "构建中 ${(it * 100).toInt()}%" } ?: "构建中"
        is EnvironmentHealth.Provisioning -> h.progress?.let { "构建中 ${(it * 100).toInt()}%" } ?: "构建中"
        is EnvironmentHealth.Failed -> if (h.environmentAvailable) "构建失败，原环境可用" else "构建失败"
        is EnvironmentHealth.Ready -> if (h.applying == null) "就绪" else "构建中"
        is EnvironmentHealth.NeedsRestart -> "需要重启"
    }
    SettingRow("环境状态", status)
    TextButton(onClick = c::openEnvironmentDeclaration) { Text("编辑环境配置") }
    when (val h = health) {
        is EnvironmentHealth.NeedsRestart -> TextButton(enabled = !busy, onClick = { c.context.scope.launch {
            val decision = c.context.commands.decide(top.flysoftbeta.workflow.app.panel.DecisionRequest(
                title = "重启环境", message = "正在运行的终端和助手进程将停止并重新连接。", options = listOf(top.flysoftbeta.workflow.app.panel.DecisionOption("restart", "重启"))))
            if (decision == "restart") {
                busy = true
                runCatching { c.services.restartEnvironment() }.onFailure { c.context.commands.snackbar("环境重启失败") }
                busy = false
            }
        } }) { Text(if (busy) "重启中" else "重启环境") }
        is EnvironmentHealth.Failed -> { InlineError(h.message); TextButton(onClick = c.services::retryEnvironment) { Text("重试") } }
        is EnvironmentHealth.Unavailable -> InlineError(h.reason)
        else -> Unit
    }
}
