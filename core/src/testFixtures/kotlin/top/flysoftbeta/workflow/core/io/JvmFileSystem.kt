package top.flysoftbeta.workflow.core.io

import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.io.OutputStream
import java.nio.channels.FileChannel
import java.nio.file.AtomicMoveNotSupportedException
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.nio.file.StandardOpenOption
import java.util.UUID

/**
 * [FileSystem] over java.io/java.nio, using only APIs available on Android API 26+.
 *
 * Paths are resolved canonically and must stay below [root]; a symlink that points outside is
 * treated as outside. Temporary files are staged in `.workflow/tmp` (same file system as the
 * workspace) so a crash never leaves temp files in user folders; if a rename from there is not
 * possible, a hidden sibling temp file is used instead.
 */
open class JvmFileSystem(private val requestedRoot: File) : FileSystem {
    /** Created and canonicalized on first use, so constructing this never touches the disk. */
    val root: File by lazy {
        if (!requestedRoot.isDirectory && !requestedRoot.mkdirs() && !requestedRoot.isDirectory) throw IOException("Cannot create workspace directory")
        requestedRoot.canonicalFile
    }

    /** The on-disk file for a workspace path (for platform APIs such as image decoding or sharing). */
    fun file(path: String): File = resolve(path)

    override fun stat(path: String): FileStat? {
        val file = resolve(path)
        if (!file.exists()) return null
        return FileStat(file.isDirectory, if (file.isFile) file.length() else 0L, file.lastModified())
    }

    override fun readBytes(path: String): ByteArray = resolve(path).readBytes()

    override fun writeAtomic(path: String, write: (OutputStream) -> Unit) {
        val target = resolve(path)
        if (target == root) throw IOException("Cannot write the workspace root")
        if (target.isDirectory) throw IOException("A directory exists at $path")
        val parent = target.parentFile ?: throw IOException("No parent directory")
        if (!parent.isDirectory && !parent.mkdirs() && !parent.isDirectory) throw IOException("Cannot create ${parent.name}")
        val staging = File(root, WorkspacePaths.TMP)
        val temp = if (staging.isDirectory || staging.mkdirs()) File(staging, "${UUID.randomUUID()}.tmp")
        else File(parent, ".${target.name}.${UUID.randomUUID()}.tmp")
        try {
            writeDurably(temp, write)
            try {
                commit(temp, target)
            } catch (_: AtomicMoveNotSupportedException) {
                // Staging is on another file system: retry next to the target.
                temp.delete()
                val sibling = File(parent, ".${target.name}.${UUID.randomUUID()}.tmp")
                try {
                    writeDurably(sibling, write)
                    commit(sibling, target)
                } finally { sibling.delete() }
            }
            syncDirectory(parent)
        } finally {
            temp.delete()
        }
    }

    private fun writeDurably(file: File, write: (OutputStream) -> Unit) {
        FileOutputStream(file).use { output ->
            write(output)
            output.flush()
            output.fd.sync()
        }
    }

    /** Atomic rename over [target]; a test seam for crash simulation. */
    protected open fun commit(temp: File, target: File) {
        Files.move(temp.toPath(), target.toPath(), StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE)
    }

    /** Makes a rename durable. Android overrides this with `Os.fsync` on the directory. */
    protected open fun syncDirectory(directory: File) {
        try {
            FileChannel.open(directory.toPath(), StandardOpenOption.READ).use { it.force(true) }
        } catch (_: IOException) {
            // Some platforms cannot open directories; the data itself is already synced.
        } catch (_: UnsupportedOperationException) {
        }
    }

    override fun list(path: String): List<FileEntry> {
        val directory = resolve(path)
        if (!directory.isDirectory) return emptyList()
        return directory.listFiles().orEmpty().mapNotNull { child ->
            val canonical = runCatching { child.canonicalFile }.getOrNull() ?: return@mapNotNull null
            if (!isInsideRoot(canonical)) return@mapNotNull null
            val childPath = if (path.isEmpty()) child.name else "$path/${child.name}"
            FileEntry(childPath, canonical.isDirectory, if (canonical.isFile) canonical.length() else 0L, canonical.lastModified())
        }.sortedBy { it.name }
    }

    override fun createDirectories(path: String) {
        val directory = resolve(path)
        if (!directory.isDirectory && !directory.mkdirs() && !directory.isDirectory) throw IOException("Cannot create directory $path")
    }

    override fun move(from: String, to: String) {
        val source = resolve(from)
        val target = resolve(to)
        if (source == root || target == root) throw IOException("Cannot move the workspace root")
        if (!source.exists()) throw IOException("Nothing to move at $from")
        if (target.exists()) throw IOException("Already exists: $to")
        if (target.toPath().startsWith(source.toPath())) throw IOException("Cannot move a directory into itself")
        target.parentFile?.let { if (!it.isDirectory && !it.mkdirs() && !it.isDirectory) throw IOException("Cannot create ${it.name}") }
        try {
            Files.move(source.toPath(), target.toPath(), StandardCopyOption.ATOMIC_MOVE)
        } catch (_: AtomicMoveNotSupportedException) {
            Files.move(source.toPath(), target.toPath())
        }
        source.parentFile?.let(::syncDirectory)
        target.parentFile?.let(::syncDirectory)
    }

    override fun delete(path: String): Boolean {
        val file = resolve(path)
        if (file == root) throw IOException("Cannot delete the workspace root")
        if (!file.exists() && !Files.isSymbolicLink(file.toPath())) return false
        deleteTree(file)
        file.parentFile?.let(::syncDirectory)
        return true
    }

    private fun deleteTree(file: File) {
        // Symlinks are removed, never followed.
        if (file.isDirectory && !Files.isSymbolicLink(file.toPath())) file.listFiles().orEmpty().forEach(::deleteTree)
        if (!file.delete() && file.exists()) throw IOException("Cannot delete ${file.name}")
    }

    private fun resolve(path: String): File {
        val normalized = WorkspacePaths.normalize(path)
        val file = if (normalized.isEmpty()) root else File(root, normalized).canonicalFile
        if (!isInsideRoot(file)) throw IllegalArgumentException("Path is outside the workspace")
        return file
    }

    private fun isInsideRoot(file: File): Boolean = file.toPath().startsWith(root.toPath())
}
