package top.flysoftbeta.workflow.app

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.DialogProperties
import top.flysoftbeta.workflow.app.panel.DecisionOption
import top.flysoftbeta.workflow.app.panel.DecisionStyle
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/**
 * The shell's decision dialog (ui.md §4.9 archive dialog rules, reused by `WorkbenchCommands.decide`):
 * cancel on the left, options right-aligned in order, nothing preselected, an outside tap does not
 * dismiss (Back cancels). [items] lists affected resources, at most 5 plus "等 n 个".
 */
@Composable
fun DecisionDialog(
    title: String,
    message: String?,
    items: List<String>,
    options: List<DecisionOption>,
    cancelLabel: String,
    onResult: (String?) -> Unit,
    problem: String? = null,
) {
    val colors = WorkflowTheme.colors
    val text = WorkflowTheme.text
    AlertDialog(
        onDismissRequest = { onResult(null) },
        properties = DialogProperties(dismissOnClickOutside = false),
        title = { Text(title, style = text.titleMd, maxLines = 2, overflow = TextOverflow.Ellipsis) },
        text = {
            Column(Modifier.widthIn(max = 480.dp)) {
                if (message != null) Text(message, style = text.body, color = colors.onSurfaceVariant)
                if (items.isNotEmpty()) {
                    if (message != null) Spacer(Modifier.padding(top = 8.dp))
                    items.take(5).forEach { Text(it, style = text.body, color = colors.onSurface, maxLines = 1, overflow = TextOverflow.Ellipsis) }
                    if (items.size > 5) Text("等 ${items.size - 5} 个", style = text.caption, color = colors.onSurfaceVariant)
                }
                if (problem != null) {
                    Spacer(Modifier.padding(top = 8.dp))
                    Text(problem, style = text.caption, color = colors.error)
                }
            }
        },
        confirmButton = {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                TextButton(onClick = { onResult(null) }) { Text(cancelLabel) }
                Spacer(Modifier.weight(1f))
                options.forEach { option ->
                    when (option.style) {
                        DecisionStyle.Filled -> Button(onClick = { onResult(option.key) }) { Text(option.label) }
                        DecisionStyle.Tonal -> FilledTonalButton(onClick = { onResult(option.key) }) { Text(option.label) }
                        DecisionStyle.Text -> TextButton(onClick = { onResult(option.key) }) { Text(option.label) }
                        DecisionStyle.Destructive -> TextButton(
                            onClick = { onResult(option.key) },
                            colors = ButtonDefaults.textButtonColors(contentColor = colors.error),
                        ) { Text(option.label) }
                    }
                }
            }
        },
    )
}

/** "命名 Session" / "重命名 Session": naming a temporary session makes it persistent (product.md §3). */
@Composable
fun RenameSessionDialog(initial: String?, onDone: (String?) -> Unit) {
    var value by remember { mutableStateOf(TextFieldValue(initial.orEmpty(), TextRange(0, initial.orEmpty().length))) }
    val focus = remember { FocusRequester() }
    val valid = value.text.isNotBlank()
    AlertDialog(
        onDismissRequest = { onDone(null) },
        title = { Text(if (initial == null) "命名 Session" else "重命名 Session", style = WorkflowTheme.text.titleMd) },
        text = {
            OutlinedTextField(
                value = value,
                onValueChange = { value = it },
                singleLine = true,
                textStyle = WorkflowTheme.text.body,
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                keyboardActions = KeyboardActions(onDone = { if (valid) onDone(value.text.trim()) }),
                modifier = Modifier.fillMaxWidth().focusRequester(focus),
            )
        },
        confirmButton = { TextButton(onClick = { onDone(value.text.trim()) }, enabled = valid) { Text("确定") } },
        dismissButton = { TextButton(onClick = { onDone(null) }) { Text("取消") } },
    )
    LaunchedEffect(Unit) { focus.requestFocus() }
}
