package top.flysoftbeta.workflow.ui.design.gallery

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.ui.design.AddAppCell
import top.flysoftbeta.workflow.ui.design.AppGridCell
import top.flysoftbeta.workflow.ui.design.AttachmentItem
import top.flysoftbeta.workflow.ui.design.AttachmentState
import top.flysoftbeta.workflow.ui.design.AttachmentStrip
import top.flysoftbeta.workflow.ui.design.BuiltInAppIcon
import top.flysoftbeta.workflow.ui.design.EffortOption
import top.flysoftbeta.workflow.ui.design.EmptyState
import top.flysoftbeta.workflow.ui.design.GroupHeader
import top.flysoftbeta.workflow.ui.design.HairlineCaption
import top.flysoftbeta.workflow.ui.design.InlineError
import top.flysoftbeta.workflow.ui.design.InlineLoading
import top.flysoftbeta.workflow.ui.design.ModelEffortPanel
import top.flysoftbeta.workflow.ui.design.ModelEffortSelection
import top.flysoftbeta.workflow.ui.design.ModelOption
import top.flysoftbeta.workflow.ui.design.NoticeBar
import top.flysoftbeta.workflow.ui.design.NoticeTone
import top.flysoftbeta.workflow.ui.design.RegionCard
import top.flysoftbeta.workflow.ui.design.ReorderableAppGrid
import top.flysoftbeta.workflow.ui.design.StandardEffortLabels
import top.flysoftbeta.workflow.ui.design.TextAction
import top.flysoftbeta.workflow.ui.design.TimelineAge
import top.flysoftbeta.workflow.ui.design.TimelineEntry
import top.flysoftbeta.workflow.ui.design.WfIconButton
import top.flysoftbeta.workflow.ui.design.WfSnackbarHost
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.showUndo
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

private fun efforts(vararg ids: String) = ids.map { EffortOption(it, StandardEffortLabels.getValue(it)) }

internal val demoModels = listOf(
    ModelOption("gpt-5.5", "GPT-5.5", efforts("low", "medium", "high", "xhigh")),
    ModelOption("gpt-5.5-mini", "GPT-5.5 mini", efforts("minimal", "low", "medium", "high")),
    ModelOption("codex-max", "Codex Max", efforts("medium", "high", "xhigh")),
)

private data class DemoApp(val id: String, val label: String, val icon: Int?, val badge: Boolean = false)

