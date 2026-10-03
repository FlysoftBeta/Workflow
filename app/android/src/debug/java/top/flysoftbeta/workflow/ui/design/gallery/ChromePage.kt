package top.flysoftbeta.workflow.ui.design.gallery

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.ui.design.AttachmentItem
import top.flysoftbeta.workflow.ui.design.AttachmentState
import top.flysoftbeta.workflow.ui.design.AttachmentStrip
import top.flysoftbeta.workflow.ui.design.DockingLayout
import top.flysoftbeta.workflow.ui.design.ExtraKeysRow
import top.flysoftbeta.workflow.ui.design.ExtraKeysState
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.MenuGroup
import top.flysoftbeta.workflow.ui.design.ModelChip
import top.flysoftbeta.workflow.ui.design.RegionCard
import top.flysoftbeta.workflow.ui.design.RegionHeader
import top.flysoftbeta.workflow.ui.design.SessionChip
import top.flysoftbeta.workflow.ui.design.SplitOrientation
import top.flysoftbeta.workflow.ui.design.SplitPane
import top.flysoftbeta.workflow.ui.design.SplitSide
import top.flysoftbeta.workflow.ui.design.StackTabBar
import top.flysoftbeta.workflow.ui.design.TabItem
import top.flysoftbeta.workflow.ui.design.ToolAction
import top.flysoftbeta.workflow.ui.design.WfIconButton
import top.flysoftbeta.workflow.ui.design.icons.FileTypeIcons
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.rememberDockingState
import top.flysoftbeta.workflow.ui.design.rememberSplitPaneState
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

internal fun editorMoreGroups(): List<MenuGroup> = listOf(
    MenuGroup("resource", listOf(
        MenuEntry.Action("copyPath", "复制路径", Sym.ContentCopy, shortcut = "Ctrl+Shift+C") {},
        MenuEntry.Action("copyRel", "复制相对路径", Sym.ContentCopy) {},
        MenuEntry.Action("reveal", "在文件树中显示", Sym.AccountTree) {},
        MenuEntry.Action("attach", "附加到对话", Sym.AddComment) {},
        MenuEntry.Action("terminal", "在终端中打开所在目录", Sym.Terminal) {},
        MenuEntry.Action("openWith", "用其他应用打开", Sym.OpenInNew) {},
        MenuEntry.Toggle("wrap", "自动换行", checked = true) {},
        MenuEntry.Action("discard", "放弃更改", Sym.Delete, destructive = true) {},
    )),
    MenuGroup("layout", listOf(
        MenuEntry.Action("splitRight", "向右拆分", Sym.SplitscreenRight) {},
        MenuEntry.Action("splitDown", "向下拆分", Sym.SplitscreenBottom) {},
        MenuEntry.Submenu("moveTo", "移动到", Sym.MoveItem, listOf(
            MenuGroup("stacks", listOf(
                MenuEntry.Action("s1", "编辑器 1", Sym.Draft) {},
                MenuEntry.Action("s2", "底部终端", Sym.Terminal) {},
            )),
            MenuGroup("new", listOf(MenuEntry.Action("new", "新 Stack", Sym.Add) {})),
        )),
        MenuEntry.Action("max", "最大化", Sym.OpenInFull) {},
        MenuEntry.Action("closeOthers", "关闭其他", Sym.TabClose) {},
        MenuEntry.Action("closeSaved", "关闭已保存", Sym.Close) {},
    )),
    MenuGroup("session", listOf(
        MenuEntry.Action("newSession", "新建 Session", Sym.LibraryAdd) {},
        MenuEntry.Action("name", "命名 Session…", Sym.Edit) {},
        MenuEntry.Action("paradigm", "切换到 Chat", Sym.SwapHoriz) {},
        MenuEntry.Action("all", "所有会话…", Sym.History) {},
    )),
)

private fun tab(name: String, dirty: Boolean = false, caption: String? = null) =
    TabItem(name, name, FileTypeIcons.forName(name), dirty, caption)

