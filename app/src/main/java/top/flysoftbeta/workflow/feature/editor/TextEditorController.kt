package top.flysoftbeta.workflow.feature.editor

import android.content.Context
import android.content.MutableContextWrapper
import android.view.ViewGroup
import android.view.ViewTreeObserver
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.viewinterop.AndroidView
import io.github.rosemoe.sora.event.ContentChangeEvent
import io.github.rosemoe.sora.event.PublishSearchResultEvent
import io.github.rosemoe.sora.event.ScrollEvent
import io.github.rosemoe.sora.event.SelectionChangeEvent
import io.github.rosemoe.sora.event.TextSizeChangeEvent
import io.github.rosemoe.sora.langs.textmate.TextMateLanguage
import io.github.rosemoe.sora.langs.textmate.registry.ThemeRegistry
import io.github.rosemoe.sora.widget.CodeEditor
import io.github.rosemoe.sora.widget.SelectionMovement
import io.github.rosemoe.sora.widget.subscribeAlways
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import top.flysoftbeta.workflow.app.panel.DecisionOption
import top.flysoftbeta.workflow.app.panel.DecisionRequest
import top.flysoftbeta.workflow.app.panel.DecisionStyle
import top.flysoftbeta.workflow.app.panel.PanelActionKeys
import top.flysoftbeta.workflow.app.panel.PanelContext
import top.flysoftbeta.workflow.app.panel.PanelController
import top.flysoftbeta.workflow.app.panel.PanelFrame
import top.flysoftbeta.workflow.app.panel.PanelTab
import top.flysoftbeta.workflow.core.config.Appearance
import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.PanelView
import top.flysoftbeta.workflow.core.layout.TextCursor
import top.flysoftbeta.workflow.core.resource.ConflictResolution
import top.flysoftbeta.workflow.core.resource.DiskVersion
import top.flysoftbeta.workflow.core.resource.Drafts
import top.flysoftbeta.workflow.core.resource.FileDraft
import top.flysoftbeta.workflow.core.resource.FileStatus
import top.flysoftbeta.workflow.core.store.SaveResult
import top.flysoftbeta.workflow.platform.workspace.WorkspaceIntents
import top.flysoftbeta.workflow.ui.design.EmptyState
import top.flysoftbeta.workflow.ui.design.ExtraKey
import top.flysoftbeta.workflow.ui.design.ExtraKeyEvent
import top.flysoftbeta.workflow.ui.design.ExtraKeyLayouts
import top.flysoftbeta.workflow.ui.design.ExtraKeysRow
import top.flysoftbeta.workflow.ui.design.ExtraKeysState
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.NoticeBar
import top.flysoftbeta.workflow.ui.design.NoticeTone
import top.flysoftbeta.workflow.ui.design.RegionLoading
import top.flysoftbeta.workflow.ui.design.SpecialKey
import top.flysoftbeta.workflow.ui.design.TextAction
import top.flysoftbeta.workflow.ui.design.ToolAction
import top.flysoftbeta.workflow.ui.design.icons.FileTypeIcons
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme
import top.flysoftbeta.workflow.ui.sora.SoraGrammars

/**
 * A text file in sora-editor (docs/ui.md §4.3), bound to the store's Working Resource: every edit goes to
 * `editFile` (the draft survives closing the panel, switching sessions and process death), saving goes
 * through `saveFile`, which refuses to overwrite a file that changed on disk. A clean buffer follows
 * external changes silently; a draft that meets an external change shows the conflict bar.
 * The CodeEditor (and its undo history) lives in this controller, not in the composition.
 */
internal class TextEditorController(private val path: String, private val context: PanelContext) : PanelController {
    private enum class Phase { LOADING, READY, BINARY, TOO_LARGE, FAILED }

    private val appContext = context.appContext
    private val store = context.store
    private val wrapper = MutableContextWrapper(appContext)
    private var phase by mutableStateOf(Phase.LOADING)
    private var failure by mutableStateOf<String?>(null)
    private var status by mutableStateOf(FileStatus.CLEAN)
    private var canUndo by mutableStateOf(false)
    private var canRedo by mutableStateOf(false)
    private var wrap by mutableStateOf(defaultWrap(path, context.initialView))
    var findOpen by mutableStateOf(false)
    internal val find = FindState()
    private val keys = ExtraKeysState()

