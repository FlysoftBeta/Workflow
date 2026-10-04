package top.flysoftbeta.workflow.feature.terminal

import org.junit.Assert.assertEquals
import org.junit.Test
import top.flysoftbeta.workflow.ui.design.SEAM_AXIS_RATIO
import top.flysoftbeta.workflow.ui.design.SeamClaim
import top.flysoftbeta.workflow.ui.design.seamClaim

/**
 * The seam beside a terminal panel claims only drags along its own axis, so vertical swipes and scrollbar
 * drags in the terminal next to a vertical seam keep reaching the terminal (docs/ux/workbench.md).
 */
class SeamAxisTest {
    private val slop = 8f

    @Test fun nothingIsDecidedWithinTouchSlop() {
        assertEquals(SeamClaim.Undecided, seamClaim(0f, 0f, slop))
        assertEquals(SeamClaim.Undecided, seamClaim(5f, 5f, slop))
        assertEquals(SeamClaim.Undecided, seamClaim(-8f, 0f, slop))
        assertEquals(SeamClaim.Undecided, seamClaim(0f, -8f, slop))
    }

    @Test fun movementAlongTheAxisBeyondSlopIsClaimed() {
        assertEquals(SeamClaim.Claim, seamClaim(9f, 0f, slop))
        assertEquals(SeamClaim.Claim, seamClaim(-9f, 0f, slop))
        assertEquals(SeamClaim.Claim, seamClaim(-20f, 9.9f, slop))
        assertEquals(SeamClaim.Claim, seamClaim(40f, -19f, slop))
    }

    @Test fun steepMovementIsReleasedToTheContentEvenWhenItAlsoPassesSlopAlongTheAxis() {
        // A vertical swipe or scrollbar drag next to a vertical seam.
        assertEquals(SeamClaim.Release, seamClaim(0f, 9f, slop))
        assertEquals(SeamClaim.Release, seamClaim(0f, -40f, slop))
        // One coarse first move past slop on both axes: the former detector claimed this.
        assertEquals(SeamClaim.Release, seamClaim(10f, 30f, slop))
        assertEquals(SeamClaim.Release, seamClaim(-12f, 12f, slop))
        // Just steeper than the ratio.
        assertEquals(SeamClaim.Release, seamClaim(20f, 10.1f, slop))
    }

    @Test fun shallowButShortMovementWaits() {
        assertEquals(SeamClaim.Undecided, seamClaim(7.9f, 3.9f, slop))
        assertEquals(SeamClaim.Undecided, seamClaim(8f, 1f, slop))
    }

    @Test fun theRatioIsAboutTwentySevenDegrees() {
        val degrees = Math.toDegrees(kotlin.math.atan(1.0 / SEAM_AXIS_RATIO))
        assertEquals(26.57, degrees, 0.01)
        assertEquals(SeamClaim.Claim, seamClaim(30f, 14f, slop, ratio = 2f))
        assertEquals(SeamClaim.Release, seamClaim(30f, 14f, slop, ratio = 3f))
    }
}
