package top.flysoftbeta.workflow.proxy.controller

import java.io.IOException
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import top.flysoftbeta.workflow.proxy.config.ProxyControllerSettings
import top.flysoftbeta.workflow.proxy.redact.redactSecret

/**
 * Typed Mihomo 1.19 controller API for one running kernel. Mutations are read back and verified;
 * everything else is read-only. No endpoint that edits the configuration (PUT /configs, /upgrade,
 * /restart, DELETE /connections) is exposed.
 */
class MihomoController(
    private val settings: ProxyControllerSettings,
    private val client: MihomoControllerClient = MihomoControllerClient(),
) {
    val endpoint: String get() = settings.endpoint

    suspend fun version(): String {
        val body = ControllerJson.parse(client.callChecked(settings, "GET", "/version"))
        return (body["version"] as? JsonPrimitive)?.content ?: throw IOException("代理控制接口没有返回版本")
    }

    /** Raw mode string from `GET /configs` (may be a mode the app does not switch, e.g. from a newer kernel). */
    suspend fun mode(): String? = ControllerJson.mode(client.callChecked(settings, "GET", "/configs"))

    suspend fun setMode(mode: ProxyMode) {
        client.callChecked(settings, "PATCH", "/configs", buildJsonObject { put("mode", mode.wire) }.toString())
        val actual = mode()
        if (!mode.wire.equals(actual, ignoreCase = true)) throw IOException("控制接口未确认模式变更")
    }

    suspend fun proxies(): ProxySnapshot = ControllerJson.proxies(client.callChecked(settings, "GET", "/proxies"))

    suspend fun select(group: String, node: String) {
        require(group.isNotBlank() && node.isNotBlank()) { "代理组或节点名称为空" }
        val path = MihomoControllerClient.proxyPath(group)
        client.callChecked(settings, "PUT", path, buildJsonObject { put("name", node) }.toString())
        val now = ControllerJson.parse(client.callChecked(settings, "GET", path))["now"] as? JsonPrimitive
        if (now?.content != node) throw IOException("控制接口未确认节点切换")
    }

    /** `GET /proxies/{name}/delay`. The kernel records the result in the node's history as well. */
    suspend fun nodeDelay(name: String, url: String = DEFAULT_TEST_URL, timeoutMs: Int = DEFAULT_TIMEOUT_MS): DelayResult {
        val response = client.call(settings, "GET", "${MihomoControllerClient.proxyPath(name)}/delay?${query(url, timeoutMs)}", timeoutMs = timeoutMs.toLong())
        return delayResult(response) { ControllerJson.delay(it)?.let(DelayResult::Success) ?: DelayResult.Failed("延迟测试没有返回结果") }
    }

    /**
     * `GET /group/{name}/delay`: tests every member concurrently. Members that failed are absent from the
     * map. Note Mihomo clears a url-test/fallback group's manual pin when this runs.
     */
    suspend fun groupDelay(name: String, url: String = DEFAULT_TEST_URL, timeoutMs: Int = DEFAULT_TIMEOUT_MS): Map<String, DelayResult> {
        val group = proxies().group(name) ?: throw ControllerException(404, "控制接口找不到该代理组")
        val response = client.call(settings, "GET", "/group/${MihomoControllerClient.encode(name)}/delay?${query(url, timeoutMs)}", timeoutMs = timeoutMs.toLong())
        val measured = when (val result = delayResult(response) { DelayResult.Success(0) }) {
            is DelayResult.Success -> ControllerJson.groupDelays(response.body)
            else -> return group.members.associate { it.name to result }
        }
        return group.members.associate { member -> member.name to (measured[member.name]?.let(DelayResult::Success) ?: DelayResult.Timeout) }
    }

    /** One sample per second while collected. */
    fun traffic(): Flow<TrafficSample> = client.lines(settings, "/traffic").map(ControllerJson::traffic)

    /** Kernel log lines at or above [level]; payloads are secret-redacted. */
    fun logs(level: ProxyLogLevel = ProxyLogLevel.INFO): Flow<ProxyLogEntry> = client.lines(settings, "/logs?level=${level.wire}")
        .map { line -> ControllerJson.log(line).let { it.copy(payload = redactSecret(it.payload, settings.secret)) } }

    /** Read-only snapshot; closing connections is intentionally not offered. */
    suspend fun connections(): ConnectionsSnapshot = ControllerJson.connections(client.callChecked(settings, "GET", "/connections"))

    suspend fun providers(): List<ProxyProvider> =
        ControllerJson.providers(client.callChecked(settings, "GET", "/providers/proxies"), ProviderKind.PROXY) +
            ControllerJson.providers(client.callChecked(settings, "GET", "/providers/rules"), ProviderKind.RULE)

    /** `PUT /providers/{proxies|rules}/{name}`; a proxy provider is health-checked afterwards. */
    suspend fun refresh(provider: ProxyProvider): ProviderRefresh {
        val base = if (provider.kind == ProviderKind.PROXY) "/providers/proxies/" else "/providers/rules/"
        val path = base + MihomoControllerClient.encode(provider.name)
        return try {
            client.callChecked(settings, "PUT", path, "")
            if (provider.kind == ProviderKind.PROXY) {
                val health = client.call(settings, "GET", "$path/healthcheck", timeoutMs = 30_000)
                if (health.status !in 200..299) throw MihomoControllerClient.errorFor(settings, health.status, health.body)
            }
            ProviderRefresh(provider, null)
        } catch (error: IOException) {
            ProviderRefresh(provider, redactSecret(error.message ?: "更新失败", settings.secret))
        }
    }

    /** Refreshes every HTTP/File provider in turn; one failure does not stop the rest. */
    suspend fun refreshProviders(): List<ProviderRefresh> = providers().filter { it.refreshable }.map { refresh(it) }

    private fun query(url: String, timeoutMs: Int): String {
        require(timeoutMs in 100..30_000) { "测速超时须在 100–30000 ms 之间" }
        require(url.startsWith("http://") || url.startsWith("https://")) { "测速地址必须是 http(s) URL" }
        return "url=${MihomoControllerClient.encode(url)}&timeout=$timeoutMs"
    }

    private inline fun delayResult(response: ControllerResponse, success: (String) -> DelayResult): DelayResult = when (response.status) {
        in 200..299 -> success(response.body)
        408, 504 -> DelayResult.Timeout
        503 -> DelayResult.Failed(ControllerJson.errorMessage(response.body)?.let { redactSecret(it, settings.secret).take(200) } ?: "节点不可用")
        else -> throw MihomoControllerClient.errorFor(settings, response.status, response.body)
    }

    companion object {
        /** Mihomo's own default (C.DefaultTestURL). */
        const val DEFAULT_TEST_URL = "https://www.gstatic.com/generate_204"
        const val DEFAULT_TIMEOUT_MS = 5000
    }
}
