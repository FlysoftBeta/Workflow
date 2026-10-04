package top.flysoftbeta.workflow.feature.files

import android.content.Context
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import kotlinx.coroutines.FlowPreview
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.debounce
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import top.flysoftbeta.workflow.app.panel.launchAction
import top.flysoftbeta.workflow.app.panel.DecisionOption
import top.flysoftbeta.workflow.app.panel.DecisionRequest
import top.flysoftbeta.workflow.app.panel.DecisionStyle
import top.flysoftbeta.workflow.app.panel.ExplorerContext
import top.flysoftbeta.workflow.app.panel.ExplorerController
import top.flysoftbeta.workflow.app.panel.ExplorerProvider
import top.flysoftbeta.workflow.app.panel.ExplorerVariant
import top.flysoftbeta.workflow.core.io.FileEntry
import top.flysoftbeta.workflow.core.io.FileNames
import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.core.layout.Edge
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.Placement
import top.flysoftbeta.workflow.core.store.FileOpResult
import top.flysoftbeta.workflow.core.store.RestoreResult
import top.flysoftbeta.workflow.core.store.TrashResult
import top.flysoftbeta.workflow.platform.importer.ImportKind
import top.flysoftbeta.workflow.platform.importer.ImportResult
import top.flysoftbeta.workflow.platform.importer.ImportService
import top.flysoftbeta.workflow.platform.workspace.WorkspaceIntents
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.MenuGroup
import top.flysoftbeta.workflow.ui.design.ToolAction
import top.flysoftbeta.workflow.ui.design.icons.Sym

/** The file explorer region (docs/ux/README.md §4.2) for Files' side region and Chat's tree column. */
class FilesExplorerProvider(context: Context) : ExplorerProvider {
    private val appContext = context.applicationContext
    override fun create(context: ExplorerContext): ExplorerController = FilesExplorer(context, appContext)
}

/** Inline name editing (new file / new folder / rename). */
internal sealed interface InlineEdit {
    val text: String
    val error: String?

    data class Create(val parent: String, val directory: Boolean, override val text: String = "", override val error: String? = null) : InlineEdit
    data class Rename(val path: String, override val text: String, override val error: String? = null) : InlineEdit
}

@OptIn(FlowPreview::class)
internal class FilesExplorer(val context: ExplorerContext, private val appContext: Context) : ExplorerController {
    private val store = context.store
    /** Loaded listings by directory ("" = root). Only the root and expanded directories are loaded (lazy tree). */
    val children = mutableStateMapOf<String, List<FileEntry>>()
    private val watches = HashMap<String, Job>()
    var showHidden by mutableStateOf(context.view.value.showHidden)
        private set
    var filterOpen by mutableStateOf(false)
    var filter by mutableStateOf("")
    var edit by mutableStateOf<InlineEdit?>(null)
    /** Path to bring into view once its row exists (reveal, follow editor, new items). */
    var scrollTarget by mutableStateOf<String?>(null)
    /** Paths being moved with "移动到…" (the folder picker is open). */
    var moving by mutableStateOf<List<String>?>(null)
    var loaded by mutableStateOf(false)
        private set
    private val importer by lazy { ImportService.get(appContext) }

    init {
        context.scope.launch {
            context.view.map { (setOf("") + it.expanded) to it.showHidden }.distinctUntilChanged().collect { (directories, hidden) ->
                val changed = showHidden != hidden
                showHidden = hidden
                sync(directories)
                if (changed) refresh()
            }
        }
        context.scope.launch {
            // Follow the editor: expand ancestors of the active file, select it, scroll to it.
            context.activeFile.filter { it != null }.collect { path -> reveal(path!!, updateSelection = true) }
        }
        context.scope.launch {
            importer.orphaned.filter { it.first.owner == OWNER }.collect { (request, result) -> reportImport(request.targetDir, result) }
        }
    }

    // ---- Loading and watching ----------------------------------------------------------------------

