package top.flysoftbeta.workflow.core.layout

import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import top.flysoftbeta.workflow.core.layout.LayoutOp.*
import top.flysoftbeta.workflow.core.layout.Workbench.Companion.AUX
import top.flysoftbeta.workflow.core.layout.Workbench.Companion.BOTTOM
import top.flysoftbeta.workflow.core.layout.Workbench.Companion.FIRST_EDITOR
import top.flysoftbeta.workflow.core.resource.ResourceRef

class LayoutOpTest {
    @Before fun strict() { LayoutReducer.strict = true }

    private val empty = Workbench.empty().assertValid("empty")

    // ---- Open ----

    @Test fun `files open in the focused editor stack and become focused`() {
        val w = empty.ops(Open(file("a.txt")), Open(file("b.txt")))
        assertEquals(listOf(file("a.txt"), file("b.txt")), w.targetsIn(FIRST_EDITOR))
        w.assertFocused(file("b.txt"))
        assertEquals(listOf(w.id(file("b.txt")), w.id(file("a.txt"))), w.mru)
    }

    @Test fun `a new panel is inserted right after the active one`() {
        val w = empty.ops(Open(file("a")), Open(file("b")), Focus("p1"), Open(file("c")))
        assertEquals(listOf(file("a"), file("c"), file("b")), w.targetsIn(FIRST_EDITOR))
    }

    @Test fun `reopening a resource focuses the existing panel, including image and file of one path`() {
        val w = empty.ops(Open(file("a.png")), Open(file("b.txt")))
        val again = w.ops(Open(PanelTarget.Image("a.png")))
        assertEquals(2, again.panels.size)
        again.assertFocused(file("a.png"))
        assertTrue(again.panelFor(file("a.png"))!!.target is PanelTarget.File)
    }

    @Test fun `terminals go to the bottom stack and expand it, conversations go to aux`() {
        val w = empty.ops(Open(terminal("t1")), Open(conversation("c1")))
        assertEquals(listOf(terminal("t1")), w.targetsIn(BOTTOM))
        assertFalse(w.files.bottom.collapsed)
        assertEquals(listOf(conversation("c1")), w.targetsIn(AUX))
        assertEquals(AUX, w.focusedStack)
        val t2 = w.ops(Open(terminal("t2")))
        assertEquals(listOf(terminal("t1"), terminal("t2")), t2.targetsIn(BOTTOM))
    }

    @Test fun `proxy settings and diff open as editor panels, proxy pages are distinct resources`() {
        val w = empty.ops(Open(PanelTarget.Proxy()), Open(PanelTarget.Settings), Open(PanelTarget.Proxy(ProxyPage.LOGS)), Open(PanelTarget.Diff("a")))
        assertEquals(4, w.panelsIn(FIRST_EDITOR).size)
    }

    @Test fun `invalid targets and missing ids are no-ops returning the same instance`() {
        val w = empty.ops(Open(file("a")))
        for (op in listOf(
            Open(file("../x")), Open(file("/abs")), Open(file("")), Open(file("a/./b")), Open(terminal(" ")),
            Focus("nope"), FocusStack("nope"), Close("nope"), Move("nope", DropTarget.Center(BOTTOM)),
            Move("p1", DropTarget.Center("nope")), SplitStack("nope", Edge.RIGHT), ResizeSplit("nope", listOf(1.0)),
            ResetSplit("nope"), SetMaximized("nope"), PromoteConversation("p1"), SetChatSideStack(AUX), Retarget("nope", file("b")),
            UpdateView("nope", PanelView(scrollOffset = 1)), RenamePath("x", "../y"), SwitchParadigm(Paradigm.SOLO),
            SwitchParadigm(Paradigm.FILES), ReturnToFiles, ResizeRegion(Region.AUX, Double.NaN),
        )) assertSame("$op", w, w.apply(op))
    }

