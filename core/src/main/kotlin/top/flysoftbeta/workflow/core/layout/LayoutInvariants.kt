package top.flysoftbeta.workflow.core.layout

import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.core.layout.Workbench.Companion.AUX
import top.flysoftbeta.workflow.core.layout.Workbench.Companion.BOTTOM
import kotlin.math.abs

/**
 * Normalization (applied after every op and when loading) and the invariant checker used by tests.
 *
 * Rules: invalid or duplicate panels are dropped (per target key the most recently used panel
 * wins); empty editor stacks are removed unless the editor would become empty or the stack is
 * the origin of a promoted conversation; single-child splits collapse; nested splits on the same
 * axis are flattened; weights are finite, >= [MIN_WEIGHT] and sum to 1; bottom and aux always
 * exist; focus, side stack, maximized, promotion and solo references are repaired or cleared.
 */
object LayoutInvariants {
    const val MIN_WEIGHT = 0.05
    private const val EPSILON = 1e-9

    fun normalize(input: Workbench): Workbench {
        var nextSeq = input.nextSeq.coerceAtLeast(1)
        val candidates = input.panels.filter { (id, panel) -> panel.id == id && PanelTarget.isValid(panel.target) }

        // Stack order decides where a panel listed twice stays.
        val editorLeaves = Workbench.leaves(input.editor).filter { it != BOTTOM && it != AUX && it in input.stacks }.distinct()
        val order = editorLeaves + BOTTOM + AUX
        val home = linkedMapOf<PanelId, StackId>()
        for (stackId in order) for (id in input.stacks[stackId]?.panels.orEmpty()) if (id in candidates && id !in home) home[id] = stackId

        // One panel per target key: the most recently used wins, then stack order.
        val rank = input.mru.distinct().withIndex().associate { it.value to it.index }
        val homeOrder = home.keys.withIndex().associate { it.value to it.index }
        val keep = home.keys.groupBy { candidates.getValue(it).target.key }.values.map { group ->
            group.minWith(compareBy<PanelId> { rank[it] ?: Int.MAX_VALUE }.thenBy { homeOrder.getValue(it) })
        }.toSet()
        val panels = candidates.filterKeys { it in keep }
        val mru = input.mru.filter { it in keep }.distinct()

        val stacks = linkedMapOf<StackId, Stack>()
        for (stackId in order) {
            val listed = input.stacks[stackId]?.panels.orEmpty().filter { it in keep && home[it] == stackId }.distinct()
            val previous = input.stacks[stackId]?.active
            val active = when {
                listed.isEmpty() -> null
                previous in listed -> previous
                else -> mru.firstOrNull { it in listed } ?: listed.first()
            }
            stacks[stackId] = Stack(stackId, listed, active)
        }

        // Promotion is valid while its panel sits in aux; its origin stack survives being empty.
        // Files never carries a promotion: returning to Files restores or clears it.
        val promoted = input.chat.promotedFrom?.takeIf {
            input.paradigm != Paradigm.FILES && stacks.getValue(AUX).panels.contains(it.panelId) && it.stackId != AUX
        }
        val keptEmpty = setOfNotNull(promoted?.stackId)
        val nonEmpty = editorLeaves.filter { stacks.getValue(it).panels.isNotEmpty() || it in keptEmpty }
        val keepLeaves = when {
            nonEmpty.isNotEmpty() -> nonEmpty.toSet()
            input.lastEditorStack in editorLeaves -> setOf(input.lastEditorStack)
            editorLeaves.isNotEmpty() -> setOf(editorLeaves.first())
            else -> emptySet()
        }
        val usedSplitIds = mutableSetOf<SplitId>()
        var editor = prune(input.editor, keepLeaves, mutableSetOf(), usedSplitIds) { prefix ->
            val taken = candidates.keys + input.stacks.keys + Workbench.splitIds(input.editor)
            while ("$prefix$nextSeq" in taken || "$prefix$nextSeq" in usedSplitIds) nextSeq++
            "$prefix${nextSeq++}"
        }
        if (editor == null) {
            val taken = candidates.keys + input.stacks.keys
            while ("s$nextSeq" in taken) nextSeq++
            val id = "s${nextSeq++}"
            stacks[id] = Stack(id)
            editor = SplitNode.Leaf(id)
        }
        val editorStacks = Workbench.leaves(editor)
        val finalStacks = linkedMapOf<StackId, Stack>()
        for (id in editorStacks + BOTTOM + AUX) finalStacks[id] = stacks.getValue(id)

        fun stackOf(panelId: PanelId): StackId? = finalStacks.values.firstOrNull { panelId in it.panels }?.id
        fun recentStack(allowed: (StackId) -> Boolean): StackId? = mru.firstNotNullOfOrNull { id -> stackOf(id)?.takeIf(allowed) }

        val lastEditor = input.lastEditorStack.takeIf { it in editorStacks }
            ?: recentStack { it in editorStacks } ?: editorStacks.first()
        val filesFocus = input.files.focus.takeIf { it in finalStacks } ?: recentStack { true } ?: lastEditor
        val sideStack = input.chat.sideStack.takeIf { it in editorStacks || it == BOTTOM } ?: lastEditor
        val chatFocus = when (input.chat.focus) {
            AUX, sideStack -> input.chat.focus
            else -> if (input.chat.focus in finalStacks) sideStack else AUX
        }

        var paradigm = input.paradigm
        var solo = input.solo
        if (solo != null && (solo.panelId !in panels || solo.returnTo == Paradigm.SOLO)) {
            solo = if (solo.panelId in panels) solo.copy(returnTo = Paradigm.FILES) else null
        }
        if (paradigm == Paradigm.SOLO && solo == null) paradigm = input.solo?.returnTo?.takeIf { it != Paradigm.SOLO } ?: Paradigm.FILES
        if (paradigm != Paradigm.SOLO) solo = null

        return Workbench(
            paradigm = paradigm,
            panels = panels,
            stacks = finalStacks,
            editor = editor,
            files = FilesArrangement(
                explorer = region(input.files.explorer, Region.EXPLORER),
                aux = region(input.files.aux, Region.AUX),
                bottom = region(input.files.bottom, Region.BOTTOM),
                focus = filesFocus,
                maximized = input.files.maximized?.takeIf { it in finalStacks },
            ),
            chat = ChatArrangement(
                rail = region(input.chat.rail, Region.CHAT_RAIL),
                side = region(input.chat.side, Region.CHAT_SIDE),
                tree = region(input.chat.tree, Region.CHAT_TREE),
                sideStack = sideStack,
                focus = chatFocus,
                promotedFrom = promoted?.takeIf { it.stackId in finalStacks },
            ),
            solo = solo,
            lastEditorStack = lastEditor,
            explorer = ExplorerView(
                expanded = input.explorer.expanded.filter { WorkspacePaths.isValidEntry(it) }.distinct(),
                selected = input.explorer.selected?.takeIf { WorkspacePaths.isValidEntry(it) },
                showHidden = input.explorer.showHidden,
            ),
            mru = mru,
            nextSeq = nextSeq,
        )
    }