    private fun sync(directories: Set<String>) {
        (watches.keys - directories).forEach { directory ->
            watches.remove(directory)?.cancel()
            children.remove(directory)
        }
        (directories - watches.keys).forEach { directory ->
            // A watch that fails keeps the last listing; the connection card explains a lost connection.
            watches[directory] = context.scope.launchAction({}) {
                load(directory)
                store.directoryChanges(directory).debounce(150).collect { load(directory) }
            }
        }
    }

    private suspend fun load(directory: String) {
        val entries = runCatching { store.listDirectory(directory, showHidden) }.getOrNull()
        if (entries == null) {
            // The folder vanished: drop it from the expanded set.
            if (directory.isNotEmpty()) collapse(directory)
            children.remove(directory)
        } else {
            children[directory] = entries
        }
        if (directory.isEmpty()) loaded = true
    }

    fun refresh() {
        context.scope.launchAction({}) { (children.keys.toList()).forEach { load(it) } }
    }

    private fun refresh(directory: String) {
        context.scope.launchAction({}) { if (children.containsKey(directory)) load(directory) }
    }

    fun isDirectory(path: String): Boolean? =
        children[WorkspacePaths.parent(path)]?.firstOrNull { it.path == path }?.isDirectory

    // ---- Expansion and selection -------------------------------------------------------------------

    fun toggle(path: String) {
        val view = context.view.value
        val expanded = if (path in view.expanded) view.expanded.filterNot { WorkspacePaths.isWithin(it, path) } else view.expanded + path
        context.updateView(view.copy(expanded = expanded, selected = path))
    }

    fun expand(path: String) {
        val view = context.view.value
        if (path.isEmpty() || path in view.expanded) return
        context.updateView(view.copy(expanded = view.expanded + path))
    }

    private fun collapse(path: String) {
        val view = context.view.value
        if (path in view.expanded) context.updateView(view.copy(expanded = view.expanded.filterNot { WorkspacePaths.isWithin(it, path) }))
    }

    fun select(path: String?) {
        val view = context.view.value
        if (view.selected != path) context.updateView(view.copy(selected = path))
    }

    fun collapseAll() = context.updateView(context.view.value.copy(expanded = emptyList()))

    override fun reveal(path: String) = reveal(path, updateSelection = true)

    private fun reveal(path: String, updateSelection: Boolean) {
        val view = context.view.value
        val ancestors = ExplorerModel.ancestors(path)
        val expanded = (view.expanded + ancestors).distinct()
        if (expanded != view.expanded || (updateSelection && view.selected != path)) {
            context.updateView(view.copy(expanded = expanded, selected = if (updateSelection) path else view.selected))
        }
        if (!showHidden && ExplorerModel.needsHiddenFiles(path)) changeShowHidden(true)
        scrollTarget = path
    }

    fun changeShowHidden(value: Boolean) {
        if (showHidden == value) return
        context.updateView(context.view.value.copy(showHidden = value))
    }

    /** Where `＋` items and uploads go: the selected folder, else the selection's folder, else the root. */
    fun targetDirectory(): String = ExplorerModel.targetDirectory(context.view.value.selected, ::isDirectory)

    // ---- Opening ------------------------------------------------------------------------------------

    fun open(entry: FileEntry) {
        select(entry.path)
        if (entry.isDirectory) toggle(entry.path) else context.commands.open(PanelTarget.forFile(entry.path))
    }

    private fun openInSplit(path: String) {
        val wb = store.state.value.session(context.sessionId)?.workbench ?: return
        context.layout(LayoutOp.Open(PanelTarget.forFile(path), Placement.SplitEdge(wb.lastEditorStack, Edge.RIGHT)))
    }

    // ---- Create / rename (inline) -------------------------------------------------------------------

    override fun beginCreate(directory: Boolean) = beginCreate(targetDirectory(), directory)

    fun beginCreate(parent: String, directory: Boolean) {
        if (parent.isNotEmpty()) expand(parent)
        edit = InlineEdit.Create(parent, directory)
        scrollTarget = parent.ifEmpty { null }
    }

