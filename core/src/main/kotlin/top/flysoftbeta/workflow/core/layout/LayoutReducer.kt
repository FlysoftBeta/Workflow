package top.flysoftbeta.workflow.core.layout

import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.core.layout.Workbench.Companion.AUX
import top.flysoftbeta.workflow.core.layout.Workbench.Companion.BOTTOM

internal object LayoutReducer {
    /** Tests set this to surface reducer bugs instead of silently returning the input. */
    @Volatile var strict = false

    fun apply(w: Workbench, op: LayoutOp): Workbench {
        val reduced = try {
            reduce(w, op)
        } catch (error: RuntimeException) {
            if (strict) throw error
            return w
        }
        if (reduced === w) return w
        val normalized = LayoutInvariants.normalize(reduced)
        return if (normalized == w) w else normalized
    }

    private fun reduce(w: Workbench, op: LayoutOp): Workbench = when (op) {
        is LayoutOp.Open -> open(w, op.target, op.placement, op.focus)
        is LayoutOp.Focus -> if (op.panelId in w.panels) reveal(w, op.panelId) else w
        is LayoutOp.FocusStack -> focusStack(w, op.stackId)
        is LayoutOp.Close -> close(w, op.panelIds)
        is LayoutOp.Move -> move(w, op.panelId, op.to)
        is LayoutOp.SplitStack -> {
            val stack = w.stacks[op.stackId]
            if (stack != null && w.canSplit(op.stackId) && stack.panels.size >= 2 && stack.active != null)
                move(w, stack.active, DropTarget.Edge(op.stackId, op.edge)) else w
        }
        is LayoutOp.ResizeSplit -> resizeSplit(w, op.splitId, op.weights)
        is LayoutOp.ResetSplit -> w.split(op.splitId)?.let { resizeSplit(w, op.splitId, List(it.children.size) { 1.0 }) } ?: w
        is LayoutOp.ResizeRegion -> if (!op.size.isFinite()) w else
            withRegion(w, op.region) { it.copy(size = op.size.coerceIn(op.region.minSize, op.region.maxSize)) }
        is LayoutOp.SetRegionCollapsed -> withRegion(w, op.region) { it.copy(collapsed = op.collapsed) }
        is LayoutOp.ToggleRegion -> withRegion(w, op.region) { it.copy(collapsed = !it.collapsed) }
        is LayoutOp.SetMaximized -> setMaximized(w, op.stackId)
        is LayoutOp.SwitchParadigm -> switchParadigm(w, op.paradigm)
        is LayoutOp.PromoteConversation -> promote(w, op.panelId)
        LayoutOp.ReturnToFiles -> returnToFiles(w)
        is LayoutOp.EnterSolo -> enterSolo(w, op.target)
        is LayoutOp.SetChatSideStack -> setChatSideStack(w, op.stackId)
        is LayoutOp.Retarget -> retarget(w, op.panelId, op.target)
        is LayoutOp.UpdateView -> w.panels[op.panelId]?.let { panel ->
            if (panel.view == op.view) w else w.copy(panels = w.panels + (panel.id to panel.copy(view = op.view)))
        } ?: w
        is LayoutOp.UpdateExplorer -> w.copy(explorer = op.explorer)
        is LayoutOp.RenamePath -> renamePath(w, op.from, op.to)
    }

    // ---- Open, focus, reveal ----