    @Test fun `in-stack placement honors index, replacement of the same kind and moving an existing panel`() {
        var w = empty.ops(Open(file("a")), Open(file("b")), Open(file("c"), Placement.InStack(FIRST_EDITOR, index = 0)))
        assertEquals(listOf(file("c"), file("a"), file("b")), w.targetsIn(FIRST_EDITOR))
        w = w.ops(Open(conversation("x")), Open(conversation("y"), Placement.InStack(AUX, replaceActive = true)))
        assertEquals(listOf(conversation("y")), w.targetsIn(AUX))
        w = w.ops(Open(terminal("t"), Placement.InStack(AUX)), Open(conversation("z"), Placement.InStack(AUX, replaceActive = true)))
        assertEquals("a terminal is never replaced by a conversation", listOf(conversation("y"), terminal("t"), conversation("z")), w.targetsIn(AUX))
        w = w.ops(Open(file("a"), Placement.InStack(BOTTOM, moveExisting = true)))
        assertEquals(listOf(file("a")), w.targetsIn(BOTTOM))
        assertEquals(listOf(file("c"), file("b")), w.targetsIn(FIRST_EDITOR))
    }

    @Test fun `background open keeps the active panel and focus`() {
        val w = empty.ops(Open(file("a")), Open(terminal("t")), Open(file("b"), focus = false))
        assertEquals(BOTTOM, w.focusedStack)
        assertEquals(w.id(file("a")), w.stacks.getValue(FIRST_EDITOR).active)
        val fresh = empty.ops(Open(file("a"), focus = false))
        assertEquals(fresh.id(file("a")), fresh.stacks.getValue(FIRST_EDITOR).active)
    }

    @Test fun `split placement opens beside the target stack`() {
        val w = empty.ops(Open(file("a")), Open(file("b"), Placement.SplitEdge(FIRST_EDITOR, Edge.RIGHT)))
        val split = w.editor as SplitNode.Split
        assertEquals(Axis.ROW, split.axis)
        assertEquals(listOf(0.5, 0.5), split.weights)
        assertEquals(FIRST_EDITOR, (split.children[0] as SplitNode.Leaf).stackId)
        w.assertFocused(file("b"))
        assertEquals(w.stackOf(w.id(file("b"))), w.lastEditorStack)
        val bottom = empty.ops(Open(file("c"), Placement.SplitEdge(BOTTOM, Edge.TOP)))
        assertEquals("bottom cannot split", listOf(file("c")), bottom.targetsIn(BOTTOM))
    }

    // ---- Close ----

    @Test fun `closing the active panel activates the most recently used remaining one`() {
        val w = empty.ops(Open(file("a")), Open(file("b")), Open(file("c")), Focus("p1"), Focus("p3"), Close("p3"))
        assertEquals("p1", w.stacks.getValue(FIRST_EDITOR).active)
        assertFalse("p3" in w.panels)
    }

    @Test fun `closing the last panel of a split stack removes the stack and collapses the split`() {
        val w = empty.ops(Open(file("a")), Open(file("b"), Placement.SplitEdge(FIRST_EDITOR, Edge.RIGHT)))
        val closed = w.ops(Close(w.id(file("b"))))
        assertEquals(SplitNode.Leaf(FIRST_EDITOR), closed.editor)
        assertEquals(FIRST_EDITOR, closed.focusedStack)
        assertEquals(setOf(FIRST_EDITOR, BOTTOM, AUX), closed.stacks.keys)
    }

    @Test fun `the only editor stack survives being emptied`() {
        val w = empty.ops(Open(file("a")), Close("p1"))
        assertEquals(SplitNode.Leaf(FIRST_EDITOR), w.editor)
        assertTrue(w.panels.isEmpty())
        assertNull(w.stacks.getValue(FIRST_EDITOR).active)
    }

    @Test fun `closing the last terminal collapses the bottom region and returns focus to the editor`() {
        val w = empty.ops(Open(file("a")), Open(terminal("t")), Close(listOf("p2")))
        assertTrue(w.files.bottom.collapsed)
        assertEquals(FIRST_EDITOR, w.focusedStack)
    }

