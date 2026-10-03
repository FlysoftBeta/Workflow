package top.flysoftbeta.workflow.platform.pty

import android.content.Context
import java.io.File

/**
 * The app-owned workspace root shared by the editor, terminals and agents (`filesDir/.workspace`).
 * Terminals run through [top.flysoftbeta.workflow.platform.engine.EngineTerminalBackend] (the `TerminalBackend` port); this class only remains as
 * the workspace locator used by other platform code.
 */
class LocalRuntime private constructor(context: Context) {
    val workspace: File = File(context.applicationContext.filesDir, ".workspace").canonicalFile

    companion object {
        @Volatile private var instance: LocalRuntime? = null
        fun get(context: Context): LocalRuntime = instance ?: synchronized(this) {
            instance ?: LocalRuntime(context.applicationContext).also { instance = it }
        }
    }
}
