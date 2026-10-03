package top.flysoftbeta.workflow.platform.proxy

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import java.io.File
import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.platform.connection.WorkspaceConnectionManager
import top.flysoftbeta.workflow.platform.connection.WorkspaceConnectionSession
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import top.flysoftbeta.workflow.proxy.runtime.WorkspaceProxyApi
import top.flysoftbeta.workflow.platform.service.LocalRuntimeService
import top.flysoftbeta.workflow.proxy.runtime.ForegroundLease
import top.flysoftbeta.workflow.proxy.runtime.GuardianLauncher
import top.flysoftbeta.workflow.proxy.runtime.JavaNetworkProbe
import top.flysoftbeta.workflow.proxy.runtime.ProcessKernelTool
import top.flysoftbeta.workflow.proxy.runtime.ProxyApi
import top.flysoftbeta.workflow.proxy.runtime.ProxyFiles
import top.flysoftbeta.workflow.proxy.runtime.ProxyPorts
import top.flysoftbeta.workflow.proxy.runtime.ProxyRuntime
import top.flysoftbeta.workflow.proxy.runtime.RootShell
import top.flysoftbeta.workflow.proxy.runtime.SuGuardianLauncher
import top.flysoftbeta.workflow.proxy.runtime.SuRootShell

/**
 * Process-scoped owner of the Mihomo kernel, reached through `AppGraph.proxy` (docs/app/proxy.md).
 * Engine commands own business intent; [ProxyRuntime] supplies the local executor mechanics:
 * `su` for root, the packaged guardian/kernel in nativeLibraryDir, ConnectivityManager for VPN detection
 * and the foreground-service lease that keeps the process alive while the kernel runs.
 *
 * Engine owns `.workspace/proxy/config.yaml` and acknowledged service state.
 * Mihomo uses an ephemeral Android cache populated only from Engine-returned configuration.
 */
class ProxyService private constructor(
    private val token: String,
    private val executor: ProxyRuntime,
    api: ProxyApi,
) : ProxyApi by api {
    companion object {
        /** Workspace-relative path of the configuration, for "编辑配置". */
        const val CONFIG_PATH = WorkspacePaths.PROXY + "/config.yaml"
        /** Workspace-relative path of the redacted kernel output. */
        const val LOG_PATH = WorkspacePaths.PROXY + "/runtime.log"

        @Volatile private var instance: ProxyService? = null

        fun get(context: Context): ProxyApi {
            val session = WorkspaceConnectionManager.get(context).requireSession()
            return synchronized(this) {
                instance?.takeIf { it.token == session.token } ?: run {
                    instance?.executor?.closeOwnedChannel()
                    create(context.applicationContext, session).also { instance = it }
                }
            }
        }

        /** Disposable staging and kernel data; canonical workspace files are accessed only by Engine RPC. */
        fun directory(context: Context): File = File(context.cacheDir,
            "proxy-executors/${WorkspaceConnectionManager.get(context).requireSession().profile.id}")
        fun kernel(context: Context): File = File(context.applicationInfo.nativeLibraryDir, "libmihomo.so")
        fun guardian(context: Context): File = File(context.applicationInfo.nativeLibraryDir, "libworkflow_proxy_guard.so")

        private fun create(context: Context, session: WorkspaceConnectionSession): ProxyService {
            val runtime = ProxyRuntime(ProxyFiles(directory(context)), kernel(context), guardian(context), ports(context))
            session.scope.coroutineContext[Job]?.invokeOnCompletion { runtime.closeOwnedChannel() }
            session.scope.launch { session.rpc.failure.filterNotNull().first(); runtime.closeOwnedChannel() }
            return ProxyService(session.token, runtime, WorkspaceProxyApi(runtime, EngineProxyWorkspace(session.rpc, session.token, session.store::readBytes), session.scope))
        }

        internal fun ports(context: Context, rootShell: RootShell = SuRootShell(), launcher: GuardianLauncher = SuGuardianLauncher()) =
            ProxyPorts(rootShell, launcher, ProcessKernelTool(), AndroidNetworkProbe(context.applicationContext), RuntimeServiceLease(context.applicationContext))

    }
}

/** Interfaces via java.net plus Android's own VPN knowledge (TRANSPORT_VPN), all without root. */
internal class AndroidNetworkProbe(context: Context) : JavaNetworkProbe() {
    private val connectivity = context.getSystemService(ConnectivityManager::class.java)
    /** untrusted_app may not search sysfs_net on API 28 (SELinux); probing only produces denials. Use names. */
    override fun tunFlag(name: String): Boolean? = null
    @Suppress("DEPRECATION") // getAllNetworks is the API 28 way to see every network, including other apps' VPNs.
    override fun vpnActive(): Boolean = try {
        connectivity?.allNetworks.orEmpty().any { connectivity?.getNetworkCapabilities(it)?.hasTransport(NetworkCapabilities.TRANSPORT_VPN) == true }
    } catch (_: SecurityException) { false }
}

/** The kernel keeps the process in the foreground; the lease is released when the guardian exits. */
internal class RuntimeServiceLease(private val context: Context) : ForegroundLease {
    override suspend fun retain(id: String): Result<Unit> = LocalRuntimeService.retainProxy(context, id)
    override fun release(id: String) = LocalRuntimeService.releaseProxy(context, id)
}
