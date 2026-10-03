package top.flysoftbeta.workflow.platform.proxy

import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import top.flysoftbeta.workflow.core.connection.WorkspaceRpc
import top.flysoftbeta.workflow.core.json.Json
import top.flysoftbeta.workflow.platform.clientservices.WorkspaceDocuments
import top.flysoftbeta.workflow.proxy.controller.DelayResult
import top.flysoftbeta.workflow.proxy.controller.ProxyGroup
import top.flysoftbeta.workflow.proxy.runtime.ProxyState
import top.flysoftbeta.workflow.proxy.runtime.ProxyWorkspace

/** Canonical files + CAS metadata are Engine-owned; Android sees only documents and acknowledged reports. */
internal class EngineProxyWorkspace(
    private val rpc: WorkspaceRpc,
    private val executorId: String = java.util.UUID.randomUUID().toString(),
    private val readFile: suspend (String, Int) -> ByteArray,
) : ProxyWorkspace {
    private val documents = WorkspaceDocuments(rpc)
    private val registration = Mutex()
    private var epoch: String? = null
    private suspend fun lease(): String = registration.withLock {
        epoch ?: WorkspaceRpc.obj(rpc.request("services.executor.register", mapOf("serviceId" to "proxy", "executorId" to executorId)))
            .let { it["epoch"] as? String ?: error("Engine omitted executor epoch") }.also { epoch = it }
    }
    private suspend fun control(name: String, args: Map<String, Any?> = emptyMap()): Map<String, Any?> =
        WorkspaceRpc.obj(rpc.request("services.command", mapOf("serviceId" to "proxy", "epoch" to lease(), "name" to name, "args" to args)))
    private fun config(value: Any?): ProxyWorkspace.Config = WorkspaceRpc.obj(value).let {
        ProxyWorkspace.Config(it["text"] as? String, (it["revision"] as? Number)?.toLong() ?: error("Engine omitted config revision"))
    }
    override suspend fun ensureConfig(): ProxyWorkspace.Config = config(control("ensureConfig")["config"])
    override suspend fun command(name: String, args: Map<String, Any?>): ProxyWorkspace.Ticket {
        val value = WorkspaceRpc.obj(control(name, args)["operation"])
        return ProxyWorkspace.Ticket(value["id"] as? String ?: error("Engine omitted operation ID"),
            value["name"] as? String ?: error("Engine omitted operation name"), WorkspaceRpc.obj(value["args"]), value["config"]?.let(::config))
    }
    override suspend fun complete(ticket: ProxyWorkspace.Ticket, success: Boolean, measured: ProxyState) {
        val report = proxyReport(measured)
        val response = WorkspaceRpc.obj(rpc.request("services.complete", mapOf("serviceId" to "proxy", "epoch" to lease(), "operationId" to ticket.id, "success" to success, "state" to report)))
        acknowledge(response, report)
    }
    suspend fun retire() { epoch?.let { rpc.request("services.executor.retire", mapOf("serviceId" to "proxy", "epoch" to it)) } }
    private fun acknowledge(response: Map<String, Any?>, report: Map<String, Any?>) {
        check(response["serviceId"] == "proxy" && response["state"] == Json.parse(Json.stringify(report))) { "工作区未确认代理实测状态" }
    }
    private val logs = Mutex()
    private var lastLog: String? = null

    override suspend fun readConfig() = documents.read(NAMESPACE, "config.yaml").let { ProxyWorkspace.Config(it.text, it.revision) }
    override suspend fun writeConfig(text: String, expectedRevision: Long) { control("importConfig", mapOf("text" to text, "expectedRevision" to expectedRevision)) }
    override suspend fun readAsset(path: String): ByteArray =
        readFile("${top.flysoftbeta.workflow.core.io.WorkspacePaths.PROXY}/$path", top.flysoftbeta.workflow.proxy.config.LocalProxyConfig.MAX_CONFIG_BYTES)
    override suspend fun report(measured: ProxyState) {
        val report = proxyReport(measured)
        val response = WorkspaceRpc.obj(rpc.request("services.report", mapOf("serviceId" to "proxy", "epoch" to lease(), "state" to report)))
        acknowledge(response, report)
    }
    override suspend fun writeLog(text: String) = logs.withLock {
        if (text != lastLog) {
            val previous = documents.read(NAMESPACE, "runtime.log")
            control("publishLog", mapOf("text" to text, "expectedRevision" to previous.revision))
            lastLog = text
        }
    }
    companion object { private const val NAMESPACE = "services.proxy" }
}

/** Never send the parsed configuration, controller secret, subscription URLs or test URLs as a report. */
internal fun proxyReport(state: ProxyState): Map<String, Any?> = linkedMapOf(
    "phase" to state.phase.name.lowercase(), "progress" to state.progress, "error" to state.error,
    "stopUnconfirmed" to state.stopUnconfirmed, "pid" to state.pid, "version" to state.version,
    "controllerReachable" to state.controllerReachable, "mode" to state.mode?.wire,
    "capabilities" to mapOf("kernel" to state.capabilities.kernel, "guardian" to state.capabilities.guardian, "root" to state.capabilities.root.name.lowercase()),
    "config" to mapOf("exists" to state.config.exists, "error" to state.config.error),
    "conflict" to mapOf("vpnActive" to state.conflict.vpnActive, "foreignInterfaces" to state.conflict.foreignInterfaces,
        "deviceTaken" to state.conflict.deviceTaken, "ruleCollisions" to state.conflict.ruleCollisions),
    "tun" to state.tun?.let { mapOf("device" to it.device, "up" to it.up, "routed" to it.routed) },
    "groups" to mapOf("groups" to state.groups.groups.map(::groupReport), "global" to state.groups.global?.let(::groupReport)),
    "delays" to state.delays.mapValues { (_, delay) -> when (delay) {
        is DelayResult.Success -> mapOf("kind" to "success", "ms" to delay.ms)
        is DelayResult.Timeout -> mapOf("kind" to "timeout")
        is DelayResult.Failed -> mapOf("kind" to "failed", "message" to delay.message)
    } },
    "testing" to state.testing.toList(),
    "traffic" to state.traffic?.let { mapOf("up" to it.up, "down" to it.down, "upTotal" to it.upTotal, "downTotal" to it.downTotal) },
    "providers" to state.providers.map { mapOf("name" to it.name, "kind" to it.kind.name.lowercase(), "vehicleType" to it.vehicleType, "updatedAt" to it.updatedAt, "size" to it.size) },
)
private fun groupReport(group: ProxyGroup): Map<String, Any?> = mapOf(
    "name" to group.name, "type" to group.type, "now" to group.now, "hidden" to group.hidden, "fixed" to group.fixed,
    "members" to group.members.map { mapOf("name" to it.name, "type" to it.type, "alive" to it.alive, "delayMs" to it.delayMs, "udp" to it.udp, "now" to it.now) },
)
