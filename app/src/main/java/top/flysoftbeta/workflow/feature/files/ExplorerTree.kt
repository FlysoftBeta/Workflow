package top.flysoftbeta.workflow.feature.files

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.scrollBy
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import top.flysoftbeta.workflow.app.panel.ExplorerVariant
import top.flysoftbeta.workflow.core.io.FileEntry
import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.platform.importer.ImportKind
import top.flysoftbeta.workflow.ui.design.CompositeMenu
import top.flysoftbeta.workflow.ui.design.EmptyState
import top.flysoftbeta.workflow.ui.design.SearchField
import top.flysoftbeta.workflow.ui.design.SymbolIcon
import top.flysoftbeta.workflow.ui.design.TextAction
import top.flysoftbeta.workflow.ui.design.WfIconButton
import top.flysoftbeta.workflow.ui.design.dnd.AreaStyle
import top.flysoftbeta.workflow.ui.design.dnd.DragDropState
import top.flysoftbeta.workflow.ui.design.dnd.DragPayload
import top.flysoftbeta.workflow.ui.design.dnd.DropTargetKind
import top.flysoftbeta.workflow.ui.design.dnd.ExternalDragPayload
import top.flysoftbeta.workflow.ui.design.dnd.FilesDragPayload
import top.flysoftbeta.workflow.ui.design.dnd.LocalDragDropState
import top.flysoftbeta.workflow.ui.design.dnd.dragAutoScroll
import top.flysoftbeta.workflow.ui.design.dnd.dragSource
import top.flysoftbeta.workflow.ui.design.dnd.dropTarget
import top.flysoftbeta.workflow.ui.design.icons.FileTypeIcons
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** Open files and drafts, for the row weight and the draft dot. */
private data class TreeMarks(val open: Set<String>, val drafts: Set<String>)

