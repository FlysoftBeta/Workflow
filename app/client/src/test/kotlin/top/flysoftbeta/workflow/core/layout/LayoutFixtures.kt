package top.flysoftbeta.workflow.core.layout

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue

internal fun file(path: String) = PanelTarget.File(path)
internal fun conversation(id: String) = PanelTarget.Conversation(id)
internal fun terminal(id: String) = PanelTarget.Terminal(id)

internal fun Workbench.ops(vararg ops: LayoutOp): Workbench = ops.fold(this) { w, op -> w.apply(op).also { it.assertValid(op) } }

internal fun Workbench.assertValid(context: Any? = null): Workbench {
    val problems = violations()
    assertTrue("invariants broken after $context: $problems\n$this", problems.isEmpty())
    return this
}

internal fun Workbench.id(target: PanelTarget): PanelId = requireNotNull(panelFor(target)) { "$target is not open" }.id

internal fun Workbench.targetsIn(stackId: StackId): List<PanelTarget> = panelsIn(stackId).map { it.target }

internal fun Workbench.assertFocused(target: PanelTarget) {
    assertEquals(id(target), focusedPanel?.id)
}
