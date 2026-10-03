package top.flysoftbeta.workflow.feature.proxy

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.ButtonGroupDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.Text
import androidx.compose.material3.ToggleButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import top.flysoftbeta.workflow.app.panel.DecisionOption
import top.flysoftbeta.workflow.app.panel.DecisionRequest
import top.flysoftbeta.workflow.app.panel.DecisionStyle
import top.flysoftbeta.workflow.app.panel.PanelContext
import top.flysoftbeta.workflow.app.panel.PanelFrame
import top.flysoftbeta.workflow.app.panel.PanelTab
import top.flysoftbeta.workflow.core.layout.ProxyPage
import top.flysoftbeta.workflow.proxy.config.IssueCode
import top.flysoftbeta.workflow.proxy.config.IssueSeverity
import top.flysoftbeta.workflow.proxy.config.LocalProxyConfig
import top.flysoftbeta.workflow.proxy.controller.ProxyGroup
import top.flysoftbeta.workflow.proxy.controller.ProxyMode
import top.flysoftbeta.workflow.proxy.controller.ProxyNode
import top.flysoftbeta.workflow.proxy.io.readProxyBytes
import top.flysoftbeta.workflow.proxy.runtime.ProxyApi
import top.flysoftbeta.workflow.proxy.runtime.ProxyPhase
import top.flysoftbeta.workflow.proxy.runtime.ProxyState
import top.flysoftbeta.workflow.proxy.runtime.RootStatus
import top.flysoftbeta.workflow.ui.design.EmptyState
import top.flysoftbeta.workflow.ui.design.InlineError
import top.flysoftbeta.workflow.ui.design.InlineLoading
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.NoticeBar
import top.flysoftbeta.workflow.ui.design.NoticeTone
import top.flysoftbeta.workflow.ui.design.RegionLoading
import top.flysoftbeta.workflow.ui.design.SymbolIcon
import top.flysoftbeta.workflow.ui.design.TextAction
import top.flysoftbeta.workflow.ui.design.ToolAction
import top.flysoftbeta.workflow.ui.design.WfIconButton
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

private const val COLLAPSED_KEY = "collapsed"
private val TUN_ISSUES = setOf(
    IssueCode.TUN_DEFAULT_TABLE, IssueCode.TUN_DEFAULT_RULE_INDEX, IssueCode.TUN_MARK_COLLISION, IssueCode.TUN_OVERRIDES_ANDROID_VPN,
)

/**
 * 代理 (docs/ui.md §4.12): one compact screen. A 48dp status strip (start/stop, mode, live traffic) over the
 * proxy groups with node chips and latency. While the kernel is stopped the mode and groups come from
 * config.yaml and are shown disabled.
 */
internal class ProxyOverviewController(api: ProxyApi, context: PanelContext) : ProxyPanelBase(api, context) {
    private var view = context.initialView
    private var collapsed by mutableStateOf(view.extras[COLLAPSED_KEY]?.split('\n')?.filter { it.isNotEmpty() }?.toSet().orEmpty())
    /** The first refresh finished (before it, "没有代理配置" would flash). */
    private var loaded by mutableStateOf(false)
    private var testingAll by mutableStateOf(false)
    private var updatingProviders by mutableStateOf(false)
    /** Incremented by "导入配置…"; Content launches the document picker for each new value. */
    private var importRequests by mutableIntStateOf(0)

    init {
        context.scope.launch { api.refresh(); loaded = true }
    }

    override val tab: PanelTab get() = PanelTab("代理", Sym.Shield)

    override val actions: List<ToolAction> get() {
        val current = state
        return listOf(
            ToolAction("test", Sym.Bolt, "延迟测试", enabled = current.live && !testingAll) { testAll() },
        )
    }

    override val resourceMenu: List<MenuEntry> get() = buildList {
        val current = state
        add(MenuEntry.Action("edit", "编辑配置", Sym.EditDocument) { editConfig() })
        add(MenuEntry.Action("import", "导入配置…", Sym.UploadFile, enabled = !current.busy) { importRequests++ })
        val inspection = current.config.inspection
        if (inspection != null && (inspection.proxyProviders.isNotEmpty() || inspection.ruleProviders.isNotEmpty())) {
            add(MenuEntry.Action("providers", if (updatingProviders) "正在更新订阅与规则…" else "更新订阅与规则", Sym.Refresh, enabled = current.live && !updatingProviders) { refreshProviders() })
        }
        addAll(pageEntries(ProxyPage.OVERVIEW))
    }

