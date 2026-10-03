package top.flysoftbeta.workflow.platform.workspace

import java.io.File
import java.util.UUID
import top.flysoftbeta.workflow.core.io.WorkspacePaths

internal data class ResourceChunk(val bytes: ByteArray, val nextOffset: Long, val eof: Boolean, val size: Long)

/** One caller serializes stage operations. Incomplete/failed copies are never handed to a consumer. */
internal class BoundedResourceCache(
    private val directory: File,
    private val budget: Long = 256L * 1024 * 1024,
    private val now: () -> Long = System::currentTimeMillis,
) {
    suspend fun stage(path: String, maxBytes: Long, read: suspend (Long, Int) -> ResourceChunk): File {
        require(WorkspacePaths.isValidEntry(path) && !WorkspacePaths.isReserved(path)) { "文件路径无效" }
        require(maxBytes in 1..budget)
        check(directory.mkdirs() || directory.isDirectory) { "无法创建临时文件目录" }
        val existing = directory.listFiles().orEmpty()
        existing.filter { now() - it.lastModified() >= RETENTION_MS }.forEach { it.deleteRecursively() }
        val used = directory.walkTopDown().filter { it.isFile }.sumOf { it.length() }
        val copy = File(directory, UUID.randomUUID().toString()).apply { check(mkdir()) }
        val target = File(copy, WorkspacePaths.name(path))
        val pending = File(copy, ".download")
        try {
            pending.outputStream().use { output ->
                var offset = 0L
                var expectedSize: Long? = null
                while (true) {
                    val chunk = read(offset, CHUNK_BYTES)
                    require(chunk.bytes.size <= CHUNK_BYTES) { "工作区返回的文件块过大" }
                    require(chunk.size in 0..maxBytes && chunk.size <= budget - used) { "文件超过临时缓存大小限制" }
                    check(expectedSize == null || expectedSize == chunk.size) { "文件在读取时改变，请重试" }
                    expectedSize = chunk.size
                    check(chunk.nextOffset == offset + chunk.bytes.size && chunk.nextOffset <= chunk.size) { "文件读取偏移无效" }
                    check(chunk.eof == (chunk.nextOffset == chunk.size)) { "文件读取结束标记无效" }
                    check(chunk.eof || chunk.bytes.isNotEmpty()) { "文件读取没有进展" }
                    output.write(chunk.bytes)
                    offset = chunk.nextOffset
                    if (chunk.eof) break
                }
            }
            check(pending.renameTo(target)) { "无法完成临时文件" }
            copy.setLastModified(now())
            return target
        } catch (error: Throwable) {
            copy.deleteRecursively()
            throw error
        }
    }

    companion object {
        private const val CHUNK_BYTES = 65_536
        private const val RETENTION_MS = 60 * 60 * 1000L
    }
}
