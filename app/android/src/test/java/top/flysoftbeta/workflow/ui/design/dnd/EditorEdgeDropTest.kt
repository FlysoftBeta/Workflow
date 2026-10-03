package top.flysoftbeta.workflow.ui.design.dnd

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import org.junit.Assert.*
import org.junit.Test

class EditorEdgeDropTest {
    @Test fun outerEdgeWinsOnlyOnPerimeterAndCommitsOnce() {
        val state = DragDropState()
        var outerDrops = 0
        var innerDrops = 0
        val outer = DropTargetEntry("outer").apply {
            bounds = Rect(0f, 0f, 1000f, 600f); size = Size(1000f, 600f)
            kind = DropTargetKind.EditorEdges(12f); priority = 2
            onDrop = { _, result -> assertEquals(DropZone.Right, result.zone); outerDrops++ }
        }
        val child = DropTargetEntry("child").apply {
            bounds = Rect(500f, 0f, 1000f, 600f); origin = Offset(500f, 0f); size = Size(500f, 600f)
            kind = DropTargetKind.Zones()
            onDrop = { _, _ -> innerDrops++ }
        }
        state.targets["outer"] = outer; state.targets["child"] = child
        val payload = PanelDragPayload("file", "editor", 0, "notes", 0)
        state.start(payload, "tab", Offset.Zero, Offset(980f, 300f))
        assertEquals("child", state.session!!.targetKey)
        state.move(Offset(994f, 300f))
        assertEquals("outer", state.session!!.targetKey)
        assertEquals(Rect(500f, 0f, 1000f, 600f), (state.session!!.feedback as DropFeedback.Placement).preview)
        assertTrue(state.drop()); assertFalse(state.drop())
        assertEquals(1, outerDrops); assertEquals(0, innerDrops)
        state.start(payload, "tab", Offset.Zero, Offset(980f, 300f))
        assertTrue(state.drop()); assertEquals(1, innerDrops)
    }
}
