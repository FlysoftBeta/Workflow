package top.flysoftbeta.workflow.platform.proxy

import android.content.Context
import java.io.File
import java.util.UUID
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeout
import top.flysoftbeta.workflow.core.connection.WorkspaceConnectionConfig
import top.flysoftbeta.workflow.core.connection.WorkspaceEndpoint
import top.flysoftbeta.workflow.core.connection.WorkspaceRpc
import top.flysoftbeta.workflow.core.store.FileOpResult
import top.flysoftbeta.workflow.platform.clientservices.WorkspaceDocuments
import top.flysoftbeta.workflow.platform.connection.ConnectionStatus
import top.flysoftbeta.workflow.platform.connection.WorkspaceConnectionManager
import top.flysoftbeta.workflow.platform.connection.WorkspaceConnectionSession
import top.flysoftbeta.workflow.proxy.runtime.ProxyApi
import top.flysoftbeta.workflow.proxy.runtime.ProxyFiles
import top.flysoftbeta.workflow.proxy.runtime.ProxyPorts
import top.flysoftbeta.workflow.proxy.runtime.ProxyRuntime
import top.flysoftbeta.workflow.proxy.runtime.ProxyTiming
import top.flysoftbeta.workflow.proxy.runtime.WorkspaceProxyApi

/** Real packaged Rust Engine; only the device root channel and disposable executor are injected. */
internal class EngineProxyFixture private constructor(
    private val context: Context,
    private val manager: WorkspaceConnectionManager,
    val session: WorkspaceConnectionSession,
) {
    val staging = File(context.cacheDir, "proxy-acceptance/${UUID.randomUUID()}")
    val documents = WorkspaceDocuments(session.rpc)
    private var executor: ProxyRuntime? = null

    fun bind(ports: ProxyPorts, timing: ProxyTiming = ProxyTiming()): ProxyApi {
        check(executor == null)
        require(!staging.canonicalPath.startsWith(File(context.cacheDir, "proxy-executors").canonicalPath + File.separator)) {
            "Fixture must not use the production proxy directory"
        }
        val runtime = ProxyRuntime(ProxyFiles(staging), ProxyService.kernel(context), ProxyService.guardian(context), ports, timing)
        executor = runtime
        session.scope.coroutineContext[Job]!!.invokeOnCompletion { runtime.closeOwnedChannel() }
        session.scope.launch { session.rpc.failure.filterNotNull().first(); runtime.closeOwnedChannel() }
        return WorkspaceProxyApi(runtime, EngineProxyWorkspace(session.rpc, session.token, session.store::readBytes), session.scope)
    }

    suspend fun config(text: String) {
        val old = documents.read("services.proxy", "config.yaml")
        documents.write("services.proxy", "config.yaml", text, old.revision)
        check(read(ProxyService.CONFIG_PATH) == text) { "documents and files.read disagree" }
    }

    suspend fun asset(path: String, text: String) {
        val target = ".workspace/services/proxy/$path"
        val bytes = text.toByteArray()
        check(session.store.createFile(target, bytes) == FileOpResult.Done) { "Engine asset upload failed" }
        check(read(target) == text) { "Engine asset readback differs" }
    }

    suspend fun read(path: String): String = session.store.readBytes(path, 16 * 1024 * 1024).toString(Charsets.UTF_8)

    suspend fun serviceState(): Map<String, Any?> {
        val result = WorkspaceRpc.obj(session.rpc.request("services.status"))
        val services = WorkspaceRpc.obj(result["services"])
        return WorkspaceRpc.obj(WorkspaceRpc.obj(services["proxy"])["state"])
    }

    suspend fun disconnect() {
        manager.disconnect()
        withTimeout(15_000) { session.scope.coroutineContext[Job]!!.join() }
    }
    fun close() {
        executor?.closeOwnedChannel()
        manager.disconnect()
        staging.deleteRecursively()
    }

    companion object {
        suspend fun connect(context: Context): EngineProxyFixture {
            val manager = WorkspaceConnectionManager.get(context)
            withTimeout(60_000) { manager.status.first { it !is ConnectionStatus.Loading && it !is ConnectionStatus.Connecting } }
            manager.disconnect()
            val id = "proxy-qa-${UUID.randomUUID()}"
            manager.connect(WorkspaceConnectionConfig(id, "Proxy acceptance", WorkspaceEndpoint.Embedded(id)))
            val status = withTimeout(60_000) { manager.status.first {
                it is ConnectionStatus.Connected && it.session.profile.id == id || it is ConnectionStatus.Failed
            } }
            check(status is ConnectionStatus.Connected) { "Packaged Engine fixture failed: $status" }
            return EngineProxyFixture(context.applicationContext, manager, status.session)
        }
    }
}
