package top.flysoftbeta.workflow.app.panel

import android.content.Context
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.ui.Modifier
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.StateFlow
import top.flysoftbeta.workflow.core.layout.ExplorerView
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.session.SessionId
import top.flysoftbeta.workflow.core.store.WorkspaceStore
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.ToolAction

/*
 * Fixed regions that are not panels (workspace.md §3): the file explorer (Files' side region and
 * Chat's tree column) and the conversation list (Chat's rail). The shell renders their header row
 * (injecting the corner cluster and the ②/③ menu groups) and card; the feature renders the content.
 */

/** What the shell gives a region controller. One controller per session and region. */
interface RegionContext {
    val appContext: Context
    val store: WorkspaceStore
    val sessionId: SessionId
    /** Main-dispatcher scope, cancelled on dispose. */
    val scope: CoroutineScope
    val commands: WorkbenchCommands

    /** A layout transaction on this session (e.g. `LayoutOp.showConversation(id)` from the rail). */
    fun layout(op: LayoutOp)
}

/** How the explorer is shown. */
enum class ExplorerVariant {
    /** Files' side region (264dp default), with its header row. */
    SIDEBAR,
    /** Chat side region's tree column (200dp), no header of its own. */
    TREE_COLUMN,
}

interface ExplorerContext : RegionContext {
    /** Expanded directories and selection, persisted with the session. */
    val view: StateFlow<ExplorerView>

    /** Path of the file in the focused editor (the explorer follows it: expand ancestors, scroll to it). */
    val activeFile: StateFlow<String?>

    fun updateView(view: ExplorerView) = layout(LayoutOp.UpdateExplorer(view))
}

@Stable
interface ExplorerController {
    /** Header actions: the `＋` menu (新建文件 · 新建文件夹 ┆ 拍照 · 相册 · 设备文件), via [ToolAction.menu]. */
    val actions: List<ToolAction> get() = emptyList()

    /** More ① group: 筛选… · 全部折叠 · 刷新 · ☐显示隐藏文件 · 在终端中打开. */
    val resourceMenu: List<MenuEntry> get() = emptyList()

    @Composable
    fun Content(variant: ExplorerVariant, modifier: Modifier)

    /** Expand ancestors, select and scroll to [path] (the shell has already made the region visible). */
    fun reveal(path: String) {}

    /** Empty editor area "新建文件": start inline creation in the selected (else root) directory. */
    fun beginCreate(directory: Boolean) {}

    fun dispose() {}
}

interface ExplorerProvider {
    fun create(context: ExplorerContext): ExplorerController
}

interface RailContext : RegionContext {
    /** The conversation shown in Chat's main column (the active aux panel), for the selected pill. */
    val activeConversation: StateFlow<String?>
}

/** Chat's conversation list (docs/ui.md §3.4): 今天 / 昨天 / 近 7 天 / 更早 groups, ⌕ and ✎ in the header. */
@Stable
interface RailController {
    /** Header actions (⌕ filter, ✎ new conversation). */
    val actions: List<ToolAction> get() = emptyList()

    val resourceMenu: List<MenuEntry> get() = emptyList()

    @Composable
    fun Content(modifier: Modifier)

    fun dispose() {}
}

interface RailProvider {
    fun create(context: RailContext): RailController
}
