package top.flysoftbeta.workflow.core.terminal

import org.junit.Assert.*
import org.junit.Test

class TerminalOutputWindowTest {
    @Test fun offsetCountsOnlyActuallyDiscardedCodeUnits() {
        var window = TerminalOutputWindow("", 0)
        window = window.append("abcd", 6)
        assertEquals(TerminalOutputWindow("abcd", 0), window)
        window = window.append("efgh", 6)
        assertEquals(TerminalOutputWindow("cdefgh", 2), window)
        assertEquals(8L, window.endOffset)
        window = window.append("ij", 6)
        assertEquals(TerminalOutputWindow("efghij", 4), window)
        assertEquals(10L, window.endOffset)
    }

    @Test fun unchangedRepeatedWindowStillAdvancesAndProvidesOnlyNewData() {
        val first = TerminalOutputWindow("xxxx", 0)
        val second = first.append("xx", 4)
        assertEquals(first.text, second.text)
        assertNotEquals(first.startOffset, second.startOffset)
        assertEquals("xx", second.text.substring((first.endOffset - second.startOffset).toInt()))
        assertEquals(6L, second.endOffset)
    }

    @Test fun trimmingSurrogateTailCountsTheAdditionalDroppedUnit() {
        val window = TerminalOutputWindow("", 0).append("A😀B", 2)
        assertEquals("B", window.text)
        assertEquals(3L, window.startOffset)
        assertEquals(4L, window.endOffset)
        assertFalse(window.text.first().isLowSurrogate())
    }

    @Test fun continuousConsumerGetsExactlyOnceDeltasAcrossManyWindowSlides() {
        var window = TerminalOutputWindow("", 0)
        var consumed = 0L
        val all = StringBuilder()
        val delivered = StringBuilder()
        repeat(2000) { index ->
            val chunk = if (index % 3 == 0) "\u001b[32m😀" else "same\r\n"
            all.append(chunk)
            window = window.append(chunk, 64)
            assertTrue(consumed >= window.startOffset)
            delivered.append(window.text.substring((consumed - window.startOffset).toInt()))
            consumed = window.endOffset
        }
        assertEquals(all.toString(), delivered.toString())
        assertEquals(all.length.toLong(), consumed)
    }

    @Test fun chunkLargerThanRetentionLeavesAnExplicitGap() {
        val before = TerminalOutputWindow("abc", 0)
        val after = before.append("0123456789", 4)
        assertEquals("6789", after.text)
        assertEquals(9L, after.startOffset)
        assertTrue(before.endOffset < after.startOffset)
        assertEquals(13L, after.endOffset)
    }

    @Test fun emptyOutputAndExitOnlyUpdatesDoNotMoveTheWindow() {
        val window = TerminalOutputWindow("saved", 123)
        assertSame(window, window.append("", 5))
    }

    @Test fun offsetsContinuePastIntegerRangeWithoutWrapping() {
        val before = TerminalOutputWindow("abcd", Int.MAX_VALUE.toLong())
        val after = before.append("ef", 4)
        assertEquals(Int.MAX_VALUE.toLong() + 2, after.startOffset)
        assertEquals(before.endOffset + 2, after.endOffset)
    }

    @Test fun realOneMillionLimitStillProducesOnlyTheNewSuffix() {
        val before = TerminalOutputWindow("x".repeat(1_000_000), 0)
        val after = before.append("\u001b[31mnew", 1_000_000)
        assertEquals(1_000_000, after.text.length)
        assertEquals(8L, after.startOffset)
        assertEquals("\u001b[31mnew", after.text.substring((before.endOffset - after.startOffset).toInt()))
    }
}