/** A Files-paradigm workbench built only from design components (docs/ux/README.md §3.3). */
@Composable
fun ChromePage() {
    val docking = rememberDockingState()
    val split = rememberSplitPaneState(0.65f)
    val tabs = remember {
        mutableStateListOf(
            tab("prompt.md", dirty = true), tab("Theme.kt"), tab("libs.versions.toml"), tab("build.gradle.kts", caption = "app"),
            tab("build.gradle.kts", caption = "core").copy(key = "build2"), tab("README.md"), tab("container.json", dirty = true), tab("icon.png"),
        )
    }
    var active by remember { mutableStateOf("Theme.kt") }
    var terminalActive by remember { mutableStateOf("bash") }
    val keys = remember { ExtraKeysState() }
    var selected by remember { mutableStateOf("Theme.kt") }

    DockingLayout(
        state = docking,
        side = {
            Column(Modifier.fillMaxSize()) {
                RegionHeader(
                    leading = {
                        WfIconButton(Sym.Home, "主页", {})
                        WfIconButton(if (docking.sideOpen) Sym.LeftPanelClose else Sym.LeftPanelOpen, "侧栏", { docking.sideOpen = !docking.sideOpen })
                        SessionChip("周日 14:32", {}, {})
                    },
                    actions = listOf(ToolAction("add", Sym.Add, "新建") {}),
                    moreGroups = listOf(MenuGroup("tree", listOf(
                        MenuEntry.Action("filter", "筛选…", Sym.FilterList) {},
                        MenuEntry.Action("collapse", "全部折叠", Sym.UnfoldLess) {},
                        MenuEntry.Action("refresh", "刷新", Sym.Refresh) {},
                        MenuEntry.Toggle("hidden", "显示隐藏文件", false) {},
                    ))),
                )
                RegionCard(Modifier.fillMaxSize()) {
                    Column(Modifier.padding(vertical = 4.dp)) {
                        DemoTreeRow("app", 0, Sym.FolderOpen)
                        DemoTreeRow("src", 1, Sym.FolderOpen)
                        DemoTreeRow("ui", 2, Sym.Folder, dirty = true)
                        DemoTreeRow("Theme.kt", 2, Sym.Code, selected = selected == "Theme.kt", open = true) { selected = "Theme.kt" }
                        DemoTreeRow("build.gradle.kts", 1, Sym.Code, open = true) { selected = "build" }
                        DemoTreeRow("docs", 0, Sym.Folder)
                        DemoTreeRow("prompt.md", 0, Sym.Article, dirty = true, open = true)
                        DemoTreeRow("container.json", 0, Sym.DataObject, dirty = true, open = true)
                        DemoTreeRow("icon.png", 0, Sym.Image)
                    }
                }
            }
        },
        center = {
            SplitPane(
                state = split,
                orientation = SplitOrientation.Vertical,
                firstMin = 120.dp, secondMin = WorkflowTheme.dimens.bar + 60.dp,
                collapsible = setOf(SplitSide.Second), collapsedSize = WorkflowTheme.dimens.bar,
                first = {
                    Column(Modifier.fillMaxSize()) {
                        StackTabBar(
                            tabs = tabs, activeKey = active, onSelect = { active = it }, onClose = { k -> tabs.removeAll { it.key == k } },
                            actions = listOf(
                                ToolAction("save", Sym.Save, "保存") {},
                                ToolAction("undo", Sym.Undo, "撤销") {},
                                ToolAction("redo", Sym.Redo, "重做", enabled = false) {},
                                ToolAction("find", Sym.Search, "查找") {},
                                ToolAction("attach", Sym.AddComment, "附加到对话") {},
                            ),
                            moreGroups = editorMoreGroups(),
                            tabMenuGroups = { editorMoreGroups().take(2) },
                        )
                        RegionCard(Modifier.fillMaxSize()) { FakeLines(demoCode) }
                    }
                },
                second = {
                    Column(Modifier.fillMaxSize()) {
                        StackTabBar(
                            tabs = listOf(TabItem("bash", "bash", Sym.Terminal), TabItem("bash 2", "bash 2", Sym.Terminal)),
                            activeKey = terminalActive, onSelect = { terminalActive = it }, onClose = {}, focused = false,
                            actions = listOf(
                                ToolAction("new", Sym.Add, "新建终端") {},
                                ToolAction("collapse", Sym.KeyboardArrowDown, "收起") { split.collapsed = SplitSide.Second },
                            ),
                            moreGroups = listOf(MenuGroup("t", listOf(
                                MenuEntry.Action("clear", "清屏", Sym.ClearAll) {},
                                MenuEntry.Action("rename", "重命名", Sym.Edit) {},
                                MenuEntry.Action("restart", "重启", Sym.RestartAlt) {},
                                MenuEntry.Toggle("keys", "常驻特殊键行", true) {},
                                MenuEntry.Action("end", "结束", Sym.StopCircle, destructive = true) {},
                            ))),
                        )
                        RegionCard(Modifier.fillMaxSize()) {
                            Column(Modifier.fillMaxSize()) {
                                Box(Modifier.weight(1f)) { FakeLines(demoTerminal, mono = true) }
                                ExtraKeysRow(keys, onKey = {})
                            }
                        }
                    }
                },
            )
        },
        aux = {
            Column(Modifier.fillMaxSize()) {
                RegionHeader(
                    title = {
                        Text("梳理 Context Engine", style = WorkflowTheme.text.titleSm, color = WorkflowTheme.colors.onSurface, maxLines = 1)
                        top.flysoftbeta.workflow.ui.design.SymbolIcon(Sym.ArrowDropDown, null, size = 18.dp)
                    },
                    actions = listOf(ToolAction("primary", Sym.OpenInFull, "设为主要内容") {}),
                    moreGroups = listOf(MenuGroup("c", listOf(
                        MenuEntry.Action("rename", "重命名", Sym.Edit) {},
                        MenuEntry.Action("fork", "分叉", Sym.ForkRight) {},
                        MenuEntry.Action("md", "复制为 Markdown", Sym.Markdown) {},
                        MenuEntry.Action("archive", "归档", Sym.Archive) {},
                    ))),
                    trailing = {
                        WfIconButton(Sym.RightPanelClose, "辅助区", { docking.auxOpen = !docking.auxOpen }, badge = WorkflowTheme.colors.tertiary)
                        WfIconButton(Sym.Settings, "设置", {})
                    },
                )
                RegionCard(Modifier.fillMaxSize()) {
                    Column(Modifier.fillMaxSize().padding(12.dp)) {
                        Box(Modifier.weight(1f).fillMaxWidth()) {
                            Text(
                                "助手回复（无气泡）…", Modifier.align(Alignment.TopStart),
                                style = WorkflowTheme.text.chat, color = WorkflowTheme.colors.onSurface,
                            )
                        }
                        DemoComposer()
                    }
                }
            }
        },
    )
}

