package top.flysoftbeta.workflow.platform.workspace

import android.content.Context
import java.io.File
import java.util.Base64
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import top.flysoftbeta.workflow.core.connection.WorkspaceRpc
import top.flysoftbeta.workflow.platform.connection.WorkspaceConnectionManager

/** A bounded transient copy for decoders/FileProvider. No consumer receives a local workspace path. */
object WorkspaceResourceCache {
    const val MAX_IMAGE_BYTES = 32 * 1024 * 1024L
    const val MAX_SHARE_BYTES = 64 * 1024 * 1024L
    private val staging = Mutex()

    suspend fun stage(context: Context, path: String, maxBytes: Long = MAX_SHARE_BYTES): File {
        val session = WorkspaceConnectionManager.get(context).requireSession()
        return withContext(Dispatchers.IO) {
            staging.withLock {
                val cache = BoundedResourceCache(File(context.cacheDir, "workspace-resources"))
                cache.stage(path, maxBytes) { offset, length ->
                    val result = WorkspaceRpc.obj(session.rpc.request("files.read", mapOf("path" to path, "offset" to offset, "length" to length)))
                    ResourceChunk(Base64.getDecoder().decode(result["data"] as? String ?: error("文件数据缺失")),
                        (result["nextOffset"] as? Number)?.toLong() ?: error("文件偏移缺失"),
                        result["eof"] as? Boolean ?: error("文件状态缺失"),
                        (result["size"] as? Number)?.toLong() ?: error("文件大小缺失"))
                }
            }
        }
    }
}
