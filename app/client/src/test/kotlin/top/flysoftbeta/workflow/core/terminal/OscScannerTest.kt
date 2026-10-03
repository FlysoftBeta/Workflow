package top.flysoftbeta.workflow.core.terminal

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class OscScannerTest {
    @Test fun `titles and directories are reported across chunk boundaries`() {
        val seen = mutableListOf<Pair<Int, String>>()
        val scanner = OscScanner { code, payload -> seen += code to payload }
        scanner.feed("hello \u001b]0;vim no")
        scanner.feed("tes.md\u0007 more \u001b]7;file://host/workspace/my%20dir\u001b")
        scanner.feed("\\ tail \u001b[31mred\u001b[0m")
        assertEquals(listOf(0 to "vim notes.md", 7 to "file://host/workspace/my%20dir"), seen)
    }

    @Test fun `malformed sequences are ignored`() {
        val seen = mutableListOf<Int>()
        val scanner = OscScanner { code, _ -> seen += code }
        scanner.feed("\u001b]x;y\u0007\u001b]2;ok\u0007\u001bc")
        assertEquals(listOf(2), seen)
    }

    @Test fun `osc 7 payloads decode to absolute paths`() {
        assertEquals("/workspace/my dir", OscScanner.cwdOf("file://host/workspace/my%20dir"))
        assertEquals("/workspace/中文", OscScanner.cwdOf("file:///workspace/%E4%B8%AD%E6%96%87"))
        assertNull(OscScanner.cwdOf("http://x/y"))
        assertNull(OscScanner.cwdOf("file://host"))
    }
}
