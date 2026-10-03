package top.flysoftbeta.workflow.platform.workspace

import android.content.Context
import java.io.File

/** Bootstrap identity only. Features must use the Engine store/RPC for workspace resources. */
object WorkspaceLocation {
    fun root(context: Context): File =
        top.flysoftbeta.workflow.platform.connection.WorkspaceConnectionManager.get(context).requireSession().root
}
