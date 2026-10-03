package top.flysoftbeta.workflow.feature.workbench

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.movableContentOf
import androidx.compose.ui.Modifier
import top.flysoftbeta.workflow.app.Shell
import top.flysoftbeta.workflow.app.panel.ExplorerVariant
import top.flysoftbeta.workflow.app.panel.PanelController
import top.flysoftbeta.workflow.app.panel.PanelFrame
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.Panel
import top.flysoftbeta.workflow.core.layout.PanelId
import top.flysoftbeta.workflow.core.layout.Workbench
import top.flysoftbeta.workflow.core.session.Session
import top.flysoftbeta.workflow.ui.design.dnd.DragDropState

/**
 * Everything the Workbench composables of one session share: the shell, the session runtime, the DnD
 * state and the movable contents (a panel's content moves with it between stacks, regions and
 * paradigms instead of being recreated, ui.md §3.4).
 */
@Stable
internal class WorkbenchEnv(
    val shell: Shell,
    val runtime: SessionRuntime,
    val dnd: DragDropState,
) {
    private val contents = HashMap<PanelId, Pair<String, @Composable (PanelFrame, Modifier) -> Unit>>()

    val explorerContent: @Composable (ExplorerVariant, Modifier) -> Unit = movableContentOf { variant: ExplorerVariant, modifier: Modifier ->
        runtime.explorerController().Content(variant, modifier)
    }

    val railContent: @Composable (Modifier) -> Unit = movableContentOf { modifier: Modifier ->
        runtime.railController().Content(modifier)
    }

    fun controller(panel: Panel): PanelController = runtime.controller(panel)

    fun content(panel: Panel): @Composable (PanelFrame, Modifier) -> Unit {
        contents[panel.id]?.let { (key, content) -> if (key == panel.target.key) return content }
        val controller = runtime.controller(panel)
        val content = movableContentOf { frame: PanelFrame, modifier: Modifier -> controller.Content(frame, modifier) }
        contents[panel.id] = panel.target.key to content
        return content
    }

    fun prune(workbench: Workbench) {
        contents.keys.retainAll { id -> workbench.panels[id]?.target?.key == contents[id]?.first }
        runtime.prune(workbench)
    }

    fun layout(op: LayoutOp) = runtime.layout(op)
}

/** A snapshot of what one frame renders: the session and its workbench plus window facts. */
internal data class WorkbenchFrame(
    val session: Session,
    val workbench: Workbench,
    val imeVisible: Boolean,
    val attention: Boolean,
)
