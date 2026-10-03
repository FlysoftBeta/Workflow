package top.flysoftbeta.workflow.feature.chat

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.MenuAnchorPosition
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import kotlinx.coroutines.launch
import top.flysoftbeta.workflow.agent.model.ConversationEntry
import top.flysoftbeta.workflow.agent.model.ConversationIndexing
import top.flysoftbeta.workflow.agent.model.ThreadKey
import top.flysoftbeta.workflow.agent.model.TurnStatus
import top.flysoftbeta.workflow.app.panel.RailContext
import top.flysoftbeta.workflow.app.panel.RailController
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.platform.agent.AgentHub
import top.flysoftbeta.workflow.ui.design.CompositeMenu
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.MenuGroup
import top.flysoftbeta.workflow.ui.design.SearchField
import top.flysoftbeta.workflow.ui.design.SymbolIcon
import top.flysoftbeta.workflow.ui.design.ToolAction
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** Rows of the conversation list, grouped (pure; unit-tested). */
internal object RailModel {
    sealed interface Row {
        data class Header(val label: String) : Row
        data class Conversation(val entry: ConversationEntry) : Row
    }

    fun rows(entries: List<ConversationEntry>, query: String, archived: Boolean, today: LocalDate, zone: ZoneId): List<Row> {
        val filtered = ConversationIndexing.search(entries, query).filter { it.archived == archived }
        if (archived) return filtered.map { Row.Conversation(it) }
        val out = ArrayList<Row>()
        var group: String? = null
        for (e in filtered) {
            val label = ChatText.railGroup(Instant.ofEpochMilli(e.updatedAtMs).atZone(zone).toLocalDate(), today)
            if (label != group) { out += Row.Header(label); group = label }
            out += Row.Conversation(e)
        }
        return out
    }
}

/**
 * Chat paradigm's conversation list (docs/ui.md §3.4): 今天 / 昨天 / 近 7 天 / 更早, ⌕ filter and ✎ new
 * in the header, running (pulsing primary) and pending (tertiary) dots, the current one as a pill,
 * long press → 重命名 · 归档, "已归档 (n) ›" at the bottom.
 */
class ChatRailController(private val context: RailContext, private val hub: AgentHub) : RailController {
    private var filtering by mutableStateOf(false)
    private var query by mutableStateOf("")
    private var showArchived by mutableStateOf(false)
    private var renaming by mutableStateOf<ConversationEntry?>(null)

    override val actions: List<ToolAction>
        get() = listOf(
            ToolAction("filter", Sym.Search, "筛选对话", checked = filtering) { filtering = !filtering; if (!filtering) query = "" },
            ToolAction("new", Sym.EditSquare, "新对话") { context.commands.newConversation() },
        )

    override val resourceMenu: List<MenuEntry>
        get() = listOf(
            MenuEntry.Toggle("archived", "显示已归档", showArchived) { showArchived = it },
        )

    @Composable
    override fun Content(modifier: Modifier) {
        val entries by hub.conversations.collectAsState()
        val state by hub.state.collectAsState()
        val active by context.activeConversation.collectAsState()
        val zone = ZoneId.systemDefault()
        val today = LocalDate.now(zone)
        val rows = remember(entries, query, showArchived, today) { RailModel.rows(entries, query, showArchived, today, zone) }
        val archivedCount = entries.count { it.archived }
        Column(modifier.fillMaxSize()) {
            if (filtering) SearchField(query, { query = it }, Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp).height(32.dp), placeholder = "搜索对话")
            if (showArchived) {
                Row(Modifier.fillMaxWidth().height(36.dp).clickable { showArchived = false }.padding(horizontal = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                    SymbolIcon(Sym.ArrowBack, "返回", tint = WorkflowTheme.colors.onSurfaceVariant)
                    Spacer(Modifier.width(8.dp))
                    Text("已归档", style = WorkflowTheme.text.titleSm, color = WorkflowTheme.colors.onSurface)
                }
            }
            LazyColumn(Modifier.weight(1f).fillMaxWidth().padding(vertical = 4.dp)) {
                items(rows, key = { row -> when (row) { is RailModel.Row.Header -> "h:" + row.label; is RailModel.Row.Conversation -> row.entry.id } }) { row ->
                    when (row) {
                        is RailModel.Row.Header -> Text(
                            row.label, Modifier.fillMaxWidth().height(28.dp).padding(horizontal = 12.dp).padding(top = 8.dp),
                            style = WorkflowTheme.text.caption, color = WorkflowTheme.colors.onSurfaceVariant,
                        )
                        is RailModel.Row.Conversation -> {
                            val e = row.entry
                            val key = e.backendThreadId?.let { ThreadKey(e.backend, it) }
                            val thread = key?.let { state.thread(it) }
                            val running = thread?.turns?.any { it.status == TurnStatus.RUNNING } == true
                            val pending = key != null && state.requests.values.any { it.status.isOpen && it.key.backend == e.backend && it.threadId == key.id }
                            var menu by remember { mutableStateOf(false) }
                            Box {
                                if (showArchived) {
                                    Row(verticalAlignment = Alignment.CenterVertically) {
                                        Box(Modifier.weight(1f)) {
                                            ConversationRow(e, selected = e.id == active, running = false, pending = false, onClick = { open(e) }, onLongClick = null)
                                        }
                                        TextButton(onClick = { context.scope.launch { hub.archive(e.id, false) } }) { Text("恢复") }
                                    }
                                } else {
                                    ConversationRow(e, selected = e.id == active, running = running, pending = pending,
                                        onClick = { open(e) }, onLongClick = { menu = true })
                                }
                                CompositeMenu(
                                    expanded = menu, onDismissRequest = { menu = false }, anchorPosition = MenuAnchorPosition.Below,
                                    groups = listOf(MenuGroup("conversation", listOf(
                                        MenuEntry.Action("rename", "重命名", Sym.Edit) { renaming = e },
                                        MenuEntry.Action("archive", "归档", Sym.Archive) {
                                            context.scope.launch { hub.archive(e.id, true) }
                                            context.commands.snackbar("已归档", "撤销") { context.scope.launch { hub.archive(e.id, false) } }
                                        },
                                    ))),
                                )
                            }
                        }
                    }
                }
            }
            if (!showArchived && archivedCount > 0) {
                Row(
                    Modifier.fillMaxWidth().heightIn(min = WorkflowTheme.dimens.treeRow).clickable { showArchived = true }.padding(horizontal = 12.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text("已归档 ($archivedCount)", Modifier.weight(1f), style = WorkflowTheme.text.label, color = WorkflowTheme.colors.onSurfaceVariant)
                    SymbolIcon(Sym.ChevronRight, null, size = 18.dp, tint = WorkflowTheme.colors.onSurfaceVariant)
                }
            }
        }
        renaming?.let { e ->
            RenameDialog(e.title ?: ChatText.untitled(e.preview), onDismiss = { renaming = null }) { title ->
                renaming = null
                context.scope.launch { hub.rename(e.id, title) }
            }
        }
    }

    private fun open(entry: ConversationEntry) {
        context.layout(LayoutOp.showConversation(entry.id))
    }
}
