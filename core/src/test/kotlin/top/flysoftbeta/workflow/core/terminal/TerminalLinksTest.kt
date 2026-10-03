package top.flysoftbeta.workflow.core.terminal

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class TerminalLinksTest {
    private val hostShell = ShellPaths("/data/user/0/top.flysoftbeta.workflow/files/.workspace", "/data/user/0/top.flysoftbeta.workflow/files/.workspace")
    private val debian = ShellPaths("/workspace", "/home/work")

    @Test fun `parse strips punctuation and splits line and column`() {
        assertEquals(FileReference("./settings.gradle.kts"), TerminalLinks.parse("./settings.gradle.kts"))
        assertEquals(FileReference("src/Main.kt", 12, 5), TerminalLinks.parse("src/Main.kt:12:5"))
        assertEquals(FileReference("src/Main.kt", 12), TerminalLinks.parse("src/Main.kt:12:"))
        assertEquals(FileReference("a.py", 3, 9), TerminalLinks.parse("a.py(3, 9)"))
        assertEquals(FileReference("notes.md"), TerminalLinks.parse("(notes.md),"))
        assertEquals(FileReference("x.txt"), TerminalLinks.parse("'x.txt'"))
        assertEquals(FileReference("/workspace/a.md", 2), TerminalLinks.parse("file:///workspace/a.md:2"))
        assertNull(TerminalLinks.parse("https://example.com/a.md"))
        assertTrue(TerminalLinks.isUrl("https://example.com/x?y=1"))
        assertFalse(TerminalLinks.isUrl("example.com"))
    }

    @Test fun `relative paths resolve against the shell cwd then the workspace root`() {
        val cwd = "${hostShell.workspaceRoot}/app"
        assertEquals(listOf("app/src/Main.kt", "src/Main.kt"),
            TerminalLinks.candidates(FileReference("src/Main.kt"), cwd, hostShell))
        assertEquals(listOf("settings.gradle.kts"), TerminalLinks.candidates(FileReference("./settings.gradle.kts"), hostShell.workspaceRoot, hostShell))
        assertEquals(listOf("README.md"), TerminalLinks.candidates(FileReference("../README.md"), cwd, hostShell))
        // Unknown cwd: the workspace root.
        assertEquals(listOf("qa.md"), TerminalLinks.candidates(FileReference("qa.md"), null, debian))
        // Git diff prefixes.
        assertEquals(listOf("a/core/X.kt", "core/X.kt"), TerminalLinks.candidates(FileReference("a/core/X.kt"), "/workspace", debian))
    }

    @Test fun `absolute and home paths map into the workspace or are dropped`() {
        assertEquals(listOf("docs/ui.md"), TerminalLinks.candidates(FileReference("/workspace/docs/ui.md"), "/tmp", debian))
        assertEquals(emptyList<String>(), TerminalLinks.candidates(FileReference("/etc/passwd"), "/workspace", debian))
        assertEquals(emptyList<String>(), TerminalLinks.candidates(FileReference("~/.bashrc"), "/workspace", debian))
        assertEquals(listOf("notes.md"), TerminalLinks.candidates(FileReference("~/notes.md"), null, hostShell))
        // Escaping the workspace through ".." is outside.
        assertEquals(emptyList<String>(), TerminalLinks.candidates(FileReference("../../etc/x"), "/workspace", debian))
        // App-internal state is never opened from a link.
        assertEquals(emptyList<String>(), TerminalLinks.candidates(FileReference(".workspace/state/sessions.json"), "/workspace", debian))
        assertEquals(listOf(".workspace/services/proxy/config.yaml"),
            TerminalLinks.candidates(FileReference(".workspace/services/proxy/config.yaml"), "/workspace", debian))
    }

    @Test fun `cursor is zero based`() {
        assertEquals(11 to 4, TerminalLinks.cursor(FileReference("a", 12, 5)))
        assertEquals(0 to 0, TerminalLinks.cursor(FileReference("a", 0)))
        assertNull(TerminalLinks.cursor(FileReference("a")))
    }

    @Test fun `shell quoting leaves safe paths and single-quotes the rest`() {
        assertEquals("docs/ui.md", ShellQuote.quote("docs/ui.md"))
        assertEquals("'my notes.md'", ShellQuote.quote("my notes.md"))
        assertEquals("'it'\\''s.txt'", ShellQuote.quote("it's.txt"))
        assertEquals("'\$HOME'", ShellQuote.quote("\$HOME"))
        assertEquals("'-rf'", ShellQuote.quote("-rf"))
        assertEquals("'~x'", ShellQuote.quote("~x"))
        assertEquals("'中文.md'", ShellQuote.quote("中文.md"))
        assertEquals("''", ShellQuote.quote(""))
    }

    @Test fun `pasted paths are relative below the cwd and absolute elsewhere`() {
        assertEquals("docs/a.md 'b c.md' ", ShellQuote.pasteText(listOf("docs/a.md", "b c.md"), "/workspace", debian))
        assertEquals("a.md ", ShellQuote.pasteText(listOf("docs/a.md"), "/workspace/docs", debian))
        assertEquals("/workspace/README.md ", ShellQuote.pasteText(listOf("README.md"), "/workspace/docs", debian))
        assertEquals(". ", ShellQuote.pasteText(listOf("docs"), "/workspace/docs/", debian))
        assertEquals("/workspace/x ", ShellQuote.pasteText(listOf("x"), null, debian))
    }

    @Test fun `shell paths normalise dot segments`() {
        assertEquals("docs", debian.toWorkspace("/workspace/./docs/../docs/"))
        assertEquals("", debian.toWorkspace("/workspace"))
        assertNull(debian.toWorkspace("/workspaces/x"))
        assertNull(debian.toWorkspace("relative"))
        assertEquals("/workspace/a", debian.toShell("a"))
    }
}