    @Test fun `close others and close to the right use the query helpers`() {
        val w = empty.ops(Open(file("a")), Open(file("b")), Open(file("c")), Open(file("d")))
        val b = w.id(file("b"))
        assertEquals(listOf(w.id(file("c")), w.id(file("d"))), w.panelsAfter(b))
        val others = w.ops(Close(w.otherPanels(b)))
        assertEquals(listOf(file("b")), others.targetsIn(FIRST_EDITOR))
    }

    // ---- Move ----

    @Test fun `tab index counts displayed tabs including the dragged one`() {
        val w = empty.ops(Open(file("a")), Open(file("b")), Open(file("c")))
        fun order(panel: String, index: Int) = w.ops(Move(w.id(file(panel)), DropTarget.Tab(FIRST_EDITOR, index))).targetsIn(FIRST_EDITOR).map { (it as PanelTarget.File).path }
        assertEquals(listOf("b", "a", "c"), order("a", 2))
        assertEquals(listOf("b", "c", "a"), order("a", 3))
        assertEquals(listOf("c", "a", "b"), order("c", 0))
        assertEquals(listOf("a", "b", "c"), order("a", 1))
        assertEquals(listOf("a", "c", "b"), order("b", 99))
        assertEquals(listOf("a", "b", "c"), order("b", 2))
    }

    @Test fun `moving between stacks keeps panel identity and view state`() {
        val w = empty.ops(Open(file("a")), UpdateView("p1", PanelView(scrollAnchor = "40")), Open(conversation("c")))
        val moved = w.ops(Move("p1", DropTarget.Tab(AUX, 0)))
        assertEquals(listOf(file("a"), conversation("c")), moved.targetsIn(AUX))
        assertEquals("40", moved.panels.getValue("p1").view.scrollAnchor)
        assertEquals(AUX, moved.focusedStack)
        val center = w.ops(Move("p1", DropTarget.Center(BOTTOM)))
        assertEquals(listOf(file("a")), center.targetsIn(BOTTOM))
    }

    @Test fun `edge drops split the target stack, repeated splits on one axis flatten`() {
        var w = empty.ops(Open(file("a")), Open(file("b")), Open(file("c")))
        w = w.ops(Move(w.id(file("b")), DropTarget.Edge(FIRST_EDITOR, Edge.RIGHT)))
        val right = w.stackOf(w.id(file("b")))!!
        w = w.ops(Move(w.id(file("c")), DropTarget.Edge(right, Edge.RIGHT)))
        val split = w.editor as SplitNode.Split
        assertEquals(3, split.children.size)
        assertEquals(listOf(0.5, 0.25, 0.25), split.weights)
        w = w.ops(Move(w.id(file("a")), DropTarget.Edge(right, Edge.BOTTOM)))
        // "a" left the first stack, which disappears; b's stack now holds a column split.
        assertEquals(2, (w.editor as SplitNode.Split).children.size)
        assertTrue((w.editor as SplitNode.Split).children[0] is SplitNode.Split)
        assertEquals(Axis.COLUMN, ((w.editor as SplitNode.Split).children[0] as SplitNode.Split).axis)
    }

    @Test fun `edge drop of a stack's only panel onto itself is a no-op, bottom edges act as centre`() {
        val w = empty.ops(Open(file("a")), Open(terminal("t")))
        assertSame(w, w.apply(Move("p1", DropTarget.Edge(FIRST_EDITOR, Edge.LEFT))))
        val intoBottom = w.ops(Move("p1", DropTarget.Edge(BOTTOM, Edge.TOP)))
        assertEquals(listOf(terminal("t"), file("a")), intoBottom.targetsIn(BOTTOM))
    }

