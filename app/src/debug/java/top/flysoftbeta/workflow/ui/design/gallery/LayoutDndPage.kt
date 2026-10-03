package top.flysoftbeta.workflow.ui.design.gallery

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshots.SnapshotStateList
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import top.flysoftbeta.workflow.ui.design.RegionCard
import top.flysoftbeta.workflow.ui.design.SplitOrientation
import top.flysoftbeta.workflow.ui.design.SplitPane
import top.flysoftbeta.workflow.ui.design.StackTabBar
import top.flysoftbeta.workflow.ui.design.StackTabDragDrop
import top.flysoftbeta.workflow.ui.design.TabItem
import top.flysoftbeta.workflow.ui.design.dnd.AreaStyle
import top.flysoftbeta.workflow.ui.design.dnd.DndGeometry
import top.flysoftbeta.workflow.ui.design.dnd.DragDropHost
import top.flysoftbeta.workflow.ui.design.dnd.DragDropState
import top.flysoftbeta.workflow.ui.design.dnd.DragPayload
import top.flysoftbeta.workflow.ui.design.dnd.DropTargetKind
import top.flysoftbeta.workflow.ui.design.dnd.ExternalDragPayload
import top.flysoftbeta.workflow.ui.design.dnd.FilesDragPayload
import top.flysoftbeta.workflow.ui.design.dnd.PanelDragPayload
import top.flysoftbeta.workflow.ui.design.dnd.dragSource
import top.flysoftbeta.workflow.ui.design.dnd.dropTarget
import top.flysoftbeta.workflow.ui.design.icons.FileTypeIcons
import top.flysoftbeta.workflow.ui.design.rememberSplitPaneState
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

private fun demoTab(name: String) = TabItem(name, name, FileTypeIcons.forName(name))

/**
 * Live drag-and-drop playground: file rows and tabs are sources; tab rows sort, stack contents take
 * placement zones, the composer takes attachments, the terminal pastes paths. Each drop logs its single
 * commit. [synthetic] freezes one feedback state for screenshots.
 */