    private fun testAll() {
        if (testingAll) return
        testingAll = true
        context.scope.launch {
            try {
                val tested = HashSet<String>()
                for (group in ProxyFormat.visibleGroups(api.state.value.groups, api.state.value.mode)) {
                    val members = group.members.filter(ProxyFormat::testable).map { it.name }
                    if (members.isEmpty() || tested.containsAll(members)) continue
                    val result = api.testGroup(group.name)
                    result.getOrNull()?.let { tested += it.keys }
                    if (result.isFailure) { report(result); break }
                }
            } finally { testingAll = false }
        }
    }

    private fun testGroup(group: ProxyGroup) = context.scope.launch { report(api.testGroup(group.name)) }

    /** Tests [node] with the URL of the [group] it is shown in (Mihomo's default when the group has none). */
    private fun testNode(group: ProxyGroup, node: ProxyNode) = context.scope.launch { report(api.testNode(node.name, group.testUrl)) }

    private fun select(group: ProxyGroup, node: ProxyNode) {
        if (group.now == node.name && group.fixed == null) return
        context.scope.launch { report(api.select(group.name, node.name)) }
    }

    private fun setMode(mode: ProxyMode) {
        if (state.mode == mode) return
        context.scope.launch { report(api.setMode(mode)) }
    }

    private fun refreshProviders() = context.scope.launch {
        if (updatingProviders) return@launch
        updatingProviders = true
        try { api.refreshProviders()
            .onSuccess { results ->
                val failed = results.filter { it.error != null }
                context.commands.snackbar(when {
                    results.isEmpty() -> "没有可更新的订阅或规则"
                    failed.isEmpty() -> "已更新 ${results.size} 项"
                    else -> "${failed.size} 项更新失败：${failed.first().provider.name}"
                })
            }
            .onFailure { context.commands.snackbar(it.message ?: "更新失败") }
        } finally { updatingProviders = false }
    }

    private fun toggleCollapsed(group: String) {
        collapsed = if (group in collapsed) collapsed - group else collapsed + group
        view = view.copy(extras = view.extras + (COLLAPSED_KEY to collapsed.sorted().joinToString("\n")))
        context.updateView(view)
    }

    private fun import(uri: Uri) = context.scope.launch {
        val bytes = runCatching {
            withContext(Dispatchers.IO) {
                context.appContext.contentResolver.openInputStream(uri)?.use { readProxyBytes(it, LocalProxyConfig.MAX_CONFIG_BYTES) }
                    ?: error("无法读取所选文件")
            }
        }.getOrElse { context.commands.snackbar(if (it is java.io.IOException) "文件超过 16 MiB" else it.message ?: "无法读取所选文件"); return@launch }
        if (api.state.value.config.exists) {
            val choice = context.commands.decide(DecisionRequest("替换代理配置？", "当前的 config.yaml 会被所选文件替换。",
                options = listOf(DecisionOption("replace", "替换", DecisionStyle.Destructive))))
            if (choice != "replace") return@launch
        }
        api.importConfig(bytes)
            .onSuccess {
                api.refresh()
                context.commands.snackbar(if (api.state.value.running) "已导入配置 · 重新启动后生效" else "已导入配置")
            }
            .onFailure { context.commands.snackbar(it.message ?: "导入失败") }
    }

    private fun showTunIssues(state: ProxyState) = context.scope.launch {
        val issues = state.config.inspection?.issues.orEmpty().filter { it.code in TUN_ISSUES }
        val choice = context.commands.decide(DecisionRequest("TUN 设置可能与其他代理冲突", null, issues.map { it.message },
            listOf(DecisionOption("edit", "编辑配置", DecisionStyle.Tonal))))
        if (choice == "edit") editConfig()
    }