/** Content primitives: timeline, launcher grid, model slider, attachments, states, snackbar, icon buttons. */
@Composable
fun ContentPage() {
    val snackbar = remember { SnackbarHostState() }
    LaunchedEffect(Unit) { snackbar.showUndo("已归档 · 周日 14:32") }
    Box(Modifier.fillMaxSize()) {
        Row(Modifier.fillMaxSize().padding(8.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Column(Modifier.weight(1f).fillMaxSize().verticalScroll(rememberScrollState())) {
                GallerySection("Session view: temporary timeline (age fading)") {
                    RegionCard(Modifier.fillMaxWidth()) {
                        Column(Modifier.padding(vertical = 4.dp)) {
                            GroupHeader("今天")
                            TimelineEntry("14:32", listOf("prompt.md", "Theme.kt", "梳理 Context Engine", "README.md"), TimelineAge.Today, {}, current = true, isFirst = true)
                            TimelineEntry("09:10", listOf("container.json", "bash"), TimelineAge.Today, {})
                            GroupHeader("昨天")
                            TimelineEntry("周六 21:04", listOf("修复 Agent Runtime", "LocalCodexSession.kt", "x"), TimelineAge.Yesterday, {})
                            GroupHeader("本周")
                            TimelineEntry("周一 10:15", listOf("proxy config.yaml", "日志"), TimelineAge.ThisWeek, {})
                            GroupHeader("更早")
                            TimelineEntry("9月14日 16:40", listOf("ui.md", "product.md"), TimelineAge.Older, {}, isLast = true)
                        }
                    }
                }
                GallerySection("Model + effort slider (models = segments, efforts = detents)") {
                    var a by remember { mutableStateOf(ModelEffortSelection("gpt-5.5", "high")) }
                    var b by remember { mutableStateOf(ModelEffortSelection("gpt-5.5-mini", "minimal")) }
                    Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        ModelEffortPanel(demoModels, a, { a = it }, {}, speedActive = true)
                        ModelEffortPanel(demoModels.take(2), b, { b = it }, {})
                    }
                }
                GallerySection("Attachment strip") {
                    RegionCard(Modifier.fillMaxWidth()) {
                        AttachmentStrip(
                            listOf(
                                AttachmentItem("1", "screenshot.png", isImage = true),
                                AttachmentItem("2", "upload.jpg", isImage = true, state = AttachmentState.Uploading(null)),
                                AttachmentItem("3", "Theme.kt", isImage = false, icon = Sym.Code),
                                AttachmentItem("4", "a-very-long-file-name-that-ellipsizes.md", isImage = false, icon = Sym.Article),
                                AttachmentItem("5", "broken.zip", isImage = false, icon = Sym.FolderZip, state = AttachmentState.Failed),
                            ),
                            onRemove = {},
                        )
                    }
                }
                GallerySection("Transcript separators") {
                    HairlineCaption("15:02")
                    HairlineCaption("已压缩上下文")
                }
            }
            Column(Modifier.weight(1f).fillMaxSize().verticalScroll(rememberScrollState())) {
                GallerySection("Launcher grid (long-press to reorder / menu)") {
                    val apps = remember {
                        mutableStateListOf(
                            DemoApp("wb", "工作台", top.flysoftbeta.workflow.R.drawable.ic_workflow_glyph, badge = true), DemoApp("proxy", "代理", Sym.Shield),
                            DemoApp("settings", "Workflow 设置", Sym.Settings), DemoApp("brave", "Brave", null), DemoApp("gboard", "Gboard", null),
                        )
                    }
                    RegionCard(Modifier.fillMaxWidth()) {
                        ReorderableAppGrid(
                            items = apps, key = { it.id },
                            onMove = { from, to -> apps.add(to, apps.removeAt(from)) },
                            onLongPress = {},
                            trailing = { AddAppCell("添加", {}) },
                        ) { app ->
                            AppGridCell(app.label, {}, badge = app.badge) {
                                if (app.icon != null) BuiltInAppIcon(app.icon) else ThirdPartyPlaceholder(app.label)
                            }
                        }
                    }
                }
                GallerySection("Notice bars / inline error / empty state / loading") {
                    NoticeBar("磁盘版本已更改", NoticeTone.Tertiary, actions = listOf(TextAction("保留我的") {}, TextAction("使用磁盘版本") {}, TextAction("对比") {}))
                    Spacer(Modifier.height(4.dp))
                    NoticeBar("另一个 VPN 正在运行", NoticeTone.Error, actions = listOf(TextAction("查看") {}))
                    Spacer(Modifier.height(4.dp))
                    NoticeBar("环境配置已更改", NoticeTone.Neutral, actions = listOf(TextAction("重启环境") {}))
                    InlineError("无法连接到代理控制接口", actions = listOf(TextAction("重试") {}, TextAction("详情") {}))
                    RegionCard(Modifier.fillMaxWidth().height(96.dp)) {
                        EmptyState(listOf(TextAction("打开文件") {}, TextAction("新建文件") {}, TextAction("新建终端") {}))
                    }
                    Row(Modifier.padding(8.dp), verticalAlignment = Alignment.CenterVertically) {
                        InlineLoading(true)
                        Spacer(Modifier.width(8.dp))
                        Text("16dp inline loading (after 300ms)", style = WorkflowTheme.text.caption, color = WorkflowTheme.colors.onSurfaceVariant)
                    }
                }
                GallerySection("Icon buttons: normal · disabled · checked · badge · tint") {
                    Row {
                        WfIconButton(Sym.Save, "保存", {})
                        WfIconButton(Sym.Redo, "重做", {}, enabled = false)
                        WfIconButton(Sym.WrapText, "自动换行", {}, checked = true)
                        WfIconButton(Sym.RightPanelOpen, "辅助区", {}, badge = WorkflowTheme.colors.tertiary)
                        WfIconButton(Sym.Warning, "完全访问", {}, tint = WorkflowTheme.extendedColors.warning)
                        WfIconButton(Sym.Stop, "停止", {}, tint = WorkflowTheme.colors.error)
                    }
                }
            }
        }
        WfSnackbarHost(snackbar, Modifier.align(Alignment.BottomCenter))
    }
}

@Composable
private fun ThirdPartyPlaceholder(label: String) {
    Box(
        Modifier
            .width(56.dp).height(56.dp)
            .background(WorkflowTheme.colors.tertiaryContainer, androidx.compose.foundation.shape.CircleShape),
        contentAlignment = Alignment.Center,
    ) { Text(label.take(1), style = WorkflowTheme.text.titleMd, color = WorkflowTheme.colors.onTertiaryContainer) }
}
