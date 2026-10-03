package top.flysoftbeta.workflow.feature.terminal

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.LinearWavyProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import top.flysoftbeta.workflow.platform.engine.EngineController
import top.flysoftbeta.workflow.platform.engine.EnvironmentHealth
import top.flysoftbeta.workflow.ui.design.NoticeBar
import top.flysoftbeta.workflow.ui.design.NoticeTone
import top.flysoftbeta.workflow.ui.design.TextAction
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/**
 * The environment's state where the terminal is visible (docs/ui.md §6): progress while it is prepared,
 * "环境配置已更改 [重启环境]" only after env.json changed, and a failed build with its log. Nothing
 * while the environment is simply ready.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun EnvironmentNotice(controller: EngineController, onRestart: () -> Unit) {
    val health by controller.health.collectAsState()
    var details by remember { mutableStateOf<EnvironmentHealth.Failed?>(null) }
    when (val h = health) {
        is EnvironmentHealth.Installing, is EnvironmentHealth.Provisioning -> {
            val progress = when (h) {
                is EnvironmentHealth.Installing -> h.progress
                is EnvironmentHealth.Provisioning -> h.progress
                else -> null
            }
            val label = when (h) {
                is EnvironmentHealth.Provisioning -> h.step
                else -> "正在准备环境"
            }
            Column(
                Modifier.fillMaxWidth().background(WorkflowTheme.colors.surfaceContainerHigh).padding(horizontal = 12.dp, vertical = 6.dp)
                    .testTag("environment:preparing"),
            ) {
                val percent = progress?.let { " ${(it * 100).toInt()}%" }.orEmpty()
                Text("$label$percent", style = WorkflowTheme.text.label, color = WorkflowTheme.colors.onSurface, maxLines = 1,
                    overflow = TextOverflow.Ellipsis)
                if (progress != null) LinearWavyProgressIndicator(progress = { progress }, modifier = Modifier.fillMaxWidth().padding(top = 4.dp))
                else LinearWavyProgressIndicator(modifier = Modifier.fillMaxWidth().padding(top = 4.dp))
            }
        }
        is EnvironmentHealth.NeedsRestart -> NoticeBar(
            if (h.reason == top.flysoftbeta.workflow.core.environment.ActivationReason.ROLLBACK) "已回滚环境配置" else "环境配置已更改",
            NoticeTone.Tertiary, Modifier.testTag("environment:restart"),
            actions = listOf(TextAction("重启环境", onRestart)),
        )
        is EnvironmentHealth.Ready -> h.applying?.let { applying ->
            NoticeBar("正在应用环境配置 ${applying.step}/${applying.steps}", NoticeTone.Neutral, Modifier.testTag("environment:applying"))
        }
        is EnvironmentHealth.Failed -> NoticeBar(
            if (h.environmentAvailable) "环境配置未能应用：${h.message}" else "环境安装失败：${h.message}",
            if (h.environmentAvailable) NoticeTone.Warning else NoticeTone.Error, Modifier.testTag("environment:failed"),
            actions = listOf(TextAction("详情") { details = h }, TextAction("重试") { controller.retry() }),
        )
        EnvironmentHealth.NotInstalled, is EnvironmentHealth.Unavailable -> Unit
    }
    details?.let { failed -> FailureDialog(controller, failed) { details = null } }
}

@Composable
private fun FailureDialog(controller: EngineController, failed: EnvironmentHealth.Failed, onDismiss: () -> Unit) {
    var tail by remember { mutableStateOf("") }
    LaunchedEffect(failed) { tail = withContext(Dispatchers.IO) { controller.logTail(failed.log) } }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("环境（${failed.stage}）", style = WorkflowTheme.text.titleMd) },
        text = {
            Column(Modifier.heightIn(max = 360.dp).verticalScroll(rememberScrollState())) {
                Text(failed.message, style = WorkflowTheme.text.body)
                if (tail.isNotEmpty()) {
                    Text(tail, Modifier.padding(top = 8.dp), style = WorkflowTheme.text.monoBlock, color = WorkflowTheme.colors.onSurfaceVariant)
                }
            }
        },
        confirmButton = { TextButton(onClick = { onDismiss(); controller.retry() }) { Text("重试") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("关闭") } },
    )
}
