package top.flysoftbeta.workflow.feature.settings

import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import android.content.Intent
import android.provider.Settings
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import top.flysoftbeta.workflow.core.config.AppConfig
import top.flysoftbeta.workflow.core.config.LauncherEntry
import top.flysoftbeta.workflow.platform.*
import top.flysoftbeta.workflow.platform.apps.InstalledApps
import top.flysoftbeta.workflow.ui.design.SearchField
import top.flysoftbeta.workflow.ui.design.InlineError
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

private tailrec fun Context.activity(): Activity? = when (this) { is Activity -> this; is ContextWrapper -> baseContext.activity(); else -> null }

@Composable private fun capabilityState(refresh: Int = 0): Pair<DeviceCapabilities, CapabilitySnapshot> {
    val context = LocalContext.current
    val capabilities = remember(context) { DeviceCapabilities(context.activity() ?: context) }
    var state by remember { mutableStateOf(capabilities.snapshot()) }
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    val running by WorkflowOverlayService.running.collectAsState()
    DisposableEffect(lifecycle, capabilities) {
        val observer = LifecycleEventObserver { _, event -> if (event == Lifecycle.Event.ON_RESUME) state = capabilities.snapshot() }
        lifecycle.addObserver(observer)
        onDispose { lifecycle.removeObserver(observer) }
    }
    LaunchedEffect(running, refresh) { state = capabilities.snapshot() }
    return capabilities to state
}

@Composable internal fun OverlaySettings(c: SettingsController, config: AppConfig) {
    val (capabilities, state) = capabilityState()
    val serviceFailure by WorkflowOverlayService.failure.collectAsState()
    var showApps by c::openExtraApps
    fun error(result: Result<Unit>) { result.onFailure { c.context.commands.snackbar(it.message ?: "操作未完成") } }
    SettingRow("悬浮窗", if (state.overlayGranted) if (state.overlayRunning) "运行中" else "已停止" else "未授权", trailing = {
        Switch(checked = state.overlayRunning && state.overlayGranted, onCheckedChange = { enabled ->
            if (enabled && !state.overlayGranted) error(capabilities.requestOverlayPermission())
            else c.update(after = {
                if (enabled) error(WorkflowOverlayService.start(c.context.appContext)) else WorkflowOverlayService.stop(c.context.appContext)
            }) { it.copy(overlay = it.overlay.copy(enabled = enabled)) }
        })
    })
    serviceFailure?.let { InlineError(it) }
    SettingRow("额外应用", "${config.overlay.extraApps.size}", { showApps = true })
    SettingRow("重置位置", onClick = { WorkflowOverlayService.resetPosition(c.context.appContext); c.context.commands.snackbar("悬浮球位置已重置") })
    if (showApps) ExtraApps(c, config) { showApps = false }
}

@Composable private fun ExtraApps(c: SettingsController, config: AppConfig, dismiss: () -> Unit) {
    val catalog = remember { InstalledApps.get(c.context.appContext) }
    val apps by catalog.apps.collectAsState()
    var query by remember { mutableStateOf("") }
    LaunchedEffect(catalog) { catalog.refresh() }
    val launcherIds = config.launcher.filterIsInstance<LauncherEntry.App>().map { it.ref.id }.toSet()
    AlertDialog(onDismissRequest = dismiss, title = { Text("额外应用") }, text = {
        Column {
            SearchField(query, { query = it }, Modifier.fillMaxWidth(), placeholder = "搜索应用")
            LazyColumn(Modifier.heightIn(max = 420.dp)) {
                items(apps.filter { it.label.contains(query, true) }, key = { it.ref.id }) { app ->
                    val inLauncher = app.ref.id in launcherIds || app.ref.packageName in launcherIds
                    val checked = config.overlay.extraApps.any { it.id == app.ref.id }
                    fun toggle() { c.update { current -> current.copy(overlay = current.overlay.copy(extraApps =
                        if (checked) current.overlay.extraApps.filterNot { it.id == app.ref.id } else current.overlay.extraApps + app.ref)) } }
                    SettingRow(app.label, if (inLauncher) "来自启动器" else "", onClick = if (inLauncher) null else ::toggle,
                        trailing = { Checkbox(checked || inLauncher, onCheckedChange = if (inLauncher) null else { _ -> toggle() }) })
                }
            }
        }
    }, confirmButton = { TextButton(onClick = dismiss) { Text("完成") } })
}