    private fun open(input: Workbench, target: PanelTarget, placement: Placement, focus: Boolean): Workbench {
        if (!PanelTarget.isValid(target)) return input
        var w = input
        if (w.paradigm == Paradigm.SOLO && w.solo?.let { w.panels[it.panelId]?.target?.key } != target.key) w = leaveSolo(w)
        val existing = w.panelFor(target)
        if (existing != null) {
            val source = w.stackOf(existing.id)
            if (placement is Placement.InStack && placement.moveExisting && placement.stackId in w.stacks && source != placement.stackId) {
                w = removeFromStack(w, existing.id)
                w = insertIntoStack(w, placement.stackId, existing.id, placement.index)
            }
            return if (focus) reveal(w, existing.id) else w
        }
        when (placement) {
            is Placement.SplitEdge -> if (w.canSplit(placement.stackId)) {
                val (panelId, w1) = newId(w, "p")
                val (stackId, w2) = newId(w1, "s")
                val (splitId, w3) = newId(w2, "x")
                w = w3.copy(
                    panels = w3.panels + (panelId to Panel(panelId, target)),
                    stacks = w3.stacks + (stackId to Stack(stackId, listOf(panelId), panelId)),
                    editor = insertAtStackEdge(w3.editor, placement.stackId, stackId, placement.edge, splitId),
                )
                return if (focus) reveal(w, panelId) else w
            } else if (placement.stackId in w.stacks) {
                return open(w, target, Placement.InStack(placement.stackId), focus)
            }
            is Placement.InStack -> if (placement.stackId in w.stacks) {
                val active = w.activePanel(placement.stackId)
                if (placement.replaceActive && active != null && active.target.kind == target.kind) {
                    w = w.copy(panels = w.panels + (active.id to Panel(active.id, target)))
                    return if (focus) reveal(w, active.id) else w
                }
                return addPanel(w, placement.stackId, target, placement.index, focus)
            }
            Placement.Auto -> Unit
        }
        return addPanel(w, autoStack(w, target), target, null, focus)
    }

    private fun addPanel(input: Workbench, stackId: StackId, target: PanelTarget, index: Int?, focus: Boolean): Workbench {
        val (panelId, w0) = newId(input, "p")
        var w = w0.copy(panels = w0.panels + (panelId to Panel(panelId, target)))
        val wasEmpty = w.stacks.getValue(stackId).panels.isEmpty()
        w = insertIntoStack(w, stackId, panelId, index)
        if (!focus && !wasEmpty) w = setActive(w, stackId, input.stacks.getValue(stackId).active)
        return if (focus) reveal(w, panelId) else w
    }

    private fun autoStack(w: Workbench, target: PanelTarget): StackId = when (target.kind) {
        PanelKind.TERMINAL -> BOTTOM
        PanelKind.CONVERSATION -> AUX
        else -> defaultEditorStack(w)
    }

    private fun defaultEditorStack(w: Workbench): StackId = when (w.paradigm) {
        Paradigm.CHAT -> if (w.isEditorStack(w.chat.sideStack)) w.chat.sideStack else w.lastEditorStack
        else -> if (w.isEditorStack(w.files.focus)) w.files.focus else w.lastEditorStack
    }

    /** Activate [panelId], focus its stack and make it visible in the current paradigm. */
    private fun reveal(input: Workbench, panelId: PanelId): Workbench {
        var w = input
        if (w.paradigm == Paradigm.SOLO && w.solo?.panelId != panelId) w = leaveSolo(w)
        val stackId = w.stackOf(panelId) ?: return w
        w = setActive(w, stackId, panelId)
        w = w.copy(mru = listOf(panelId) + w.mru.filter { it != panelId })
        return focusStackOnly(w, stackId)
    }

    private fun focusStack(input: Workbench, stackId: StackId): Workbench {
        val stack = input.stacks[stackId] ?: return input
        stack.active?.let { return reveal(input, it) }
        var w = input
        if (w.paradigm == Paradigm.SOLO) w = leaveSolo(w)
        return focusStackOnly(w, stackId)
    }

    private fun focusStackOnly(w: Workbench, stackId: StackId): Workbench {
        val lastEditor = if (w.isEditorStack(stackId)) stackId else w.lastEditorStack
        return when (w.paradigm) {
            Paradigm.FILES, Paradigm.SOLO -> w.copy(
                lastEditorStack = lastEditor,
                files = w.files.copy(
                    focus = stackId,
                    bottom = if (stackId == BOTTOM) w.files.bottom.copy(collapsed = false) else w.files.bottom,
                    aux = if (stackId == AUX) w.files.aux.copy(collapsed = false) else w.files.aux,
                    maximized = w.files.maximized?.takeIf { it == stackId },
                ),
            )
            Paradigm.CHAT -> if (stackId == AUX) w.copy(chat = w.chat.copy(focus = AUX)) else w.copy(
                lastEditorStack = lastEditor,
                chat = w.chat.copy(sideStack = stackId, focus = stackId, side = w.chat.side.copy(collapsed = false)),
            )
        }
    }

    // ---- Close and move ----