    private var editor: CodeEditor? = null
    /** The retained sora view (tests drive typing through it). */
    internal val editorView: CodeEditor? get() = editor
    private var scheme: WorkflowEditorScheme? = null
    /** Text the buffer starts from before the editor exists. */
    private var loadedText = ""
    /** The disk version the buffer came from (passed to `editFile`). */
    private var shown: DiskVersion = DiskVersion.MISSING
    /** True while the buffer is being replaced programmatically (not a user edit). */
    private var applying = false
    private var saving = false
    private var editedDuringSave = false
    private var pushJob: Job? = null
    private var viewJob: Job? = null
    private var zoomJob: Job? = null
    private var pendingCursor: TextCursor? = context.initialView.cursor
    private var pendingScrollLine: Int? = context.initialView.scrollAnchor?.toIntOrNull()?.coerceAtLeast(0)
    private var restoreViewport: ViewTreeObserver.OnPreDrawListener? = null
    private var lastDraft: FileDraft? = null

    init {
        context.scope.launch { load(initial = true) }
        context.scope.launch {
            store.state.map { it.drafts[path] to it.disk[path] }.distinctUntilChanged().collect { (draft, disk) -> onStore(draft, disk) }
        }
    }

    // ---- Loading and following the store -------------------------------------------------------------

    private suspend fun load(initial: Boolean) {
        try {
            if (initial) withContext(Dispatchers.IO) { runCatching { SoraGrammars.load(appContext) } }
            val snapshot = store.openFile(path)
            if (snapshot.tooLarge) { phase = Phase.TOO_LARGE; return }
            if (snapshot.binary && snapshot.draft == null) { phase = Phase.BINARY; return }
            shown = snapshot.shownVersion
            replaceText(snapshot.text)
            phase = Phase.READY
        } catch (error: Exception) {
            failure = error.message ?: "无法打开文件"
            phase = Phase.FAILED
        }
    }

    private suspend fun onStore(draft: FileDraft?, disk: DiskVersion?) {
        status = Drafts.status(draft, disk)
        val hadDraft = lastDraft != null
        lastDraft = draft
        if (phase != Phase.READY || saving) return
        if (draft == null) {
            // Draft gone (take disk / discard / external write equal to it) or the clean file changed on disk.
            val diskMoved = disk != null && disk.exists && !disk.sameContent(shown)
            if (hadDraft || diskMoved) load(initial = false)
        }
    }

    /** Replaces the buffer (keeping cursor and scroll) without producing an edit. */
    private fun replaceText(text: String) {
        loadedText = text
        val view = editor ?: return
        if (view.text.toString() == text) return
        val line = view.cursor.leftLine
        val column = view.cursor.leftColumn
        val scrollY = view.offsetY
        applying = true
        try {
            view.setText(text)
            val safeLine = line.coerceAtMost((view.lineCount - 1).coerceAtLeast(0))
            view.setSelection(safeLine, column.coerceAtMost(view.text.getColumnCount(safeLine)), false)
            view.scroller.startScroll(0, 0, 0, scrollY, 0)
        } finally {
            applying = false
        }
        updateHistory()
    }

    // ---- Edits and saving ---------------------------------------------------------------------------------

    private fun onEdited() {
        if (applying) return
        updateHistory()
        if (saving) { editedDuringSave = true; return }
        pushJob?.cancel()
        pushJob = context.scope.launch { delay(PUSH_DELAY_MS); pushNow() }
    }

    private fun pushNow() {
        pushJob?.cancel()
        pushJob = null
        val text = editor?.text?.toString() ?: return
        store.editFile(path, text, shown)
    }

