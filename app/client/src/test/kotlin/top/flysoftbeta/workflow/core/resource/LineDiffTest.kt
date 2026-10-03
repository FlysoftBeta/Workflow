package top.flysoftbeta.workflow.core.resource

import org.junit.Assert.assertEquals
import org.junit.Test

class LineDiffTest {
    private fun render(left: String, right: String) = LineDiff.diff(LineDiff.lines(left), LineDiff.lines(right)).joinToString("\n") {
        when (it) {
            is DiffLine.Same -> "  ${it.leftNo}/${it.rightNo} ${it.text}"
            is DiffLine.Removed -> "- ${it.leftNo} ${it.text}"
            is DiffLine.Added -> "+ ${it.rightNo} ${it.text}"
        }
    }

    @Test fun `changed middle line is a removal plus an addition with correct numbers`() {
        assertEquals(
            "  1/1 a\n- 2 b\n+ 2 B\n  3/3 c",
            render("a\nb\nc", "a\nB\nc"),
        )
    }

    @Test fun `insertions and deletions keep both sides aligned`() {
        assertEquals("  1/1 a\n+ 2 x\n+ 3 y\n  2/4 b\n- 3 c", render("a\nb\nc", "a\nx\ny\nb"))
        assertEquals("+ 1 new", render("", "new"))
        assertEquals("- 1 old", render("old", ""))
    }

    @Test fun `the result replays to both inputs`() {
        val random = java.util.Random(7)
        repeat(200) {
            val left = List(random.nextInt(30)) { "l${random.nextInt(6)}" }
            val right = List(random.nextInt(30)) { "l${random.nextInt(6)}" }
            val diff = LineDiff.diff(left, right)
            assertEquals(left, diff.mapNotNull { (it as? DiffLine.Same)?.text ?: (it as? DiffLine.Removed)?.text })
            assertEquals(right, diff.mapNotNull { (it as? DiffLine.Same)?.text ?: (it as? DiffLine.Added)?.text })
        }
    }

    @Test fun `too many edits fall back to replace all`() {
        val left = List(50) { "a$it" }
        val right = List(50) { "b$it" }
        val diff = LineDiff.diff(left, right, maxEdits = 10)
        assertEquals(100, diff.size)
        assertEquals(50, diff.count { it is DiffLine.Removed })
    }
}
