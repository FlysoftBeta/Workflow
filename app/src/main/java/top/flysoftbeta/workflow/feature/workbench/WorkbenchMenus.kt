package top.flysoftbeta.workflow.feature.workbench

import top.flysoftbeta.workflow.core.layout.DropTarget
import top.flysoftbeta.workflow.core.layout.Edge
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelId
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.Paradigm
import top.flysoftbeta.workflow.core.layout.Region
import top.flysoftbeta.workflow.core.layout.StackId
import top.flysoftbeta.workflow.core.layout.Workbench
import top.flysoftbeta.workflow.core.session.Session
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.MenuGroup
import top.flysoftbeta.workflow.ui.design.icons.Sym

/** Human name of a stack for "移动到 ▸" and the `[1/3 ▾]` switcher. */
internal fun stackName(wb: Workbench, stackId: StackId, titleOf: (PanelId) -> String?): String = when (stackId) {
    Workbench.BOTTOM -> "底部"
    Workbench.AUX -> "对话区"
    else -> {
        val index = wb.editorStacks.indexOf(stackId) + 1
        val active = wb.stacks[stackId]?.active?.let(titleOf)
        if (active != null) "编辑器 $index · $active" else "编辑器 $index"
    }
}

/**
 * Group ② (panel / layout) of the More and long-press menus (docs/ui.md §2.2) for [panelId] in
 * [stackId] (panel null: the stack is empty).
 */
internal fun layoutGroup(env: WorkbenchEnv, wb: Workbench, stackId: StackId, panelId: PanelId?, titleOf: (PanelId) -> String?): MenuGroup {
    val runtime = env.runtime
    val stack = wb.stacks[stackId]
    val count = stack?.panels?.size ?: 0
    val entries = mutableListOf<MenuEntry>()
    val editor = wb.isEditorStack(stackId)
    val chatSide = wb.paradigm == Paradigm.CHAT && stackId == wb.chat.sideStack

    if (panelId != null && editor) {
        entries += MenuEntry.Action("splitRight", "向右拆分", Sym.SplitscreenRight, enabled = count >= 2) {
            env.layout(LayoutOp.Move(panelId, DropTarget.Edge(stackId, Edge.RIGHT)))
        }
        entries += MenuEntry.Action("splitDown", "向下拆分", Sym.SplitscreenBottom, enabled = count >= 2) {
            env.layout(LayoutOp.Move(panelId, DropTarget.Edge(stackId, Edge.BOTTOM)))
        }
    }
    if (panelId != null) {
        val index = stack?.panels?.indexOf(panelId) ?: -1
        entries += MenuEntry.Action("earlier", "向前移动标签", Sym.MoveItem, enabled = index > 0) {
            env.layout(LayoutOp.Move(panelId, DropTarget.Tab(stackId, index - 1)))
        }
        entries += MenuEntry.Action("later", "向后移动标签", Sym.MoveItem, enabled = index >= 0 && index < count - 1) {
            env.layout(LayoutOp.Move(panelId, DropTarget.Tab(stackId, index + 2)))
        }
        val destinations = (wb.editorStacks + Workbench.BOTTOM + Workbench.AUX).filter { it != stackId }
        val editors = destinations.filter { wb.isEditorStack(it) }.map { id ->
            MenuEntry.Action("to:$id", stackName(wb, id, titleOf), Sym.Draft) { env.layout(LayoutOp.Move(panelId, DropTarget.Center(id))) }
        }
        val fixed = destinations.filter { !wb.isEditorStack(it) }.map { id ->
            MenuEntry.Action("to:$id", stackName(wb, id, titleOf), if (id == Workbench.BOTTOM) Sym.Terminal else Sym.Chat) {
                env.layout(LayoutOp.Move(panelId, DropTarget.Center(id)))
            }
        }
        val fresh = listOf(
            Edge.LEFT to "编辑区左侧", Edge.RIGHT to "编辑区右侧",
            Edge.TOP to "编辑区上方", Edge.BOTTOM to "编辑区下方",
        ).map { (edge, label) ->
            MenuEntry.Action("to:edge:$edge", label, Sym.Add) { env.layout(LayoutOp.Move(panelId, DropTarget.EditorEdge(edge))) }
        }
        entries += MenuEntry.Submenu("moveTo", "移动到", Sym.MoveItem, listOf(MenuGroup("editors", editors), MenuGroup("fixed", fixed), MenuGroup("new", fresh)))
    }
    if (editor && wb.paradigm == Paradigm.FILES) {
        val maximized = wb.files.maximized == stackId
        entries += MenuEntry.Action("maximize", if (maximized) "还原" else "最大化", if (maximized) Sym.CloseFullscreen else Sym.OpenInFull) {
            env.layout(LayoutOp.SetMaximized(if (maximized) null else stackId))
        }
    }
    if (panelId != null) {
        val others = wb.otherPanels(panelId)
        val after = wb.panelsAfter(panelId)
        val saved = stack?.panels.orEmpty().filter { id -> runtime.existing(id)?.tab?.dirty != true }
        entries += MenuEntry.Action("closeOthers", "关闭其他", Sym.TabClose, enabled = others.isNotEmpty()) { runtime.close(others) }
        entries += MenuEntry.Action("closeRight", "关闭右侧", Sym.TabClose, enabled = after.isNotEmpty()) { runtime.close(after) }
        entries += MenuEntry.Action("closeSaved", "关闭已保存", Sym.Close, enabled = saved.isNotEmpty()) { runtime.close(saved) }
    }
    when {
        stackId == Workbench.AUX && wb.paradigm == Paradigm.FILES -> {
            val active = stack?.active?.let { wb.panels[it] }
            if (active?.target is PanelTarget.Conversation) {
                entries += MenuEntry.Action("promote", "设为主要内容", Sym.OpenInFull) { env.layout(LayoutOp.PromoteConversation(active.id)) }
            }
            entries += MenuEntry.Action("collapseAux", "收起面板", Sym.RightPanelClose) { hideRegion(env, Region.AUX) }
        }
        stackId == Workbench.AUX && wb.paradigm == Paradigm.CHAT -> {
            entries += MenuEntry.Action("returnFiles", "回到 Files", Sym.SwapHoriz) { env.layout(LayoutOp.ReturnToFiles) }
            entries += MenuEntry.Toggle("fileSide", "文件侧栏", checked = !wb.chat.side.collapsed || runtime.presence.auxOverlay) {
                runtime.toggleRegion(Region.CHAT_SIDE)
            }
        }
        stackId == Workbench.BOTTOM && wb.paradigm == Paradigm.FILES -> {
            val collapsed = wb.files.bottom.collapsed
            entries += MenuEntry.Action("bottom", if (collapsed) "展开" else "收起", if (collapsed) Sym.KeyboardArrowUp else Sym.KeyboardArrowDown) {
                env.layout(LayoutOp.SetRegionCollapsed(Region.BOTTOM, !collapsed))
            }
        }
        chatSide -> {
            entries += MenuEntry.Action("returnFiles", "回到 Files", Sym.OpenInFull) { env.layout(LayoutOp.ReturnToFiles) }
        }
    }
    return MenuGroup("layout", entries)
}