    private fun region(state: RegionState, region: Region): RegionState {
        val size = if (state.size.isFinite()) state.size.coerceIn(region.minSize, region.maxSize) else region.defaultSize
        return if (size == state.size) state else state.copy(size = size)
    }

    private fun prune(
        node: SplitNode,
        keep: Set<StackId>,
        seen: MutableSet<StackId>,
        usedSplitIds: MutableSet<SplitId>,
        freshId: (String) -> String,
    ): SplitNode? = when (node) {
        is SplitNode.Leaf -> node.takeIf { it.stackId in keep && seen.add(it.stackId) }
        is SplitNode.Split -> {
            val weights = if (node.weights.size == node.children.size && node.weights.all { it.isFinite() && it > 0 }) node.weights
            else List(node.children.size) { 1.0 }
            val kept = node.children.zip(weights).mapNotNull { (child, weight) ->
                prune(child, keep, seen, usedSplitIds, freshId)?.let { it to weight }
            }
            when (kept.size) {
                0 -> null
                1 -> kept.single().first
                else -> {
                    val flatChildren = mutableListOf<SplitNode>()
                    val flatWeights = mutableListOf<Double>()
                    for ((child, weight) in kept) {
                        if (child is SplitNode.Split && child.axis == node.axis) {
                            usedSplitIds.remove(child.id)
                            val total = child.weights.sum()
                            child.children.zip(child.weights).forEach { (c, w) -> flatChildren += c; flatWeights += weight * w / total }
                        } else { flatChildren += child; flatWeights += weight }
                    }
                    val id = if (node.id.isNotBlank() && usedSplitIds.add(node.id)) node.id else freshId("x").also { usedSplitIds.add(it) }
                    SplitNode.Split(id, node.axis, flatChildren, normalizeWeights(flatWeights))
                }
            }
        }
    }

    /**
     * Positive weights summing to 1, each at least [MIN_WEIGHT] (or 1/n when there are many).
     * Already-normalized input is returned unchanged, so normalization is idempotent.
     */
    fun normalizeWeights(weights: List<Double>, force: Boolean = false): List<Double> {
        val n = weights.size
        if (n == 0) return weights
        if (weights.any { !it.isFinite() || it <= 0 }) return List(n) { 1.0 / n }
        val floor = minOf(MIN_WEIGHT, 1.0 / n)
        if (!force && abs(weights.sum() - 1.0) < EPSILON && weights.all { it >= floor - EPSILON }) return weights
        val sum = weights.sum()
        val result = weights.map { it / sum }.toMutableList()
        repeat(n) {
            val low = result.indices.filter { result[it] < floor }
            if (low.isEmpty()) return result
            val free = result.indices.filter { it !in low && result[it] > floor }
            val freeSum = free.sumOf { result[it] }
            val remaining = 1.0 - (n - free.size) * floor
            result.indices.forEach { if (it !in free) result[it] = floor }
            if (freeSum > 0) free.forEach { result[it] = result[it] * remaining / freeSum }
        }
        return result
    }