    @Composable
    override fun Content(frame: PanelFrame, modifier: Modifier) {
        val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri -> uri?.let(::import) }
        var handledImports by remember { mutableIntStateOf(importRequests) }
        LaunchedEffect(importRequests) {
            if (importRequests != handledImports) {
                handledImports = importRequests
                runCatching { picker.launch(arrayOf("*/*")) }.onFailure { context.commands.snackbar("没有可用的文件选择器") }
            }
        }
        // Poll only while visible: config edits, other VPNs appearing, url-test switches.
        LaunchedEffect(Unit) {
            while (true) {
                delay(if (api.state.value.live) 3000 else 2000)
                api.refresh()
            }
        }
        val current = state
        Box(modifier.fillMaxSize().background(WorkflowTheme.colors.surface).testTag("proxy:overview")) {
            when {
                !loaded -> RegionLoading(true)
                !current.config.exists && !current.running && !current.busy -> EmptyState(
                    listOf(TextAction("导入配置") { importRequests++ }, TextAction("编辑配置") { editConfig() }),
                    message = "没有代理配置",
                )
                else -> Column(Modifier.fillMaxSize()) {
                    Notices(current)
                    StatusStrip(current)
                    Groups(current)
                }
            }
        }
    }

    @Composable
    private fun Notices(current: ProxyState) {
        val inspection = current.config.inspection
        val tunEnabled = inspection?.tun?.enable == true
        if (current.phase == ProxyPhase.CONFLICT || (tunEnabled && current.conflict.any && !current.busy)) {
            val title = if (current.conflict.vpnActive || current.conflict.foreignInterfaces.isNotEmpty()) "另一个 VPN 正在运行" else "TUN 设置与其他代理冲突"
            NoticeBar(title, NoticeTone.Error, Modifier.testTag("proxy:conflict"),
                listOf(TextAction("查看") { context.scope.launch { explainConflict(current.conflict) } }))
        } else if (tunEnabled && !current.running && inspection.issues.any { it.code in TUN_ISSUES }) {
            NoticeBar("TUN 设置可能与其他代理冲突", NoticeTone.Warning, actions = listOf(TextAction("详情") { showTunIssues(current) }))
        }
        val problem = when {
            !current.capabilities.kernel -> "此设备没有可用的代理内核"
            !current.capabilities.guardian -> "代理进程管理组件不可用"
            current.config.error != null -> current.config.error
            else -> inspection?.issues?.firstOrNull { it.severity == IssueSeverity.BLOCKING }?.message
                ?: inspection?.issues?.firstOrNull { it.code == IssueCode.CONTROLLER_MISSING || it.code == IssueCode.CONTROLLER_REJECTED }?.message
        }
        val error = current.error.takeIf { current.phase != ProxyPhase.CONFLICT }
        when {
            error != null -> InlineError(error, Modifier.testTag("proxy:error"), buildList {
                add(TextAction("日志") { openPage(ProxyPage.LOGS) })
                if (current.stopUnconfirmed && !current.busy) add(TextAction("停止") { toggle() })
                else if (current.phase == ProxyPhase.ERROR) add(TextAction("关闭") { api.dismiss() })
            })
            current.stopUnconfirmed && !current.busy -> InlineError("上次运行的代理未确认结束", actions = listOf(TextAction("停止") { toggle() }))
            problem != null && !current.running -> InlineError(problem, actions = listOf(TextAction("编辑配置") { editConfig() }))
        }
    }

    @OptIn(ExperimentalLayoutApi::class)
    @Composable
    private fun StatusStrip(current: ProxyState) {
        val dimens = WorkflowTheme.dimens
        FlowRow(
            Modifier.fillMaxWidth().heightIn(min = 48.dp).padding(horizontal = dimens.padH, vertical = 4.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp),
            itemVerticalAlignment = Alignment.CenterVertically,
        ) {
            PowerControl(current)
            ModeControl(current)
            Traffic(current, Modifier.weight(1f))
        }
    }

    @OptIn(ExperimentalMaterial3ExpressiveApi::class)
    @Composable
    private fun PowerControl(current: ProxyState) {
        val text = WorkflowTheme.text
        if (current.capabilities.root == RootStatus.DENIED && !current.running && !current.busy && current.canStart) {
            FilledTonalButton(onClick = { toggle() }, Modifier.height(40.dp).testTag("proxy:root"),
                contentPadding = PaddingValues(horizontal = 16.dp)) {
                SymbolIcon(Sym.Key, null, size = 18.dp)
                Spacer(Modifier.width(8.dp))
                Text("请求 Root", style = text.labelActive)
            }
            return
        }
        val label = when (current.phase) {
            ProxyPhase.STARTING -> current.progress ?: "正在启动"
            ProxyPhase.STOPPING -> current.progress ?: "正在停止"
            ProxyPhase.RUNNING -> "运行中"
            else -> "已停止"
        }
        ToggleButton(
            checked = current.running,
            onCheckedChange = { toggle() },
            modifier = Modifier.height(40.dp).widthIn(max = 280.dp).testTag("proxy:power")
                .semantics { stateDescription = if (current.running) "运行中" else "已停止"; contentDescription = "代理：$label" },
            enabled = current.toggleEnabled,
            contentPadding = PaddingValues(horizontal = 16.dp),
        ) {
            if (current.busy) LoadingIndicator(Modifier.size(18.dp), color = LocalContentColor.current)
            else SymbolIcon(Sym.PowerSettingsNew, null, size = 18.dp)
            Spacer(Modifier.width(8.dp))
            Text(label, style = text.labelActive, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
    }

    @OptIn(ExperimentalMaterial3ExpressiveApi::class)
    @Composable
    private fun ModeControl(current: ProxyState) {
        val modes = listOf(ProxyMode.RULE to "规则", ProxyMode.GLOBAL to "全局", ProxyMode.DIRECT to "直连")
        Row(horizontalArrangement = Arrangement.spacedBy(ButtonGroupDefaults.ConnectedSpaceBetween), modifier = Modifier.testTag("proxy:mode")) {
            modes.forEachIndexed { index, (mode, label) ->
                val shapes = when (index) {
                    0 -> ButtonGroupDefaults.connectedLeadingButtonShapes()
                    modes.lastIndex -> ButtonGroupDefaults.connectedTrailingButtonShapes()
                    else -> ButtonGroupDefaults.connectedMiddleButtonShapes()
                }
                ToggleButton(
                    checked = current.mode == mode,
                    onCheckedChange = { setMode(mode) },
                    modifier = Modifier.height(40.dp).semantics { role = Role.RadioButton },
                    enabled = current.live,
                    shapes = shapes,
                    contentPadding = PaddingValues(horizontal = 14.dp),
                ) { Text(label, style = WorkflowTheme.text.label, maxLines = 1) }
            }
        }
    }

    @Composable
    private fun Traffic(current: ProxyState, modifier: Modifier) {
        val sample = current.traffic.takeIf { current.running } ?: return Spacer(modifier)
        val style = WorkflowTheme.text.label.copy(fontFeatureSettings = "tnum")
        val color = WorkflowTheme.colors.onSurfaceVariant
        Row(modifier.testTag("proxy:traffic"), horizontalArrangement = Arrangement.End, verticalAlignment = Alignment.CenterVertically) {
            SymbolIcon(Sym.ArrowUpward, "上传", size = 14.dp, tint = color)
            Text(ProxyFormat.rate(sample.up), style = style, color = color, maxLines = 1)
            Spacer(Modifier.width(12.dp))
            SymbolIcon(Sym.ArrowDownward, "下载", size = 14.dp, tint = color)
            Text(ProxyFormat.rate(sample.down), style = style, color = color, maxLines = 1)
        }
    }

    @Composable
    private fun Groups(current: ProxyState) {
        val groups = ProxyFormat.visibleGroups(current.groups, current.mode)
        if (groups.isEmpty()) {
            if (current.config.inspection != null && !current.busy) {
                EmptyState(listOf(TextAction("编辑配置") { editConfig() }), message = "配置中没有代理组")
            }
            return
        }
        // Stopped: read from config.yaml, not selectable (docs/ui.md §4.12). Direct mode: groups still apply later.
        val dimmed = !current.live || current.mode == ProxyMode.DIRECT
        BoxWithConstraints(Modifier.fillMaxSize()) {
            val density = LocalDensity.current
            val padH = WorkflowTheme.dimens.padH
            val columns = with(density) {
                ProxyFormat.columns((maxWidth - padH * 2).toPx(), 168.dp.toPx(), 8.dp.toPx())
            }
            val list = rememberLazyListState()
            LazyColumn(
                Modifier.fillMaxSize().alpha(if (dimmed) 0.62f else 1f).testTag("proxy:groups"),
                state = list,
                contentPadding = PaddingValues(bottom = 12.dp),
            ) {
                for (group in groups) {
                    item(key = "g:${group.name}", contentType = "header") { GroupHeader(group, current) }
                    if (group.name !in collapsed) {
                        val rows = group.members.chunked(columns)
                        rows.forEachIndexed { index, row ->
                            item(key = "n:${group.name}:$index", contentType = "nodes") {
                                Row(
                                    Modifier.fillMaxWidth().padding(start = padH, end = padH, bottom = 8.dp),
                                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                                ) {
                                    row.forEach { node -> NodeChip(group, node, current, Modifier.weight(1f)) }
                                    repeat(columns - row.size) { Spacer(Modifier.weight(1f)) }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    @Composable
    private fun GroupHeader(group: ProxyGroup, current: ProxyState) {
        val colors = WorkflowTheme.colors
        val text = WorkflowTheme.text
        val expanded = group.name !in collapsed
        Row(
            Modifier.fillMaxWidth().height(40.dp).clickable(onClickLabel = if (expanded) "收起" else "展开") { toggleCollapsed(group.name) }
                .padding(start = WorkflowTheme.dimens.padH - 4.dp, end = 4.dp)
                .semantics { stateDescription = if (expanded) "已展开" else "已收起" },
            verticalAlignment = Alignment.CenterVertically,
        ) {
            SymbolIcon(if (expanded) Sym.KeyboardArrowDown else Sym.ChevronRight, null, size = 18.dp, tint = colors.onSurfaceVariant)
            Spacer(Modifier.width(4.dp))
            SymbolIcon(if (ProxyFormat.automatic(group)) Sym.Speed else Sym.Language,
                if (ProxyFormat.automatic(group)) "自动测速" else "手动选择", size = 18.dp, tint = colors.onSurfaceVariant)
            Spacer(Modifier.width(8.dp))
            Text(group.name, style = text.titleSm, color = colors.onSurface, maxLines = 1, overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f, fill = false))
            Spacer(Modifier.width(16.dp).weight(1f))
            group.now?.let {
                Text(it, style = text.label, color = colors.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.widthIn(max = 240.dp))
            }
            Box(Modifier.size(WorkflowTheme.dimens.iconButtonTouch), contentAlignment = Alignment.Center) {
                if (group.name in current.testing) {
                    InlineLoading(true)
                } else {
                    WfIconButton(Sym.Bolt, "测速：${group.name}", { testGroup(group) }, enabled = current.live && current.mode != ProxyMode.DIRECT)
                }
            }
        }
    }

    @OptIn(ExperimentalFoundationApi::class)
    @Composable
    private fun NodeChip(group: ProxyGroup, node: ProxyNode, current: ProxyState, modifier: Modifier) {
        val colors = WorkflowTheme.colors
        val extended = WorkflowTheme.extendedColors
        val text = WorkflowTheme.text
        val selected = group.now == node.name
        val interactive = current.live && current.mode != ProxyMode.DIRECT && ProxyFormat.canSelect(group)
        val testable = current.live && current.mode != ProxyMode.DIRECT && ProxyFormat.testable(node)
        val latency = ProxyFormat.latency(current.delays[node.name], node.delayMs)
        val latencyColor = when (latency?.tier) {
            ProxyFormat.Tier.GOOD -> extended.success
            ProxyFormat.Tier.FAIR -> extended.warning
            ProxyFormat.Tier.BAD -> colors.error
            null -> colors.onSurfaceVariant
        }
        val container = if (selected) colors.secondaryContainer else colors.surfaceContainerLow
        val content = if (selected) colors.onSecondaryContainer else colors.onSurface
        Row(
            modifier.height(36.dp).background(container, WorkflowShapes.sm)
                .combinedClickable(enabled = interactive, role = Role.Button,
                    onLongClick = if (testable) ({ testNode(group, node) }) else null) { select(group, node) }
                .semantics {
                    this.selected = selected
                    contentDescription = node.name + (latency?.let { "，${it.text}" + if (it.text.all(Char::isDigit)) " 毫秒" else "" } ?: "")
                    if (testable) customActions = listOf(CustomAccessibilityAction("测速") { testNode(group, node); true })
                }
                .testTag("proxy:node:${group.name}:${node.name}"),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Spacer(Modifier.width(10.dp))
            if (selected) {
                SymbolIcon(Sym.Check, null, size = 16.dp, tint = content)
                Spacer(Modifier.width(4.dp))
            }
            Text(node.name, style = if (selected) text.labelActive else text.label, color = content,
                maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
            Box(
                Modifier.fillMaxHeight().widthIn(min = 44.dp)
                    .clickable(enabled = testable, onClickLabel = "测速") { testNode(group, node) }
                    .padding(horizontal = 8.dp),
                contentAlignment = Alignment.CenterEnd,
            ) {
                when {
                    node.name in current.testing -> InlineLoading(true)
                    latency != null -> Text(latency.text, style = text.caption.copy(fontFeatureSettings = "tnum"), color = latencyColor, maxLines = 1)
                    testable -> SymbolIcon(Sym.Bolt, null, size = 14.dp, tint = colors.onSurfaceVariant.copy(alpha = 0.7f))
                }
            }
        }
    }
}