    private fun close(input: Workbench, ids: List<PanelId>): Workbench {
        val closing = ids.filter { it in input.panels }.distinct()
        if (closing.isEmpty()) return input
        val bottomHadPanels = input.stacks.getValue(BOTTOM).panels.isNotEmpty()
        var w = input
        for (id in closing) w = removeFromStack(w, id)
        w = w.copy(panels = w.panels - closing.toSet(), mru = w.mru.filter { it !in closing })
        if (w.solo?.panelId in closing) w = leaveSolo(w)
        return afterBottomChange(w, bottomHadPanels)
    }

    /** An emptied terminal stack collapses and hands focus back to the editor. */
    private fun afterBottomChange(w: Workbench, bottomHadPanels: Boolean): Workbench {
        if (!bottomHadPanels || w.stacks.getValue(BOTTOM).panels.isNotEmpty()) return w
        return w.copy(
            files = w.files.copy(
                bottom = w.files.bottom.copy(collapsed = true),
                focus = if (w.files.focus == BOTTOM) w.lastEditorStack else w.files.focus,
                maximized = w.files.maximized?.takeIf { it != BOTTOM },
            ),
            chat = if (w.chat.sideStack != BOTTOM) w.chat else w.chat.copy(
                sideStack = w.lastEditorStack,
                focus = if (w.chat.focus == BOTTOM) w.lastEditorStack else w.chat.focus,
            ),
        )
    }

    private fun move(input: Workbench, panelId: PanelId, to: DropTarget): Workbench {
        val source = input.stackOf(panelId) ?: return input
        val bottomHadPanels = input.stacks.getValue(BOTTOM).panels.isNotEmpty()
        val moved = when (to) {
            is DropTarget.Center -> when {
                to.stackId !in input.stacks || to.stackId == source -> return input
                else -> insertIntoStack(removeFromStack(input, panelId), to.stackId, panelId, Int.MAX_VALUE)
            }
            is DropTarget.Tab -> {
                val target = input.stacks[to.stackId] ?: return input
                if (to.stackId == source) {
                    val from = target.panels.indexOf(panelId)
                    var index = to.index.coerceIn(0, target.panels.size)
                    if (index > from) index--
                    if (index == from) return reveal(input, panelId)
                    val reordered = target.panels.toMutableList().apply { removeAt(from); add(index, panelId) }
                    input.copy(stacks = input.stacks + (to.stackId to target.copy(panels = reordered)))
                } else insertIntoStack(removeFromStack(input, panelId), to.stackId, panelId, to.index)
            }
            is DropTarget.Edge -> {
                if (to.stackId !in input.stacks) return input
                if (!input.canSplit(to.stackId)) return move(input, panelId, DropTarget.Center(to.stackId))
                if (to.stackId == source && input.stacks.getValue(source).panels.size == 1) return input
                val (stackId, w1) = newId(input, "s")
                val (splitId, w2) = newId(w1, "x")
                val w3 = removeFromStack(w2, panelId)
                w3.copy(
                    stacks = w3.stacks + (stackId to Stack(stackId, listOf(panelId), panelId)),
                    editor = insertAtStackEdge(w3.editor, to.stackId, stackId, to.edge, splitId),
                )
            }
            is DropTarget.EditorEdge -> {
                val editorStacks = input.editorStacks
                if (editorStacks == listOf(source) && input.stacks.getValue(source).panels.size == 1) return input
                val (stackId, w1) = newId(input, "s")
                val (splitId, w2) = newId(w1, "x")
                val w3 = removeFromStack(w2, panelId)
                w3.copy(
                    stacks = w3.stacks + (stackId to Stack(stackId, listOf(panelId), panelId)),
                    editor = insertAtRootEdge(w3.editor, stackId, to.edge, splitId),
                )
            }
        }
        return afterBottomChange(reveal(moved, panelId), bottomHadPanels)
    }

    private fun removeFromStack(w: Workbench, panelId: PanelId): Workbench {
        val stackId = w.stackOf(panelId) ?: return w
        val stack = w.stacks.getValue(stackId)
        val index = stack.panels.indexOf(panelId)
        val remaining = stack.panels.filter { it != panelId }
        val active = when {
            stack.active != panelId -> stack.active
            remaining.isEmpty() -> null
            else -> w.mru.firstOrNull { it in remaining } ?: remaining[index.coerceAtMost(remaining.size - 1)]
        }
        return w.copy(stacks = w.stacks + (stackId to stack.copy(panels = remaining, active = active)))
    }

