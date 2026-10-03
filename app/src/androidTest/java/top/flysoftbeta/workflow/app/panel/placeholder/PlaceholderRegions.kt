package top.flysoftbeta.workflow.app.panel.placeholder

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import top.flysoftbeta.workflow.app.panel.ExplorerContext
import top.flysoftbeta.workflow.app.panel.ExplorerController
import top.flysoftbeta.workflow.app.panel.ExplorerProvider
import top.flysoftbeta.workflow.app.panel.ExplorerVariant
import top.flysoftbeta.workflow.app.panel.RailContext
import top.flysoftbeta.workflow.app.panel.RailController
import top.flysoftbeta.workflow.app.panel.RailProvider
import top.flysoftbeta.workflow.core.io.FileEntry
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.MenuGroup
import top.flysoftbeta.workflow.ui.design.SymbolIcon
import top.flysoftbeta.workflow.ui.design.ToolAction
import top.flysoftbeta.workflow.ui.design.dnd.FilesDragPayload
import top.flysoftbeta.workflow.ui.design.dnd.LocalDragDropState
import top.flysoftbeta.workflow.ui.design.dnd.dragSource
import top.flysoftbeta.workflow.ui.design.icons.FileTypeIcons
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** A minimal conversation rail until the chat workstream lands: the conversations open in this session. */
class PlaceholderRailProvider : RailProvider {
    override fun create(context: RailContext): RailController = PlaceholderRail(context)
}

private class PlaceholderRail(private val context: RailContext) : RailController {
    override val actions: List<ToolAction> get() = listOf(
        ToolAction("new", Sym.EditSquare, "新对话") { context.commands.newConversation() },
    )

    @Composable
    override fun Content(modifier: Modifier) {
        val active by context.activeConversation.collectAsState()
        val conversations by remember(context) {
            context.store.state.map { state ->
                state.session(context.sessionId)?.workbench?.panels?.values?.mapNotNull { (it.target as? PanelTarget.Conversation)?.conversationId }.orEmpty()
            }
        }.collectAsState(emptyList())
        LazyColumn(modifier.fillMaxSize().padding(vertical = 4.dp)) {
            items(conversations, key = { it }) { id ->
                Row(
                    Modifier
                        .fillMaxWidth()
                        .heightIn(min = WorkflowTheme.dimens.treeRow)
                        .padding(horizontal = 4.dp)
                        .background(if (id == active) WorkflowTheme.colors.secondaryContainer else androidx.compose.ui.graphics.Color.Transparent, WorkflowShapes.sm)
                        .clickable { context.layout(LayoutOp.showConversation(id)) }
                        .padding(horizontal = 8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text("对话 ${id.take(4)}", style = WorkflowTheme.text.label, color = WorkflowTheme.colors.onSurface, maxLines = 1)
                }
            }
        }
    }
}
