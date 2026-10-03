package top.flysoftbeta.workflow.core.store

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
