package top.flysoftbeta.workflow.feature.editor

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import top.flysoftbeta.workflow.app.panel.PanelContext
import top.flysoftbeta.workflow.app.panel.PanelController
import top.flysoftbeta.workflow.app.panel.PanelFrame
import top.flysoftbeta.workflow.app.panel.PanelTab
import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.core.resource.ConflictResolution
import top.flysoftbeta.workflow.core.resource.DiffLine
import top.flysoftbeta.workflow.core.resource.FileStatus
import top.flysoftbeta.workflow.core.resource.LineDiff
import top.flysoftbeta.workflow.ui.design.EmptyState
import top.flysoftbeta.workflow.ui.design.NoticeBar
import top.flysoftbeta.workflow.ui.design.NoticeTone
import top.flysoftbeta.workflow.ui.design.TextAction
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/**
 * Read-only "对比" panel for a conflict (docs/ux/README.md §4.3): disk version on the left, the draft on the
 * right; side by side at ≥ 720dp, else one inline column. The conflict actions stay available on top.
 */
internal class DiffController(private val path: String, private val context: PanelContext) : PanelController {
    private var lines by mutableStateOf<List<DiffLine>?>(null)
    private var status by mutableStateOf(FileStatus.CLEAN)

    init {
        context.scope.launch {
            context.store.state.map { it.drafts[path] to it.disk[path] }.distinctUntilChanged().collect { (draft, _) ->
                status = context.store.state.value.fileStatus(path)
                val snapshot = runCatching { context.store.openFile(path) }.getOrNull()
                val disk = snapshot?.diskText.orEmpty()
                val mine = draft?.text ?: disk
                lines = withContext(Dispatchers.Default) { LineDiff.diff(LineDiff.lines(disk), LineDiff.lines(mine)) }
            }
        }
    }

    override val tab: PanelTab get() = PanelTab("对比 · ${WorkspacePaths.name(path)}", Sym.SwapHoriz)

    @Composable
    override fun Content(frame: PanelFrame, modifier: Modifier) {
        val colors = WorkflowTheme.colors
        Column(modifier.fillMaxSize().background(colors.surface).testTag("diff:$path")) {
            if (status == FileStatus.CONFLICT || status == FileStatus.DELETED) {
                NoticeBar("磁盘版本 ← → 我的更改", NoticeTone.Tertiary, actions = listOf(
                    TextAction("保留我的") { context.scope.launch { context.store.resolveConflict(path, ConflictResolution.KEEP_MINE) } },
                    TextAction("使用磁盘版本") { context.scope.launch { context.store.resolveConflict(path, ConflictResolution.TAKE_DISK) } },
                ))
            }
            val diff = lines ?: return@Column
            if (diff.none { it !is DiffLine.Same }) {
                EmptyState(emptyList(), Modifier.fillMaxSize(), message = "没有差异")
                return@Column
            }
            BoxWithConstraints(Modifier.weight(1f).fillMaxWidth()) {
                if (maxWidth >= 720.dp) SideBySide(diff) else Inline(diff)
            }
        }
    }

    @Composable
    private fun Inline(diff: List<DiffLine>) {
        val horizontal = rememberScrollState()
        LazyColumn(Modifier.fillMaxSize().horizontalScroll(horizontal)) {
            itemsIndexed(diff) { _, line ->
                when (line) {
                    is DiffLine.Same -> DiffRow(line.leftNo, line.rightNo, " ", line.text, Color.Transparent)
                    is DiffLine.Removed -> DiffRow(line.leftNo, null, "-", line.text, WorkflowTheme.colors.errorContainer.copy(alpha = 0.6f))
                    is DiffLine.Added -> DiffRow(null, line.rightNo, "+", line.text, WorkflowTheme.extendedColors.successContainer.copy(alpha = 0.6f))
                }
            }
        }
    }

    @Composable
    private fun SideBySide(diff: List<DiffLine>) {
        val removed = WorkflowTheme.colors.errorContainer.copy(alpha = 0.6f)
        val added = WorkflowTheme.extendedColors.successContainer.copy(alpha = 0.6f)
        // Pair removals with following additions on one row.
        val rows = ArrayList<Pair<DiffLine?, DiffLine?>>()
        var i = 0
        while (i < diff.size) {
            val line = diff[i]
            if (line is DiffLine.Removed) {
                val removals = ArrayList<DiffLine>()
                while (i < diff.size && diff[i] is DiffLine.Removed) removals += diff[i++]
                val additions = ArrayList<DiffLine>()
                while (i < diff.size && diff[i] is DiffLine.Added) additions += diff[i++]
                for (k in 0 until maxOf(removals.size, additions.size)) rows += removals.getOrNull(k) to additions.getOrNull(k)
            } else {
                rows += if (line is DiffLine.Added) null to line else line to line
                i++
            }
        }
        LazyColumn(Modifier.fillMaxSize()) {
            itemsIndexed(rows) { _, (left, right) ->
                Row(Modifier.fillMaxWidth()) {
                    Box(Modifier.weight(1f)) {
                        when (left) {
                            is DiffLine.Same -> DiffRow(left.leftNo, null, " ", left.text, Color.Transparent)
                            is DiffLine.Removed -> DiffRow(left.leftNo, null, "-", left.text, removed)
                            else -> DiffRow(null, null, " ", "", Color.Transparent)
                        }
                    }
                    Box(Modifier.weight(1f)) {
                        when (right) {
                            is DiffLine.Same -> DiffRow(null, right.rightNo, " ", right.text, Color.Transparent)
                            is DiffLine.Added -> DiffRow(null, right.rightNo, "+", right.text, added)
                            else -> DiffRow(null, null, " ", "", Color.Transparent)
                        }
                    }
                }
            }
        }
    }

    @Composable
    private fun DiffRow(left: Int?, right: Int?, sign: String, text: String, background: Color) {
        val colors = WorkflowTheme.colors
        val mono = WorkflowTheme.text.mono
        Row(Modifier.fillMaxWidth().background(background).padding(horizontal = 4.dp)) {
            Text(left?.toString().orEmpty(), Modifier.width(36.dp), style = mono, color = colors.onSurfaceVariant, textAlign = TextAlign.End, maxLines = 1)
            Text(right?.toString().orEmpty(), Modifier.width(36.dp), style = mono, color = colors.onSurfaceVariant, textAlign = TextAlign.End, maxLines = 1)
            Text(" $sign ", style = mono, color = colors.onSurfaceVariant)
            Text(text, style = mono, color = colors.onSurface, softWrap = false, maxLines = 1)
        }
    }
}
