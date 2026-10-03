package top.flysoftbeta.workflow.platform

import androidx.compose.foundation.Image
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import top.flysoftbeta.workflow.app.AppGraph
import top.flysoftbeta.workflow.core.config.BuiltinApp
import top.flysoftbeta.workflow.core.config.Launcher
import top.flysoftbeta.workflow.core.config.LauncherEntry
import top.flysoftbeta.workflow.platform.apps.InstalledApps
import top.flysoftbeta.workflow.ui.design.SymbolIcon
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.*

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable internal fun OverlayPanel(
    capabilities: DeviceCapabilities,
    initialError: String? = null,
    onManageApps: () -> Unit,
    onPermissions: () -> Unit,
    onLaunch: (String) -> Unit,
    onClose: () -> Unit,
    onLock: () -> Unit,
) {
    val context = LocalContext.current
    val store = remember { AppGraph.workspaceStore(context) }
    val workspace by store.state.collectAsState()
    val catalog = remember { InstalledApps.get(context) }
    val known by catalog.known.collectAsState()
    val apps = Launcher.overlayApps(workspace.config)
    LaunchedEffect(apps) { catalog.ensure(apps.filterIsInstance<LauncherEntry.App>().map { it.ref }) }
    val mode = when (workspace.config.appearance.theme) {
        top.flysoftbeta.workflow.core.config.ThemeMode.LIGHT -> ThemeMode.Light
        top.flysoftbeta.workflow.core.config.ThemeMode.DARK -> ThemeMode.Dark
        else -> ThemeMode.System
    }
    WorkflowDesignTheme(themeMode = mode, density = UiDensity.Standard) {
        var state by remember { mutableStateOf(capabilities.snapshot()) }
        var error by remember { mutableStateOf(initialError) }
        var rootBusy by remember { mutableStateOf(false) }
        var displayedBrightness by remember { mutableFloatStateOf(state.brightness.toFloat()) }
        fun failed(throwable: Throwable) { error = throwable.message ?: "操作未完成" }
        LaunchedEffect(error) { if (error != null) { delay(2_000); error = null } }
        // Use a conflated channel so old slider events never overwrite a newer value after release.
        val brightnessChanges = remember { kotlinx.coroutines.channels.Channel<Int>(kotlinx.coroutines.channels.Channel.CONFLATED) }
        LaunchedEffect(brightnessChanges) {
            for (value in brightnessChanges) {
                val result = withContext(Dispatchers.IO) { capabilities.setBrightness(value) }
                state = capabilities.snapshot()
                result.onFailure { displayedBrightness = state.brightness.toFloat(); failed(it) }
            }
        }
        DisposableEffect(brightnessChanges) { onDispose { brightnessChanges.close() } }
        Surface(shape = WorkflowShapes.xl, color = WorkflowTheme.colors.surfaceContainerHigh, tonalElevation = 6.dp,
            modifier = Modifier.fillMaxWidth()) {
            Column(Modifier.padding(12.dp)) {
                val rows = ((apps.size + 1 + 3) / 4).coerceIn(1, 3)
                LazyVerticalGrid(columns = GridCells.Fixed(4), modifier = Modifier.fillMaxWidth().height((rows * 72).dp)) {
                    items(apps, key = { it.id }) { entry ->
                        val external = entry as? LauncherEntry.App
                        val builtin = (entry as? LauncherEntry.Builtin)?.app
                        val info = external?.let { known[it.ref.id] }
                        val title = when (builtin) { BuiltinApp.WORKBENCH -> "工作台"; BuiltinApp.PROXY -> "代理"; BuiltinApp.SETTINGS -> "设置"; null -> info?.label ?: "应用" }
                        Column(Modifier.height(72.dp).clickable {
                            if (external != null) { if (catalog.launch(external.ref)) onClose() else error = "应用当前不可用" }
                            else onLaunch(when (builtin) { BuiltinApp.PROXY -> "PROXY"; BuiltinApp.SETTINGS -> "SETTINGS"; else -> "WORKBENCH" })
                        }.semantics { contentDescription = "打开$title" }, horizontalAlignment = Alignment.CenterHorizontally) {
                            Box(Modifier.size(44.dp), contentAlignment = Alignment.Center) {
                                if (info?.icon != null) Image(info.icon, null, Modifier.size(40.dp))
                                else SymbolIcon(when (builtin) { BuiltinApp.PROXY -> Sym.SwapHoriz; BuiltinApp.SETTINGS -> Sym.Settings; else -> Sym.Apps }, null, size = 26.dp)
                            }
                            Text(title, style = WorkflowTheme.text.micro, maxLines = 1, overflow = TextOverflow.Ellipsis)
                        }
                    }
                    item(key = "manage") {
                        Column(Modifier.height(72.dp).clickable(onClick = onManageApps).semantics { contentDescription = "管理额外应用" }, horizontalAlignment = Alignment.CenterHorizontally) {
                            Box(Modifier.size(44.dp), contentAlignment = Alignment.Center) { SymbolIcon(Sym.Edit, null, size = 24.dp) }
                            Text("编辑", style = WorkflowTheme.text.micro)
                        }
                    }
                }
                Row(Modifier.fillMaxWidth().heightIn(min = 48.dp), verticalAlignment = Alignment.CenterVertically) {
                    SymbolIcon(Sym.LightMode, null, size = 20.dp)
                    Slider(displayedBrightness, onValueChange = { displayedBrightness = it; brightnessChanges.trySend(it.toInt()) }, valueRange = 1f..255f,
                        enabled = state.writeSettingsGranted, modifier = Modifier.weight(1f).padding(horizontal = 6.dp).semantics { contentDescription = "屏幕亮度" })
                    if (!state.writeSettingsGranted) TextButton(onClick = { capabilities.requestWriteSettings().onSuccess { onClose() }.onFailure(::failed) }) { Text("授权") }
                    else Text("${(displayedBrightness * 100 / 255).toInt()}%", style = WorkflowTheme.text.caption)
                }
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Column(Modifier.weight(1f)) {
                        ToggleButton(checked = state.grayscaleEnabled, onCheckedChange = { enabled ->
                            rootBusy = true
                            capabilities.setGrayscale(enabled) { result ->
                                rootBusy = false
                                state = capabilities.snapshot()
                                result.onFailure(::failed)
                            }
                        }, enabled = state.rootAccess == RootAccess.AVAILABLE && !rootBusy, modifier = Modifier.fillMaxWidth().height(56.dp)) { Text("黑白") }
                        if (state.rootAccess != RootAccess.AVAILABLE) TextButton(onClick = {
                            rootBusy = true; capabilities.probeRoot { state = it; rootBusy = false; if (it.rootAccess != RootAccess.AVAILABLE) error = it.rootMessage }
                        }, enabled = !rootBusy, modifier = Modifier.fillMaxWidth()) { Text(if (rootBusy) "检查中" else "授权") }
                    }
                    Column(Modifier.weight(1f)) {
                        FilledTonalButton(onClick = onLock, enabled = state.deviceAdminActive || state.rootAccess == RootAccess.AVAILABLE, modifier = Modifier.fillMaxWidth().height(56.dp)) { Text("锁屏") }
                        if (!state.deviceAdminActive && state.rootAccess != RootAccess.AVAILABLE) TextButton(onClick = onPermissions, modifier = Modifier.fillMaxWidth()) { Text("授权") }
                    }
                }
                error?.let { message -> Text(message, Modifier.padding(top = 4.dp).semantics {
                    contentDescription = "操作失败：$message"
                    liveRegion = LiveRegionMode.Polite
                }, color = WorkflowTheme.colors.error, style = WorkflowTheme.text.caption, maxLines = 2) }
            }
        }
    }
}
