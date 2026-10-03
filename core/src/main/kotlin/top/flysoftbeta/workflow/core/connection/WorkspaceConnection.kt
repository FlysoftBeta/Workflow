package top.flysoftbeta.workflow.core.connection

import java.io.Closeable
import java.io.InputStream
import java.io.OutputStream

/** Only connection profiles and Engine-delivered local configuration are client-persistent. */
data class WorkspaceConnectionConfig(
    val id: String,
    val name: String,
    val endpoint: WorkspaceEndpoint,
)

sealed interface WorkspaceEndpoint {
    /** The client bootstrapper resolves this opaque id to its app-owned workspace directory. */
    data class Embedded(val workspaceId: String) : WorkspaceEndpoint
    /** Configuration/extension point only: this release has no remote connection implementation. */
    data class Remote(val bootstrap: RemoteBootstrapSpec) : WorkspaceEndpoint
}

data class RemoteBootstrapSpec(val provider: String, val address: String, val workspacePath: String)

interface WorkspaceTransport : Closeable {
    val input: InputStream
    val output: OutputStream
}

/** OS-specific bootstrapping stays outside workspace business logic. */
fun interface WorkspaceBootstrapper {
    suspend fun connect(configuration: WorkspaceConnectionConfig): WorkspaceTransport
}

class UnsupportedWorkspaceTransport(message: String) : IllegalStateException(message)
