package top.flysoftbeta.workflow.feature.files

import top.flysoftbeta.workflow.core.io.FileEntry
import top.flysoftbeta.workflow.core.io.WorkspacePaths

/** One visible row of the explorer tree. */
internal sealed interface TreeRow {
    val key: String
    val depth: Int

    data class Node(val entry: FileEntry, override val depth: Int, val expanded: Boolean) : TreeRow {
        override val key: String get() = entry.path
    }

    /** Inline "new file / new folder" field shown first in [parent]. */
    data class Creating(val parent: String, val directory: Boolean, override val depth: Int) : TreeRow {
        override val key: String get() = "\u0000new:$parent"
    }
}

/** Pure tree logic of the explorer (unit-tested). */
internal object ExplorerModel {
    /**
     * Rows in display order: loaded children of the root and of every expanded, loaded directory.
     * With [filter], only matching names and their ancestors remain (among loaded entries).
     */
    fun flatten(
        children: Map<String, List<FileEntry>>,
        expanded: Set<String>,
        creating: TreeRow.Creating? = null,
        filter: String = "",
    ): List<TreeRow> {
        val needle = filter.trim().lowercase()
        val keep: Set<String>? = if (needle.isEmpty()) null else buildSet {
            children.values.flatten().filter { it.name.lowercase().contains(needle) }.forEach { match ->
                add(match.path)
                var parent = WorkspacePaths.parent(match.path)
                while (parent.isNotEmpty()) { add(parent); parent = WorkspacePaths.parent(parent) }
            }
        }
        val rows = ArrayList<TreeRow>()
        fun walk(directory: String, depth: Int) {
            if (creating != null && creating.parent == directory) rows += creating.copy(depth = depth)
            for (entry in children[directory].orEmpty()) {
                if (keep != null && entry.path !in keep) continue
                val open = entry.isDirectory && (entry.path in expanded || (keep != null && children.containsKey(entry.path)))
                rows += TreeRow.Node(entry, depth, open)
                if (open) walk(entry.path, depth + 1)
            }
        }
        walk("", 0)
        return rows
    }

    /** Directories that must be expanded to show [path]. */
    fun ancestors(path: String): List<String> {
        val result = ArrayList<String>()
        var parent = WorkspacePaths.parent(path)
        while (parent.isNotEmpty()) { result += parent; parent = WorkspacePaths.parent(parent) }
        return result.asReversed()
    }

    /** The folder new items and uploads go to: the selection if it is a folder, else its parent. */
    fun targetDirectory(selected: String?, isDirectory: (String) -> Boolean?): String {
        if (selected.isNullOrEmpty()) return ""
        return when (isDirectory(selected)) {
            true -> selected
            else -> WorkspacePaths.parent(selected)
        }
    }

    /** Whether [paths] may be moved into [folder]: not into itself, a descendant, or where they already are. */
    fun canMoveInto(paths: List<String>, folder: String): Boolean = paths.isNotEmpty() && paths.all { path ->
        !WorkspacePaths.isWithin(folder, path) && WorkspacePaths.parent(path) != folder
    }
}
