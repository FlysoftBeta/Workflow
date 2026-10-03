package top.flysoftbeta.workflow.feature.launcher

import top.flysoftbeta.workflow.platform.apps.InstalledApps
import top.flysoftbeta.workflow.platform.apps.InstalledApp

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
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
import androidx.compose.material3.Checkbox
import androidx.compose.material3.Surface
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
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import top.flysoftbeta.workflow.ui.design.RegionLoading
import top.flysoftbeta.workflow.ui.design.SearchField
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/**
 * "添加应用" (ui.md §3.1): 560dp wide and at most 80% high from 600dp, full screen below; a 40dp search
 * field and "完成" on top; 44dp rows with a 32dp icon, the name and a checkbox that applies at once.
 * No package names; equal names get the application name as caption.
 */
@Composable
fun AddAppsSheet(catalog: InstalledApps, added: Set<String>, onToggle: (InstalledApp, Boolean) -> Unit, onDone: () -> Unit) {
    Dialog(onDismissRequest = onDone, properties = DialogProperties(usePlatformDefaultWidth = false)) {
        BoxWithConstraints(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            val wide = maxWidth >= 600.dp
            Surface(
                if (wide) Modifier.width(560.dp).heightIn(max = maxHeight * 0.8f) else Modifier.fillMaxSize(),
                shape = if (wide) WorkflowShapes.lg else androidx.compose.ui.graphics.RectangleShape,
                color = WorkflowTheme.colors.surfaceContainerLow,
            ) { AddAppsContent(catalog, added, onToggle, onDone) }
        }
    }
}

@Composable
private fun AddAppsContent(catalog: InstalledApps, added: Set<String>, onToggle: (InstalledApp, Boolean) -> Unit, onDone: () -> Unit) {
    val apps by catalog.apps.collectAsState()
    var query by remember { mutableStateOf("") }
    val colors = WorkflowTheme.colors
    val text = WorkflowTheme.text
    val shown = apps.filter { query.isBlank() || it.label.contains(query.trim(), ignoreCase = true) }
    val duplicated = apps.groupingBy { it.label }.eachCount().filterValues { it > 1 }.keys
    Column(Modifier.fillMaxWidth()) {
        Row(Modifier.fillMaxWidth().padding(start = 12.dp, end = 4.dp, top = 8.dp, bottom = 4.dp), verticalAlignment = Alignment.CenterVertically) {
            SearchField(query, { query = it }, Modifier.weight(1f).height(40.dp), placeholder = "搜索应用")
            TextButton(onClick = onDone) { Text("完成") }
        }
        Box(Modifier.fillMaxWidth().heightIn(min = 120.dp)) {
            RegionLoading(apps.isEmpty(), Modifier.fillMaxWidth().height(120.dp))
            LazyColumn(Modifier.fillMaxWidth()) {
                items(shown, key = { it.ref.id }) { info ->
                    val checked = info.ref.id in added || info.ref.packageName in added
                    Row(
                        Modifier
                            .fillMaxWidth()
                            .heightIn(min = 44.dp)
                            .clickable { onToggle(info, !checked) }
                            .semantics { role = Role.Checkbox }
                            .padding(start = 16.dp, end = 8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        AppIcon(info, 32.dp)
                        Spacer(Modifier.width(12.dp))
                        Column(Modifier.weight(1f)) {
                            Text(info.label, style = text.body, color = colors.onSurface, maxLines = 1, overflow = TextOverflow.Ellipsis)
                            if (info.label in duplicated && info.appLabel != info.label) {
                                Text(info.appLabel, style = text.caption, color = colors.onSurfaceVariant, maxLines = 1)
                            }
                        }
                        Checkbox(checked = checked, onCheckedChange = { onToggle(info, it) })
                    }
                }
            }
        }
    }
}
