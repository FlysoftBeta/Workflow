package top.flysoftbeta.workflow.feature.proxy

import android.content.ClipData
import android.content.ClipboardManager
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import top.flysoftbeta.workflow.app.panel.PanelContext
import top.flysoftbeta.workflow.app.panel.PanelFrame
import top.flysoftbeta.workflow.app.panel.PanelTab
import top.flysoftbeta.workflow.core.layout.ProxyPage
import top.flysoftbeta.workflow.platform.proxy.ProxyService
import top.flysoftbeta.workflow.proxy.runtime.ProxyApi
import top.flysoftbeta.workflow.ui.design.EmptyState
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.InlineError
import kotlinx.coroutines.CancellationException
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

private const val LOG_TAIL_BYTES = 256 * 1024

/**
 * 日志: the kernel's own output (runtime.log, secret-redacted by the runtime when written), newest at the
 * bottom and followed while scrolled to the end. Works whether or not the kernel runs, so a failed start
 * can be read here.
 */
internal class ProxyLogsController(api: ProxyApi, context: PanelContext) : ProxyPanelBase(api, context) {
    private var lines by mutableStateOf<List<ProxyFormat.LogLine>?>(null)
    private var raw = ""
    private var failure by mutableStateOf<String?>(null)

    override val tab: PanelTab get() = PanelTab("日志", Sym.ReceiptLong)

    override val resourceMenu: List<MenuEntry> get() = buildList {
        add(MenuEntry.Action("copy", "复制日志", Sym.ContentCopy, enabled = !lines.isNullOrEmpty()) { copy() })
        add(MenuEntry.Action("open", "在编辑器中打开", Sym.EditDocument) { context.commands.openFile(ProxyService.LOG_PATH) })
        add(MenuEntry.Action("edit", "编辑配置", Sym.EditDocument) { editConfig() })
        addAll(pageEntries(ProxyPage.LOGS))
    }

    private suspend fun reload() {
        val text = try { api.logTail(LOG_TAIL_BYTES) } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (_: Exception) { failure = "无法读取代理日志"; return }
        failure = null
        if (text == raw && lines != null) return
        raw = text
        lines = withContext(Dispatchers.Default) { text.lineSequence().filter { it.isNotBlank() }.map(ProxyFormat::parseLog).toList() }
    }

    private fun copy() {
        val clipboard = context.appContext.getSystemService(ClipboardManager::class.java) ?: return
        clipboard.setPrimaryClip(ClipData.newPlainText("代理日志", raw))
        context.commands.snackbar("已复制")
    }

    @Composable
    override fun Content(frame: PanelFrame, modifier: Modifier) {
        LaunchedEffect(Unit) { while (true) { reload(); delay(1000) } }
        val colors = WorkflowTheme.colors
        val extended = WorkflowTheme.extendedColors
        val style = WorkflowTheme.text.mono.copy(fontSize = 12.sp, lineHeight = 17.sp)
        val current = lines
        Box(modifier.fillMaxSize().background(colors.surface).testTag("proxy:logs")) {
            when {
                failure != null -> InlineError(failure!!)
                current == null -> Unit
                current.isEmpty() -> EmptyState(emptyList(), message = "没有日志")
                else -> {
                    val list = rememberLazyListState()
                    var follow by androidx.compose.runtime.remember { mutableStateOf(true) }
                    LaunchedEffect(list) {
                        snapshotFlow { list.layoutInfo.visibleItemsInfo.lastOrNull()?.index to list.layoutInfo.totalItemsCount }
                            .collect { (last, total) -> if (list.isScrollInProgress) follow = last != null && last >= total - 1 }
                    }
                    LaunchedEffect(current.size) { if (follow) list.scrollToItem(current.lastIndex) }
                    SelectionContainer {
                        LazyColumn(Modifier.fillMaxSize(), state = list, contentPadding = PaddingValues(vertical = 6.dp)) {
                            itemsIndexed(current, contentType = { _, _ -> "line" }) { _, line ->
                                val levelColor = when (line.level) {
                                    "error", "fatal", "panic" -> colors.error
                                    "warning", "warn" -> extended.warning
                                    "debug", "trace" -> colors.outline
                                    else -> colors.primary
                                }
                                Text(
                                    buildAnnotatedString {
                                        line.time?.let { withStyle(SpanStyle(color = colors.onSurfaceVariant)) { append(it); append("  ") } }
                                        line.level?.let { withStyle(SpanStyle(color = levelColor)) { append(it.take(4).uppercase().padEnd(4)); append("  ") } }
                                        append(line.message)
                                    },
                                    Modifier.fillMaxWidth().padding(horizontal = WorkflowTheme.dimens.padH, vertical = 1.dp),
                                    style = style, color = colors.onSurface,
                                )
                            }
                        }
                    }
                }
            }
        }
    }

    init {
        context.scope.launch { reload() }
    }
}