    /** Human-readable invariant violations; empty for a normalized workbench. */
    fun violations(w: Workbench): List<String> {
        val problems = mutableListOf<String>()
        w.panels.forEach { (id, panel) ->
            if (panel.id != id) problems += "panel key $id != id ${panel.id}"
            if (!PanelTarget.isValid(panel.target)) problems += "invalid target ${panel.target}"
        }
        val leaves = Workbench.leaves(w.editor)
        if (leaves.isEmpty()) problems += "editor has no stack"
        if (leaves.size != leaves.distinct().size) problems += "editor lists a stack twice"
        if (BOTTOM in leaves || AUX in leaves) problems += "bottom or aux inside the editor tree"
        if (w.stacks.keys != (leaves + BOTTOM + AUX).toSet()) problems += "stacks ${w.stacks.keys} != editor + bottom + aux"
        val placements = w.stacks.values.flatMap { stack -> stack.panels.map { it to stack.id } }
        if (placements.map { it.first }.distinct().size != placements.size) problems += "a panel is in two stacks"
        if (placements.map { it.first }.toSet() != w.panels.keys) problems += "panels and stack contents differ"
        val keys = w.panels.values.map { it.target.key }
        if (keys.size != keys.distinct().size) problems += "a resource is open twice"
        val promoted = w.chat.promotedFrom
        w.stacks.forEach { (id, stack) ->
            if (stack.id != id) problems += "stack key $id != id ${stack.id}"
            if ((stack.active == null) != stack.panels.isEmpty() || (stack.active != null && stack.active !in stack.panels)) problems += "stack $id active ${stack.active}"
            if (id in leaves && stack.panels.isEmpty() && leaves.size > 1 && id != promoted?.stackId) problems += "empty editor stack $id"
        }
        fun checkSplit(node: SplitNode, parentAxis: Axis?) {
            if (node !is SplitNode.Split) return
            if (node.children.size < 2) problems += "split ${node.id} has ${node.children.size} children"
            if (node.axis == parentAxis) problems += "split ${node.id} nested on the same axis"
            if (node.weights.size != node.children.size) problems += "split ${node.id} weights"
            if (node.weights.any { !it.isFinite() || it < minOf(MIN_WEIGHT, 1.0 / node.children.size) - 1e-6 }) problems += "split ${node.id} weight bounds"
            if (abs(node.weights.sum() - 1.0) > 1e-6) problems += "split ${node.id} weights sum ${node.weights.sum()}"
            node.children.forEach { checkSplit(it, node.axis) }
        }
        checkSplit(w.editor, null)
        val splitIds = Workbench.splitIds(w.editor)
        if (splitIds.size != splitIds.distinct().size) problems += "duplicate split ids"
        if (w.lastEditorStack !in leaves) problems += "lastEditorStack ${w.lastEditorStack}"
        if (w.files.focus !in w.stacks) problems += "files.focus ${w.files.focus}"
        if (w.files.maximized != null && w.files.maximized !in w.stacks) problems += "maximized ${w.files.maximized}"
        if (w.chat.sideStack !in leaves && w.chat.sideStack != BOTTOM) problems += "chat.sideStack ${w.chat.sideStack}"
        if (w.chat.focus != AUX && w.chat.focus != w.chat.sideStack) problems += "chat.focus ${w.chat.focus}"
        if (promoted != null && (w.stackOf(promoted.panelId) != AUX || promoted.stackId !in w.stacks || w.paradigm == Paradigm.FILES)) problems += "stale promotion $promoted"
        if ((w.paradigm == Paradigm.SOLO) != (w.solo != null)) problems += "paradigm ${w.paradigm} with solo ${w.solo}"
        w.solo?.let { if (it.panelId !in w.panels || it.returnTo == Paradigm.SOLO) problems += "solo $it" }
        if (w.mru.distinct().size != w.mru.size || !w.panels.keys.containsAll(w.mru)) problems += "mru ${w.mru}"
        Region.entries.forEach { region ->
            val size = w.regionState(region).size
            if (!size.isFinite() || size < region.minSize || size > region.maxSize) problems += "region $region size $size"
        }
        if (w.explorer.expanded.any { !WorkspacePaths.isValidEntry(it) }) problems += "explorer paths"
        val ids = w.panels.keys + w.stacks.keys + splitIds
        val maxSeq = ids.mapNotNull { it.drop(1).toLongOrNull() }.maxOrNull() ?: 0
        if (w.nextSeq < 1) problems += "nextSeq ${w.nextSeq} (max $maxSeq)"
        return problems
    }
}
