package top.flysoftbeta.workflow.feature.launcher

import top.flysoftbeta.workflow.platform.apps.InstalledApps
import top.flysoftbeta.workflow.platform.apps.InstalledApp

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.FilterQuality
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import top.flysoftbeta.workflow.app.Shell
import top.flysoftbeta.workflow.app.sessionLabel
import top.flysoftbeta.workflow.core.config.BuiltinApp
import top.flysoftbeta.workflow.core.config.Launcher
import top.flysoftbeta.workflow.core.config.LauncherEntry
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.store.ConfigUpdate
import top.flysoftbeta.workflow.ui.design.AddAppCell
import top.flysoftbeta.workflow.ui.design.AppGridCell
import top.flysoftbeta.workflow.ui.design.AppGridMetrics
import top.flysoftbeta.workflow.ui.design.BuiltInAppIcon
import top.flysoftbeta.workflow.ui.design.CompositeMenu
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.MenuGroup
import top.flysoftbeta.workflow.ui.design.ReorderableAppGrid
import top.flysoftbeta.workflow.ui.design.SessionChip
import top.flysoftbeta.workflow.ui.design.SymbolIcon
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme
import java.time.ZoneId

/**
 * The Launcher space (product.md §2, ui.md §3.1): the Apps grid (built-ins + added third-party apps),
 * the session chip top-right, nothing else. Back does nothing here.
 */
@Composable
fun LauncherScreen(shell: Shell, modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val catalog = remember { InstalledApps.get(context) }
    val state by shell.store.state.collectAsState()
    val known by catalog.known.collectAsState()
    val attention by remember(shell) { shell.registry.attention }.collectAsState(false)
    val scope = rememberCoroutineScope()
    val entries = Launcher.normalize(state.config.launcher)
    var menuFor by remember { mutableStateOf<String?>(null) }
    var adding by remember { mutableStateOf(false) }

    BackHandler(enabled = true) { /* Launcher: Back does nothing (product.md §2). */ }
    LaunchedEffect(entries) { catalog.ensure(entries.filterIsInstance<LauncherEntry.App>().map { it.ref }) }

    fun update(transform: (List<LauncherEntry>) -> List<LauncherEntry>) {
        scope.launch {
            val result = shell.store.updateConfig { it.copy(launcher = transform(Launcher.normalize(it.launcher))) }
            when (result) {
                is ConfigUpdate.Blocked -> shell.showSnackbar("配置有错误，未保存更改")
                is ConfigUpdate.Failed -> shell.showSnackbar(result.message)
                is ConfigUpdate.Updated -> Unit
            }
        }
    }

    val thirdPartyLabels = entries.filterIsInstance<LauncherEntry.App>().mapNotNull { known[it.id]?.label }.toSet()
    fun builtinLabel(app: BuiltinApp): String {
        val plain = when (app) {
            BuiltinApp.WORKBENCH -> "工作台"
            BuiltinApp.PROXY -> "代理"
            BuiltinApp.SETTINGS -> "设置"
        }
        return if (plain in thirdPartyLabels) "Workflow $plain" else plain
    }

    fun open(entry: LauncherEntry) {
        when (entry) {
            is LauncherEntry.Builtin -> when (entry.app) {
                BuiltinApp.WORKBENCH -> shell.enterWorkbench()
                BuiltinApp.PROXY -> shell.openInCurrentSession(PanelTarget.Proxy())
                BuiltinApp.SETTINGS -> shell.openInCurrentSession(PanelTarget.Settings)
            }
            is LauncherEntry.App -> if (!catalog.launch(entry.ref)) shell.showSnackbar("应用未安装")
        }
    }

    Column(modifier.fillMaxSize().background(WorkflowTheme.colors.surface)) {
        Row(Modifier.fillMaxWidth().heightIn(min = WorkflowTheme.dimens.launcherTouchMin).padding(horizontal = 8.dp), verticalAlignment = Alignment.CenterVertically) {
            Spacer(Modifier.weight(1f))
            val session = state.activeSession ?: state.liveSessions.maxByOrNull { it.lastUsedAt }
            if (session != null) {
                SessionChip(
                    modifier = Modifier.heightIn(min = WorkflowTheme.dimens.launcherTouchMin),
                    label = sessionLabel(session, ZoneId.systemDefault()),
                    onClick = { shell.sessionsOpen = true },
                    onLongClick = { shell.renaming = session.id },
                )
            }
        }
        Box(Modifier.fillMaxWidth().weight(1f).verticalScroll(rememberScrollState()).padding(top = 16.dp, start = 16.dp, end = 16.dp)) {
            ReorderableAppGrid(
                items = entries,
                key = { it.id },
                onMove = { from, to -> entries.getOrNull(from)?.let { moved -> update { Launcher.move(it, moved.id, to) } } },
                onLongPress = { menuFor = it.id },
                trailing = { AddAppCell("添加", onClick = { adding = true; catalog.refresh() }) },
            ) { entry ->
                Box {
                    when (entry) {
                        is LauncherEntry.Builtin -> AppGridCell(
                            builtinLabel(entry.app), { open(entry) },
                            badge = entry.app == BuiltinApp.WORKBENCH && attention,
                        ) {
                            BuiltInAppIcon(when (entry.app) {
                                BuiltinApp.WORKBENCH -> Sym.SpaceDashboard
                                BuiltinApp.PROXY -> Sym.Shield
                                BuiltinApp.SETTINGS -> Sym.Settings
                            })
                        }
                        is LauncherEntry.App -> {
                            val info = known[entry.id]
                            AppGridCell(info?.label ?: "未安装", { open(entry) }) { AppIcon(info) }
                        }
                    }
                    CompositeMenu(
                        expanded = menuFor == entry.id,
                        onDismissRequest = { menuFor = null },
                        groups = tileMenu(entry, shell, catalog, { open(entry) }, { id -> update { Launcher.remove(it, id) } }) +
                            MenuGroup("order", listOf(
                                MenuEntry.Action("earlier", "向前移动", Sym.MoveItem, enabled = entries.indexOf(entry) > 0) {
                                    update { list -> Launcher.move(list, entry.id, (list.indexOfFirst { it.id == entry.id } - 1).coerceAtLeast(0)) }
                                },
                                MenuEntry.Action("later", "向后移动", Sym.MoveItem, enabled = entries.indexOf(entry) < entries.lastIndex) {
                                    update { list -> Launcher.move(list, entry.id, (list.indexOfFirst { it.id == entry.id } + 1).coerceAtMost(list.lastIndex)) }
                                },
                            )),
                    )
                }
            }
        }
    }
    if (adding) {
        AddAppsSheet(
            catalog = catalog,
            added = entries.filterIsInstance<LauncherEntry.App>().map { it.ref.id }.toSet(),
            onToggle = { info, add ->
                update { if (add) Launcher.add(it, LauncherEntry.App(info.ref)) else Launcher.remove(it, info.ref.id) }
            },
            onDone = { adding = false },
        )
    }
}