    fun beginRename(path: String) {
        edit = InlineEdit.Rename(path, WorkspacePaths.name(path))
    }

    fun updateEdit(text: String) {
        val current = edit ?: return
        val clean = text.replace("\n", "")
        edit = when (current) {
            is InlineEdit.Create -> current.copy(text = clean, error = null)
            is InlineEdit.Rename -> current.copy(text = clean, error = null)
        }
    }

    fun cancelEdit() { edit = null }

    fun commitEdit() {
        val current = edit ?: return
        val name = current.text.trim()
        if (current is InlineEdit.Create && name.isEmpty()) { cancelEdit(); return }
        val parent = when (current) {
            is InlineEdit.Create -> current.parent
            is InlineEdit.Rename -> WorkspacePaths.parent(current.path)
        }
        if (current is InlineEdit.Rename && name == WorkspacePaths.name(current.path)) { edit = null; return }
        val problem = FileNames.problem(name, parent)
            ?: if (children[parent].orEmpty().any { it.name == name }) "已存在同名项" else null
        if (problem != null) { edit = withError(current, problem); return }
        val path = WorkspacePaths.child(parent, name)
        context.scope.launchAction({ failure -> edit = withError(current, failure.summary) }) {
            val result = when (current) {
                is InlineEdit.Create -> if (current.directory) store.createDirectory(path) else store.createFile(path)
                is InlineEdit.Rename -> store.movePath(current.path, path)
            }
            when (result) {
                FileOpResult.Done -> {
                    edit = null
                    load(parent)
                    select(path)
                    scrollTarget = path
                    if (current is InlineEdit.Create && !current.directory) context.commands.open(PanelTarget.forFile(path))
                }
                is FileOpResult.Failed -> edit = withError(current, shortReason(result.message))
            }
        }
    }

    private fun withError(edit: InlineEdit, error: String): InlineEdit = when (edit) {
        is InlineEdit.Create -> edit.copy(error = error)
        is InlineEdit.Rename -> edit.copy(error = error)
    }

    // ---- Delete (trash + undo), duplicate, move ----------------------------------------------------------

    fun delete(path: String) {
        context.scope.launchAction(context.commands) {
            when (val result = store.trashPath(path)) {
                is TrashResult.Trashed -> {
                    refresh(WorkspacePaths.parent(path))
                    context.commands.snackbar("已删除「${WorkspacePaths.name(path)}」", "撤销") {
                        context.scope.launchAction(context.commands) {
                            when (val restored = store.restoreFromTrash(result.entry.id)) {
                                is RestoreResult.Restored -> { refresh(WorkspacePaths.parent(restored.path)); select(restored.path); scrollTarget = restored.path }
                                is RestoreResult.Failed -> context.commands.snackbar("无法恢复：${shortReason(restored.message)}")
                            }
                        }
                    }
                }
                is TrashResult.Failed -> context.commands.snackbar("无法删除：${shortReason(result.message)}")
            }
        }
    }

    fun duplicate(path: String) {
        context.scope.launchAction(context.commands) {
            val parent = WorkspacePaths.parent(path)
            val taken = runCatching { store.listDirectory(parent, showHidden = true) }.getOrDefault(emptyList()).map { it.name }.toSet()
            val target = WorkspacePaths.child(parent, FileNames.unique(WorkspacePaths.name(path), taken))
            when (val result = store.copyPath(path, target)) {
                FileOpResult.Done -> { load(parent); select(target); scrollTarget = target }
                is FileOpResult.Failed -> context.commands.snackbar("无法复制：${shortReason(result.message)}")
            }
        }
    }

    /** Drag and drop onto a folder (or "移动到…"): confirm, then move each path. */
    fun requestMove(paths: List<String>, folder: String) {
        if (!ExplorerModel.canMoveInto(paths, folder)) return
        context.scope.launchAction(context.commands) {
            val label = if (folder.isEmpty()) "工作区" else WorkspacePaths.name(folder)
            val choice = context.commands.decide(DecisionRequest(
                title = "移动到「$label」？",
                items = paths.map { WorkspacePaths.name(it) },
                options = listOf(DecisionOption("move", "移动", DecisionStyle.Filled)),
            ))
            if (choice == "move") move(paths, folder)
        }
    }