    fun save() {
        val view = editor ?: return
        if (saving) return
        context.scope.launch {
            pushJob?.cancel(); pushJob = null
            saving = true
            editedDuringSave = false
            val text = view.text.toString()
            val result = try { store.saveFile(path, text) } finally { saving = false }
            when (result) {
                is SaveResult.Saved -> shown = result.version
                is SaveResult.Unchanged -> shown = result.version
                is SaveResult.Conflict -> context.commands.snackbar("磁盘版本已更改，未保存")
                is SaveResult.Invalid -> context.commands.snackbar("无法保存：${result.message.take(80)}")
                is SaveResult.Failed -> context.commands.snackbar("无法保存：${result.message.take(80)}")
            }
            if (editedDuringSave || view.text.toString() != text) pushNow()
        }
    }

    private fun resolve(resolution: ConflictResolution) {
        context.scope.launch {
            pushNow()
            store.resolveConflict(path, resolution)
            if (resolution == ConflictResolution.TAKE_DISK) load(initial = false)
        }
    }

    private fun discard() {
        context.scope.launch {
            val choice = context.commands.decide(DecisionRequest(
                title = "放弃对「${name()}」的更改？",
                options = listOf(DecisionOption("discard", "放弃更改", DecisionStyle.Destructive)),
            ))
            if (choice == "discard") {
                pushJob?.cancel(); pushJob = null
                store.discardDraft(path)
                load(initial = false)
            }
        }
    }

    private fun updateHistory() {
        val view = editor ?: return
        canUndo = view.canUndo()
        canRedo = view.canRedo()
    }

    // ---- Editor view -------------------------------------------------------------------------------------