    /** Inserts at [index] (clamped), or right after the active panel when null; activates it. */
    private fun insertIntoStack(w: Workbench, stackId: StackId, panelId: PanelId, index: Int?): Workbench {
        val stack = w.stacks.getValue(stackId)
        val position = index?.coerceIn(0, stack.panels.size)
            ?: stack.active?.let { stack.panels.indexOf(it) + 1 } ?: stack.panels.size
        val panels = stack.panels.toMutableList().apply { add(position, panelId) }
        return w.copy(stacks = w.stacks + (stackId to stack.copy(panels = panels, active = panelId)))
    }

    private fun setActive(w: Workbench, stackId: StackId, panelId: PanelId?): Workbench {
        val stack = w.stacks.getValue(stackId)
        return if (stack.active == panelId) w else w.copy(stacks = w.stacks + (stackId to stack.copy(active = panelId)))
    }

    private fun insertAtStackEdge(node: SplitNode, target: StackId, newStack: StackId, edge: Edge, splitId: SplitId): SplitNode = when (node) {
        is SplitNode.Leaf -> if (node.stackId != target) node else {
            val fresh = SplitNode.Leaf(newStack)
            SplitNode.Split(splitId, edge.axis, if (edge.before) listOf(fresh, node) else listOf(node, fresh), listOf(0.5, 0.5))
        }
        is SplitNode.Split -> {
            val index = node.children.indexOfFirst { it is SplitNode.Leaf && it.stackId == target }
            if (index >= 0 && node.axis == edge.axis) {
                val half = node.weights[index] / 2
                val at = if (edge.before) index else index + 1
                val children = node.children.toMutableList().apply { add(at, SplitNode.Leaf(newStack)) }
                val weights = node.weights.toMutableList().apply { set(index, half); add(at, half) }
                node.copy(children = children, weights = weights)
            } else node.copy(children = node.children.map { insertAtStackEdge(it, target, newStack, edge, splitId) })
        }
    }

    private fun insertAtRootEdge(root: SplitNode, newStack: StackId, edge: Edge, splitId: SplitId): SplitNode {
        val fresh = SplitNode.Leaf(newStack)
        if (root is SplitNode.Split && root.axis == edge.axis) {
            val n = root.children.size
            val scaled = root.weights.map { it * n / (n + 1) }
            val share = 1.0 / (n + 1)
            return if (edge.before) root.copy(children = listOf(fresh) + root.children, weights = listOf(share) + scaled)
            else root.copy(children = root.children + fresh, weights = scaled + share)
        }
        return SplitNode.Split(splitId, edge.axis, if (edge.before) listOf(fresh, root) else listOf(root, fresh), listOf(0.5, 0.5))
    }

    // ---- Sizes and regions ----

    private fun resizeSplit(w: Workbench, splitId: SplitId, weights: List<Double>): Workbench {
        val split = w.split(splitId) ?: return w
        if (weights.size != split.children.size || weights.any { !it.isFinite() || it <= 0.0 }) return w
        val updated = split.copy(weights = LayoutInvariants.normalizeWeights(weights, force = true))
        return w.copy(editor = replaceSplit(w.editor, updated))
    }

    private fun replaceSplit(node: SplitNode, updated: SplitNode.Split): SplitNode = when (node) {
        is SplitNode.Leaf -> node
        is SplitNode.Split -> if (node.id == updated.id) updated else node.copy(children = node.children.map { replaceSplit(it, updated) })
    }

    private fun withRegion(w: Workbench, region: Region, change: (RegionState) -> RegionState): Workbench = when (region) {
        Region.EXPLORER -> w.copy(files = w.files.copy(explorer = change(w.files.explorer)))
        Region.AUX -> w.copy(files = w.files.copy(aux = change(w.files.aux)))
        Region.BOTTOM -> w.copy(files = w.files.copy(bottom = change(w.files.bottom)))
        Region.CHAT_RAIL -> w.copy(chat = w.chat.copy(rail = change(w.chat.rail)))
        Region.CHAT_SIDE -> w.copy(chat = w.chat.copy(side = change(w.chat.side)))
        Region.CHAT_TREE -> w.copy(chat = w.chat.copy(tree = change(w.chat.tree)))
    }

