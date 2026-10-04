package top.flysoftbeta.workflow.feature.settings

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import top.flysoftbeta.workflow.app.panel.*
import top.flysoftbeta.workflow.core.config.AppConfig
import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.core.layout.Panel
import top.flysoftbeta.workflow.core.layout.TextCursor
import top.flysoftbeta.workflow.core.store.ConfigUpdate
import top.flysoftbeta.workflow.ui.design.*
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.*

class SettingsPanelProvider(private val services: SettingsFeatureServices) : PanelProvider {
    override fun create(panel: Panel, context: PanelContext): PanelController = SettingsController(context, services)
}

internal enum class Category(val label: String) {
    APPEARANCE("外观"), ACCOUNTS("账户"), ENVIRONMENT("环境"), OVERLAY("悬浮窗"), HOME("默认桌面"), PERMISSIONS("权限"), ABOUT("关于")
}

internal class SettingsController(val context: PanelContext, val services: SettingsFeatureServices) : PanelController {
    var selected by mutableStateOf<Category?>(null)
    var openExtraApps by mutableStateOf(false)
    private var narrow by mutableStateOf(true)
    override val tab get() = PanelTab(if (narrow) selected?.label ?: "设置" else "设置", Sym.Settings)
    override val actions get() = if (narrow && selected != null)
        listOf(ToolAction("settingsBack", Sym.ArrowBack, "返回设置", onClick = { selected = null })) else emptyList()

    fun update(after: (() -> Unit)? = null, transform: (AppConfig) -> AppConfig) {
        context.scope.launch {
            when (val result = context.store.updateConfig(transform)) {
                is ConfigUpdate.Updated -> after?.invoke()
                is ConfigUpdate.Blocked -> context.commands.snackbar("配置文件有错误，请先修正：${result.problem}")
                is ConfigUpdate.Failed -> context.commands.snackbar("设置未保存：${result.message}")
            }
        }
    }

    /** The environment declaration is the `environment` section of config.json; open the file there. */
    fun openEnvironmentDeclaration() {
        context.scope.launch {
            val text = runCatching { context.store.openFile(WorkspacePaths.CONFIG).diskText }.getOrNull()
            val line = text?.lineSequence()?.indexOfFirst { it.trimStart().startsWith("\"environment\"") } ?: -1
            context.commands.openFile(WorkspacePaths.CONFIG, if (line >= 0) TextCursor(line, 0) else null)
        }
    }

    @Composable override fun Content(frame: PanelFrame, modifier: Modifier) {
        val state by context.store.state.collectAsState()
        val request by top.flysoftbeta.workflow.platform.SettingsEntryPoint.destination.collectAsState()
        LaunchedEffect(request) {
            when (request) {
                top.flysoftbeta.workflow.platform.SettingsEntryPoint.Destination.OVERLAY_APPS -> { selected = Category.OVERLAY; openExtraApps = true }
                top.flysoftbeta.workflow.platform.SettingsEntryPoint.Destination.PERMISSIONS -> selected = Category.PERMISSIONS
                null -> Unit
            }
            top.flysoftbeta.workflow.platform.SettingsEntryPoint.destination.value = null
        }
        BoxWithConstraints(modifier) {
            val wide = maxWidth >= 720.dp
            SideEffect { narrow = !wide }
            BackHandler(frame.focused && !wide && selected != null) { selected = null }
            Row(Modifier.fillMaxSize()) {
                if (wide || selected == null) Column(
                    (if (wide) Modifier.width(220.dp) else Modifier.fillMaxWidth()).verticalScroll(rememberScrollState()).padding(8.dp)
                ) {
                    Category.entries.forEach { category ->
                        val active = wide && (selected ?: Category.APPEARANCE) == category
                        Row(Modifier.fillMaxWidth().heightIn(min = 48.dp).clip(WorkflowShapes.md)
                            .background(if (active) WorkflowTheme.colors.secondaryContainer else androidx.compose.ui.graphics.Color.Transparent)
                            .clickable { selected = category }.padding(horizontal = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                            Text(category.label, Modifier.weight(1f), style = WorkflowTheme.text.body)
                            if (!wide) SymbolIcon(Sym.ChevronRight, null, size = 18.dp)
                        }
                    }
                }
                if (wide || selected != null) {
                    Column(Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = 16.dp, vertical = 8.dp)) {
                        when (selected ?: Category.APPEARANCE) {
                            Category.APPEARANCE -> AppearanceSettings(this@SettingsController, state.config)
                            Category.ACCOUNTS -> AccountSettings(this@SettingsController)
                            Category.ENVIRONMENT -> EnvironmentSettings(this@SettingsController)
                            Category.OVERLAY -> OverlaySettings(this@SettingsController, state.config)
                            Category.HOME -> HomeSettings(this@SettingsController)
                            Category.PERMISSIONS -> PermissionSettings(this@SettingsController)
                            Category.ABOUT -> AboutSettings(this@SettingsController)
                        }
                    }
                }
            }
        }
    }
}