/** The lazy tree (docs/ui.md §4.2): 32dp rows, 12dp indent with 1dp guides, selection pill, draft dots. */
@Composable
internal fun ExplorerTree(explorer: FilesExplorer, variant: ExplorerVariant, modifier: Modifier) {
    val context = explorer.context
    val view by context.view.collectAsState()
    val marks by remember(context) {
        context.store.state.map { state ->
            val open = state.session(context.sessionId)?.workbench?.panels?.values?.mapNotNull {
                when (val t = it.target) { is PanelTarget.File -> t.path; is PanelTarget.Image -> t.path; else -> null }
            }.orEmpty().toSet()
            TreeMarks(open, state.drafts.keys)
        }.distinctUntilChanged()
    }.collectAsState(TreeMarks(emptySet(), emptySet()))
    val edit = explorer.edit
    val creating = (edit as? InlineEdit.Create)?.let { TreeRow.Creating(it.parent, it.directory, 0) }
    val rows = ExplorerModel.flatten(explorer.children, view.expanded.toSet(), creating, if (explorer.filterOpen) explorer.filter else "")
    val list = rememberLazyListState()
    val dnd = LocalDragDropState.current
    val rootKey = remember(explorer) { "treeRoot:${System.identityHashCode(explorer)}" }

    LaunchedEffect(explorer.scrollTarget, rows.size) {
        val target = explorer.scrollTarget ?: return@LaunchedEffect
        val index = rows.indexOfFirst { it.key == target }
        if (index < 0) return@LaunchedEffect
        val visible = list.layoutInfo.visibleItemsInfo
        val shown = visible.any { it.index == index } && visible.lastOrNull()?.index != index
        if (!shown) list.scrollToItem((index - 3).coerceAtLeast(0))
        explorer.scrollTarget = null
    }
    if (edit != null) BackHandler { explorer.cancelEdit() }

    Column(modifier.fillMaxSize().testTag("explorer")) {
        if (explorer.filterOpen) {
            Row(Modifier.fillMaxWidth().padding(horizontal = 4.dp, vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                SearchField(explorer.filter, { explorer.filter = it }, Modifier.weight(1f), placeholder = "筛选")
                WfIconButton(Sym.Close, "关闭筛选", { explorer.filterOpen = false; explorer.filter = "" })
            }
            BackHandler { explorer.filterOpen = false; explorer.filter = "" }
        }
        Box(
            Modifier.weight(1f).fillMaxWidth()
                .then(if (dnd != null) Modifier.dropTarget(
                    dnd, key = rootKey, kind = DropTargetKind.Area(AreaStyle.Self), priority = 1,
                    accepts = { payload -> accepts(payload, "") },
                    onDrop = { payload, _ -> drop(explorer, payload, "") },
                ) else Modifier),
        ) {
            if (explorer.loaded && rows.isEmpty()) {
                EmptyState(
                    listOf(TextAction("新建") { explorer.beginCreate("", false) }, TextAction("上传") { explorer.upload(ImportKind.FILES, "") }),
                    message = if (explorer.filterOpen && explorer.filter.isNotBlank()) "没有匹配的文件" else "空文件夹",
                )
            }
            LazyColumn(
                Modifier.fillMaxSize()
                    .then(if (dnd != null) Modifier.dragAutoScroll(dnd, horizontal = false) { list.scrollBy(it) } else Modifier),
                state = list,
                contentPadding = androidx.compose.foundation.layout.PaddingValues(vertical = 4.dp),
            ) {
                items(rows, key = { it.key }) { row ->
                    when (row) {
                        is TreeRow.Creating -> EditRow(explorer, row.depth, row.directory, null)
                        is TreeRow.Node -> {
                            val renaming = (edit as? InlineEdit.Rename)?.takeIf { it.path == row.entry.path }
                            if (renaming != null) EditRow(explorer, row.depth, row.entry.isDirectory, row.entry)
                            else NodeRow(explorer, row, view.selected == row.entry.path, marks, dnd)
                        }
                    }
                }
            }
        }
    }
    explorer.moving?.let { paths -> MoveDialog(explorer, paths) }
}

private fun accepts(payload: DragPayload, folder: String): Boolean = when (payload) {
    is FilesDragPayload -> ExplorerModel.canMoveInto(payload.paths, folder)
    is ExternalDragPayload -> true
    else -> false
}

private fun drop(explorer: FilesExplorer, payload: DragPayload, folder: String) {
    when (payload) {
        is FilesDragPayload -> explorer.requestMove(payload.paths, folder)
        is ExternalDragPayload -> explorer.importExternal(payload.uris, folder)
    }
}

@Composable
private fun NodeRow(explorer: FilesExplorer, row: TreeRow.Node, selected: Boolean, marks: TreeMarks, dnd: DragDropState?) {
    val entry = row.entry
    val colors = WorkflowTheme.colors
    val dimens = WorkflowTheme.dimens
    var menu by remember { mutableStateOf(false) }
    val folder = if (entry.isDirectory) entry.path else WorkspacePaths.parent(entry.path)
    val targetKey = "treeRow:${System.identityHashCode(explorer)}:${entry.path}"
    val hovered = dnd?.isHovered(targetKey) == true
    val dirty = if (entry.isDirectory) !row.expanded && marks.drafts.any { WorkspacePaths.isWithin(it, entry.path) && it != entry.path }
        else entry.path in marks.drafts
    val guideColor = colors.outlineVariant
    val indent = dimens.indent
    Box {
        Row(
            Modifier
                .fillMaxWidth()
                .heightIn(min = dimens.treeRow)
                .padding(horizontal = 4.dp)
                .clip(WorkflowShapes.sm)
                .background(when {
                    hovered && entry.isDirectory -> colors.primaryContainer
                    hovered -> colors.surfaceContainerHigh
                    selected -> colors.secondaryContainer
                    else -> Color.Transparent
                })
                .then(if (dnd != null) Modifier
                    .dropTarget(
                        dnd, key = targetKey, kind = DropTargetKind.Area(AreaStyle.Self), priority = 2,
                        accepts = { accepts(it, folder) },
                        onHoverActivate = if (entry.isDirectory) ({ explorer.expand(entry.path) }) else null,
                        onDrop = { payload, _ -> drop(explorer, payload, folder) },
                    )
                    .dragSource(dnd, key = "tree:${entry.path}", onLongPressWithoutMove = { explorer.select(entry.path); menu = true }) {
                        FilesDragPayload(listOf(entry.path), icon = FileTypeIcons.forName(entry.name, entry.isDirectory))
                    } else Modifier)
                .clickable { explorer.open(entry) }
                .drawBehind {
                    // 1dp indent guides at the centre of each ancestor's chevron slot.
                    for (level in 0 until row.depth) {
                        val x = (indent * level + 10.dp).toPx()
                        drawLine(guideColor, Offset(x, 0f), Offset(x, size.height), strokeWidth = 1.dp.toPx())
                    }
                }
                .padding(start = indent * row.depth, end = 8.dp)
                .semantics {
                    if (entry.isDirectory) stateDescription = if (row.expanded) "已展开" else "已折叠"
                },
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(Modifier.width(20.dp), contentAlignment = Alignment.Center) {
                if (entry.isDirectory) SymbolIcon(
                    if (row.expanded) Sym.KeyboardArrowDown else Sym.ChevronRight, null, size = 16.dp, tint = colors.onSurfaceVariant,
                )
            }
            SymbolIcon(FileTypeIcons.forName(entry.name, entry.isDirectory, row.expanded), null, size = 16.dp, tint = colors.onSurfaceVariant)
            Spacer(Modifier.width(6.dp))
            Text(
                entry.name,
                Modifier.weight(1f),
                style = if (entry.path in marks.open) WorkflowTheme.text.labelActive else WorkflowTheme.text.label,
                color = if (entry.name.startsWith(".")) colors.onSurfaceVariant else colors.onSurface,
                maxLines = 1, overflow = TextOverflow.Ellipsis,
            )
            if (dirty) Box(Modifier.padding(start = 4.dp).size(6.dp).clip(CircleShape).background(colors.primary))
        }
        CompositeMenu(expanded = menu, onDismissRequest = { menu = false }, groups = explorer.rowMenu(entry))
    }
}

/** Inline name field (28dp) for new items and rename; Enter or ✓ commits, Back cancels. */
@Composable
private fun EditRow(explorer: FilesExplorer, depth: Int, directory: Boolean, entry: FileEntry?) {
    val colors = WorkflowTheme.colors
    val dimens = WorkflowTheme.dimens
    val edit = explorer.edit ?: return
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) { runCatching { focus.requestFocus() } }
    Column(Modifier.fillMaxWidth().padding(horizontal = 4.dp)) {
        Row(
            Modifier.fillMaxWidth().heightIn(min = dimens.treeRow).padding(start = dimens.indent * depth + 20.dp, end = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            SymbolIcon(FileTypeIcons.forName(edit.text, directory), null, size = 16.dp, tint = colors.onSurfaceVariant)
            Spacer(Modifier.width(6.dp))
            BasicTextField(
                value = edit.text,
                onValueChange = explorer::updateEdit,
                modifier = Modifier.weight(1f).focusRequester(focus).testTag("explorer:edit"),
                singleLine = true,
                textStyle = WorkflowTheme.text.label.copy(color = colors.onSurface),
                cursorBrush = SolidColor(colors.primary),
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done, autoCorrectEnabled = false),
                keyboardActions = KeyboardActions(onDone = { explorer.commitEdit() }),
                decorationBox = { inner ->
                    Box(
                        Modifier.height(28.dp).fillMaxWidth()
                            .border(1.dp, if (edit.error != null) colors.error else colors.primary, WorkflowShapes.xs)
                            .padding(horizontal = 6.dp),
                        contentAlignment = Alignment.CenterStart,
                    ) {
                        if (edit.text.isEmpty() && entry == null) {
                            Text(if (directory) "文件夹名称" else "文件名称", style = WorkflowTheme.text.label, color = colors.onSurfaceVariant)
                        }
                        inner()
                    }
                },
            )
            WfIconButton(Sym.Check, "确定", { explorer.commitEdit() }, iconSize = 18.dp)
        }
        edit.error?.let {
            Text(it, Modifier.padding(start = dimens.indent * depth + 42.dp, bottom = 4.dp), style = WorkflowTheme.text.caption, color = colors.error)
        }
    }
}
