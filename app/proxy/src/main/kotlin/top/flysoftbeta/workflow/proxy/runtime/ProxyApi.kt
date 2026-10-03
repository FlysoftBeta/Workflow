package top.flysoftbeta.workflow.proxy.runtime

import java.io.File
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.StateFlow
import top.flysoftbeta.workflow.proxy.controller.ConnectionsSnapshot
import top.flysoftbeta.workflow.proxy.controller.DelayResult
import top.flysoftbeta.workflow.proxy.controller.MihomoController
import top.flysoftbeta.workflow.proxy.controller.ProviderRefresh
import top.flysoftbeta.workflow.proxy.controller.ProxyLogEntry
import top.flysoftbeta.workflow.proxy.controller.ProxyLogLevel
import top.flysoftbeta.workflow.proxy.controller.ProxyMode
import top.flysoftbeta.workflow.proxy.controller.ProxyProvider
import top.flysoftbeta.workflow.proxy.controller.ProxySnapshot

/**
 * What a proxy screen / ViewModel uses. Every suspend call is main-safe (IO happens on Dispatchers.IO) and
 * returns a [Result]; lifecycle errors are additionally reflected in [state]. Only [start] and [stop] may
 * ask for root.
 */
interface ProxyApi {
    val state: StateFlow<ProxyState>
    /** Disposable kernel staging file. UI editors use the Engine-owned service document path. */
    val configFile: File

    suspend fun refresh(): Result<ProxyState>
    suspend fun ensureConfig(): Result<File>
    suspend fun importConfig(bytes: ByteArray): Result<Unit>
    suspend fun checkConfig(): Result<ProxyConfigCheck>
    suspend fun logTail(maxBytes: Int = 64 * 1024): String

    /** Fails with [ProxyConflictException] when another VPN/TUN refused the start ([ProxyPhase.CONFLICT]). */
    suspend fun start(): Result<Unit>
    suspend fun stop(): Result<Unit>
    fun dismiss()

    suspend fun setMode(mode: ProxyMode): Result<Unit>
    suspend fun select(group: String, node: String): Result<Unit>
    suspend fun refreshGroups(): Result<ProxySnapshot>
    suspend fun testNode(name: String, url: String? = null, timeoutMs: Int = MihomoController.DEFAULT_TIMEOUT_MS): Result<DelayResult>
    suspend fun testGroup(group: String, url: String? = null, timeoutMs: Int = MihomoController.DEFAULT_TIMEOUT_MS): Result<Map<String, DelayResult>>
    suspend fun loadProviders(): Result<List<ProxyProvider>>
    suspend fun refreshProviders(): Result<List<ProviderRefresh>>
    suspend fun connections(): Result<ConnectionsSnapshot>
    fun logs(level: ProxyLogLevel = ProxyLogLevel.INFO): Flow<ProxyLogEntry>
}
