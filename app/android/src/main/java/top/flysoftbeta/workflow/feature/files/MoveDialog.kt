package top.flysoftbeta.workflow.feature.files

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.ui.design.SymbolIcon
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** "移动到…": pick a destination folder (the workspace root first), then move without a second question. */
@Composable
internal fun MoveDialog(explorer: FilesExplorer, paths: List<String>) {
    val colors = WorkflowTheme.colors
    val folders by produceState<List<String>?>(null, paths) {
        // Breadth-first over folders, bounded so a huge tree cannot stall the dialog.
        val result = ArrayList<String>()
        val queue = ArrayDeque(listOf(""))
        while (queue.isNotEmpty() && result.size < 3000) {
            val directory = queue.removeFirst()
            val entries = runCatching { explorer.context.store.listDirectory(directory, explorer.showHidden) }.getOrDefault(emptyList())
            entries.filter { it.isDirectory && !ExplorerModel.isProtected(it.path) && paths.none { moving -> WorkspacePaths.isWithin(it.path, moving) } }.forEach {
                result += it.path
                queue += it.path
            }
        }
        value = listOf("") + result.sorted()
    }
    var chosen by remember { mutableStateOf<String?>(null) }
    AlertDialog(
        onDismissRequest = { explorer.moving = null },
        title = { Text("移动到", style = WorkflowTheme.text.titleMd) },
        text = {
            LazyColumn(Modifier.fillMaxWidth().heightIn(max = 360.dp)) {
                items(folders.orEmpty(), key = { it }) { folder ->
                    val depth = if (folder.isEmpty()) 0 else folder.count { it == '/' } + 1
                    val enabled = ExplorerModel.canMoveInto(paths, folder)
                    Row(
                        Modifier.fillMaxWidth().heightIn(min = WorkflowTheme.dimens.treeRow)
                            .clip(WorkflowShapes.sm)
                            .background(if (chosen == folder) colors.secondaryContainer else Color.Transparent)
                            .clickable(enabled = enabled) { chosen = folder }
                            .padding(start = 8.dp + WorkflowTheme.dimens.indent * depth, end = 8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        SymbolIcon(Sym.Folder, null, size = 16.dp, tint = if (enabled) colors.onSurfaceVariant else colors.outline)
                        Spacer(Modifier.width(6.dp))
                        Text(
                            if (folder.isEmpty()) "工作区" else WorkspacePaths.name(folder),
                            style = WorkflowTheme.text.label,
                            color = if (enabled) colors.onSurface else colors.outline,
                            maxLines = 1, overflow = TextOverflow.Ellipsis,
                        )
                    }
                }
            }
        },
        confirmButton = {
            TextButton(enabled = chosen != null, onClick = {
                val target = chosen ?: return@TextButton
                explorer.moving = null
                explorer.move(paths, target)
            }) { Text("移动") }
        },
        dismissButton = { TextButton(onClick = { explorer.moving = null }) { Text("取消") } },
    )
}
