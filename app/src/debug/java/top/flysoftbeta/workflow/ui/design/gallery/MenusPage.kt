package top.flysoftbeta.workflow.ui.design.gallery

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.ui.design.CompositeMenu
import top.flysoftbeta.workflow.ui.design.ItemListMenu
import top.flysoftbeta.workflow.ui.design.ListMenuItem
import top.flysoftbeta.workflow.ui.design.MenuGroupStyle
import top.flysoftbeta.workflow.ui.design.ModelChip
import top.flysoftbeta.workflow.ui.design.ModelEffortPopover
import top.flysoftbeta.workflow.ui.design.ModelEffortSelection
import top.flysoftbeta.workflow.ui.design.WfIconButton
import top.flysoftbeta.workflow.ui.design.icons.FileTypeIcons
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** Menus shown open: composite More menu (segmented + divider fallback), tab overflow list, model popover. */
@Composable
fun MenusPage() {
    var segmented by remember { mutableStateOf(true) }
    var dividers by remember { mutableStateOf(true) }
    var list by remember { mutableStateOf(true) }
    var model by remember { mutableStateOf(true) }
    var selection by remember { mutableStateOf(ModelEffortSelection("gpt-5.5", "high")) }
    Box(Modifier.fillMaxSize().padding(16.dp)) {
        Row(Modifier.align(Alignment.TopStart)) {
            Column {
                Text("More (Expressive groups)", style = WorkflowTheme.text.caption, color = WorkflowTheme.colors.onSurfaceVariant)
                Box {
                    WfIconButton(Sym.MoreHoriz, "更多", { segmented = true })
                    CompositeMenu(segmented, { segmented = false }, editorMoreGroups())
                }
            }
        }
        Column(Modifier.align(Alignment.TopCenter)) {
            Text("More (divider fallback)", style = WorkflowTheme.text.caption, color = WorkflowTheme.colors.onSurfaceVariant)
            Box {
                WfIconButton(Sym.MoreHoriz, "更多", { dividers = true })
                CompositeMenu(dividers, { dividers = false }, editorMoreGroups(), style = MenuGroupStyle.Dividers)
            }
        }
        Column(Modifier.align(Alignment.TopEnd)) {
            Text("⌄n tab list (search > 8)", style = WorkflowTheme.text.caption, color = WorkflowTheme.colors.onSurfaceVariant)
            Box {
                WfIconButton(Sym.KeyboardArrowDown, "全部标签", { list = true })
                ItemListMenu(
                    expanded = list, onDismissRequest = { list = false },
                    items = listOf(
                        "prompt.md", "Theme.kt", "libs.versions.toml", "build.gradle.kts", "README.md", "container.json",
                        "icon.png", "StackTabBar.kt", "CompositeMenu.kt", "SplitPane.kt",
                    ).mapIndexed { i, name -> ListMenuItem(name, name, FileTypeIcons.forName(name), dot = i == 0 || i == 5) },
                    selectedKey = "Theme.kt", onSelect = { list = false },
                )
            }
        }
        Column(Modifier.align(Alignment.BottomEnd)) {
            Box {
                ModelChip("GPT-5.5 高", { model = true })
                ModelEffortPopover(
                    expanded = model, onDismissRequest = { model = false },
                    models = demoModels, selection = selection, onSelectionChange = { selection = it },
                    onOpenModelList = {}, speedActive = false,
                )
            }
        }
    }
}
