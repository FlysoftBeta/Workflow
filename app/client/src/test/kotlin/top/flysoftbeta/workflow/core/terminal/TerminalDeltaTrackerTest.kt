package top.flysoftbeta.workflow.core.terminal

import org.junit.Assert.*
import org.junit.Test

class TerminalDeltaTrackerTest {
    @Test fun slidingAndIdenticalWindowsStillAppendOnlyNewCharacters() {
        val tracker = TerminalDeltaTracker()
        assertEquals(TerminalPatch(true, "aaaa"), tracker.update("pty", 0, "aaaa"))
        assertEquals(TerminalPatch(false, "aa"), tracker.update("pty", 2, "aaaa"))
        assertEquals(TerminalPatch(false, "b"), tracker.update("pty", 3, "aaab"))
        assertNull(tracker.update("pty", 3, "aaab"))
    }
    @Test fun streamSwitchActualGapAndReloadResetOnce() {
        val tracker = TerminalDeltaTracker()
        tracker.update("first", 0, "abc")
        assertEquals(TerminalPatch(true, "def"), tracker.update("first", 9, "def"))
        assertEquals(TerminalPatch(false, "g"), tracker.update("first", 10, "efg"))
        assertEquals(TerminalPatch(true, "other"), tracker.update("second", 0, "other"))
        tracker.clear()
        assertEquals(TerminalPatch(true, "other"), tracker.update("second", 0, "other"))
    }
    @Test fun longOffsetsAndUtf16SurrogatesUseTheSamePositionsAsTheProducer() {
        val tracker = TerminalDeltaTracker()
        val start = Int.MAX_VALUE.toLong() + 100
        tracker.update("pty", start, "x🚀")
        assertEquals(TerminalPatch(false, "中"), tracker.update("pty", start + 1, "🚀中"))
    }
}
