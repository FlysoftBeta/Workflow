package top.flysoftbeta.workflow.feature.terminal

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.content.MutableContextWrapper
import androidx.core.net.toUri
import android.view.ViewGroup
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.compose.ui.window.Popup
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.Dispatchers
import top.flysoftbeta.workflow.app.panel.DropAffordance
import top.flysoftbeta.workflow.app.panel.NewPanelRequest
import top.flysoftbeta.workflow.app.panel.PanelActionKeys
import top.flysoftbeta.workflow.app.panel.PanelContext
import top.flysoftbeta.workflow.app.panel.PanelController
import top.flysoftbeta.workflow.app.panel.PanelFrame
import top.flysoftbeta.workflow.app.panel.PanelProvider
import top.flysoftbeta.workflow.app.panel.PanelTab
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.Panel
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.TextCursor
import top.flysoftbeta.workflow.core.terminal.ShellQuote
import top.flysoftbeta.workflow.core.terminal.TerminalKey
import top.flysoftbeta.workflow.core.terminal.TerminalKeys
import top.flysoftbeta.workflow.core.terminal.TerminalLinks
import top.flysoftbeta.workflow.ui.design.ExtraKey
import top.flysoftbeta.workflow.ui.design.ExtraKeyEvent
import top.flysoftbeta.workflow.ui.design.ExtraKeyLayouts
import top.flysoftbeta.workflow.ui.design.ExtraKeysRow
import top.flysoftbeta.workflow.ui.design.ExtraKeysState
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.NoticeBar
import top.flysoftbeta.workflow.ui.design.NoticeTone
import top.flysoftbeta.workflow.ui.design.SpecialKey
import top.flysoftbeta.workflow.ui.design.TextAction
import top.flysoftbeta.workflow.ui.design.ToolAction
import top.flysoftbeta.workflow.ui.design.dnd.AreaStyle
import top.flysoftbeta.workflow.ui.design.dnd.DragPayload
import top.flysoftbeta.workflow.ui.design.dnd.FilesDragPayload
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme
import java.io.File

/** Terminals in the bottom stack (docs/ui.md §4.4). Processes live in [TerminalHost]. */
class TerminalPanelProvider internal constructor(private val hostFactory: () -> TerminalHost) : PanelProvider {
    constructor(context: Context) : this({ TerminalHost.get(context.applicationContext) })

    private val host by lazy(hostFactory)

    override fun create(panel: Panel, context: PanelContext): PanelController {
        return TerminalController((panel.target as PanelTarget.Terminal).terminalId, host, context)
    }

    override suspend fun newTarget(request: NewPanelRequest): PanelTarget = PanelTarget.Terminal(host.create(request.directory))
}

