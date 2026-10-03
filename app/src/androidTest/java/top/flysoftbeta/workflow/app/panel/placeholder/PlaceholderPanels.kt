package top.flysoftbeta.workflow.app.panel.placeholder

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import top.flysoftbeta.workflow.app.panel.DropAffordance
import top.flysoftbeta.workflow.app.panel.NewPanelRequest
import top.flysoftbeta.workflow.app.panel.PanelContext
import top.flysoftbeta.workflow.app.panel.PanelController
import top.flysoftbeta.workflow.app.panel.PanelFrame
import top.flysoftbeta.workflow.app.panel.PanelProvider
import top.flysoftbeta.workflow.app.panel.PanelTab
import top.flysoftbeta.workflow.core.layout.Panel
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.ProxyPage
import top.flysoftbeta.workflow.core.resource.FileStatus
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.icons.FileTypeIcons
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.dnd.AreaStyle
import top.flysoftbeta.workflow.ui.design.dnd.DragPayload
import top.flysoftbeta.workflow.ui.design.dnd.ExternalDragPayload
import top.flysoftbeta.workflow.ui.design.dnd.FilesDragPayload
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme
import java.util.UUID

/**
 * Stand-ins for panel kinds whose feature workstream has not landed yet, so the shell builds and runs.
 * Each is replaced by its feature's provider in [top.flysoftbeta.workflow.app.panel.PanelWiring].
 * Files show a read-only preview; terminal and conversation accept drops so routing can be exercised.
 */
class PlaceholderPanelProvider : PanelProvider {
    override fun create(panel: Panel, context: PanelContext): PanelController = PlaceholderController(panel.target, context)

    override suspend fun newTarget(request: NewPanelRequest): PanelTarget? = null
}

/** Placeholder for conversations: ids are generated locally until the agent layer provides them. */
class PlaceholderConversationProvider : PanelProvider {
    override fun create(panel: Panel, context: PanelContext): PanelController = PlaceholderController(panel.target, context)
    override suspend fun newTarget(request: NewPanelRequest): PanelTarget = PanelTarget.Conversation(UUID.randomUUID().toString())
}

private class PlaceholderController(private val target: PanelTarget, private val context: PanelContext) : PanelController {
    private var dirty by mutableStateOf(false)
    private var preview by mutableStateOf<String?>(null)
    private val received = mutableStateListOf<String>()

    init {
        val path = (target.resource as? top.flysoftbeta.workflow.core.resource.ResourceRef.File)?.path
        if (path != null) {
            context.scope.launch {
                context.store.state.map { it.fileStatus(path) != FileStatus.CLEAN }.distinctUntilChanged().collect { dirty = it }
            }
            if (target is PanelTarget.File) context.scope.launch {
                val snapshot = runCatching { context.store.openFile(path) }.getOrNull()
                preview = when {
                    snapshot == null -> ""
                    snapshot.binary || snapshot.tooLarge -> null
                    else -> snapshot.text.take(200_000)
                }
            }
        }
    }

    override val tab: PanelTab get() = when (target) {
        is PanelTarget.File -> PanelTab(name(target.path), FileTypeIcons.forName(name(target.path)), dirty)
        is PanelTarget.Image -> PanelTab(name(target.path), Sym.Image, dirty)
        is PanelTarget.Diff -> PanelTab("对比 · ${name(target.path)}", Sym.SwapHoriz)
        is PanelTarget.Terminal -> PanelTab("终端", Sym.Terminal)
        is PanelTarget.Conversation -> PanelTab("新对话", Sym.AddComment)
        is PanelTarget.Proxy -> when (target.page) {
            ProxyPage.OVERVIEW -> PanelTab("代理", Sym.Shield)
            ProxyPage.LOGS -> PanelTab("日志", Sym.ReceiptLong)
            ProxyPage.CONNECTIONS -> PanelTab("连接", Sym.Lan)
        }
        PanelTarget.Settings -> PanelTab("设置", Sym.Settings)
    }

    override val resourceMenu: List<MenuEntry> get() {
        val path = (target as? PanelTarget.File)?.path ?: (target as? PanelTarget.Image)?.path ?: return emptyList()
        return listOf(
            MenuEntry.Action("reveal", "在文件树中显示", Sym.AccountTree) { context.commands.revealInExplorer(path) },
            MenuEntry.Action("attach", "附加到对话", Sym.AddComment) { context.commands.attachToConversation(listOf(path)) },
            MenuEntry.Action("terminal", "在终端中打开所在目录", Sym.Terminal) {
                context.commands.newTerminal(path.substringBeforeLast('/', ""))
            },
        )
    }

    override fun dropAffordance(payload: DragPayload): DropAffordance? = when {
        target is PanelTarget.Terminal && payload is FilesDragPayload -> DropAffordance(AreaStyle.Scrim, "粘贴路径")
        target is PanelTarget.Conversation && (payload is FilesDragPayload || payload is ExternalDragPayload) ->
            DropAffordance(AreaStyle.Outline, "添加为附件")
        else -> null
    }

    override fun onDrop(payload: DragPayload) {
        when (payload) {
            is FilesDragPayload -> received += payload.paths
            is ExternalDragPayload -> received += payload.uris.map { it.lastPathSegment ?: it.toString() }
        }
    }

    @Composable
    override fun Content(frame: PanelFrame, modifier: Modifier) {
        val colors = WorkflowTheme.colors
        val text = WorkflowTheme.text
        val file = target as? PanelTarget.File
        if (file != null) {
            val content = preview
            if (content == null) {
                Box(modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text("无法作为文本打开", style = text.body, color = colors.onSurfaceVariant)
                }
            } else {
                Text(
                    content,
                    modifier.fillMaxSize().verticalScroll(rememberScrollState()).horizontalScroll(rememberScrollState()).padding(8.dp),
                    style = text.mono, color = colors.onSurface, softWrap = false,
                )
            }
            return
        }
        Box(modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Text(tab.title, style = text.titleSm, color = colors.onSurfaceVariant)
                received.takeLast(5).forEach { Text(it, style = text.caption, color = colors.onSurfaceVariant) }
            }
        }
    }

    private fun name(path: String) = path.substringAfterLast('/')
}
