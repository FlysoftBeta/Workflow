package top.flysoftbeta.workflow.core.layout

import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import top.flysoftbeta.workflow.core.layout.LayoutOp.*
import top.flysoftbeta.workflow.core.layout.Workbench.Companion.AUX
import top.flysoftbeta.workflow.core.layout.Workbench.Companion.BOTTOM
import kotlin.random.Random

/** Property-style tests: invariants over random op sequences (seeded, reproducible). */
class LayoutPropertyTest {
    @Before fun strict() { LayoutReducer.strict = true }

    companion object {
        private val paths = listOf("a.txt", "b.md", "src/c.kt", "src/d.kt", "img/e.png", "docs/f.md", "docs/sub/g.md")
        private val conversations = listOf("c1", "c2", "c3", "c4")
        private val terminals = listOf("t1", "t2", "t3")

        fun randomTarget(random: Random): PanelTarget = when (random.nextInt(10)) {
            0, 1, 2, 3 -> PanelTarget.File(paths.random(random))
            4 -> PanelTarget.Image(paths.random(random))
            5 -> PanelTarget.Diff(paths.random(random))
            6 -> PanelTarget.Conversation(conversations.random(random))
            7 -> PanelTarget.Terminal(terminals.random(random))
            8 -> PanelTarget.Proxy(ProxyPage.entries.random(random))
            else -> if (random.nextInt(4) == 0) PanelTarget.File("../escape") else PanelTarget.Settings
        }

        fun randomOp(w: Workbench, random: Random): LayoutOp {
            val panelIds = w.panels.keys.toList() + "ghost"
            val stackIds = w.stacks.keys.toList() + "ghost"
            val splitIds = Workbench.splitIds(w.editor) + "ghost"
            fun panel() = panelIds.random(random)
            fun stack() = stackIds.random(random)
            fun edge() = Edge.entries.random(random)
            return when (random.nextInt(24)) {
                0, 1, 2, 3 -> Open(randomTarget(random), focus = random.nextInt(5) != 0)
                4 -> Open(randomTarget(random), Placement.InStack(stack(), random.nextInt(-1, 5).takeIf { it >= 0 }, random.nextBoolean(), random.nextBoolean()))
                5 -> Open(randomTarget(random), Placement.SplitEdge(stack(), edge()))
                6 -> Focus(panel())
                7 -> FocusStack(stack())
                8 -> Close(List(random.nextInt(1, 3)) { panel() })
                9 -> Move(panel(), DropTarget.Center(stack()))
                10 -> Move(panel(), DropTarget.Tab(stack(), random.nextInt(-1, 6)))
                11 -> Move(panel(), DropTarget.Edge(stack(), edge()))
                12 -> Move(panel(), DropTarget.EditorEdge(edge()))
                13 -> SplitStack(stack(), edge())
                14 -> splitIds.random(random).let { id ->
                    val n = w.split(id)?.children?.size ?: 2
                    ResizeSplit(id, List(n) { random.nextDouble(-0.1, 1.0) })
                }
                15 -> ResizeRegion(Region.entries.random(random), random.nextDouble(-100.0, 3000.0))
                16 -> if (random.nextBoolean()) ToggleRegion(Region.entries.random(random)) else SetMaximized(if (random.nextBoolean()) null else stack())
                17 -> SwitchParadigm(Paradigm.entries.random(random))
                18 -> PromoteConversation(panel())
                19 -> ReturnToFiles
                20 -> if (random.nextBoolean()) EnterSolo(randomTarget(random)) else SetChatSideStack(stack())
                21 -> Retarget(panel(), randomTarget(random))
                22 -> UpdateView(panel(), PanelView(scrollAnchor = random.nextInt(100).toString(), cursor = TextCursor(random.nextInt(9), random.nextInt(9))))
                else -> if (random.nextBoolean()) RenamePath(paths.random(random), paths.random(random)) else LayoutOp.showConversation(conversations.random(random))
            }
        }

        fun randomWorkbench(random: Random, steps: Int): Workbench {
            var w = Workbench.empty()
            repeat(steps) { w = w.apply(randomOp(w, random)) }
            return w
        }
    }