@Composable internal fun PermissionSettings(c: SettingsController) {
    var refresh by remember { mutableIntStateOf(0) }
    val (capabilities, snapshot) = capabilityState(refresh)
    var probed by remember { mutableStateOf<CapabilitySnapshot?>(null) }
    var checking by remember { mutableStateOf(false) }
    val state = probed?.let { snapshot.copy(rootAccess = it.rootAccess, rootMessage = it.rootMessage) } ?: snapshot
    val context = LocalContext.current
    fun handle(result: Result<Unit>) { result.onFailure { c.context.commands.snackbar(it.message ?: "系统页面无法打开") } }
    fun probe() { checking = true; capabilities.probeRoot { probed = it; checking = false; if (it.rootAccess != RootAccess.AVAILABLE) c.context.commands.snackbar(it.rootMessage) } }
    SettingRow("悬浮窗", if (state.overlayGranted) "已授权" else "未授权", trailing = { TextButton(onClick = { handle(capabilities.requestOverlayPermission()) }) { Text(if (state.overlayGranted) "管理" else "授权") } })
    SettingRow("修改系统设置", if (state.writeSettingsGranted) "已授权" else "未授权", trailing = { TextButton(onClick = { handle(capabilities.requestWriteSettings()) }) { Text(if (state.writeSettingsGranted) "管理" else "授权") } })
    SettingRow("黑白", if (state.grayscaleEnabled) "已开启" else "已关闭", trailing = { TextButton(onClick = ::probe, enabled = !checking) { Text(if (state.rootAccess == RootAccess.AVAILABLE) "已授权" else "授权") } })
    SettingRow("锁屏", if (state.deviceAdminActive) "设备管理员" else if (state.rootAccess == RootAccess.AVAILABLE) "Root 已授权" else "未授权", trailing = {
        TextButton(onClick = { handle(if (state.deviceAdminActive) capabilities.revokeDeviceAdmin() else capabilities.requestDeviceAdmin()); refresh++ }) { Text(if (state.deviceAdminActive) "撤销" else "授权") }
    })
    SettingRow("Root", if (checking) "检查中" else when (state.rootAccess) { RootAccess.AVAILABLE -> "已授权"; RootAccess.UNAVAILABLE -> "不可用"; else -> "未检查" }, trailing = {
        TextButton(onClick = ::probe, enabled = !checking) { Text("检查") }
    })
    SettingRow("通知", if (state.notificationsGranted) "已授权" else "未授权", trailing = { TextButton(onClick = {
        val activity = context.activity()
        if (!state.notificationsGranted && android.os.Build.VERSION.SDK_INT >= 33 && activity != null) capabilities.requestNotificationPermission(activity)
        else handle(capabilities.openNotificationSettings())
    }) { Text(if (state.notificationsGranted) "管理" else "授权") } })
}

@Composable internal fun HomeSettings(c: SettingsController) {
    val context = LocalContext.current
    var default by remember { mutableStateOf(false) }
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    fun refresh() {
        val intent = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_HOME)
        default = context.packageManager.resolveActivity(intent, android.content.pm.PackageManager.MATCH_DEFAULT_ONLY)?.activityInfo?.packageName == context.packageName
    }
    DisposableEffect(lifecycle) {
        refresh()
        val observer = LifecycleEventObserver { _, event -> if (event == Lifecycle.Event.ON_RESUME) refresh() }
        lifecycle.addObserver(observer); onDispose { lifecycle.removeObserver(observer) }
    }
    SettingRow("默认桌面", if (default) "Workflow" else "其他桌面", trailing = {
        if (!default) TextButton(onClick = { runCatching { context.startActivity(Intent(Settings.ACTION_HOME_SETTINGS)) }.onFailure { c.context.commands.snackbar("无法打开默认桌面设置") } }) { Text("设为默认") }
    })
}

@Composable internal fun AboutSettings(c: SettingsController) {
    var version by remember { mutableStateOf("") }
    var licenses by remember { mutableStateOf<String?>(null) }
    var show by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) { version = withContext(Dispatchers.IO) { c.context.appContext.packageManager.getPackageInfo(c.context.appContext.packageName, 0).versionName.orEmpty() } }
    LaunchedEffect(show) { if (show && licenses == null) licenses = withContext(Dispatchers.IO) {
        val assets = c.context.appContext.assets
        assets.list("notices").orEmpty().filter { it.endsWith("LICENSE") || it.endsWith("NOTICE") }.sorted().joinToString("\n\n") { name ->
            name.substringBeforeLast('-') + "\n\n" + assets.open("notices/$name").bufferedReader().use { it.readText() }
        }.ifBlank { "此安装包没有可显示的许可文件" }
    } }
    SettingRow("版本", version)
    SettingRow("开源许可", onClick = { show = true })
    if (show) AlertDialog(onDismissRequest = { show = false }, title = { Text("开源许可") }, text = {
        Text(licenses ?: "正在读取…", Modifier.heightIn(max = 480.dp).verticalScroll(rememberScrollState()), style = WorkflowTheme.text.caption)
    }, confirmButton = { TextButton(onClick = { show = false }) { Text("关闭") } })
}