    fun move(paths: List<String>, folder: String) {
        context.scope.launchAction(context.commands) {
            val taken = runCatching { store.listDirectory(folder, showHidden = true) }.getOrDefault(emptyList()).map { it.name }.toMutableSet()
            val failures = ArrayList<String>()
            var last: String? = null
            for (path in paths) {
                val name = WorkspacePaths.name(path)
                if (name in taken) { failures += "「$name」已存在"; continue }
                val target = WorkspacePaths.child(folder, name)
                when (val result = store.movePath(path, target)) {
                    FileOpResult.Done -> { taken += name; last = target; refresh(WorkspacePaths.parent(path)) }
                    is FileOpResult.Failed -> failures += shortReason(result.message)
                }
            }
            if (folder.isNotEmpty()) expand(folder)
            refresh(folder)
            last?.let { select(it); scrollTarget = it }
            if (failures.isNotEmpty()) context.commands.snackbar("无法移动：${failures.first()}")
        }
    }

    // ---- Import ------------------------------------------------------------------------------------

    fun upload(kind: ImportKind, directory: String = targetDirectory()) {
        if (!ExplorerModel.canImportInto(directory)) { context.commands.snackbar(PROTECTED); return }
        context.scope.launchAction(context.commands) { reportImport(directory, importer.pick(kind, directory, OWNER)) }
    }

    fun importExternal(uris: List<android.net.Uri>, directory: String) {
        if (!ExplorerModel.canImportInto(directory)) { context.commands.snackbar(PROTECTED); return }
        context.scope.launchAction(context.commands) { reportImport(directory, importer.importUris(uris, directory)) }
    }

    private suspend fun reportImport(directory: String, result: ImportResult) {
        when (result) {
            is ImportResult.Imported -> {
                if (directory.isNotEmpty()) expand(directory)
                if (children.containsKey(directory)) load(directory)
                result.paths.lastOrNull()?.let { select(it); scrollTarget = it }
                if (result.failures.isNotEmpty()) context.commands.snackbar("无法导入 ${result.failures.joinToString("、")}")
            }
            is ImportResult.Failed -> context.commands.snackbar(result.message)
            ImportResult.Cancelled -> Unit
        }
    }

    // ---- Menus -------------------------------------------------------------------------------------

    private fun uploadGroup(directory: () -> String) = MenuGroup("upload", listOf(
        MenuEntry.Action("camera", "拍照", Sym.PhotoCamera) { upload(ImportKind.CAMERA, directory()) },
        MenuEntry.Action("gallery", "相册", Sym.PhotoLibrary) { upload(ImportKind.GALLERY, directory()) },
        MenuEntry.Action("files", "设备文件", Sym.UploadFile) { upload(ImportKind.FILES, directory()) },
    ))

    override val actions: List<ToolAction> get() = listOf(
        ToolAction("add", Sym.Add, "新建", menu = listOf(
            MenuGroup("new", listOf(
                MenuEntry.Action("newFile", "新建文件", Sym.NoteAdd) { beginCreate(false) },
                MenuEntry.Action("newFolder", "新建文件夹", Sym.CreateNewFolder) { beginCreate(true) },
            )),
            uploadGroup { targetDirectory() },
        )),
    )

    override val resourceMenu: List<MenuEntry> get() = listOf(
        MenuEntry.Action("filter", "筛选…", Sym.FilterList) { filterOpen = true },
        MenuEntry.Action("collapseAll", "全部折叠", Sym.UnfoldLess) { collapseAll() },
        MenuEntry.Action("refresh", "刷新", Sym.Refresh) { refresh() },
        MenuEntry.Toggle("hidden", "显示隐藏文件", showHidden) { changeShowHidden(it) },
        MenuEntry.Action("terminal", "在终端中打开", Sym.Terminal) {
            context.commands.newTerminal(ExplorerModel.terminalDirectory(targetDirectory()))
        },
    )

