package top.flysoftbeta.workflow.core.store

import top.flysoftbeta.workflow.core.io.FileNames
import top.flysoftbeta.workflow.core.io.FileSystem
import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.core.io.readText
import top.flysoftbeta.workflow.core.io.writeAtomic
import top.flysoftbeta.workflow.core.json.Json
import java.io.IOException

/** A deleted file or folder kept in `.workspace/trash/<id>/` until [TrashPolicy.RETENTION_MS] passes. */
data class TrashEntry(val id: String, val path: String, val trashedAt: Long, val isDirectory: Boolean)

sealed interface TrashResult {
    data class Trashed(val entry: TrashEntry) : TrashResult
    data class Failed(val message: String) : TrashResult
}

sealed interface RestoreResult {
    /** Restored at [path]: the original path, or a "name (n)" variant when that was taken meanwhile. */
    data class Restored(val path: String) : RestoreResult
    data class Failed(val message: String) : RestoreResult
}

/** Retention of deleted files (docs/ui.md §8-5): 7 days, then purged by maintenance. */
object TrashPolicy {
    const val RETENTION_MS: Long = 7L * 24 * 60 * 60 * 1000

    /** An entry is purged once it is at least [retention] old. Entries dated in the future are kept. */
    fun isExpired(trashedAt: Long, now: Long, retention: Long = RETENTION_MS): Boolean = now - trashedAt >= retention
}

/**
 * The trash under `.workspace/trash` (hidden from the explorer, masked in the environment). Each entry is
 * a directory holding `meta.json` (original path, time) and the item under its original name. Used only
 * from the store's actor.
 */
class WorkspaceTrash(private val fs: FileSystem, private val clock: () -> Long, private val newId: () -> String) {
    fun put(path: String): TrashEntry {
        val stat = fs.stat(path) ?: throw IOException("Nothing to delete at $path")
        val now = clock()
        val id = "$now-${newId().filter { it.isLetterOrDigit() }.take(12)}"
        val directory = "$ROOT/$id"
        val entry = TrashEntry(id, path, now, stat.isDirectory)
        fs.createDirectories(directory)
        fs.writeAtomic("$directory/$META", Json.stringify(linkedMapOf(
            "format" to 1, "path" to path, "trashedAt" to now, "directory" to stat.isDirectory,
        ), pretty = true))
        try {
            fs.move(path, "$directory/${WorkspacePaths.name(path)}")
        } catch (error: Exception) {
            runCatching { fs.delete(directory) }
            throw error
        }
        return entry
    }

    fun entry(id: String): TrashEntry? {
        if (!ID.matches(id)) return null
        return runCatching {
            val meta = Json.parse(fs.readText("$ROOT/$id/$META")) as Map<*, *>
            val path = WorkspacePaths.normalize(meta["path"] as String)
            TrashEntry(id, path, (meta["trashedAt"] as Number).toLong(), meta["directory"] == true)
        }.getOrNull()
    }

    /** Moves the item back; returns where it landed. [isFree] decides whether a path may be taken. */
    fun restore(id: String): String {
        val entry = entry(id) ?: throw IOException("Nothing to restore")
        val item = "$ROOT/$id/${WorkspacePaths.name(entry.path)}"
        if (fs.stat(item) == null) throw IOException("Nothing to restore")
        val parent = WorkspacePaths.parent(entry.path)
        if (parent.isNotEmpty() && fs.stat(parent)?.isDirectory != true) fs.createDirectories(parent)
        val taken = fs.list(parent).map { it.name }.toSet()
        val name = FileNames.unique(WorkspacePaths.name(entry.path), taken)
        val target = WorkspacePaths.child(parent, name)
        fs.move(item, target)
        runCatching { fs.delete("$ROOT/$id") }
        return target
    }

    /** Deletes expired entries (and unreadable ones by their directory time). Returns how many were removed. */
    fun purge(now: Long): Int {
        var removed = 0
        for (child in fs.list(ROOT)) {
            if (!child.isDirectory) continue
            val trashedAt = entry(child.name)?.trashedAt ?: child.modifiedAt
            if (TrashPolicy.isExpired(trashedAt, now)) {
                if (runCatching { fs.delete(child.path) }.getOrDefault(false)) removed++
            }
        }
        return removed
    }

    companion object {
        const val ROOT = "${WorkspacePaths.INTERNAL}/trash"
        private const val META = "meta.json"
        private val ID = Regex("[0-9]+-[A-Za-z0-9]{1,12}")
    }
}
