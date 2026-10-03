package top.flysoftbeta.workflow.feature.editor

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import io.github.rosemoe.sora.widget.CodeEditor
import io.github.rosemoe.sora.widget.EditorSearcher
import top.flysoftbeta.workflow.ui.design.WfIconButton
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** Find/replace state of one editor (kept with its controller). */
@Stable
internal class FindState {
    var query by mutableStateOf("")
    var replacement by mutableStateOf("")
    var replaceOpen by mutableStateOf(false)
    var caseSensitive by mutableStateOf(false)
    var regex by mutableStateOf(false)
    var count by mutableIntStateOf(0)
    var current by mutableIntStateOf(-1)
    var invalid by mutableStateOf(false)
    /** A find/replace field has keyboard focus (the editor's extra-keys row stays hidden then). */
    var fieldFocused by mutableStateOf(false)

    fun update(searcher: EditorSearcher) {
        count = runCatching { searcher.matchedPositionCount }.getOrDefault(0).coerceAtLeast(0)
        current = runCatching { searcher.currentMatchedPositionIndex }.getOrDefault(-1)
    }

    fun search(editor: CodeEditor) {
        invalid = false
        if (query.isEmpty()) { editor.searcher.stopSearch(); count = 0; current = -1; return }
        if (regex && runCatching { Regex(query) }.isFailure) { invalid = true; return }
        runCatching { editor.searcher.search(query, EditorSearcher.SearchOptions(!caseSensitive, regex)) }
            .onFailure { invalid = true }
    }
}

/** The 40dp find bar (docs/ui.md §4.3): 查找 · ▸替换 · ↑↓ · n/m · Aa · .* · ×. */
@Composable
internal fun FindBar(state: FindState, editor: CodeEditor, onClose: () -> Unit) {
    val colors = WorkflowTheme.colors
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) { runCatching { focus.requestFocus() } }
    LaunchedEffect(state.query, state.caseSensitive, state.regex) { state.search(editor) }
    BackHandler(onBack = onClose)
    Column(Modifier.fillMaxWidth().background(colors.surfaceContainerLow)) {
        Row(Modifier.fillMaxWidth().heightIn(min = 40.dp).padding(horizontal = 4.dp), verticalAlignment = Alignment.CenterVertically) {
            WfIconButton(if (state.replaceOpen) Sym.KeyboardArrowDown else Sym.ChevronRight, "替换", { state.replaceOpen = !state.replaceOpen }, iconSize = 18.dp)
            Field(state.query, { state.query = it }, "查找", Modifier.weight(1f).focusRequester(focus).onFocusChanged { state.fieldFocused = it.isFocused }, invalid = state.invalid) {
                if (state.count > 0) editor.searcher.gotoNext()
            }
            Text(
                when {
                    state.query.isEmpty() -> ""
                    state.count == 0 -> "无结果"
                    state.current < 0 -> "${state.count} 项"
                    else -> "${state.current + 1}/${state.count}"
                },
                Modifier.widthIn(min = 48.dp).padding(horizontal = 6.dp),
                style = WorkflowTheme.text.caption, color = colors.onSurfaceVariant, maxLines = 1,
            )
            WfIconButton(Sym.KeyboardArrowUp, "上一个", { if (state.count > 0) { editor.searcher.gotoPrevious(); state.update(editor.searcher) } }, iconSize = 18.dp)
            WfIconButton(Sym.KeyboardArrowDown, "下一个", { if (state.count > 0) { editor.searcher.gotoNext(); state.update(editor.searcher) } }, iconSize = 18.dp)
            ToggleText("Aa", "区分大小写", state.caseSensitive) { state.caseSensitive = !state.caseSensitive }
            ToggleText(".*", "正则表达式", state.regex) { state.regex = !state.regex }
            WfIconButton(Sym.Close, "关闭查找", onClose, iconSize = 18.dp)
        }
        if (state.replaceOpen) {
            Row(Modifier.fillMaxWidth().heightIn(min = 40.dp).padding(start = 40.dp, end = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                Field(state.replacement, { state.replacement = it }, "替换", Modifier.weight(1f).onFocusChanged { state.fieldFocused = it.isFocused }, invalid = false) {
                    if (state.count > 0) runCatching { editor.searcher.replaceCurrentMatch(state.replacement) }
                }
                TextButton(onClick = { if (state.count > 0) runCatching { editor.searcher.replaceCurrentMatch(state.replacement) } }) {
                    Text("替换", style = WorkflowTheme.text.label)
                }
                TextButton(onClick = { if (state.count > 0) runCatching { editor.searcher.replaceAll(state.replacement) } }) {
                    Text("全部替换", style = WorkflowTheme.text.label)
                }
            }
        }
    }
}

@Composable
private fun Field(value: String, onChange: (String) -> Unit, placeholder: String, modifier: Modifier, invalid: Boolean, onDone: () -> Unit) {
    val colors = WorkflowTheme.colors
    BasicTextField(
        value = value,
        onValueChange = onChange,
        modifier = modifier,
        singleLine = true,
        textStyle = WorkflowTheme.text.body.copy(color = colors.onSurface),
        cursorBrush = SolidColor(colors.primary),
        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search, autoCorrectEnabled = false),
        keyboardActions = KeyboardActions(onSearch = { onDone() }),
        decorationBox = { inner ->
            Box(
                Modifier.heightIn(min = 32.dp).clip(WorkflowShapes.sm)
                    .background(if (invalid) colors.errorContainer else colors.surfaceContainerHighest)
                    .padding(horizontal = 8.dp),
                contentAlignment = Alignment.CenterStart,
            ) {
                if (value.isEmpty()) Text(placeholder, style = WorkflowTheme.text.body, color = colors.onSurfaceVariant)
                inner()
            }
        },
    )
}

@Composable
private fun ToggleText(label: String, description: String, checked: Boolean, onClick: () -> Unit) {
    val colors = WorkflowTheme.colors
    TextButton(onClick = onClick, modifier = Modifier.heightIn(min = 32.dp).semantics {
        contentDescription = description
        stateDescription = if (checked) "开" else "关"
    }) {
        Text(label, style = WorkflowTheme.text.mono, color = if (checked) colors.primary else colors.onSurfaceVariant)
    }
}
