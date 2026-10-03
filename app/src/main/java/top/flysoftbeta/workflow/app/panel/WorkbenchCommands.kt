package top.flysoftbeta.workflow.app.panel

import androidx.compose.runtime.Immutable
import top.flysoftbeta.workflow.core.layout.Placement
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.TextCursor

/**
 * Cross-feature actions, implemented by the shell for the active session (architecture.md §1 rule 3:
 * features never call each other; they go through store commands or this contract).
 */
interface WorkbenchCommands {
    /** Opens [target], or focuses it when already open in the session (resources open once). */
    fun open(target: PanelTarget, placement: Placement = Placement.Auto)

    /**
     * Opens a workspace file (image preview for image types) and places the cursor when given
     * (terminal `path:line:col` links, chat file links). In Chat the file opens in the side region,
     * which is expanded when collapsed.
     */
    fun openFile(path: String, cursor: TextCursor? = null)

    /**
     * "附加到对话": delivers [paths] as a FilesDragPayload to the conversation in view (Files' aux or
     * Chat's main column), opening a new conversation in aux when none is open.
     */
    fun attachToConversation(paths: List<String>)

    /**
     * "在终端中粘贴路径": delivers [paths] as a FilesDragPayload to the focused (else most recent)
     * terminal, opening a terminal when none is open. The terminal shell-escapes them.
     */
    fun pasteIntoTerminal(paths: List<String>)

    /** "新建终端" / "在终端中打开": a new terminal in the bottom stack, started in [directory]. */
    fun newTerminal(directory: String? = null)

    /** "新对话": a new conversation shown in aux (Files) or the main column (Chat). */
    fun newConversation()

    /** "在文件树中显示": expands the ancestors, selects [path] and makes the explorer visible. */
    fun revealInExplorer(path: String)

    /** Transient confirmation at the bottom centre ("已复制", "已归档 · 撤销"). */
    fun snackbar(message: String, actionLabel: String? = null, onAction: (() -> Unit)? = null)

    /**
     * A shell-rendered decision dialog (docs/ui.md §4.9 style: no preselected button, outside tap does
     * not dismiss). Returns the chosen [DecisionOption.key], or null when cancelled.
     */
    suspend fun decide(request: DecisionRequest): String?
}

enum class DecisionStyle { Filled, Tonal, Text, Destructive }

@Immutable
data class DecisionOption(val key: String, val label: String, val style: DecisionStyle = DecisionStyle.Text)

/**
 * [options] are laid out right-aligned in the given order after a left-aligned [cancelLabel].
 * [items] lists affected resources (at most 5 shown, the rest as "等 n 个").
 */
@Immutable
data class DecisionRequest(
    val title: String,
    val message: String? = null,
    val items: List<String> = emptyList(),
    val options: List<DecisionOption>,
    val cancelLabel: String = "取消",
)