    @Test fun `invariants hold after every op of random sequences`() {
        repeat(400) { seed ->
            val random = Random(seed)
            var w = Workbench.empty()
            repeat(80) { step ->
                val op = randomOp(w, random)
                val next = w.apply(op)
                val problems = next.violations()
                assertTrue("seed $seed step $step $op: $problems\nbefore=$w\nafter=$next", problems.isEmpty())
                assertEquals("normalization is idempotent (seed $seed step $step $op)", next, LayoutInvariants.normalize(next))
                w = next
            }
        }
    }

    @Test fun `ops never create duplicates and only open or solo add panels, one at a time`() {
        repeat(300) { seed ->
            val random = Random(seed)
            var w = Workbench.empty()
            repeat(60) {
                val op = randomOp(w, random)
                val next = w.apply(op)
                val keys = next.panels.values.map { it.target.key }
                assertEquals(keys.size, keys.toSet().size)
                val grew = next.panels.size - w.panels.size
                if (op is Open || op is EnterSolo || op is LayoutOp.Retarget) assertTrue(grew <= 1) else assertTrue("$op grew by $grew", grew <= 0)
                if (op !is Close && op !is RenamePath && op !is Retarget && op !is Open) {
                    assertTrue("$op lost panels", next.panels.keys.containsAll(w.panels.keys))
                }
                w = next
            }
        }
    }

    @Test fun `opening a valid target always leaves it open, focused and visible`() {
        repeat(300) { seed ->
            val random = Random(seed)
            val w = randomWorkbench(random, random.nextInt(40))
            val target = randomTarget(random)
            val next = w.apply(Open(target))
            if (!PanelTarget.isValid(target)) { assertSame(w, next); return@repeat }
            val panel = next.panelFor(target)!!
            assertEquals(panel.id, next.focusedPanel?.id)
            assertNotEquals(Paradigm.SOLO.takeIf { next.solo?.panelId != panel.id }, next.paradigm)
            val stack = next.stackOf(panel.id)!!
            when (next.paradigm) {
                Paradigm.FILES -> {
                    if (stack == BOTTOM) assertFalse(next.files.bottom.collapsed)
                    if (stack == AUX) assertFalse(next.files.aux.collapsed)
                    assertTrue(next.files.maximized == null || next.files.maximized == stack)
                }
                Paradigm.CHAT -> assertTrue(stack == AUX || (stack == next.chat.sideStack && !next.chat.side.collapsed))
                Paradigm.SOLO -> assertEquals(panel.id, next.solo!!.panelId)
            }
        }
    }

    @Test fun `files to chat and back restores files for any workbench`() {
        repeat(300) { seed ->
            val random = Random(seed)
            val w = randomWorkbench(random, random.nextInt(60)).apply(SwitchParadigm(Paradigm.FILES))
            if (w.paradigm != Paradigm.FILES) return@repeat
            val back = w.apply(SwitchParadigm(Paradigm.CHAT)).apply(SwitchParadigm(Paradigm.FILES))
            assertEquals("seed $seed", w, back.copy(chat = w.chat))
        }
    }

    @Test fun `promote and return restores stacks, editor tree and files arrangement`() {
        var checked = 0
        repeat(400) { seed ->
            val random = Random(seed)
            val w = randomWorkbench(random, random.nextInt(60)).apply(SwitchParadigm(Paradigm.FILES))
            if (w.paradigm != Paradigm.FILES) return@repeat
            // The header button is on a visible conversation: the active panel of its stack.
            val panel = w.stacks.values.mapNotNull { it.active }.map { w.panels.getValue(it) }
                .firstOrNull { it.target is PanelTarget.Conversation } ?: return@repeat
            val back = w.apply(PromoteConversation(panel.id)).also { assertEquals(Paradigm.CHAT, it.paradigm) }.apply(ReturnToFiles)
            assertEquals("seed $seed stacks", w.stacks, back.stacks)
            assertEquals("seed $seed editor", w.editor, back.editor)
            assertEquals("seed $seed files", w.files, back.files)
            checked++
        }
        assertTrue(checked > 50)
    }

    @Test fun `apply is deterministic`() {
        repeat(50) { seed ->
            assertEquals(randomWorkbench(Random(seed), 80), randomWorkbench(Random(seed), 80))
        }
    }
}