internal class TerminalController(
    private val terminalId: String,
    private val host: TerminalHost,
    private val context: PanelContext,
) : PanelController, TerminalPageListener {
    private val appContext = context.appContext
    private val wrapper = MutableContextWrapper(appContext)
    private var view: TerminalWebView? = null
    private var session by mutableStateOf<TerminalSession?>(host.attach(terminalId))
    /** The terminal this panel shows (null while unknown, e.g. after the process was recreated). */
    internal val terminalSession: TerminalSession? get() = session
    private var title by mutableStateOf<String?>(null)
    private var status by mutableStateOf(session?.status?.value)
    private val keys = ExtraKeysState()
    private var pinnedKeys by mutableStateOf(context.store.state.value.config.terminal.extraKeysPinned)
    private var renaming by mutableStateOf(false)
    private var restartPending = false
    private var selection by mutableStateOf<Pair<String, IntOffset>?>(null)
    /** End offset of the output already written to the page, and the session generation it belongs to. */
    private var rendered = -1L
    private var renderedGeneration = -1L

    init {
        observeSession()
        context.scope.launch {
            context.store.state.map { it.config.terminal.extraKeysPinned }.distinctUntilChanged().collect { pinnedKeys = it }
        }
        context.scope.launch {
            context.store.state.map { it.config.appearance.monoFontSize }.distinctUntilChanged().collect { size -> view?.font(size.toFloat()) }
        }
    }

    private fun observeSession() {
        val current = session ?: return
        context.scope.launch {
            kotlinx.coroutines.flow.combine(current.customTitle, current.programTitle) { custom, program -> custom ?: program }
                .collect { title = it }
        }
        context.scope.launch { current.status.collect { status = it } }
        context.scope.launch { current.generation.collect { pump() } }
        context.scope.launch { current.outputEnd.collect { pump() } }
    }

    override val tab: PanelTab get() = PanelTab(
        title ?: session?.let { "终端 ${it.ordinal}" } ?: "终端",
        Sym.Terminal,
    )

    override val actions: List<ToolAction> get() = listOf(
        ToolAction(PanelActionKeys.NEW_TERMINAL, Sym.Add, "新建终端") { context.commands.newTerminal() },
    )

    override val resourceMenu: List<MenuEntry> get() = listOf(
        MenuEntry.Action("clear", "清屏", Sym.ClearAll) { session?.clearScrollback() },
        MenuEntry.Action("rename", "重命名", Sym.Edit) { context.layout(LayoutOp.Focus(context.panelId)); renaming = true },
        MenuEntry.Action("restart", "重启", Sym.RestartAlt) { restart() },
        MenuEntry.Toggle("pinKeys", "常驻特殊键行", pinnedKeys) { pinned ->
            context.scope.launch { context.store.updateConfig { it.copy(terminal = it.terminal.copy(extraKeysPinned = pinned)) } }
        },
        MenuEntry.Action("end", "结束", Sym.StopCircle, enabled = status is TerminalStatus.Running, destructive = true) { session?.terminate() },
    )

    private fun restart() {
        val restarted = host.restart(terminalId)
        lastSize?.let { (rows, columns) -> restarted.resize(rows, columns) }
        if (restarted !== session) { session = restarted; observeSession() }
    }

    // ---- Output --------------------------------------------------------------------------------------

    /** Writes new output to the page (a reset replays the retained window after a restart or reattach). */
    private fun pump() {
        val page = view?.takeIf { it.isReady } ?: return
        val current = session ?: return
        val (generation, slice) = current.outputSince(rendered, renderedGeneration)
        renderedGeneration = generation
        if (slice.reset || rendered < 0) page.reset(slice.text) else page.write(slice.text)
        rendered = slice.endOffset
    }

    private fun ensureView(): TerminalWebView = view ?: TerminalWebView(wrapper, this).also { view = it }

    // ---- Page events ---------------------------------------------------------------------------------

    override fun onReady() {
        rendered = -1
        pump()
    }

    override fun onInput(data: String) {
        selection = null
        val modifiers = keys.consume()
        session?.write(TerminalKeys.applyModifiers(data, modifiers.ctrl, modifiers.alt))
    }

    override fun onKey(data: String) { session?.write(data) }

    override fun onResize(columns: Int, rows: Int) {
        lastSize = rows to columns
        session?.resize(rows, columns)
    }

    /** The page's current size (rows to columns), applied to a session that starts later. */
    private var lastSize: Pair<Int, Int>? = null

    override fun onOpenUrl(url: String) {
        val uri = runCatching { url.toUri() }.getOrNull() ?: return
        if (uri.scheme?.lowercase() !in setOf("http", "https")) return
        runCatching { appContext.startActivity(Intent(Intent.ACTION_VIEW, uri).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) }
            .onFailure { context.commands.snackbar("没有可打开链接的应用") }
    }

    override fun onOpenPath(text: String) {
        context.scope.launch {
            val resolved = resolve(text)
            if (resolved == null) { context.commands.snackbar("找不到 ${text.take(60)}"); return@launch }
            val (path, isDirectory, reference) = resolved
            if (isDirectory) context.commands.revealInExplorer(path)
            else context.commands.openFile(path, TerminalLinks.cursor(reference)?.let { (line, column) -> TextCursor(line, column) })
        }
    }

    override fun onCheckLinks(requestId: Int, texts: List<String>) {
        context.scope.launch {
            val results = texts.map { resolve(it) != null }
            view?.linksChecked(requestId, results)
        }
    }

    override fun onSelection(text: String, x: Int, y: Int) {
        selection = if (text.isEmpty() && x == 0 && y == 0) null else text to IntOffset(x, y)
    }

    override fun onGone() {
        view = null
        rendered = -1
    }

    private data class Resolved(val path: String, val isDirectory: Boolean, val reference: top.flysoftbeta.workflow.core.terminal.FileReference)

    /** The first existing workspace path the link text names, relative to the shell's current directory. */
    private suspend fun resolve(text: String): Resolved? {
        val reference = TerminalLinks.parse(text) ?: return null
        val cwd = session?.currentDirectory()
        val candidates = TerminalLinks.candidates(reference, cwd, session?.paths ?: host.backend.paths)
        for (path in candidates) {
            val parent = top.flysoftbeta.workflow.core.io.WorkspacePaths.parent(path)
            val entry = runCatching { context.store.listDirectory(parent, showHidden = true) }.getOrNull()?.firstOrNull { it.path == path }
            if (entry != null) return Resolved(path, entry.isDirectory, reference)
        }
        return null
    }

    // ---- Extra keys ----------------------------------------------------------------------------------

    private fun onExtraKey(event: ExtraKeyEvent) {
        val current = session ?: return
        when (val key = event.key) {
            is ExtraKey.Special -> {
                val encoded = TerminalKeys.encode(key.key.toTerminalKey(), event.modifiers.ctrl, event.modifiers.alt)
                val page = view
                if (page != null && page.isReady) page.key(encoded.normal, encoded.application) else current.write(encoded.normal)
            }
            is ExtraKey.Text -> current.write(TerminalKeys.applyModifiers(key.text, event.modifiers.ctrl, event.modifiers.alt))
            is ExtraKey.Control -> current.write(TerminalKeys.applyModifiers(key.char.toString(), true, event.modifiers.alt))
            is ExtraKey.Latching -> Unit
        }
    }

    // ---- Drops and commands ---------------------------------------------------------------------------

    override fun dropAffordance(payload: DragPayload): DropAffordance? =
        if (payload is FilesDragPayload && status is TerminalStatus.Running) DropAffordance(AreaStyle.Scrim, "粘贴路径") else null

    override fun onDrop(payload: DragPayload) {
        val files = payload as? FilesDragPayload ?: return
        context.scope.launch {
            val text = ShellQuote.pasteText(files.paths, session?.currentDirectory(), session?.paths ?: host.backend.paths)
            val page = view
            if (page != null && page.isReady) page.paste(text) else session?.write(text)
            requestInputFocus()
        }
    }

    override fun requestInputFocus() {
        view?.let { it.requestFocus(); it.focusTerminal() }
    }

    override fun navigate(cursor: TextCursor) {}

    override fun dispose() {
        view?.release()
        view = null
    }

    // ---- Content -------------------------------------------------------------------------------------

    @Composable
    override fun Content(frame: PanelFrame, modifier: Modifier) {
        val colors = WorkflowTheme.colors
        val dark = WorkflowTheme.isDark
        val activity = LocalContext.current
        val density = LocalDensity.current
        // Views are created with the wrapper: point it at the activity first (popups, IME, themes).
        if (wrapper.baseContext !== activity) wrapper.baseContext = activity
        val page = remember { ensureView() }
        DisposableEffect(activity) {
            wrapper.baseContext = activity
            onDispose { wrapper.baseContext = appContext }
        }
        LaunchedEffect(page, dark, colors.surface) {
            page.theme(TerminalTheme.json(colors, dark), TerminalTheme.link(colors))
            page.font(context.store.state.value.config.appearance.monoFontSize.toFloat())
        }
        val current = status
        val ended = session == null || current is TerminalStatus.Ended || current is TerminalStatus.Failed
        Column(modifier.background(colors.surface).testTag("terminal:$terminalId")) {
            host.environment?.let { environment ->
                EnvironmentNotice(environment) {
                    if (!restartPending) {
                        restartPending = true
                        context.scope.launch {
                            try {
                                val decision = context.commands.decide(top.flysoftbeta.workflow.app.panel.DecisionRequest(
                                    title = "重启环境", message = "正在运行的终端和助手进程将停止并重新连接。",
                                    options = listOf(top.flysoftbeta.workflow.app.panel.DecisionOption("restart", "重启"))))
                                if (decision == "restart") environment.restartEnvironment()
                            } catch (cancelled: kotlinx.coroutines.CancellationException) { throw cancelled }
                            catch (error: Exception) { context.commands.snackbar(error.message ?: "环境重启失败") }
                            finally { restartPending = false }
                        }
                    }
                }
            }
            Box(Modifier.weight(1f).fillMaxWidth()) {
                AndroidView(
                    factory = { (page.parent as? ViewGroup)?.removeView(page); page },
                    modifier = Modifier.fillMaxSize(),
                    onRelease = { (it.parent as? ViewGroup)?.removeView(it) },
                )
                if (ended) Box(Modifier.matchParentSize().background(colors.surface.copy(alpha = 0.45f)))
                selection?.let { (_, at) ->
                    // Above the touch point when there is room, else below it (never over the selection start).
                    val y = if (at.y >= 64) at.y.dp - 52.dp else at.y.dp + 28.dp
                    val offset = with(density) { IntOffset((at.x.dp - 40.dp).roundToPx().coerceAtLeast(0), y.roundToPx()) }
                    Popup(offset = offset, onDismissRequest = { selection = null; page.clearSelection() }) {
                        SelectionToolbar()
                    }
                }
            }
            if (ended) {
                val message = when (current) {
                    is TerminalStatus.Failed -> current.message
                    else -> "已结束"
                }
                val environmentFailed = current is TerminalStatus.Failed && current.environment
                NoticeBar(message, if (environmentFailed) NoticeTone.Error else NoticeTone.Neutral, Modifier.testTag("terminal:ended"),
                    actions = buildList {
                        add(TextAction(if (environmentFailed) "重试" else "重启") { if (environmentFailed) host.environment?.retry(); restart() })
                        add(TextAction("关闭") { context.layout(LayoutOp.Close(listOf(context.panelId))) })
                    })
            }
            if (!ended && (pinnedKeys || (frame.focused && frame.imeVisible))) {
                ExtraKeysRow(keys, ::onExtraKey, groups = ExtraKeyLayouts.Terminal)
            }
        }
        if (renaming) RenameDialog()
    }

    @Composable
    private fun SelectionToolbar() {
        val colors = WorkflowTheme.colors
        Row(
            Modifier.background(colors.surfaceContainerHigh, WorkflowShapes.sm).padding(horizontal = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            TextButton(onClick = {
                val text = selection?.first.orEmpty()
                if (text.isNotEmpty()) {
                    appContext.getSystemService(ClipboardManager::class.java)?.setPrimaryClip(ClipData.newPlainText("终端", text))
                }
                selection = null
                view?.clearSelection()
            }) { Text("复制", style = WorkflowTheme.text.label) }
            TextButton(onClick = {
                val clip = appContext.getSystemService(ClipboardManager::class.java)?.primaryClip
                val text = clip?.takeIf { it.itemCount > 0 }?.getItemAt(0)?.coerceToText(appContext)?.toString().orEmpty()
                selection = null
                view?.clearSelection()
                if (text.isNotEmpty()) view?.paste(text)
            }) { Text("粘贴", style = WorkflowTheme.text.label) }
            TextButton(onClick = { view?.selectAll() }) { Text("全选", style = WorkflowTheme.text.label) }
        }
    }

    @Composable
    private fun RenameDialog() {
        var value by remember { mutableStateOf(title ?: session?.let { "终端 ${it.ordinal}" }.orEmpty()) }
        AlertDialog(
            onDismissRequest = { renaming = false },
            title = { Text("重命名终端", style = WorkflowTheme.text.titleMd) },
            text = { OutlinedTextField(value, { value = it.take(80) }, singleLine = true) },
            confirmButton = {
                TextButton(onClick = {
                    session?.rename(value.trim().ifEmpty { null })
                    renaming = false
                }) { Text("确定") }
            },
            dismissButton = { TextButton(onClick = { renaming = false }) { Text("取消") } },
        )
    }
}

internal fun SpecialKey.toTerminalKey(): TerminalKey = when (this) {
    SpecialKey.Escape -> TerminalKey.ESCAPE
    SpecialKey.Tab -> TerminalKey.TAB
    SpecialKey.Left -> TerminalKey.LEFT
    SpecialKey.Down -> TerminalKey.DOWN
    SpecialKey.Up -> TerminalKey.UP
    SpecialKey.Right -> TerminalKey.RIGHT
    SpecialKey.Home -> TerminalKey.HOME
    SpecialKey.End -> TerminalKey.END
    SpecialKey.PageUp -> TerminalKey.PAGE_UP
    SpecialKey.PageDown -> TerminalKey.PAGE_DOWN
    SpecialKey.Delete -> TerminalKey.DELETE
    SpecialKey.Enter -> TerminalKey.ENTER
    SpecialKey.Backspace -> TerminalKey.BACKSPACE
    SpecialKey.F1 -> TerminalKey.F1
    SpecialKey.F2 -> TerminalKey.F2
    SpecialKey.F3 -> TerminalKey.F3
    SpecialKey.F4 -> TerminalKey.F4
    SpecialKey.F5 -> TerminalKey.F5
    SpecialKey.F6 -> TerminalKey.F6
    SpecialKey.F7 -> TerminalKey.F7
    SpecialKey.F8 -> TerminalKey.F8
    SpecialKey.F9 -> TerminalKey.F9
    SpecialKey.F10 -> TerminalKey.F10
    SpecialKey.F11 -> TerminalKey.F11
    SpecialKey.F12 -> TerminalKey.F12
}