/** Long-press menu of a tile (ui.md §3.1): 打开 · 在单独的会话中打开 / 新建 Session ┆ 应用信息 · 从 Apps 移除. */
private fun tileMenu(entry: LauncherEntry, shell: Shell, catalog: InstalledApps, open: () -> Unit, remove: (String) -> Unit): List<MenuGroup> {
    val first = buildList {
        add(MenuEntry.Action("open", "打开", Sym.OpenInNew, onClick = open))
        if (entry is LauncherEntry.Builtin) when (entry.app) {
            BuiltinApp.WORKBENCH -> add(MenuEntry.Action("newSession", "新建 Session", Sym.LibraryAdd) { shell.newSession() })
            BuiltinApp.PROXY -> add(MenuEntry.Action("solo", "在单独的会话中打开", Sym.OpenInFull) { shell.openInSeparateSession(PanelTarget.Proxy()) })
            BuiltinApp.SETTINGS -> add(MenuEntry.Action("solo", "在单独的会话中打开", Sym.OpenInFull) { shell.openInSeparateSession(PanelTarget.Settings) })
        }
    }
    val second = if (entry is LauncherEntry.App) listOf(
        MenuEntry.Action("info", "应用信息", Sym.Info) { catalog.openInstalledApp(entry.ref) },
        MenuEntry.Action("remove", "从 Apps 移除", Sym.DoNotDisturbOn, destructive = true) { remove(entry.id) },
    ) else emptyList()
    return listOf(MenuGroup("open", first), MenuGroup("app", second))
}

@Composable
internal fun AppIcon(info: InstalledApp?, size: androidx.compose.ui.unit.Dp = AppGridMetrics.iconSize) {
    val icon = info?.icon
    if (icon != null) {
        Image(icon, null, Modifier.size(size), filterQuality = FilterQuality.Medium)
    } else {
        Box(Modifier.size(size).clip(WorkflowShapes.lg).background(WorkflowTheme.colors.surfaceContainerHigh), contentAlignment = Alignment.Center) {
            SymbolIcon(Sym.Apps, null, tint = WorkflowTheme.colors.onSurfaceVariant)
        }
    }
}
