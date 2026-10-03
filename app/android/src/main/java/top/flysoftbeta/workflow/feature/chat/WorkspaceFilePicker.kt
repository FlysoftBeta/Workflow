package top.flysoftbeta.workflow.feature.chat

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import top.flysoftbeta.workflow.core.io.FileEntry
import top.flysoftbeta.workflow.core.store.WorkspaceStore
import top.flysoftbeta.workflow.ui.design.SymbolIcon
import top.flysoftbeta.workflow.ui.design.icons.FileTypeIcons
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/**
 * "工作区文件…" (docs/ux/README.md §4.8): a tree with checkboxes and "添加 (n)". Directories expand on tap;
 * only files can be picked. Listing happens off the main thread through the store.
 */
@Composable
internal fun WorkspaceFilePicker(store: WorkspaceStore, onDismiss: () -> Unit, onAdd: (List<String>) -> Unit) {
    val children = remember { mutableStateMapOf<String, List<FileEntry>>() }
    val expanded = remember { mutableStateListOf<String>() }
    val selected = remember { mutableStateListOf<String>() }
    val scope = rememberCoroutineScope()
    fun load(dir: String) = scope.launch { children[dir] = runCatching { store.listDirectory(dir) }.getOrDefault(emptyList()) }
    LaunchedEffect(Unit) { load("") }
    val rows = buildList {
        fun walk(dir: String, depth: Int) {
            children[dir].orEmpty().sortedWith(compareBy<FileEntry>({ !it.isDirectory }, { it.name.lowercase() })).forEach { entry ->
                add(entry to depth)
                if (entry.isDirectory && entry.path in expanded) walk(entry.path, depth + 1)
            }
        }
        walk("", 0)
    }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("工作区文件", style = WorkflowTheme.text.titleMd) },
        text = {
            LazyColumn(Modifier.heightIn(max = 420.dp).fillMaxWidth()) {
                items(rows, key = { it.first.path }) { (entry, depth) ->
                    val open = entry.path in expanded
                    Row(
                        Modifier
                            .fillMaxWidth()
                            .heightIn(min = WorkflowTheme.dimens.treeRow)
                            .clickable {
                                if (entry.isDirectory) {
                                    if (open) expanded.remove(entry.path) else { expanded.add(entry.path); load(entry.path) }
                                } else if (entry.path in selected) selected.remove(entry.path) else selected.add(entry.path)
                            }
                            .padding(start = WorkflowTheme.dimens.indent * depth),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Box(Modifier.size(36.dp), contentAlignment = Alignment.Center) {
                            if (entry.isDirectory) SymbolIcon(if (open) Sym.KeyboardArrowDown else Sym.ChevronRight, null, size = 16.dp, tint = WorkflowTheme.colors.onSurfaceVariant)
                            else Checkbox(checked = entry.path in selected, onCheckedChange = { on -> if (on) selected.add(entry.path) else selected.remove(entry.path) })
                        }
                        SymbolIcon(FileTypeIcons.forName(entry.name, entry.isDirectory, open), null, size = 16.dp, tint = WorkflowTheme.colors.onSurfaceVariant)
                        Spacer(Modifier.width(6.dp))
                        Text(entry.name, style = WorkflowTheme.text.label, color = WorkflowTheme.colors.onSurface, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    }
                }
            }
        },
        confirmButton = { Button(onClick = { onAdd(selected.toList()) }, enabled = selected.isNotEmpty()) { Text("添加 (${selected.size})") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("取消") } },
    )
}