@Composable
fun LayoutDndPage(synthetic: GalleryDnd?) {
    val state = remember { DragDropState() }
    val stacks = remember {
        mapOf(
            "A" to mutableStateListOf(demoTab("prompt.md"), demoTab("Theme.kt"), demoTab("README.md"), demoTab("ui.md")),
            "B" to mutableStateListOf(demoTab("container.json"), demoTab("icon.png")),
        )
    }
    val active = remember { mutableStateOf(mapOf("A" to "Theme.kt", "B" to "container.json")) }
    var log by remember { mutableStateOf("长按文件或标签 300ms 后拖动。每次松手只提交一次。") }
    val split = rememberSplitPaneState(0.55f)

    fun moveTab(payload: PanelDragPayload, toStack: String, insertion: Int?) {
        val from = stacks.getValue(payload.sourceStackId)
        val to = stacks.getValue(toStack)
        val item = from.firstOrNull { it.key == payload.panelId } ?: return
        if (payload.sourceStackId == toStack && insertion != null) {
            if (DndGeometry.isNoOpMove(payload.sourceIndex, insertion)) { log = "commit: 无变化"; return }
            val target = DndGeometry.targetIndexAfterRemoval(payload.sourceIndex, insertion)
            from.removeAt(payload.sourceIndex); from.add(target, item)
            log = "commit: reorder ${item.title} → $toStack[$target]"
        } else {
            from.remove(item)
            to.add((insertion ?: to.size).coerceIn(0, to.size), item)
            active.value = active.value + (toStack to item.key)
            log = "commit: move ${item.title} → stack $toStack"
        }
    }

    DragDropHost(state, Modifier.fillMaxSize()) {
        Row(Modifier.fillMaxSize().padding(4.dp), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            // File list (sources)
            Column(Modifier.width(220.dp).fillMaxHeight()) {
                Text(log, Modifier.padding(8.dp).height(56.dp), style = WorkflowTheme.text.caption, color = WorkflowTheme.colors.onSurfaceVariant)
                RegionCard(Modifier.fillMaxSize()) {
                    Column(Modifier.padding(vertical = 4.dp)) {
                        listOf("docs/ui.md", "docs/product.md", "app/Theme.kt", "icon.png", "container.json").forEach { path ->
                            Box(
                                Modifier.dragSource(state, key = "file:$path", onLongPressWithoutMove = { log = "menu: $path" }) {
                                    FilesDragPayload(listOf(path), icon = FileTypeIcons.forName(path))
                                },
                            ) { DemoTreeRow(path.substringAfterLast('/'), 0, FileTypeIcons.forName(path)) }
                        }
                    }
                }
            }
            Column(Modifier.weight(1f).fillMaxHeight()) {
                SplitPane(
                    state = split, orientation = SplitOrientation.Horizontal, firstMin = 240.dp, secondMin = 240.dp,
                    modifier = Modifier.weight(1f),
                    first = { DemoStack("A", stacks, active, state, ::moveTab) { log = it } },
                    second = { DemoStack("B", stacks, active, state, ::moveTab) { log = it } },
                )
                Row(Modifier.fillMaxWidth().height(120.dp).padding(top = 4.dp), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    RegionCard(
                        Modifier.weight(1f).fillMaxHeight().dropTarget(
                            state, "composer", DropTargetKind.Area(AreaStyle.Outline, "添加为附件"),
                            accepts = { it is FilesDragPayload || it is ExternalDragPayload || it is PanelDragPayload },
                        ) { p, _ -> log = "commit: attach ${p.label}" },
                    ) { DemoComposer() }
                    RegionCard(
                        Modifier.weight(1f).fillMaxHeight().dropTarget(
                            state, "terminal", DropTargetKind.Area(AreaStyle.Scrim, "粘贴路径"),
                            accepts = { it is FilesDragPayload || it is PanelDragPayload },
                        ) { p, _ -> log = "commit: paste '${p.label}'" },
                    ) { FakeLines(demoTerminal.takeLast(3)) }
                }
            }
        }
    }

    // Frozen feedback for screenshots.
    LaunchedEffect(synthetic) {
        if (synthetic == null) return@LaunchedEffect
        delay(800)
        val targets = state.targets
        fun center(key: String) = targets[key]?.bounds?.center ?: Offset.Zero
        val payload: DragPayload = when (synthetic) {
            GalleryDnd.Sort, GalleryDnd.Zone -> PanelDragPayload("README.md", "A", 2, "README.md", FileTypeIcons.forName("README.md"))
            else -> FilesDragPayload(listOf("docs/ui.md"), icon = FileTypeIcons.forName("ui.md"))
        }
        val origin = center("content:A")
        val point = when (synthetic) {
            GalleryDnd.Zone -> targets["content:B"]?.bounds?.let { Offset(it.right - it.width * 0.1f, it.center.y) } ?: Offset.Zero
            GalleryDnd.Sort -> targets["tabs:A"]?.bounds?.let { Offset(it.left + 10f, it.center.y) } ?: Offset.Zero
            GalleryDnd.Area -> center("composer")
            GalleryDnd.Scrim -> center("terminal")
            GalleryDnd.Rejected -> center("tabs:B")
        }
        state.start(payload, sourceKey = "file:docs/ui.md", origin = origin, pointer = point)
        // For the sort case put the pointer between the 1st and 2nd tab.
        if (synthetic == GalleryDnd.Sort) {
            val tabs = targets["tabs:A"]
            if (tabs != null) state.move(Offset(tabs.origin.x + tabs.size.width * 0.2f, tabs.bounds.center.y))
        }
    }
}

@Composable
private fun DemoStack(
    id: String,
    stacks: Map<String, SnapshotStateList<TabItem>>,
    active: androidx.compose.runtime.MutableState<Map<String, String>>,
    state: DragDropState,
    moveTab: (PanelDragPayload, String, Int?) -> Unit,
    log: (String) -> Unit,
) {
    val tabs = stacks.getValue(id)
    Column(Modifier.fillMaxSize()) {
        StackTabBar(
            tabs = tabs,
            activeKey = active.value[id],
            onSelect = { active.value = active.value + (id to it) },
            onClose = { key -> tabs.removeAll { it.key == key } },
            focused = id == "A",
            dragDrop = StackTabDragDrop(state, id) { payload, index -> moveTab(payload as PanelDragPayload, id, index) },
            tabMenuGroups = { editorMoreGroups().take(2) },
        )
        RegionCard(
            Modifier.fillMaxSize().dropTarget(
                state, "content:$id", DropTargetKind.Zones(),
                accepts = { it is PanelDragPayload || it is FilesDragPayload },
            ) { payload, result ->
                when (payload) {
                    is PanelDragPayload -> if (result.zone == top.flysoftbeta.workflow.ui.design.dnd.DropZone.Center) moveTab(payload, id, null)
                        else log("commit: split ${payload.label} → $id ${result.zone}")
                    else -> log("commit: open ${payload.label} in $id ${result.zone}")
                }
            },
        ) {
            Box(Modifier.fillMaxSize().background(WorkflowTheme.colors.surface), contentAlignment = Alignment.TopStart) {
                FakeLines(demoCode.take(6))
            }
        }
    }
}
