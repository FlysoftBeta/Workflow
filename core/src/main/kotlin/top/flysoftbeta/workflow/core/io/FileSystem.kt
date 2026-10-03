package top.flysoftbeta.workflow.core.io

import java.io.IOException
import java.io.OutputStream
import java.security.MessageDigest

data class FileStat(val isDirectory: Boolean, val size: Long, val modifiedAt: Long)

data class FileEntry(val path: String, val isDirectory: Boolean, val size: Long, val modifiedAt: Long) {
    val name: String get() = WorkspacePaths.name(path)
}

/**
 * Port for all workspace IO. Paths are normalized workspace-relative paths ("" = root);
 * implementations reject paths that resolve outside the root (including through symlinks).
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

object Hashing {
    fun sha256(bytes: ByteArray): String = MessageDigest.getInstance("SHA-256").digest(bytes).toHex()
    fun sha256(text: String): String = sha256(text.toByteArray(Charsets.UTF_8))

    private val digits = "0123456789abcdef".toCharArray()
    private fun ByteArray.toHex(): String {
        val out = CharArray(size * 2)
        forEachIndexed { index, byte ->
            val value = byte.toInt() and 0xff
            out[index * 2] = digits[value ushr 4]
            out[index * 2 + 1] = digits[value and 0x0f]
        }
        return String(out)
    }
}
