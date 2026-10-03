package top.flysoftbeta.workflow.core.io

import java.io.IOException
import java.io.OutputStream

data class FileStat(val isDirectory: Boolean, val size: Long, val modifiedAt: Long)

/**
 * Blocking filesystem port for the reference-store fixtures. Paths are normalized and relative to
 * the workspace ("" = root); implementations reject paths outside it, including through symlinks.
 * Calls block; callers run them on an IO dispatcher. Every method may throw [IOException].
 */
interface FileSystem {
    /** Null when nothing exists at [path]. */
    fun stat(path: String): FileStat?

    fun readBytes(path: String): ByteArray

    /**
     * Crash-safe replace: the content is written to a temporary file, flushed to storage, then
     * renamed over [path] and the directory entry is synced. Readers see the old or the new
     * content, never a mix. Missing parent directories are created. No backup file is left behind.
     */
    fun writeAtomic(path: String, write: (OutputStream) -> Unit)

    /** Direct children, sorted by name. Empty when [path] is not a directory. */
    fun list(path: String): List<FileEntry>

    fun createDirectories(path: String)

    /** Renames [from] to [to]; fails when [to] already exists. */
    fun move(from: String, to: String)

    /** Deletes a file or a directory tree. Returns false when nothing existed. */
    fun delete(path: String): Boolean
}

fun FileSystem.writeAtomic(path: String, bytes: ByteArray) = writeAtomic(path) { it.write(bytes) }
fun FileSystem.writeAtomic(path: String, text: String) = writeAtomic(path, text.toByteArray(Charsets.UTF_8))
fun FileSystem.readText(path: String): String = readBytes(path).toString(Charsets.UTF_8)
fun FileSystem.exists(path: String): Boolean = stat(path) != null
fun FileSystem.readTextOrNull(path: String): String? = if (stat(path)?.isDirectory == false) readText(path) else null
