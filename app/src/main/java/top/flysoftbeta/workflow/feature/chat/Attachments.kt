package top.flysoftbeta.workflow.feature.chat

import androidx.compose.ui.graphics.ImageBitmap
import java.time.LocalDate
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** Where composer imports go: a hidden, dated folder in the workspace (visible with "显示隐藏文件"). */
fun attachmentDirectory(today: LocalDate = LocalDate.now()): String = ".attachments/$today"

/** Small decoded preview for the attachment strip (56dp). Blocking IO off the main thread. */
suspend fun loadThumbnail(services: ChatFeatureServices, path: String, sizePx: Int): ImageBitmap? = withContext(Dispatchers.IO) {
    val data = runCatching { services.readResource(path, 8 * 1024 * 1024) }.getOrNull() ?: return@withContext null
    decodeThumbnail(data, sizePx)
}


fun isImagePath(path: String): Boolean = path.substringAfterLast('.', "").lowercase() in setOf("png", "jpg", "jpeg", "gif", "webp", "bmp")
