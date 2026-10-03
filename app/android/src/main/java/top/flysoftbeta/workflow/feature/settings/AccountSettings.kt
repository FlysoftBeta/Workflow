package top.flysoftbeta.workflow.feature.settings

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import top.flysoftbeta.workflow.agent.model.*
import top.flysoftbeta.workflow.platform.engine.EnvironmentHealth
import top.flysoftbeta.workflow.ui.design.InlineError
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

@OptIn(ExperimentalMaterial3Api::class)
@Composable internal fun AccountSettings(c: SettingsController) {
    val hub = remember(c.services) { c.services.hub }
    val state by hub.state.collectAsState()
    val available by hub.available.collectAsState()
    val chatConfiguration by hub.configuration.collectAsState()
    val health by c.services.environment.collectAsState()
    var selected by remember { mutableStateOf<BackendKind?>(null) }
    var busy by remember { mutableStateOf(false) }
    var secretMethod by remember { mutableStateOf<LoginMethod?>(null) }
    var secret by remember { mutableStateOf("") }
    var flow by remember { mutableStateOf<LoginFlow?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    fun task(action: suspend () -> Unit) { c.context.scope.launch {
        busy = true; error = null
        try { action() } catch (e: Exception) { if (e is kotlinx.coroutines.CancellationException) throw e; error = "操作未完成，请重试" } finally { busy = false }
    } }
    LaunchedEffect(available, health.usable) { if (health.usable) available.forEach(hub::warmUp) }
    BackendKind.entries.forEach { kind ->
        val account = state.backend(kind).account
        val enabled = kind in available && health.usable
        SettingRow(if (kind == BackendKind.CODEX) "Codex" else "Claude Code", when {
            !health.usable -> "等待环境"
            !enabled -> "尚未就绪"
            account.state == LoginState.LOGGED_IN -> account.email ?: account.organization ?: "已登录"
            account.state == LoginState.LOGGING_IN -> "登录中"
            account.state == LoginState.UNKNOWN -> "检查中"
            else -> "未登录"
        }, trailing = {
            TextButton(enabled = enabled && !busy, onClick = {
                if (account.state == LoginState.LOGGED_IN) task { hub.logout(kind) }
                else { selected = kind; flow = account.login; secretMethod = null; secret = ""; error = null }
            }) { Text(if (account.state == LoginState.LOGGED_IN) "退出" else "登录") }
        })
    }
    error?.let { InlineError(it) }
    val kind = selected
    if (kind != null) ModalBottomSheet(onDismissRequest = { selected = null; secret = "" }, sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)) {
        val current = state.backend(kind).account
        LaunchedEffect(current.login, current.state) { flow = current.login }
        LaunchedEffect(current.state) { if (current.state == LoginState.LOGGED_IN) { selected = null; secret = "" } }
        fun browse(url: String) { runCatching {
            require(Uri.parse(url).scheme in listOf("https", "http"))
            c.context.appContext.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        }.onFailure { error = "无法打开浏览器" } }
        Column(Modifier.fillMaxWidth().padding(24.dp)) {
            Text(if (kind == BackendKind.CODEX) "登录 Codex" else "登录 Claude Code", style = WorkflowTheme.text.titleMd)
            Spacer(Modifier.height(12.dp))
            when (val active = flow) {
                is LoginFlow.DeviceCode -> {
                    Text(active.userCode, style = WorkflowTheme.text.titleMd)
                    Row {
                        TextButton(onClick = { browse(active.verificationUrl) }) { Text("打开验证页面") }
                        TextButton(onClick = { c.context.appContext.getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText("设备码", active.userCode)) }) { Text("复制代码") }
                    }
                    active.loginId?.let { id -> TextButton(onClick = { task { hub.cancelLogin(kind, id); flow = null } }) { Text("取消登录") } }
                }
                is LoginFlow.Browser -> {
                    Button(onClick = { browse(active.authUrl) }) { Text("在浏览器中继续") }
                    active.loginId?.let { id -> TextButton(onClick = { task { hub.cancelLogin(kind, id); flow = null } }) { Text("取消登录") } }
                }
                is LoginFlow.Terminal -> {
                    Text(active.argv.joinToString(" "), style = WorkflowTheme.text.mono)
                    Row {
                        TextButton(onClick = {
                            c.context.appContext.getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText("登录命令", active.argv.joinToString(" ")))
                            c.context.commands.newTerminal(); selected = null
                        }) { Text("复制并打开终端") }
                        TextButton(onClick = { task { hub.refreshAccount(kind) } }) { Text("我已完成登录") }
                    }
                }
                else -> {
                    if ((active as? LoginFlow.Completed)?.success == false) InlineError("登录未完成")
                    if (secretMethod != null) {
                        OutlinedTextField(secret, { secret = it }, label = { Text(if (secretMethod == LoginMethod.CLAUDE_SETUP_TOKEN) "访问令牌" else "API Key") },
                            singleLine = true, visualTransformation = PasswordVisualTransformation(), modifier = Modifier.fillMaxWidth())
                        Button(onClick = { val value = secret; secret = ""; val method = secretMethod!!; secretMethod = null; task { flow = hub.login(kind, method, value) } }, enabled = secret.isNotBlank() && !busy) { Text("登录") }
                    } else chatConfiguration.loginMethods[kind].orEmpty().forEach { method ->
                        TextButton(enabled = !busy, onClick = {
                            if (method in listOf(LoginMethod.CODEX_API_KEY, LoginMethod.CLAUDE_API_KEY, LoginMethod.CLAUDE_SETUP_TOKEN)) secretMethod = method
                            else task { flow = hub.login(kind, method) }
                        }) { Text(when (method) {
                            LoginMethod.CODEX_DEVICE_CODE -> "设备码登录"
                            LoginMethod.CODEX_BROWSER -> "浏览器登录"
                            LoginMethod.CLAUDE_TERMINAL_LOGIN -> "浏览器登录（终端）"
                            LoginMethod.CLAUDE_SETUP_TOKEN -> "访问令牌登录"
                            else -> "API Key 登录"
                        }) }
                    }
                }
            }
            if (busy) LinearProgressIndicator(Modifier.fillMaxWidth())
            error?.let { InlineError(it) }
            Spacer(Modifier.height(12.dp))
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
    TextButton(onClick = { c.context.commands.openFile(top.flysoftbeta.workflow.core.io.WorkspacePaths.ENVIRONMENT) }) { Text("编辑环境配置") }
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
