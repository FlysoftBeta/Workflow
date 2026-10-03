package top.flysoftbeta.workflow.feature.editor

import android.content.Context
import io.github.rosemoe.sora.widget.CodeEditor

/** A viewport resize keeps the reading position when the caret has been scrolled off screen. */
internal class WorkspaceCodeEditor(context: Context) : CodeEditor(context) {
    override fun onSizeChanged(width: Int, height: Int, oldWidth: Int, oldHeight: Int) {
        val adjust = props.adjustToSelectionOnResize
        if (adjust && oldHeight > height) {
            val bottom = layout.getCharLayoutOffset(cursor.rightLine, cursor.rightColumn)[0]
            val caretWasVisible = bottom > offsetY && bottom - rowHeight < offsetY + oldHeight
            // Sora otherwise reveals the caret on every height reduction, including delayed IME
            // insets and workbench resizing, undoing a scroll while the user is reading elsewhere.
            props.adjustToSelectionOnResize = caretWasVisible
        }
        try {
            super.onSizeChanged(width, height, oldWidth, oldHeight)
        } finally {
            props.adjustToSelectionOnResize = adjust
        }
    }
}
