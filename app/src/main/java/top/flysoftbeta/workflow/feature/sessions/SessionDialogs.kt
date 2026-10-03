package top.flysoftbeta.workflow.feature.sessions

import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import top.flysoftbeta.workflow.app.DecisionDialog
import top.flysoftbeta.workflow.app.RenameSessionDialog
import top.flysoftbeta.workflow.app.Shell
import top.flysoftbeta.workflow.app.panel.DecisionOption
import top.flysoftbeta.workflow.app.panel.DecisionStyle
import top.flysoftbeta.workflow.core.session.ArchiveDecision

/**
 * Session dialogs driven by [Shell]: naming (a temporary session becomes persistent) and the manual
 * archive decision (ui.md §4.9: 归档「名称」, affected resources, [取消] ┆ [丢弃] [保留草稿] [全部保存]).
 */
@Composable
fun SessionDialogs(shell: Shell) {
    val state by shell.store.state.collectAsState()
    shell.renaming?.let { id ->
        val session = state.session(id)
        if (session == null) {
            shell.renaming = null
        } else {
            RenameSessionDialog(session.name) { name ->
                shell.renaming = null
                if (name != null) shell.rename(id, name)
            }
        }
    }
    shell.archiving?.let { request ->
        DecisionDialog(
            title = "归档「${request.title}」",
            message = null,
            items = request.resources,
            options = listOf(
                DecisionOption(ArchiveDecision.DISCARD.name, "丢弃", DecisionStyle.Destructive),
                DecisionOption(ArchiveDecision.KEEP_DRAFTS.name, "保留草稿", DecisionStyle.Tonal),
                DecisionOption(ArchiveDecision.SAVE_ALL.name, "全部保存", DecisionStyle.Filled),
            ),
            cancelLabel = "取消",
            problem = request.problem,
            onResult = { key ->
                if (key == null) shell.cancelArchive() else shell.archive(request.sessionId, ArchiveDecision.valueOf(key))
            },
        )
    }
}