@Composable internal fun SettingRow(title: String, value: String = "", onClick: (() -> Unit)? = null, trailing: @Composable (() -> Unit)? = null) {
    Row(Modifier.fillMaxWidth().heightIn(min = 48.dp).then(if (onClick == null) Modifier else Modifier.clickable(onClick = onClick)), verticalAlignment = Alignment.CenterVertically) {
        Text(title, Modifier.weight(1f), style = WorkflowTheme.text.body, maxLines = 1, overflow = TextOverflow.Ellipsis)
        if (value.isNotEmpty()) Text(value, Modifier.widthIn(max = 180.dp).padding(start = 8.dp), style = WorkflowTheme.text.caption,
            color = WorkflowTheme.colors.onSurfaceVariant, maxLines = 2, overflow = TextOverflow.Ellipsis)
        trailing?.invoke()
    }
}

@Composable private fun AppearanceSettings(c: SettingsController, config: AppConfig) {
    var choice by remember { mutableStateOf<String?>(null) }
    val appearance = config.appearance
    SettingRow("主题", when (appearance.theme) { top.flysoftbeta.workflow.core.config.ThemeMode.SYSTEM -> "跟随系统"; top.flysoftbeta.workflow.core.config.ThemeMode.LIGHT -> "浅色"; else -> "深色" }, { choice = "theme" })
    SettingRow("密度", if (appearance.density == top.flysoftbeta.workflow.core.config.Density.COMPACT) "紧凑" else "标准", { choice = "density" })
    var fontSize by remember(appearance.monoFontSize) { mutableFloatStateOf(appearance.monoFontSize.toFloat()) }
    SettingRow("编辑器 / 终端字号", "${fontSize.toInt()} sp")
    Slider(value = fontSize, onValueChange = { fontSize = it },
        onValueChangeFinished = { c.update { it.copy(appearance = it.appearance.copy(monoFontSize = fontSize.toInt().toDouble())) } },
        valueRange = 10f..20f, steps = 9, modifier = Modifier.fillMaxWidth())
    if (choice != null) AlertDialog(onDismissRequest = { choice = null }, title = { Text(if (choice == "theme") "主题" else "密度") }, text = {
        Column {
            val labels = if (choice == "theme") listOf("跟随系统", "浅色", "深色") else listOf("紧凑", "标准")
            labels.forEachIndexed { index, label ->
                SettingRow(label, onClick = {
                    if (choice == "theme") c.update { it.copy(appearance = it.appearance.copy(theme = top.flysoftbeta.workflow.core.config.ThemeMode.entries[index])) }
                    else c.update { it.copy(appearance = it.appearance.copy(density = top.flysoftbeta.workflow.core.config.Density.entries[index])) }
                    choice = null
                })
            }
        }
    }, confirmButton = { TextButton(onClick = { choice = null }) { Text("取消") } })
}
