package top.flysoftbeta.workflow.platform.connection

import android.content.Context
import android.net.ConnectivityManager
import android.net.LinkProperties
import android.net.Network
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

/** Read-only Android capability adapter; only Engine-owned guest resolver files may change. */
object WorkspaceNetworkReporter {
    fun attach(context: Context, session: WorkspaceConnectionSession) {
        val manager = context.getSystemService(ConnectivityManager::class.java) ?: return
        fun report() = session.scope.launch(Dispatchers.IO) {
            runCatching {
                val active = manager.activeNetwork
                val addresses = active?.let(manager::getLinkProperties)?.dnsServers.orEmpty().mapNotNull { it.hostAddress }.distinct()
                session.rpc.request("services.report", mapOf("serviceId" to "network", "state" to mapOf(
                    "connected" to (active != null), "dnsServers" to addresses,
                )))
            }
        }
        val callback = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) { report() }
            override fun onLost(network: Network) { report() }
            override fun onLinkPropertiesChanged(network: Network, linkProperties: LinkProperties) { report() }
        }
        report()
        if (runCatching { manager.registerDefaultNetworkCallback(callback) }.isSuccess) {
            session.scope.coroutineContext[Job]?.invokeOnCompletion { runCatching { manager.unregisterNetworkCallback(callback) } }
        }
    }
}
