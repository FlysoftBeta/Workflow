package top.flysoftbeta.workflow.feature.editor

import top.flysoftbeta.workflow.app.panel.PanelContext
import top.flysoftbeta.workflow.app.panel.PanelController
import top.flysoftbeta.workflow.app.panel.PanelProvider
import top.flysoftbeta.workflow.core.layout.Panel
import top.flysoftbeta.workflow.core.layout.PanelTarget

/** Text files (sora), image previews and conflict diffs (PanelKind FILE / IMAGE / DIFF). */
class EditorPanelProvider : PanelProvider {
    override fun create(panel: Panel, context: PanelContext): PanelController = when (val target = panel.target) {
        is PanelTarget.Image -> ImageController(target.path, context)
        is PanelTarget.Diff -> DiffController(target.path, context)
        is PanelTarget.File -> TextEditorController(target.path, context)
        else -> error("Not an editor target: $target")
    }
}
