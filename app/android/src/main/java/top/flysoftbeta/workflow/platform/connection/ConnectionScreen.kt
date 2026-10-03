package top.flysoftbeta.workflow.platform.connection

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import top.flysoftbeta.workflow.core.config.ThemeMode as ConfigTheme
import top.flysoftbeta.workflow.core.config.Density as ConfigDensity
import top.flysoftbeta.workflow.ui.design.theme.ThemeMode
import top.flysoftbeta.workflow.ui.design.theme.UiDensity
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.ui.design.theme.WorkflowDesignTheme
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** First launch explicitly chooses a connection before any workspace state or process is opened. */
@Composable
fun ConnectionScreen(manager: WorkspaceConnectionManager, status: ConnectionStatus) {
    val local by manager.localConfig.collectAsState()
    val theme = when (local.appearance.theme) { ConfigTheme.SYSTEM -> ThemeMode.System; ConfigTheme.LIGHT -> ThemeMode.Light; ConfigTheme.DARK -> ThemeMode.Dark }
    val density = if (local.appearance.density == ConfigDensity.STANDARD) UiDensity.Standard else UiDensity.Compact
    WorkflowDesignTheme(themeMode = theme, density = density) {
        Surface(Modifier.fillMaxSize(), color = WorkflowTheme.colors.surface) {
            Box(Modifier.fillMaxSize().padding(24.dp), contentAlignment = Alignment.Center) {
                Column(Modifier.widthIn(max = 440.dp).fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(16.dp)) {
                    Text("连接工作区", style = WorkflowTheme.text.titleMd)
                    when (status) {
                        ConnectionStatus.Loading -> LinearProgressIndicator(Modifier.fillMaxWidth())
                        is ConnectionStatus.Connecting -> {
                            Text("正在连接「${status.name}」", style = WorkflowTheme.text.body)
                            LinearProgressIndicator(Modifier.fillMaxWidth())
                        }
                        is ConnectionStatus.Failed -> {
                            Text(status.message, style = WorkflowTheme.text.body, color = WorkflowTheme.colors.error)
                            Button(onClick = manager::retry) { Text("重试") }
                        }
                        ConnectionStatus.Configure -> {
                            Text("使用此设备内置的工作区，开始编辑文件、运行终端或对话。", style = WorkflowTheme.text.body)
                            Button(onClick = { manager.useEmbedded() }, modifier = Modifier.fillMaxWidth()) { Text("使用此设备") }
                        }
                        is ConnectionStatus.Connected -> Unit
                    }
                }
            }
        }
    }
}