@Composable
internal fun DemoComposer() {
    val colors = WorkflowTheme.colors
    Column(
        Modifier
            .fillMaxWidth()
            .clip(WorkflowShapes.xl)
            .background(colors.surfaceContainerHigh)
            .padding(bottom = 4.dp),
    ) {
        AttachmentStrip(
            items = listOf(
                AttachmentItem("a", "img.png", isImage = true),
                AttachmentItem("b", "Theme.kt", isImage = false, icon = Sym.Code),
                AttachmentItem("c", "report-final.pdf", isImage = false, icon = Sym.PictureAsPdf, state = AttachmentState.Uploading(0.6f)),
            ),
            onRemove = {},
        )
        Text("随心输入", Modifier.padding(horizontal = 12.dp, vertical = 4.dp), style = WorkflowTheme.text.chat, color = colors.onSurfaceVariant)
        Row(Modifier.fillMaxWidth().height(40.dp).padding(horizontal = 4.dp), verticalAlignment = Alignment.CenterVertically) {
            WfIconButton(Sym.Add, "添加", {})
            WfIconButton(Sym.Warning, "完全访问", {}, tint = WorkflowTheme.extendedColors.warning)
            Spacer(Modifier.weight(1f))
            ModelChip("GPT-5.5 高", {})
            Spacer(Modifier.width(4.dp))
            WfIconButton(Sym.ArrowUpward, "发送", {}, tint = colors.primary)
        }
    }
}
