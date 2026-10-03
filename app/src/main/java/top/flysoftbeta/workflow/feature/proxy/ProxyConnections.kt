package top.flysoftbeta.workflow.feature.proxy

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import top.flysoftbeta.workflow.app.panel.PanelContext
import top.flysoftbeta.workflow.app.panel.PanelFrame
import top.flysoftbeta.workflow.app.panel.PanelTab
import top.flysoftbeta.workflow.core.layout.ProxyPage
import top.flysoftbeta.workflow.proxy.controller.ConnectionsSnapshot
import top.flysoftbeta.workflow.proxy.controller.ProxyConnection
import top.flysoftbeta.workflow.proxy.runtime.ProxyApi
import top.flysoftbeta.workflow.ui.design.EmptyState
import top.flysoftbeta.workflow.ui.design.InlineError
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme
import java.time.Duration
import java.time.OffsetDateTime

/** 连接: read-only list of the kernel's tracked connections, refreshed every second while visible. */
internal class ProxyConnectionsController(api: ProxyApi, context: PanelContext) : ProxyPanelBase(api, context) {
    private var snapshot by mutableStateOf<ConnectionsSnapshot?>(null)
    private var failure by mutableStateOf<String?>(null)

    override val tab: PanelTab get() = PanelTab("连接", Sym.Lan)

    override val resourceMenu: List<MenuEntry> get() = pageEntries(ProxyPage.CONNECTIONS)

    @Composable
    override fun Content(frame: PanelFrame, modifier: Modifier) {
        val current = state
        LaunchedEffect(current.live) {
            if (!current.live) { snapshot = null; failure = null; return@LaunchedEffect }
            while (true) {
                api.connections()
                    .onSuccess { next -> snapshot = next.copy(connections = next.connections.sortedByDescending { it.start }); failure = null }
                    .onFailure { failure = it.message ?: "无法读取连接" }
                delay(1000)
            }
        }
        val colors = WorkflowTheme.colors
        val text = WorkflowTheme.text
        val padH = WorkflowTheme.dimens.padH
        Box(modifier.fillMaxSize().background(colors.surface).testTag("proxy:connections")) {
            val data = snapshot
            when {
                !current.running -> EmptyState(listOf(), message = "代理未运行")
                !current.live -> InlineError("控制接口未连接，无法读取连接")
                failure != null && data == null -> InlineError(failure!!)
                data == null -> Unit
                else -> Column(Modifier.fillMaxSize()) {
                    failure?.let { InlineError(it) }
                    Row(Modifier.fillMaxWidth().height(36.dp).padding(horizontal = padH), verticalAlignment = Alignment.CenterVertically) {
                        Text("${data.connections.size} 个连接", style = text.label, color = colors.onSurfaceVariant, modifier = Modifier.weight(1f))
                        Text("↑ ${ProxyFormat.size(data.uploadTotal)}  ↓ ${ProxyFormat.size(data.downloadTotal)}",
                            style = text.label.copy(fontFeatureSettings = "tnum"), color = colors.onSurfaceVariant)
                    }
                    HorizontalDivider(color = colors.outlineVariant)
                    if (data.connections.isEmpty()) EmptyState(emptyList(), message = "没有活动连接")
                    LazyColumn(Modifier.fillMaxSize()) {
                        items(data.connections, key = { it.id }, contentType = { "connection" }) { ConnectionRow(it) }
                    }
                }
            }
        }
    }

    @Composable
    private fun ConnectionRow(connection: ProxyConnection) {
        val colors = WorkflowTheme.colors
        val text = WorkflowTheme.text
        val padH = WorkflowTheme.dimens.padH
        Column(
            Modifier.fillMaxWidth().heightIn(min = WorkflowTheme.dimens.listRowTwoLine).padding(horizontal = padH, vertical = 4.dp),
            verticalArrangement = Arrangement.Center,
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(connection.host.ifEmpty { connection.destination }, style = text.body, color = colors.onSurface,
                    maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
                Spacer(Modifier.width(12.dp))
                Text("↑ ${ProxyFormat.size(connection.upload)}  ↓ ${ProxyFormat.size(connection.download)}",
                    style = text.caption.copy(fontFeatureSettings = "tnum"), color = colors.onSurfaceVariant, maxLines = 1)
            }
            Text(details(connection), style = text.caption, color = colors.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
    }

    private fun details(connection: ProxyConnection): String = buildList {
        add(connection.network.uppercase())
        if (connection.host.isNotEmpty() && connection.destination.isNotEmpty()) add(connection.destination)
        add(if (connection.rulePayload.isEmpty()) connection.rule else "${connection.rule}(${connection.rulePayload})")
        if (connection.chains.isNotEmpty()) add(connection.chains.asReversed().joinToString(" › "))
        if (connection.process.isNotEmpty()) add(connection.process)
        age(connection.start)?.let(::add)
    }.filter { it.isNotBlank() }.joinToString(" · ")

    private fun age(start: String): String? {
        val started = runCatching { OffsetDateTime.parse(start) }.getOrNull() ?: return null
        val seconds = Duration.between(started, OffsetDateTime.now()).seconds.coerceAtLeast(0)
        return when {
            seconds < 60 -> "${seconds} 秒"
            seconds < 3600 -> "${seconds / 60} 分钟"
            else -> "${seconds / 3600} 小时"
        }
    }
}