    private fun setMaximized(w: Workbench, stackId: StackId?): Workbench = when {
        stackId == null -> w.copy(files = w.files.copy(maximized = null))
        stackId !in w.stacks -> w
        else -> {
            val focused = if (w.paradigm == Paradigm.FILES) focusStack(w, stackId) else w
            focused.copy(files = focused.files.copy(maximized = stackId))
        }
    }

    // ---- Paradigms ----

    private fun switchParadigm(w: Workbench, paradigm: Paradigm): Workbench = when (paradigm) {
        Paradigm.SOLO -> w
        Paradigm.FILES -> when (w.paradigm) {
            Paradigm.FILES -> w
            Paradigm.CHAT -> returnToFiles(w)
            Paradigm.SOLO -> leaveSolo(w, Paradigm.FILES)
        }
        Paradigm.CHAT -> when (w.paradigm) {
            Paradigm.CHAT -> w
            Paradigm.FILES -> enterChat(w)
            Paradigm.SOLO -> enterChat(leaveSolo(w, Paradigm.FILES))
        }
    }

    private fun enterChat(w: Workbench): Workbench {
        // Show Files' last focused editor stack; if a promotion just emptied it, prefer a stack with content.
        val side = if (w.stacks[w.lastEditorStack]?.panels?.isNotEmpty() == true) w.lastEditorStack else
            w.mru.firstNotNullOfOrNull { id -> w.stackOf(id)?.takeIf { w.isEditorStack(it) } } ?: w.lastEditorStack
        return w.copy(paradigm = Paradigm.CHAT, chat = w.chat.copy(sideStack = side, focus = AUX))
    }

    private fun promote(input: Workbench, panelId: PanelId): Workbench {
        val panel = input.panels[panelId] ?: return input
        if (panel.target !is PanelTarget.Conversation) return input
        var w = if (input.paradigm == Paradigm.SOLO) leaveSolo(input) else input
        val source = w.stackOf(panelId) ?: return input
        var origin = w.chat.promotedFrom.takeIf { w.paradigm == Paradigm.CHAT }
        if (source != AUX) {
            origin = PanelOrigin(panelId, source, w.stacks.getValue(source).panels.indexOf(panelId), w.stacks.getValue(AUX).active)
            w = insertIntoStack(removeFromStack(w, panelId), AUX, panelId, null)
        }
        if (w.paradigm != Paradigm.CHAT) w = enterChat(w)
        w = w.copy(chat = w.chat.copy(promotedFrom = origin))
        return reveal(w, panelId)
    }

    private fun returnToFiles(input: Workbench): Workbench {
        if (input.paradigm == Paradigm.SOLO) return leaveSolo(input, Paradigm.FILES)
        if (input.paradigm != Paradigm.CHAT) return input
        return restorePromotion(input.copy(paradigm = Paradigm.FILES))
    }

    /** Puts a promoted conversation back where it came from and clears the promotion. */
    private fun restorePromotion(input: Workbench): Workbench {
        val origin = input.chat.promotedFrom ?: return input
        var w = input.copy(chat = input.chat.copy(promotedFrom = null))
        if (w.stackOf(origin.panelId) == AUX && origin.stackId in w.stacks && origin.stackId != AUX) {
            w = removeFromStack(w, origin.panelId)
            val auxStack = w.stacks.getValue(AUX)
            if (origin.auxActiveBefore != null && origin.auxActiveBefore in auxStack.panels) w = setActive(w, AUX, origin.auxActiveBefore)
            w = insertIntoStack(w, origin.stackId, origin.panelId, origin.index)
        }
        return w
    }

    private fun enterSolo(input: Workbench, target: PanelTarget): Workbench {
        if (!PanelTarget.isValid(target)) return input
        val returnTo = if (input.paradigm == Paradigm.SOLO) input.solo?.returnTo ?: Paradigm.FILES else input.paradigm
        val base = if (input.paradigm == Paradigm.SOLO) input.copy(paradigm = returnTo, solo = null) else input
        val opened = open(base, target, Placement.Auto, focus = true)
        val panel = opened.panelFor(target) ?: return input
        return opened.copy(paradigm = Paradigm.SOLO, solo = SoloArrangement(panel.id, returnTo))
    }

