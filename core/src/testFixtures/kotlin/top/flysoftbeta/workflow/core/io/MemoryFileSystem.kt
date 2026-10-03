package top.flysoftbeta.workflow.core.io

import java.io.ByteArrayOutputStream
import java.io.FileNotFoundException
import java.io.IOException
import java.io.OutputStream

/**
 * In-memory [FileSystem] for tests and UI previews. Writes are atomic (a failing write leaves
 * the previous content). [failWrite] injects IO errors; [writeLog] records committed writes.
 */
class MemoryFileSystem(private val clock: () -> Long = { 0L }) : FileSystem {
    private class Node(val directory: Boolean, val bytes: ByteArray = ByteArray(0), val modifiedAt: Long)

    private val nodes = sortedMapOf<String, Node>("" to Node(true, modifiedAt = 0))
    private val log = mutableListOf<String>()

    /** Return true to make the write of that path fail before it is committed. */
    @Volatile var failWrite: (String) -> Boolean = { false }
    /** Explicit modification time for the next writes; null uses the clock. */
    @Volatile var nextModifiedAt: Long? = null

    val writeLog: List<String> @Synchronized get() = log.toList()
    @Synchronized fun clearWriteLog() = log.clear()
    @Synchronized fun paths(): List<String> = nodes.keys.filter { it.isNotEmpty() }

    @Synchronized
    override fun stat(path: String): FileStat? = nodes[key(path)]?.let { FileStat(it.directory, it.bytes.size.toLong(), it.modifiedAt) }

    @Synchronized
    override fun readBytes(path: String): ByteArray {
        val node = nodes[key(path)] ?: throw FileNotFoundException(path)
        if (node.directory) throw IOException("Is a directory: $path")
        return node.bytes.copyOf()
    }

    override fun writeAtomic(path: String, write: (OutputStream) -> Unit) {
        val target = key(path)
        if (target.isEmpty()) throw IOException("Cannot write the workspace root")
        val buffer = ByteArrayOutputStream()
        write(buffer)
        synchronized(this) {
            if (failWrite(target)) throw IOException("Injected write failure: $target")
            if (nodes[target]?.directory == true) throw IOException("A directory exists at $path")
            ensureParents(target)
            nodes[target] = Node(false, buffer.toByteArray(), nextModifiedAt ?: clock())
            log += target
        }
    }

    @Synchronized
    override fun list(path: String): List<FileEntry> {
        val directory = key(path)
        if (nodes[directory]?.directory != true) return emptyList()
        return nodes.filterKeys { it.isNotEmpty() && it != directory && WorkspacePaths.parent(it) == directory }
            .map { (childPath, node) -> FileEntry(childPath, node.directory, node.bytes.size.toLong(), node.modifiedAt) }
            .sortedBy { it.name }
    }

    @Synchronized
    override fun createDirectories(path: String) {
        val directory = key(path)
        if (nodes[directory]?.directory == false) throw IOException("A file exists at $path")
        ensureParents(directory)
        nodes.getOrPut(directory) { Node(true, modifiedAt = clock()) }
    }

    @Synchronized
    override fun move(from: String, to: String) {
        val source = key(from)
        val target = key(to)
        if (source.isEmpty() || target.isEmpty()) throw IOException("Cannot move the workspace root")
        if (source !in nodes) throw IOException("Nothing to move at $from")
        if (target in nodes) throw IOException("Already exists: $to")
        if (WorkspacePaths.isWithin(target, source)) throw IOException("Cannot move a directory into itself")
        ensureParents(target)
        val moved = nodes.filterKeys { WorkspacePaths.isWithin(it, source) }
        moved.keys.forEach { nodes.remove(it) }
        moved.forEach { (oldPath, node) -> nodes[WorkspacePaths.rebase(oldPath, source, target)!!] = node }
    }

    @Synchronized
    override fun delete(path: String): Boolean {
        val target = key(path)
        if (target.isEmpty()) throw IOException("Cannot delete the workspace root")
        if (target !in nodes) return false
        nodes.keys.filter { WorkspacePaths.isWithin(it, target) }.forEach { nodes.remove(it) }
        return true
    }

    private fun ensureParents(path: String) {
        var parent = WorkspacePaths.parent(path)
        val missing = mutableListOf<String>()
        while (parent.isNotEmpty() && parent !in nodes) { missing += parent; parent = WorkspacePaths.parent(parent) }
        if (nodes[parent]?.directory == false) throw IOException("Not a directory: $parent")
        missing.forEach { nodes[it] = Node(true, modifiedAt = clock()) }
    }

    private fun key(path: String): String = WorkspacePaths.normalize(path)
}