    private fun ensureEditor(dark: Boolean): CodeEditor = editor ?: WorkspaceCodeEditor(wrapper).also { view ->
        editor = view
        view.layoutParams = ViewGroup.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT)
        view.typefaceText = WorkflowEditorScheme.typeface(appContext.assets)
        view.typefaceLineNumber = WorkflowEditorScheme.typeface(appContext.assets)
        view.setTextSize(store.state.value.config.appearance.monoFontSize.toFloat())
        val density = appContext.resources.displayMetrics.scaledDensity
        view.setScaleTextSizes(Appearance.MONO_SIZE.start.toFloat() * density, Appearance.MONO_SIZE.endInclusive.toFloat() * density)
        view.isLineNumberEnabled = true
        val dp = appContext.resources.displayMetrics.density
        view.setLineNumberMarginLeft(8 * dp)
        view.setDividerMargin(4 * dp, 8 * dp)
        view.setLineNumberAlign(android.graphics.Paint.Align.RIGHT)
        view.setPinLineNumber(false)
        view.props.stickyScroll = false
        view.setWordwrap(wrap)
        view.isHighlightCurrentLine = true
        view.setCursorAnimationEnabled(false)
        runCatching {
            // Grammar loading is asynchronous and registers both themes. Select the shell's theme
            // after loading, before the first scheme is constructed (the dark asset loads last).
            WorkflowEditorScheme.applyTheme(dark)
            val registry = ThemeRegistry.getInstance()
            scheme = WorkflowEditorScheme(registry).also { view.colorScheme = it }
            SoraGrammars.scope(path)?.let { scope ->
                view.setEditorLanguage(TextMateLanguage.create(scope, false).apply { isAutoCompleteEnabled = false })
            }
        }
        applying = true
        try { view.setText(loadedText) } finally { applying = false }
        pendingCursor?.let { cursor -> moveTo(view, cursor, makeVisible = pendingScrollLine == null) }
        pendingCursor = null
        restoreInitialViewport(view)
        view.subscribeAlways<ContentChangeEvent> { onEdited() }
        view.subscribeAlways<SelectionChangeEvent> { scheduleViewSave() }
        view.subscribeAlways<ScrollEvent> { scheduleViewSave() }
        view.subscribeAlways<PublishSearchResultEvent> { find.update(view.searcher) }
        view.subscribeAlways<TextSizeChangeEvent> { event -> scheduleZoomSave(event.newTextSize / density) }
        updateHistory()
    }

    private fun moveTo(view: CodeEditor, cursor: TextCursor, makeVisible: Boolean = true) {
        if (view.lineCount == 0) return
        val line = cursor.line.coerceIn(0, view.lineCount - 1)
        val column = cursor.column.coerceIn(0, view.text.getColumnCount(line))
        view.setSelection(line, column, makeVisible)
    }

    private fun restoreInitialViewport(view: CodeEditor) {
        if (pendingScrollLine == null) return
        // Wrapped Sora layout is asynchronous. Restoring before it has its measured width can
        // silently clamp the saved line to zero; pre-draw also covers the unwrapped layout path.
        val listener = ViewTreeObserver.OnPreDrawListener {
            if (view.width > 0 && view.height > 0 && view.isEditable) {
                pendingScrollLine?.let { saved ->
                    val line = saved.coerceIn(0, view.lineCount - 1)
                    val column = context.initialView.extras["scrollColumn"]?.toIntOrNull()
                        ?.coerceIn(0, view.text.getColumnCount(line)) ?: 0
                    val x = context.initialView.extras["scrollX"]?.toIntOrNull()?.coerceIn(0, view.scrollMaxX) ?: 0
                    val fraction = context.initialView.extras["scrollRowFraction"]?.toFloatOrNull()
                        ?.takeIf { it.isFinite() }?.coerceIn(0f, 1f) ?: 0f
                    val y = (view.layout.getCharLayoutOffset(line, column)[0].toInt() - view.rowHeight +
                        (fraction * view.rowHeight).toInt())
                        .coerceIn(0, view.scrollMaxY)
                    view.scroller.startScroll(x, y, 0, 0, 0)
                    view.scroller.abortAnimation()
                }
                pendingScrollLine = null
                restoreViewport?.let { view.viewTreeObserver.removeOnPreDrawListener(it) }
                restoreViewport = null
            }
            true
        }
        restoreViewport = listener
        view.viewTreeObserver.addOnPreDrawListener(listener)
    }

    private fun scheduleViewSave() {
        if (applying || pendingScrollLine != null) return
        viewJob?.cancel()
        viewJob = context.scope.launch {
            delay(600)
            val view = editor ?: return@launch
            // A fling emits one ScrollEvent when it starts and may outlive the debounce.
            while (!view.scroller.isFinished) delay(100)
            viewJob = null
            saveViewNow()
        }
    }

    private fun saveViewNow() {
        viewJob?.cancel()
        viewJob = null
        val view = editor ?: return
        if (pendingScrollLine != null || !view.isEditable || view.width == 0 || view.layout.rowCount == 0) return
        val row = view.layout.getRowAt(view.firstVisibleRow.coerceAtMost(view.layout.rowCount - 1))
        context.updateView(PanelView(
            scrollAnchor = row.lineIndex.toString(),
            cursor = TextCursor(view.cursor.leftLine, view.cursor.leftColumn),
            extras = mapOf(
                "wrap" to wrap.toString(),
                "scrollColumn" to row.startColumn.toString(),
                "scrollX" to view.offsetX.toString(),
                "scrollRowFraction" to ((view.offsetY % view.rowHeight).toFloat() / view.rowHeight).toString(),
            ),
        ))
    }

    /** Pinch zoom writes the mono size to config.json (docs/ui.md §1.2: 10–20). */
    private fun scheduleZoomSave(sizeSp: Float) {
        zoomJob?.cancel()
        zoomJob = context.scope.launch {
            delay(700)
            val size = sizeSp.toDouble().coerceIn(Appearance.MONO_SIZE).let { Math.round(it * 2) / 2.0 }
            store.updateConfig { it.copy(appearance = it.appearance.copy(monoFontSize = size)) }
        }
    }

    private fun onExtraKey(event: ExtraKeyEvent) {
        val view = editor ?: return
        when (val key = event.key) {
            is ExtraKey.Special -> when (key.key) {
                SpecialKey.Tab -> view.indentOrCommitTab()
                SpecialKey.Left -> view.moveSelection(SelectionMovement.LEFT)
                SpecialKey.Right -> view.moveSelection(SelectionMovement.RIGHT)
                SpecialKey.Up -> view.moveSelection(SelectionMovement.UP)
                SpecialKey.Down -> view.moveSelection(SelectionMovement.DOWN)
                SpecialKey.Home -> view.moveSelection(SelectionMovement.LINE_START)
                SpecialKey.End -> view.moveSelection(SelectionMovement.LINE_END)
                SpecialKey.PageUp -> view.moveSelection(SelectionMovement.PAGE_UP)
                SpecialKey.PageDown -> view.moveSelection(SelectionMovement.PAGE_DOWN)
                SpecialKey.Delete -> view.deleteText()
                SpecialKey.Enter -> view.commitText("\n")
                else -> Unit
            }
            is ExtraKey.Text -> view.commitText(key.text)
            is ExtraKey.Control, is ExtraKey.Latching -> Unit
        }
    }

    // ---- Contract ------------------------------------------------------------------------------------------

    private fun name() = WorkspacePaths.name(path)

    override val tab: PanelTab get() = PanelTab(name(), FileTypeIcons.forName(name()), dirty = status != FileStatus.CLEAN)

    override val actions: List<ToolAction> get() {
        val ready = phase == Phase.READY
        return listOf(
            ToolAction(PanelActionKeys.SAVE, Sym.Save, "保存", enabled = ready && status != FileStatus.CLEAN) { save() },
            ToolAction(PanelActionKeys.UNDO, Sym.Undo, "撤销", enabled = ready && canUndo) { editor?.undo(); updateHistory() },
            ToolAction(PanelActionKeys.REDO, Sym.Redo, "重做", enabled = ready && canRedo) { editor?.redo(); updateHistory() },
            ToolAction(PanelActionKeys.FIND, Sym.Search, "查找", enabled = ready, checked = findOpen) { findOpen = !findOpen },
            ToolAction(PanelActionKeys.ATTACH, Sym.AddComment, "附加到对话") { context.commands.attachToConversation(listOf(path)) },
        )
    }

    override val resourceMenu: List<MenuEntry> get() = buildList {
        addAll(fileMenu(appContext, context, path))
        add(MenuEntry.Action("terminal", "在终端中打开所在目录", Sym.Terminal) { context.commands.newTerminal(WorkspacePaths.parent(path)) })
        add(MenuEntry.Action("openWith", "用其他应用打开", Sym.OpenInNew) {
            context.scope.launch {
                if (!WorkspaceIntents.openWith(appContext, path)) context.commands.snackbar("无法读取文件或没有可用的应用")
            }
        })
        add(MenuEntry.Toggle("wrap", "自动换行", wrap) { value ->
            wrap = value
            editor?.setWordwrap(value)
            scheduleViewSave()
        })
        add(MenuEntry.Action("discard", "放弃更改", Sym.Delete, enabled = status != FileStatus.CLEAN, destructive = true) { discard() })
    }

    override fun navigate(cursor: TextCursor) {
        pendingScrollLine = null
        val view = editor
        if (view != null && phase == Phase.READY) moveTo(view, cursor) else pendingCursor = cursor
    }

    override fun requestInputFocus() { editor?.requestFocus() }

    override fun onFocusChanged(focused: Boolean) {
        if (!focused) { pushNow(); saveViewNow() }
    }

    override fun dispose() {
        if (pushJob != null) pushNow()
        saveViewNow()
        editor?.let { view ->
            restoreViewport?.let { view.viewTreeObserver.removeOnPreDrawListener(it) }
            restoreViewport = null
            (view.parent as? ViewGroup)?.removeView(view)
            view.release()
        }
        scheme?.let { ThemeRegistry.getInstance().removeListener(it) }
        editor = null
    }

    @Composable
    override fun Content(frame: PanelFrame, modifier: Modifier) {
        val colors = WorkflowTheme.colors
        val dark = WorkflowTheme.isDark
        val activity = LocalContext.current
        // Views are created with the wrapper: point it at the activity first (popups, IME, themes).
        if (wrapper.baseContext !== activity) wrapper.baseContext = activity
        DisposableEffect(activity) {
            wrapper.baseContext = activity
            onDispose { wrapper.baseContext = appContext }
        }
        LaunchedEffect(dark) { WorkflowEditorScheme.applyTheme(dark) }
        Column(modifier.background(colors.surface).testTag("editor:$path")) {
            when (status) {
                FileStatus.CONFLICT -> NoticeBar("磁盘版本已更改", NoticeTone.Tertiary, actions = listOf(
                    TextAction("保留我的") { resolve(ConflictResolution.KEEP_MINE) },
                    TextAction("使用磁盘版本") { resolve(ConflictResolution.TAKE_DISK) },
                    TextAction("对比") { context.commands.open(PanelTarget.Diff(path)) },
                ))
                FileStatus.DELETED -> NoticeBar("文件已在磁盘上删除", NoticeTone.Tertiary, actions = listOf(
                    TextAction("保留我的") { resolve(ConflictResolution.KEEP_MINE) },
                    TextAction("放弃") { resolve(ConflictResolution.TAKE_DISK) },
                ))
                else -> Unit
            }
            when (phase) {
                Phase.LOADING -> RegionLoading(true, Modifier.fillMaxSize())
                Phase.BINARY, Phase.TOO_LARGE -> EmptyState(
                    listOf(TextAction("用其他应用打开") {
                        context.scope.launch {
                            if (!WorkspaceIntents.openWith(appContext, path)) context.commands.snackbar("无法读取文件或没有可用的应用")
                        }
                    }),
                    Modifier.fillMaxSize(),
                    message = "无法作为文本打开",
                )
                Phase.FAILED -> EmptyState(
                    listOf(TextAction("重试") { phase = Phase.LOADING; context.scope.launch { load(initial = true) } }),
                    Modifier.fillMaxSize(),
                    message = failure ?: "无法打开文件",
                )
                Phase.READY -> {
                    val view = remember { ensureEditor(dark) }
                    DisposableEffect(view) { onDispose { saveViewNow() } }
                    if (findOpen) FindBar(find, view, onClose = { findOpen = false; find.fieldFocused = false; view.searcher.stopSearch() })
                    Box(Modifier.weight(1f).fillMaxWidth()) {
                        AndroidView(
                            factory = { (view.parent as? ViewGroup)?.removeView(view); view },
                            modifier = Modifier.fillMaxSize(),
                            onRelease = { (it.parent as? ViewGroup)?.removeView(it) },
                        )
                    }
                    if (frame.focused && frame.imeVisible && !(findOpen && find.fieldFocused)) {
                        ExtraKeysRow(keys, ::onExtraKey, groups = ExtraKeyLayouts.Editor)
                    }
                }
            }
        }
    }

    companion object {
        private const val PUSH_DELAY_MS = 150L
        private val PROSE = setOf("md", "markdown", "txt", "text", "rst", "adoc", "log", "")

        fun defaultWrap(path: String, view: PanelView): Boolean =
            view.extras["wrap"]?.toBooleanStrictOrNull() ?: (WorkspacePaths.name(path).substringAfterLast('.', "").lowercase() in PROSE)
    }
}

/** More ① entries shared by text and image panels: copy path, copy relative path, reveal, attach. */
internal fun fileMenu(appContext: Context, context: PanelContext, path: String): List<MenuEntry> = listOf(
    MenuEntry.Action("copyPath", "复制路径", Sym.ContentCopy) {
        WorkspaceIntents.copyText(appContext, "路径", WorkspaceIntents.absolutePath(appContext, path))
        context.commands.snackbar("已复制")
    },
    MenuEntry.Action("copyRelative", "复制相对路径", Sym.ContentCopy) {
        WorkspaceIntents.copyText(appContext, "路径", path)
        context.commands.snackbar("已复制")
    },
    MenuEntry.Action("reveal", "在文件树中显示", Sym.AccountTree) { context.commands.revealInExplorer(path) },
    MenuEntry.Action("attach", "附加到对话", Sym.AddComment) { context.commands.attachToConversation(listOf(path)) },
)
