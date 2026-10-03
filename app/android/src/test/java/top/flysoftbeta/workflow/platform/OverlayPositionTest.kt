package top.flysoftbeta.workflow.platform

import org.junit.Assert.assertEquals
import org.junit.Test

class OverlayPositionTest {
    @Test fun corruptAndOutOfBoundsPositionsRemainOnScreen() {
        assertEquals(.42f, OverlayPosition.safeFraction(Float.NaN), 0f)
        assertEquals(.42f, OverlayPosition.safeFraction(Float.POSITIVE_INFINITY), 0f)
        assertEquals(0f, OverlayPosition.safeFraction(-.1f), 0f)
        assertEquals(1f, OverlayPosition.safeFraction(1.5f), 0f)
        assertEquals(.75f, OverlayPosition.safeFraction(.75f), 0f)
    }
}
