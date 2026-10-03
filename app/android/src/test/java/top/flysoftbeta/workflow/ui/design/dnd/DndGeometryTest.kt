package top.flysoftbeta.workflow.ui.design.dnd

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class DndGeometryTest {
    private val size = Size(400f, 200f)

    @Test fun editorOuterBandLeavesChildZonesAndOutsideUntouched() {
        assertEquals(DropZone.Left, DndGeometry.outerEdge(size, Offset(8f, 100f), 12f))
        assertEquals(DropZone.Right, DndGeometry.outerEdge(size, Offset(392f, 100f), 12f))
        assertEquals(DropZone.Top, DndGeometry.outerEdge(size, Offset(200f, 8f), 12f))
        assertEquals(DropZone.Bottom, DndGeometry.outerEdge(size, Offset(200f, 192f), 12f))
        assertEquals(null, DndGeometry.outerEdge(size, Offset(40f, 100f), 12f))
        assertEquals(null, DndGeometry.outerEdge(size, Offset(-1f, 100f), 12f))
        assertEquals(null, DndGeometry.outerEdge(size, Offset(400f, 100f), 12f))
    }

    @Test fun centreKeepsInnerHalf() {
        assertEquals(DropZone.Center, DndGeometry.resolveZone(size, Offset(200f, 100f)))
        // Exactly on the 25% boundary belongs to the centre.
        assertEquals(DropZone.Center, DndGeometry.resolveZone(size, Offset(100f, 50f)))
        assertEquals(DropZone.Center, DndGeometry.resolveZone(size, Offset(300f, 150f)))
    }

    @Test fun edgesCoverOuterQuarter() {
        assertEquals(DropZone.Left, DndGeometry.resolveZone(size, Offset(99f, 100f)))
        assertEquals(DropZone.Right, DndGeometry.resolveZone(size, Offset(301f, 100f)))
        assertEquals(DropZone.Top, DndGeometry.resolveZone(size, Offset(200f, 49f)))
        assertEquals(DropZone.Bottom, DndGeometry.resolveZone(size, Offset(200f, 151f)))
    }

    @Test fun cornersPickNearestNormalizedEdge() {
        // 10% from the left, 20% from the top → left.
        assertEquals(DropZone.Left, DndGeometry.resolveZone(size, Offset(40f, 40f)))
        // 20% from the left, 5% from the top → top.
        assertEquals(DropZone.Top, DndGeometry.resolveZone(size, Offset(80f, 10f)))
        // Exact tie prefers the horizontal edge.
        assertEquals(DropZone.Left, DndGeometry.resolveZone(size, Offset(40f, 20f)))
        assertEquals(DropZone.Right, DndGeometry.resolveZone(size, Offset(360f, 180f)))
    }

    @Test fun outsidePointsClampAndEdgesCanBeDisabled() {
        assertEquals(DropZone.Left, DndGeometry.resolveZone(size, Offset(-50f, 100f)))
        assertEquals(DropZone.Bottom, DndGeometry.resolveZone(size, Offset(200f, 500f)))
        assertEquals(DropZone.Center, DndGeometry.resolveZone(size, Offset(1f, 1f), allowEdges = false))
        assertEquals(DropZone.Center, DndGeometry.resolveZone(Size.Zero, Offset(1f, 1f)))
    }

    @Test fun previewIsResultingHalf() {
        val bounds = Rect(10f, 20f, 410f, 220f)
        assertEquals(bounds, DndGeometry.zonePreview(bounds, DropZone.Center))
        assertEquals(Rect(10f, 20f, 210f, 220f), DndGeometry.zonePreview(bounds, DropZone.Left))
        assertEquals(Rect(210f, 20f, 410f, 220f), DndGeometry.zonePreview(bounds, DropZone.Right))
        assertEquals(Rect(10f, 20f, 410f, 120f), DndGeometry.zonePreview(bounds, DropZone.Top))
        assertEquals(Rect(10f, 120f, 410f, 220f), DndGeometry.zonePreview(bounds, DropZone.Bottom))
    }

    private val tabs = listOf(0f..100f, 100f..180f, 180f..300f)

    @Test fun insertionIndexUsesMidpoints() {
        assertEquals(0, DndGeometry.insertionIndex(tabs, -10f))
        assertEquals(0, DndGeometry.insertionIndex(tabs, 49f))
        assertEquals(1, DndGeometry.insertionIndex(tabs, 51f))
        assertEquals(1, DndGeometry.insertionIndex(tabs, 139f))
        assertEquals(2, DndGeometry.insertionIndex(tabs, 141f))
        assertEquals(3, DndGeometry.insertionIndex(tabs, 250f))
        assertEquals(3, DndGeometry.insertionIndex(tabs, 1000f))
        assertEquals(0, DndGeometry.insertionIndex(emptyList(), 5f))
    }

    @Test fun indicatorSitsInGaps() {
        val gapped = listOf(0f..90f, 100f..170f)
        assertEquals(0f, DndGeometry.insertionIndicatorPosition(gapped, 0), 0f)
        assertEquals(95f, DndGeometry.insertionIndicatorPosition(gapped, 1), 0f)
        assertEquals(170f, DndGeometry.insertionIndicatorPosition(gapped, 2), 0f)
        assertEquals(0f, DndGeometry.insertionIndicatorPosition(emptyList(), 0), 0f)
    }

    @Test fun moveNormalization() {
        assertTrue(DndGeometry.isNoOpMove(from = 1, insertion = 1))
        assertTrue(DndGeometry.isNoOpMove(from = 1, insertion = 2))
        assertFalse(DndGeometry.isNoOpMove(from = 1, insertion = 3))
        assertEquals(2, DndGeometry.targetIndexAfterRemoval(from = 0, insertion = 3))
        assertEquals(0, DndGeometry.targetIndexAfterRemoval(from = 2, insertion = 0))
    }

    @Test fun autoScrollBands() {
        assertEquals(0f, DndGeometry.autoScrollSpeed(150f, 0f, 300f, 32f, 100f), 0f)
        assertEquals(-100f, DndGeometry.autoScrollSpeed(0f, 0f, 300f, 32f, 100f), 0.001f)
        assertEquals(-50f, DndGeometry.autoScrollSpeed(16f, 0f, 300f, 32f, 100f), 0.001f)
        assertEquals(100f, DndGeometry.autoScrollSpeed(300f, 0f, 300f, 32f, 100f), 0.001f)
        assertEquals(0f, DndGeometry.autoScrollSpeed(10f, 0f, 50f, 32f, 100f), 0f) // too small to have bands
    }

    @Test fun targetPickingPrefersPriorityThenSmallest() {
        val stack = Rect(0f, 0f, 400f, 300f)
        val tabRow = Rect(0f, 0f, 400f, 36f)
        val composer = Rect(10f, 200f, 200f, 280f)
        assertEquals(1, DndGeometry.pickTarget(listOf(stack to 0, tabRow to 0), Offset(50f, 10f)))
        assertEquals(0, DndGeometry.pickTarget(listOf(stack to 0, tabRow to 0), Offset(50f, 100f)))
        assertEquals(2, DndGeometry.pickTarget(listOf(stack to 0, tabRow to 0, composer to 0), Offset(50f, 250f)))
        assertEquals(0, DndGeometry.pickTarget(listOf(stack to 5, composer to 0), Offset(50f, 250f)))
        assertEquals(-1, DndGeometry.pickTarget(listOf(stack to 0), Offset(500f, 10f)))
    }

    @Test fun gridSlots() {
        // 4 columns of 100 × 100, 10 items.
        assertEquals(0, DndGeometry.gridSlot(Offset(10f, 10f), 4, 100f, 100f, 10))
        assertEquals(5, DndGeometry.gridSlot(Offset(150f, 150f), 4, 100f, 100f, 10))
        assertEquals(3, DndGeometry.gridSlot(Offset(900f, 10f), 4, 100f, 100f, 10))
        assertEquals(0, DndGeometry.gridSlot(Offset(-20f, -20f), 4, 100f, 100f, 10))
        assertEquals(9, DndGeometry.gridSlot(Offset(350f, 950f), 4, 100f, 100f, 10))
        assertEquals(-1, DndGeometry.gridSlot(Offset(1f, 1f), 4, 100f, 100f, 0))
    }

    @Test fun reorderedIndices() {
        // Move item 1 to slot 3 in [a b c d e]: a c d b e.
        assertEquals(listOf(0, 3, 1, 2, 4), (0..4).map { DndGeometry.reorderedIndex(it, 1, 3) })
        // Move item 3 to slot 0: d a b c e.
        assertEquals(listOf(1, 2, 3, 0, 4), (0..4).map { DndGeometry.reorderedIndex(it, 3, 0) })
    }
}