    @Test fun `editor edge creates a full height column and a terminal can be moved into the editor`() {
        var w = empty.ops(Open(file("a")), Open(file("b"), Placement.SplitEdge(FIRST_EDITOR, Edge.BOTTOM)), Open(terminal("t")))
        w = w.ops(Move(w.id(terminal("t")), DropTarget.EditorEdge(Edge.RIGHT)))
        val root = w.editor as SplitNode.Split
        assertEquals(Axis.ROW, root.axis)
        assertEquals(Axis.COLUMN, (root.children[0] as SplitNode.Split).axis)
        assertTrue(w.files.bottom.collapsed)
        w.assertFocused(terminal("t"))
        val only = empty.ops(Open(file("a")))
        assertSame(only, only.apply(Move("p1", DropTarget.EditorEdge(Edge.LEFT))))
    }

    @Test fun `split stack moves the active panel and needs two panels`() {
        val one = empty.ops(Open(file("a")))
        assertSame(one, one.apply(SplitStack(FIRST_EDITOR, Edge.RIGHT)))
        val two = one.ops(Open(file("b")), SplitStack(FIRST_EDITOR, Edge.BOTTOM))
        assertEquals(listOf(file("a")), two.targetsIn(FIRST_EDITOR))
        assertEquals(Axis.COLUMN, (two.editor as SplitNode.Split).axis)
        two.assertFocused(file("b"))
        val bottom = empty.ops(Open(terminal("1")), Open(terminal("2")))
        assertSame(bottom, bottom.apply(SplitStack(BOTTOM, Edge.RIGHT)))
    }

    // ---- Sizes ----

    @Test fun `split weights are validated, normalized, clamped and resettable`() {
        val w = empty.ops(Open(file("a")), Open(file("b"), Placement.SplitEdge(FIRST_EDITOR, Edge.RIGHT)))
        val id = (w.editor as SplitNode.Split).id
        assertEquals(listOf(0.75, 0.25), (w.ops(ResizeSplit(id, listOf(3.0, 1.0))).editor as SplitNode.Split).weights)
        val clamped = (w.ops(ResizeSplit(id, listOf(0.999, 0.001))).editor as SplitNode.Split).weights
        assertEquals(LayoutInvariants.MIN_WEIGHT, clamped[1], 1e-9)
        assertEquals(1.0, clamped.sum(), 1e-9)
        assertSame(w, w.apply(ResizeSplit(id, listOf(1.0))))
        assertSame(w, w.apply(ResizeSplit(id, listOf(1.0, -1.0))))
        assertEquals(listOf(0.5, 0.5), (w.ops(ResizeSplit(id, listOf(3.0, 1.0)), ResetSplit(id)).editor as SplitNode.Split).weights)
    }

    @Test fun `regions resize within bounds and collapse independently per paradigm`() {
        val w = empty.ops(ResizeRegion(Region.EXPLORER, 10.0), ResizeRegion(Region.AUX, 420.0), ResizeRegion(Region.BOTTOM, 2.0),
            ToggleRegion(Region.EXPLORER), SetRegionCollapsed(Region.CHAT_TREE, true))
        assertEquals(Region.EXPLORER.minSize, w.files.explorer.size, 0.0)
        assertTrue(w.files.explorer.collapsed)
        assertEquals(420.0, w.files.aux.size, 0.0)
        assertEquals(Region.BOTTOM.maxSize, w.files.bottom.size, 0.0)
        assertTrue(w.chat.tree.collapsed)
        assertFalse(w.chat.side.collapsed)
    }

    @Test fun `maximize focuses the stack and is cleared by revealing another stack`() {
        val w = empty.ops(Open(file("a")), Open(terminal("t")), SetMaximized(FIRST_EDITOR))
        assertEquals(FIRST_EDITOR, w.files.maximized)
        assertEquals(FIRST_EDITOR, w.focusedStack)
        assertEquals(FIRST_EDITOR, w.ops(Open(file("b"))).files.maximized)
        assertNull(w.ops(Focus(w.id(terminal("t")))).files.maximized)
        assertNull(w.ops(SetMaximized(null)).files.maximized)
    }