    private fun leaveSolo(w: Workbench, to: Paradigm? = null): Workbench {
        val destination = to ?: w.solo?.returnTo ?: Paradigm.FILES
        val left = w.copy(paradigm = if (destination == Paradigm.SOLO) Paradigm.FILES else destination, solo = null)
        return if (left.paradigm == Paradigm.CHAT) enterChat(left.copy(paradigm = Paradigm.FILES)) else restorePromotion(left)
    }

    private fun setChatSideStack(w: Workbench, stackId: StackId): Workbench {
        if (!w.isEditorStack(stackId) && stackId != BOTTOM) return w
        return w.copy(
            lastEditorStack = if (w.isEditorStack(stackId)) stackId else w.lastEditorStack,
            chat = w.chat.copy(
                sideStack = stackId,
                focus = if (w.paradigm == Paradigm.CHAT) stackId else w.chat.focus,
                side = w.chat.side.copy(collapsed = false),
            ),
        )
    }

    // ---- Targets ----

    private fun retarget(w: Workbench, panelId: PanelId, target: PanelTarget): Workbench {
        val panel = w.panels[panelId] ?: return w
        if (!PanelTarget.isValid(target)) return w
        if (panel.target.key == target.key) {
            val updated = if (panel.target == target) w else w.copy(panels = w.panels + (panelId to panel.copy(target = target)))
            return reveal(updated, panelId)
        }
        w.panelFor(target)?.let { return reveal(w, it.id) }
        return reveal(w.copy(panels = w.panels + (panelId to Panel(panelId, target))), panelId)
    }

    private fun renamePath(w: Workbench, rawFrom: String, rawTo: String): Workbench {
        val from = WorkspacePaths.normalizeOrNull(rawFrom)?.takeIf { it.isNotEmpty() } ?: return w
        val to = WorkspacePaths.normalizeOrNull(rawTo)?.takeIf { it.isNotEmpty() } ?: return w
        if (from == to) return w
        var changed = false
        val panels = w.panels.mapValues { (_, panel) ->
            val target = when (val t = panel.target) {
                is PanelTarget.File -> WorkspacePaths.rebase(t.path, from, to)?.let { PanelTarget.File(it) }
                is PanelTarget.Image -> WorkspacePaths.rebase(t.path, from, to)?.let { PanelTarget.Image(it) }
                is PanelTarget.Diff -> WorkspacePaths.rebase(t.path, from, to)?.let { PanelTarget.Diff(it) }
                else -> null
            }
            if (target == null) panel else { changed = true; panel.copy(target = target) }
        }
        val explorer = ExplorerView(
            expanded = w.explorer.expanded.map { WorkspacePaths.rebase(it, from, to) ?: it },
            selected = w.explorer.selected?.let { WorkspacePaths.rebase(it, from, to) ?: it },
            showHidden = w.explorer.showHidden,
        )
        if (!changed && explorer == w.explorer) return w
        // Renamed panels win over stale panels that already had the new path (normalization keeps
        // the most recently used panel per key), so only a collision reorders the MRU list.
        val renamedIds = panels.filter { (id, panel) -> panel != w.panels[id] }.keys
        val collision = panels.values.groupBy { it.target.key }.values.any { it.size > 1 }
        val mru = if (collision) renamedIds.toList() + w.mru.filter { it !in renamedIds } else w.mru
        return w.copy(panels = panels, explorer = explorer, mru = mru)
    }

    // ---- Ids ----

    /** Deterministic ids from [Workbench.nextSeq]; skips ids that are already in use. */
    fun newId(w: Workbench, prefix: String): Pair<String, Workbench> {
        val used = w.panels.keys + w.stacks.keys + Workbench.splitIds(w.editor)
        var seq = w.nextSeq.coerceAtLeast(1)
        while ("$prefix$seq" in used) seq++
        return "$prefix$seq" to w.copy(nextSeq = seq + 1)
    }
}
