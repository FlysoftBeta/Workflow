package top.flysoftbeta.workflow.app.panel

import android.content.Context
import top.flysoftbeta.workflow.app.panel.placeholder.PlaceholderPanelProvider
import top.flysoftbeta.workflow.app.panel.placeholder.PlaceholderRailProvider
import top.flysoftbeta.workflow.core.layout.PanelKind
import top.flysoftbeta.workflow.feature.editor.EditorPanelProvider
import top.flysoftbeta.workflow.feature.files.FilesExplorerProvider

/** Reference-store UI regression tests explicitly exclude real service/connection integration. */
object ReferencePanelRegistry {
    fun create(context: Context): PanelRegistry {
        val editor = EditorPanelProvider()
        return PanelRegistry(mapOf(PanelKind.FILE to editor, PanelKind.IMAGE to editor, PanelKind.DIFF to editor),
            FilesExplorerProvider(context), PlaceholderRailProvider(), PlaceholderPanelProvider())
    }
}