internal fun hideRegion(env: WorkbenchEnv, region: Region) {
    val presence = env.runtime.presence
    val side = region == Region.EXPLORER || region == Region.CHAT_RAIL
    if (side && presence.sideOverlay) presence.sideOverlay = false
    else if (!side && presence.auxOverlay) presence.auxOverlay = false
    else env.layout(LayoutOp.SetRegionCollapsed(region, true))
}

/** Group ③ (session) at the end of every More menu. */
internal fun sessionGroup(env: WorkbenchEnv, session: Session, wb: Workbench): MenuGroup {
    val shell = env.shell
    val paradigmEntry = when (wb.paradigm) {
        Paradigm.FILES -> MenuEntry.Action("toChat", "切换到 Chat", Sym.SwapHoriz) { env.layout(LayoutOp.SwitchParadigm(Paradigm.CHAT)) }
        Paradigm.CHAT -> MenuEntry.Action("toFiles", "切换到 Files", Sym.SwapHoriz) { env.layout(LayoutOp.ReturnToFiles) }
        Paradigm.SOLO -> MenuEntry.Action("toFiles", "切换到 Files", Sym.SwapHoriz) { env.layout(LayoutOp.SwitchParadigm(Paradigm.FILES)) }
    }
    return MenuGroup("session", listOf(
        MenuEntry.Action("newSession", "新建 Session", Sym.LibraryAdd) { shell.newSession() },
        MenuEntry.Action("nameSession", if (session.name == null) "命名 Session…" else "重命名 Session…", Sym.Edit) { shell.renaming = session.id },
        paradigmEntry,
        MenuEntry.Action("allSessions", "所有会话…", Sym.History) { shell.sessionsOpen = true },
    ))
}
