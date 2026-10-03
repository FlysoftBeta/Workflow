package top.flysoftbeta.workflow.platform.workspace

import android.system.ErrnoException
import android.system.Os
import android.system.OsConstants
import top.flysoftbeta.workflow.core.io.JvmFileSystem
import java.io.File

/** [JvmFileSystem] with a real directory fsync, so an atomic rename survives power loss. */
class AndroidFileSystem(root: File) : JvmFileSystem(root) {
    override fun syncDirectory(directory: File) {
        try {
            val fd = Os.open(directory.path, OsConstants.O_RDONLY, 0)
            try { Os.fsync(fd) } finally { Os.close(fd) }
        } catch (_: ErrnoException) {
            // The file data is already synced; a failed directory sync only weakens rename durability.
        }
    }
}
