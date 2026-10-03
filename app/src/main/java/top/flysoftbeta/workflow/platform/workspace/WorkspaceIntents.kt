package top.flysoftbeta.workflow.platform.workspace

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.webkit.MimeTypeMap
import androidx.core.content.FileProvider
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * Workspace files towards other apps and the clipboard: "用其他应用打开", "分享", "复制路径". Paths are
 * workspace-relative; the absolute path is the one the terminal's shell uses (so a copied path can be
 * pasted into the terminal or given to an agent).
 */
object WorkspaceIntents {
    private suspend fun uri(context: Context, path: String): Uri = withContext(Dispatchers.IO) {
        FileProvider.getUriForFile(context, "${context.packageName}.files", WorkspaceResourceCache.stage(context, path))
    }

    fun mimeType(path: String): String =
        MimeTypeMap.getSingleton().getMimeTypeFromExtension(path.substringAfterLast('.', "").lowercase()) ?: "application/octet-stream"

    /** Returns false if the Engine read, bounded cache copy or target application failed. */
    suspend fun openWith(context: Context, path: String): Boolean = attempt {
        val intent = Intent(Intent.ACTION_VIEW).setDataAndType(uri(context, path), mimeType(path))
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
        withContext(Dispatchers.Main) { context.startActivity(Intent.createChooser(intent, null).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) }
    }

    suspend fun share(context: Context, path: String): Boolean = attempt {
        val intent = Intent(Intent.ACTION_SEND).setType(mimeType(path)).putExtra(Intent.EXTRA_STREAM, uri(context, path))
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        withContext(Dispatchers.Main) { context.startActivity(Intent.createChooser(intent, null).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) }
    }

    private suspend fun attempt(action: suspend () -> Unit): Boolean = try {
        action(); true
    } catch (cancelled: CancellationException) { throw cancelled }
    catch (_: Exception) { false }

    /** Absolute paths always use the guest workspace, including while its environment is preparing. */
    fun absolutePath(context: Context, path: String): String =
        top.flysoftbeta.workflow.core.terminal.ShellPaths(top.flysoftbeta.workflow.platform.engine.Guest.WORKSPACE, null).toShell(path)

    fun copyText(context: Context, label: String, text: String) {
        context.getSystemService(ClipboardManager::class.java)?.setPrimaryClip(ClipData.newPlainText(label, text))
    }
}
