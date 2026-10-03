package top.flysoftbeta.workflow.feature.workbench

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.Workbench
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.MenuGroup
import top.flysoftbeta.workflow.ui.design.ToolAction
import top.flysoftbeta.workflow.ui.design.asMenuEntry
import top.flysoftbeta.workflow.ui.design.icons.Sym

class WorkbenchMenusTest {
    private fun action(key: String, label: String) = MenuEntry.Action(key, label, Sym.Close) {}

    /** What StackTabBar lists in More when [visible] surfaced actions fit and the rest collapse. */
    private fun shownMore(actions: List<ToolAction>, groups: List<MenuGroup>, visible: Int): List<String> {
        val collapsed = actions.drop(visible).map { it.asMenuEntry() }
        return (listOf(MenuGroup("surfaced", collapsed)) + moreMenu(actions, groups)).flatMap { group -> group.entries.map { it.label } }
    }

    private fun maximized(): Pair<Workbench, String> {
        val wb = Workbench.empty().apply(LayoutOp.Open(PanelTarget.File("notes/a.md")))
        val stack = wb.focusedStack
        return wb.apply(LayoutOp.SetMaximized(stack)) to stack
    }

    @Test fun maximizedStackListsRestoreOnceWhetherSurfacedOrCollapsed() {
        val (wb, stack) = maximized()
        val restore = restoreAction {}
        val layout = MenuGroup("layout", listOfNotNull(maximizeEntry(wb, stack) {}, action("closeOthers", "关闭其他")))
        val groups = listOf(layout, MenuGroup("session", listOf(action("newSession", "新建 Session"))))
        for (visible in 0..1) {
            assertEquals("visible=$visible", 1 - visible, shownMore(listOf(restore), groups, visible).count { it == "还原" })
        }
        // The tab long-press menu has no surfaced actions and keeps the layout entry.
        assertEquals("还原", layout.entries.first().label)
    }

    @Test fun maximizeEntryAppliesOnlyToFilesEditorStacks() {
        val (wb, stack) = maximized()
        var requested: String? = "unset"
        maximizeEntry(wb, stack) { requested = it }!!.onClick()
        assertNull(requested)
        val normal = Workbench.empty().apply(LayoutOp.Open(PanelTarget.File("notes/a.md")))
        val entry = maximizeEntry(normal, normal.focusedStack) { requested = it }!!
        assertEquals("最大化", entry.label)
        entry.onClick()
        assertEquals(normal.focusedStack, requested)
        assertNull(maximizeEntry(normal, Workbench.BOTTOM) {})
    }

    @Test fun aKeySharedByGroupsAppearsOnceAndEmptyGroupsDisappear() {
        val groups = listOf(
            MenuGroup("layout", listOf(action("toFiles", "回到 Files"))),
            MenuGroup("session", listOf(action("toFiles", "切换到 Files"), action("allSessions", "所有会话…"))),
            MenuGroup("resource", listOf(action("bottom", "收起"))),
        )
        val surfaced = listOf(ToolAction("bottom", Sym.KeyboardArrowDown, "收起") {})
        val more = moreMenu(surfaced, groups)
        assertEquals(listOf("layout", "session"), more.map { it.key })
        assertEquals(listOf("回到 Files", "所有会话…"), more.flatMap { group -> group.entries.map { it.label } })
    }
}
