package top.flysoftbeta.workflow.feature.workbench

import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.Workbench
import top.flysoftbeta.workflow.ui.design.dnd.FilesDragPayload
import top.flysoftbeta.workflow.ui.design.dnd.PanelDragPayload

class ResourceDropTest {
    @Test fun editorTabBecomesAnAttachmentWithoutMovingThePanel() {
        val target = PanelTarget.File("notes/a b.txt")
        val wb = Workbench.empty().apply(LayoutOp.Open(target))
        val panel = wb.panelFor(target)!!
        val payload = PanelDragPayload(panel.id, wb.focusedStack, 0, "a b.txt", 0)
        assertEquals(listOf("notes/a b.txt"), (resourceDropPayload(wb, payload) as FilesDragPayload).paths)
        assertEquals(panel, wb.panelFor(target))
    }

    @Test fun terminalAndStaleTabsCannotBecomeAttachments() {
        val wb = Workbench.empty().apply(LayoutOp.Open(PanelTarget.Terminal("t")))
        val panel = wb.panelFor(PanelTarget.Terminal("t"))!!
        assertNull(resourceDropPayload(wb, PanelDragPayload(panel.id, wb.focusedStack, 0, "terminal", 0)))
        assertNull(resourceDropPayload(wb, PanelDragPayload("gone", wb.focusedStack, 0, "gone", 0)))
    }
}