    /** Long-press menu of a row (docs/ux/README.md §4.2); `.workspace` entries are protected. */
    fun rowMenu(entry: FileEntry): List<MenuGroup> {
        val path = entry.path
        val folder = if (entry.isDirectory) path else WorkspacePaths.parent(path)
        val can = ExplorerModel.rowCapabilities(path)
        return buildList {
            if (!entry.isDirectory) add(MenuGroup("open", listOf(
                MenuEntry.Action("open", "打开", Sym.OpenInNew) { open(entry) },
                MenuEntry.Action("openSplit", "在右侧拆分中打开", Sym.SplitscreenRight) { openInSplit(path) },
            )))
            add(MenuGroup("new", buildList {
                add(MenuEntry.Action("newFile", "新建文件", Sym.NoteAdd) { beginCreate(folder, false) })
                add(MenuEntry.Action("newFolder", "新建文件夹", Sym.CreateNewFolder) { beginCreate(folder, true) })
                if (entry.isDirectory && can.upload) add(MenuEntry.Submenu("upload", "上传…", Sym.UploadFile, listOf(uploadGroup { path })))
            }))
            if (can.structural) add(MenuGroup("edit", listOf(
                MenuEntry.Action("rename", "重命名", Sym.Edit) { beginRename(path) },
                MenuEntry.Action("duplicate", "复制", Sym.ContentCopy) { duplicate(path) },
                MenuEntry.Action("moveTo", "移动到…", Sym.DriveFileMove) { moving = listOf(path) },
                MenuEntry.Action("delete", "删除", Sym.Delete, destructive = true) { delete(path) },
            )))
            add(MenuGroup("path", buildList {
                add(MenuEntry.Action("copyPath", "复制路径", Sym.ContentCopy) {
                    WorkspaceIntents.copyText(appContext, "路径", WorkspaceIntents.absolutePath(appContext, path))
                    context.commands.snackbar("已复制")
                })
                add(MenuEntry.Action("copyRelative", "复制相对路径", Sym.ContentCopy) {
                    WorkspaceIntents.copyText(appContext, "路径", path)
                    context.commands.snackbar("已复制")
                })
                add(MenuEntry.Action("attach", "附加到对话", Sym.AddComment) { context.commands.attachToConversation(listOf(path)) })
                if (can.terminal) add(MenuEntry.Action("terminal", "在终端中打开", Sym.Terminal) { context.commands.newTerminal(folder) })
                if (!entry.isDirectory) add(MenuEntry.Action("openWith", "用其他应用打开", Sym.OpenInNew) {
                    context.scope.launch {
                        if (!WorkspaceIntents.openWith(appContext, path)) context.commands.snackbar("无法读取文件或没有可用的应用")
                    }
                })
            }))
        }
    }

    @Composable
    override fun Content(variant: ExplorerVariant, modifier: Modifier) = ExplorerTree(this, variant, modifier)

    override fun dispose() {
        watches.values.forEach { it.cancel() }
        watches.clear()
    }

    companion object {
        const val OWNER = "explorer"
        private const val PROTECTED = "该位置由应用保留"

        /** Store/file-system messages are English and technical; keep them short for the UI. */
        fun shortReason(message: String): String = when {
            message.startsWith("Already exists") -> "已存在同名项"
            message.startsWith("Nothing to") -> "文件不存在"
            message.startsWith("Cannot move a directory into itself") || message.startsWith("Cannot copy a folder into itself") -> "不能移动到自身内部"
            message.startsWith("File too large") -> "文件过大"
            message.startsWith("Not an editable workspace path", ignoreCase = true) -> PROTECTED
            message.contains("protected and read-only") -> "受保护的只读项"
            message.startsWith("configuration cannot be", ignoreCase = true) -> "受保护的配置不能移动或删除"
            else -> message.take(60)
        }
    }
}