    // ---- Paradigms ----

    private fun richFiles(): Workbench = empty.ops(
        Open(file("a")), Open(file("b")), Open(file("c"), Placement.SplitEdge(FIRST_EDITOR, Edge.RIGHT)),
        Open(terminal("t")), Open(conversation("x")), Open(conversation("y")), ResizeRegion(Region.AUX, 380.0),
        ResizeRegion(Region.BOTTOM, 0.4), Focus("p1"),
    )

    private fun filesView(w: Workbench) = listOf(w.paradigm, w.panels, w.stacks, w.editor, w.files, w.explorer, w.lastEditorStack)

    @Test fun `switching to chat and back restores the files arrangement exactly`() {
        val files = richFiles()
        val chat = files.ops(SwitchParadigm(Paradigm.CHAT))
        assertEquals(Paradigm.CHAT, chat.paradigm)
        assertEquals(AUX, chat.focusedStack)
        assertEquals(files.lastEditorStack, chat.chat.sideStack)
        assertEquals(files, chat.ops(SwitchParadigm(Paradigm.FILES)).copy(chat = files.chat))
    }

    @Test fun `promoting the aux conversation and returning restores files`() {
        val files = richFiles()
        val y = files.id(conversation("y"))
        val chat = files.ops(PromoteConversation(y))
        assertEquals(Paradigm.CHAT, chat.paradigm)
        assertEquals(y, chat.focusedPanel?.id)
        assertNull(chat.chat.promotedFrom)
        assertEquals(filesView(files), filesView(chat.ops(ReturnToFiles)))
    }

    @Test fun `promoting a conversation from a split editor stack returns it to its exact place`() {
        val base = richFiles()
        val z = base.ops(Open(conversation("z"), Placement.SplitEdge(base.stackOf(base.id(file("c")))!!, Edge.BOTTOM)))
        val zId = z.id(conversation("z"))
        val origin = z.stackOf(zId)!!
        val chat = z.ops(PromoteConversation(zId))
        assertEquals(AUX, chat.stackOf(zId))
        assertTrue("origin kept while promoted", origin in chat.stacks)
        assertTrue(chat.stacks.getValue(origin).panels.isEmpty())
        assertNotEquals("side shows a stack with content", origin, chat.chat.sideStack)
        assertEquals(z.stacks.getValue(AUX).active, chat.chat.promotedFrom!!.auxActiveBefore)
        val back = chat.ops(ReturnToFiles)
        assertEquals(z.editor, back.editor)
        assertEquals(z.stacks, back.stacks)
        assertEquals(z.files, back.files)
    }

    @Test fun `closing the promoted panel in chat drops the empty origin stack`() {
        val base = empty.ops(Open(file("a")), Open(conversation("z"), Placement.SplitEdge(FIRST_EDITOR, Edge.RIGHT)))
        val chat = base.ops(PromoteConversation(base.id(conversation("z"))))
        val closed = chat.ops(Close(chat.id(conversation("z"))))
        assertEquals(SplitNode.Leaf(FIRST_EDITOR), closed.editor)
        assertNull(closed.chat.promotedFrom)
        assertEquals(SplitNode.Leaf(FIRST_EDITOR), closed.ops(ReturnToFiles).editor)
    }

    @Test fun `in chat files open in the side stack and terminals switch the side to the bottom stack`() {
        val chat = richFiles().ops(SwitchParadigm(Paradigm.CHAT), SetRegionCollapsed(Region.CHAT_SIDE, true), Open(file("new")))
        assertEquals(chat.chat.sideStack, chat.stackOf(chat.id(file("new"))))
        assertFalse(chat.chat.side.collapsed)
        assertEquals(chat.chat.sideStack, chat.focusedStack)
        val t = chat.ops(Open(terminal("t2")))
        assertEquals(BOTTOM, t.chat.sideStack)
        assertEquals(BOTTOM, t.focusedStack)
        assertEquals(Paradigm.CHAT, t.paradigm)
    }

