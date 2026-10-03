package top.flysoftbeta.workflow.feature.files

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import top.flysoftbeta.workflow.core.io.FileEntry

class ExplorerModelTest {
    // Synthetic paths exercise tree operations; they are not source navigation links.
    private fun dir(path: String) = FileEntry(path, true, 0, 0)
    private fun file(path: String) = FileEntry(path, false, 1, 0)

    private val children = mapOf(
        "" to listOf(dir("app"), dir("docs"), file("README.md")),
        "app" to listOf(dir("app/src"), file("app/build.gradle.kts")),
        "app/src" to listOf(file("app/src/Main.kt")),
        "docs" to listOf(file("docs/guide.md")),
    )

    private fun render(rows: List<TreeRow>) = rows.joinToString("|") { row ->
        when (row) {
            is TreeRow.Node -> "${"  ".repeat(row.depth)}${row.entry.name}${if (row.expanded) "/" else ""}"
            is TreeRow.Creating -> "${"  ".repeat(row.depth)}<new>"
        }
    }

    @Test fun `only expanded loaded directories contribute rows, nested by depth`() {
        assertEquals("app|docs|README.md", render(ExplorerModel.flatten(children, emptySet())))
        assertEquals("app/|  src|  build.gradle.kts|docs|README.md", render(ExplorerModel.flatten(children, setOf("app"))))
        assertEquals("app/|  src/|    Main.kt|  build.gradle.kts|docs|README.md", render(ExplorerModel.flatten(children, setOf("app", "app/src"))))
        // An expanded directory below a collapsed one stays hidden.
        assertEquals("app|docs|README.md", render(ExplorerModel.flatten(children, setOf("app/src"))))
    }

    @Test fun `inline creation appears first in its folder`() {
        val rows = ExplorerModel.flatten(children, setOf("docs"), TreeRow.Creating("docs", false, 0))
        assertEquals("app|docs/|  <new>|  guide.md|README.md", render(rows))
        assertEquals("<new>|app|docs|README.md", render(ExplorerModel.flatten(children, emptySet(), TreeRow.Creating("", true, 0))))
    }

    @Test fun `filter keeps matches and their loaded ancestors`() {
        assertEquals("app/|  src/|    Main.kt", render(ExplorerModel.flatten(children, emptySet(), filter = "main")))
        assertEquals("docs/|  guide.md", render(ExplorerModel.flatten(children, emptySet(), filter = "GUIDE.")))
        assertEquals("", render(ExplorerModel.flatten(children, emptySet(), filter = "zzz")))
    }

    @Test fun `ancestors and target directory`() {
        assertEquals(listOf("a", "a/b"), ExplorerModel.ancestors("a/b/c.txt"))
        assertEquals(emptyList<String>(), ExplorerModel.ancestors("c.txt"))
        val isDir = { path: String -> path == "a" || path == "a/b" }
        assertEquals("a/b", ExplorerModel.targetDirectory("a/b", isDir))
        assertEquals("a/b", ExplorerModel.targetDirectory("a/b/c.txt", isDir))
        assertEquals("", ExplorerModel.targetDirectory(null, isDir))
    }

    @Test fun `moves into self, descendants or the current parent are refused`() {
        assertTrue(ExplorerModel.canMoveInto(listOf("docs/guide.md"), "app"))
        assertTrue(ExplorerModel.canMoveInto(listOf("app/src"), ""))
        assertFalse(ExplorerModel.canMoveInto(listOf("app"), "app"))
        assertFalse(ExplorerModel.canMoveInto(listOf("app"), "app/src"))
        assertFalse(ExplorerModel.canMoveInto(listOf("docs/guide.md"), "docs"))
        assertFalse(ExplorerModel.canMoveInto(emptyList(), "docs"))
    }
}
