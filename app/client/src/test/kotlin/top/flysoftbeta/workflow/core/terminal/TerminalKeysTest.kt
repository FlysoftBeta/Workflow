package top.flysoftbeta.workflow.core.terminal

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class TerminalKeysTest {
    private val esc = "\u001b"

    @Test fun `cursor keys have normal and application variants and modifier parameters`() {
        assertEquals(KeySequence("$esc[A", "${esc}OA"), TerminalKeys.encode(TerminalKey.UP))
        assertEquals(KeySequence("$esc[1;5D"), TerminalKeys.encode(TerminalKey.LEFT, ctrl = true))
        assertEquals(KeySequence("$esc[1;3C"), TerminalKeys.encode(TerminalKey.RIGHT, alt = true))
        assertEquals(KeySequence("$esc[1;7B"), TerminalKeys.encode(TerminalKey.DOWN, ctrl = true, alt = true))
        assertEquals(KeySequence("$esc[H", "${esc}OH"), TerminalKeys.encode(TerminalKey.HOME))
        assertEquals(KeySequence("$esc[F", "${esc}OF"), TerminalKeys.encode(TerminalKey.END))
    }

    @Test fun `tilde and function keys`() {
        assertEquals("$esc[5~", TerminalKeys.encode(TerminalKey.PAGE_UP).normal)
        assertEquals("$esc[6;5~", TerminalKeys.encode(TerminalKey.PAGE_DOWN, ctrl = true).normal)
        assertEquals("$esc[3~", TerminalKeys.encode(TerminalKey.DELETE).normal)
        assertEquals("${esc}OP", TerminalKeys.encode(TerminalKey.F1).normal)
        assertEquals("$esc[1;5S", TerminalKeys.encode(TerminalKey.F4, ctrl = true).normal)
        assertEquals("$esc[15~", TerminalKeys.encode(TerminalKey.F5).normal)
        assertEquals("$esc[24~", TerminalKeys.encode(TerminalKey.F12).normal)
    }

    @Test fun `plain keys and alt prefix`() {
        assertEquals(esc, TerminalKeys.encode(TerminalKey.ESCAPE).normal)
        assertEquals("\t", TerminalKeys.encode(TerminalKey.TAB).normal)
        assertEquals("$esc\t", TerminalKeys.encode(TerminalKey.TAB, alt = true).normal)
        assertEquals("\r", TerminalKeys.encode(TerminalKey.ENTER).normal)
        assertEquals("\u007f", TerminalKeys.encode(TerminalKey.BACKSPACE).normal)
        assertEquals("\b", TerminalKeys.encode(TerminalKey.BACKSPACE, ctrl = true).normal)
    }

    @Test fun `latched modifiers apply to typed characters`() {
        assertEquals("\u0003", TerminalKeys.applyModifiers("c", ctrl = true, alt = false))
        assertEquals("\u0003", TerminalKeys.applyModifiers("C", ctrl = true, alt = false))
        assertEquals("${esc}b", TerminalKeys.applyModifiers("b", ctrl = false, alt = true))
        assertEquals("$esc\u0018", TerminalKeys.applyModifiers("x", ctrl = true, alt = true))
        assertEquals("\u001f", TerminalKeys.applyModifiers("/", ctrl = true, alt = false))
        // Multi-character IME commits are not turned into control characters.
        assertEquals("你好", TerminalKeys.applyModifiers("你好", ctrl = true, alt = false))
        assertEquals("|", TerminalKeys.applyModifiers("|", ctrl = true, alt = false))
        assertEquals("x", TerminalKeys.applyModifiers("x", ctrl = false, alt = false))
    }

    @Test fun `scrollback keeps a bounded window and reports resets`() {
        val scrollback = TerminalScrollback(capacity = 1024)
        scrollback.append("hello\n")
        assertEquals(ScrollbackSlice("hello\n", false, 6), scrollback.since(0))
        assertEquals(ScrollbackSlice("lo\n", false, 6), scrollback.since(3))
        repeat(400) { scrollback.append("line $it\n") }
        assertTrue(scrollback.endOffset - scrollback.startOffset <= 1024 + 256)
        val behind = scrollback.since(0)
        assertTrue(behind.reset)
        assertTrue(behind.text.startsWith("line ")) // cut at a line boundary
        val caughtUp = scrollback.since(scrollback.endOffset)
        assertEquals("", caughtUp.text)
        scrollback.clear()
        assertEquals(scrollback.startOffset, scrollback.endOffset)
    }
}