    @Test fun `show conversation replaces the aux conversation and pulls one open elsewhere`() {
        var w = richFiles().ops(SwitchParadigm(Paradigm.CHAT), LayoutOp.showConversation("new"))
        assertEquals(listOf(conversation("x"), conversation("new")), w.targetsIn(AUX))
        assertEquals(AUX, w.focusedStack)
        w = w.ops(Open(conversation("side"), Placement.InStack(w.lastEditorStack)), LayoutOp.showConversation("side"))
        assertEquals(AUX, w.stackOf(w.id(conversation("side"))))
    }

    @Test fun `chat side stack switcher accepts editor stacks and the bottom stack only`() {
        val w = richFiles().ops(SwitchParadigm(Paradigm.CHAT))
        val other = w.editorStacks.first { it != w.chat.sideStack }
        val switched = w.ops(SetChatSideStack(other))
        assertEquals(other, switched.chat.sideStack)
        assertEquals(other, switched.focusedStack)
        assertEquals(other, switched.lastEditorStack)
        assertEquals(BOTTOM, w.ops(SetChatSideStack(BOTTOM)).chat.sideStack)
        assertSame(w, w.apply(SetChatSideStack(AUX)))
    }

    @Test fun `solo shows one panel and opening anything else turns the session into files`() {
        val solo = Workbench.solo(PanelTarget.Proxy()).assertValid()
        assertEquals(Paradigm.SOLO, solo.paradigm)
        assertEquals(PanelTarget.Proxy(), solo.focusedPanel?.target)
        assertSame(solo, solo.apply(Open(PanelTarget.Proxy())))
        val files = solo.ops(Open(file(".workspace/proxy/config.yaml")))
        assertEquals(Paradigm.FILES, files.paradigm)
        assertEquals(listOf(PanelTarget.Proxy(), file(".workspace/proxy/config.yaml")), files.targetsIn(FIRST_EDITOR))
        assertEquals(Paradigm.FILES, solo.ops(ReturnToFiles).paradigm)
        assertEquals(Paradigm.CHAT, solo.ops(SwitchParadigm(Paradigm.CHAT)).paradigm)
    }

    @Test fun `entering solo from chat returns to chat, closing the solo panel leaves solo`() {
        val chat = richFiles().ops(SwitchParadigm(Paradigm.CHAT))
        val solo = chat.ops(EnterSolo(PanelTarget.Settings))
        assertEquals(Paradigm.SOLO, solo.paradigm)
        assertEquals(Paradigm.CHAT, solo.solo!!.returnTo)
        val left = solo.ops(Close(solo.id(PanelTarget.Settings)))
        assertEquals(Paradigm.CHAT, left.paradigm)
        assertEquals(Paradigm.CHAT, solo.ops(Focus(solo.id(file("a")))).paradigm)
        assertEquals(Paradigm.CHAT, solo.ops(EnterSolo(PanelTarget.Proxy())).solo!!.returnTo)
    }

    // ---- Targets and view state ----

    @Test fun `retarget replaces in place or focuses an existing panel`() {
        val w = empty.ops(Open(conversation("a")), Open(file("f")))
        val c = w.id(conversation("a"))
        val switched = w.ops(Retarget(c, conversation("b")))
        assertEquals(conversation("b"), switched.panels.getValue(c).target)
        assertEquals(c, switched.focusedPanel?.id)
        val existing = w.ops(Retarget(c, file("f")))
        assertEquals(conversation("a"), existing.panels.getValue(c).target)
        existing.assertFocused(file("f"))
        assertEquals(PanelTarget.Image("f"), w.ops(Retarget(w.id(file("f")), PanelTarget.Image("f"))).panels.getValue(w.id(file("f"))).target)
    }

