package top.flysoftbeta.workflow.core.resource

import top.flysoftbeta.workflow.core.io.Hashing

/** A resource that can own a Working Resource (unsaved content). */
sealed interface ResourceRef {
    val key: String

    /** A workspace file, identified by its normalized workspace-relative path. */
    data class File(val path: String) : ResourceRef {
        override val key: String get() = "file:$path"
    }

    /** A conversation, identified by the app-owned conversation id (AgentHub / conversation index). */
    data class Conversation(val id: String) : ResourceRef {
        override val key: String get() = "conversation:$id"
    }
}

/**
 * A version of a file on disk. [sha256] is null when the file was too large to hash.
 */
data class DiskVersion(
    val exists: Boolean,
    val size: Long = 0,
    val modifiedAt: Long = 0,
    val sha256: String? = null,
) {
    /** Same content: by hash when both hashes are known, otherwise by size and mtime. */
    fun sameContent(other: DiskVersion): Boolean = when {
        !exists && !other.exists -> true
        exists != other.exists -> false
        sha256 != null && other.sha256 != null -> sha256 == other.sha256
        else -> size == other.size && modifiedAt == other.modifiedAt && size >= 0
    }

    companion object {
        val MISSING = DiskVersion(exists = false)
        fun of(bytes: ByteArray, modifiedAt: Long) = DiskVersion(true, bytes.size.toLong(), modifiedAt, Hashing.sha256(bytes))
    }
}

/**
 * Unsaved text of a workspace file. One draft per path, shared by every session.
 * [base] is the disk version the edits started from; a draft conflicts when the disk has moved
 * away from [base]. [revision] increases with every accepted edit.
 */
data class FileDraft(
    val path: String,
    val text: String,
    val base: DiskVersion,
    val editedAt: Long,
    val revision: Long = 1,
) {
    val textSha256: String by lazy(LazyThreadSafetyMode.PUBLICATION) { Hashing.sha256(text) }
}

enum class FileStatus {
    /** No draft. */
    CLEAN,
    /** Draft based on the current disk version. */
    DIRTY,
    /** Draft, and the file changed on disk since the draft's base: keep mine / take disk / compare. */
    CONFLICT,
    /** Draft, and the file was deleted on disk. Keeping mine re-creates it on save. */
    DELETED,
}

enum class ConflictResolution { KEEP_MINE, TAKE_DISK }

/** Pure draft rules; the store applies them and persists the results. */
object Drafts {
    fun status(draft: FileDraft?, disk: DiskVersion?): FileStatus = when {
        draft == null -> FileStatus.CLEAN
        disk == null -> FileStatus.DIRTY
        !disk.exists && draft.base.exists -> FileStatus.DELETED
        !disk.sameContent(draft.base) -> FileStatus.CONFLICT
        else -> FileStatus.DIRTY
    }

    /**
     * Applies editor text. The base of an existing draft never changes here: the first edit fixes
     * the version the user was looking at ([shown]), so an external write before the first
     * keystroke is still detected. Returns null when the text equals the base content again.
     */
    fun edit(existing: FileDraft?, path: String, text: String, shown: DiskVersion, now: Long): FileDraft? {
        if (existing != null && existing.text == text) return existing
        val base = existing?.base ?: shown
        val candidate = FileDraft(path, text, base, now, (existing?.revision ?: 0) + 1)
        return if (base.exists && base.sha256 != null && candidate.textSha256 == base.sha256) null else candidate
    }

    /** After the disk changed: a draft whose text is now exactly on disk is no longer unsaved. */
    fun afterDiskChange(draft: FileDraft, disk: DiskVersion): FileDraft? =
        if (disk.exists && disk.sha256 != null && disk.sha256 == draft.textSha256) null else draft

    /** "Keep mine": the draft now intends to replace the current disk version. */
    fun rebase(draft: FileDraft, disk: DiskVersion, now: Long): FileDraft =
        draft.copy(base = disk, editedAt = now, revision = draft.revision + 1)
}