    @Test fun `view state updates do not change focus or recency`() {
        val w = empty.ops(Open(file("a")), Open(file("b")))
        val updated = w.ops(UpdateView("p1", PanelView(scrollAnchor = "12", cursor = TextCursor(3, 4), extras = mapOf("wrap" to "true"))))
        assertEquals(w.mru, updated.mru)
        assertEquals(w.focusedPanel, updated.focusedPanel)
        assertEquals(TextCursor(3, 4), updated.panels.getValue("p1").view.cursor)
        assertSame(updated, updated.apply(UpdateView("p1", updated.panels.getValue("p1").view)))
    }

    @Test fun `renames follow files and directories and resolve collisions to the renamed panel`() {
        val w = empty.ops(Open(file("docs/a.md")), Open(PanelTarget.Diff("docs/a.md")), Open(file("docs/sub/b.md")), Open(file("other.md")),
            UpdateExplorer(ExplorerView(listOf("docs", "docs/sub"), "docs/sub/b.md")))
        val renamed = w.ops(RenamePath("docs", "notes"))
        assertEquals(setOf(file("notes/a.md"), PanelTarget.Diff("notes/a.md"), file("notes/sub/b.md"), file("other.md")), renamed.panels.values.map { it.target }.toSet())
        assertEquals(listOf("notes", "notes/sub"), renamed.explorer.expanded)
        assertEquals("notes/sub/b.md", renamed.explorer.selected)
        val collided = w.ops(Focus(w.id(file("other.md"))), RenamePath("docs/a.md", "other.md"))
        assertEquals(1, collided.panels.values.count { it.target == file("other.md") })
        assertEquals(w.id(file("docs/a.md")), collided.id(file("other.md")))
    }

    @Test fun `resources and primary resources follow recency`() {
        val w = empty.ops(Open(file("a")), Open(conversation("c")), Open(terminal("t")), Open(file("b")), Focus("p2"))
        assertEquals(setOf(ResourceRef.File("a"), ResourceRef.File("b"), ResourceRef.Conversation("c")), w.resources)
        assertEquals(listOf(ResourceRef.Conversation("c"), ResourceRef.File("b"), ResourceRef.File("a")), w.primaryResources)
    }

    @Test fun `normalization repairs damaged workbenches`() {
        val damaged = Workbench(
            paradigm = Paradigm.SOLO,
            panels = mapOf("p1" to Panel("p1", file("a")), "p2" to Panel("p2", file("a")), "p3" to Panel("p3", file("../bad")), "p9" to Panel("p9", file("orphan"))),
            stacks = mapOf("s1" to Stack("s1", listOf("p1", "p2", "p1", "ghost"), "ghost"), "s2" to Stack("s2"), BOTTOM to Stack(BOTTOM, listOf("p1"))),
            editor = SplitNode.Split("x", Axis.ROW, listOf(SplitNode.Leaf("s1"), SplitNode.Split("y", Axis.ROW, listOf(SplitNode.Leaf("s2"), SplitNode.Leaf("missing")), listOf(1.0)), SplitNode.Leaf("s1")), listOf(Double.NaN)),
            files = FilesArrangement(focus = "gone", maximized = "gone", explorer = RegionState(Double.POSITIVE_INFINITY)),
            chat = ChatArrangement(sideStack = AUX, focus = "gone", promotedFrom = PanelOrigin("p1", "s1", 0, null)),
            solo = SoloArrangement("p3", Paradigm.SOLO),
            lastEditorStack = "nope",
            explorer = ExplorerView(listOf("ok", "../bad", "ok")),
            mru = listOf("p2", "p1", "ghost", "p2"),
            nextSeq = -5,
        )
        val fixed = LayoutInvariants.normalize(damaged).assertValid("repair")
        assertEquals(listOf("p2"), fixed.stacks.getValue("s1").panels)
        assertEquals(SplitNode.Leaf("s1"), fixed.editor)
        assertEquals(Paradigm.FILES, fixed.paradigm)
        assertEquals(listOf("ok"), fixed.explorer.expanded)
        assertEquals(fixed, LayoutInvariants.normalize(fixed))
    }
}
