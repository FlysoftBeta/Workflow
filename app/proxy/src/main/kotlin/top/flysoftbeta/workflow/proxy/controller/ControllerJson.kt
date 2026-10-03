package top.flysoftbeta.workflow.proxy.controller

import java.io.IOException
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.longOrNull

/**
 * Decoding of Mihomo 1.19 controller payloads (hub/route). Unknown fields are ignored. Parse errors become a
 * fixed message: kotlinx exception text quotes the input, which may contain the secret.
 */
internal object ControllerJson {
    private val json = Json { isLenient = false }

    fun parse(text: String): JsonObject = try {
        json.parseToJsonElement(text) as? JsonObject ?: throw IOException("代理控制接口返回了无效 JSON")
    } catch (_: Exception) { throw IOException("代理控制接口返回了无效 JSON") }

    fun errorMessage(text: String): String? = runCatching { parse(text).string("message") }.getOrNull()

    fun mode(text: String): String? = parse(text).string("mode")

    fun proxies(text: String): ProxySnapshot {
        val all = parse(text)["proxies"] as? JsonObject ?: throw IOException("代理控制接口缺少 proxies")
        fun node(name: String): ProxyNode {
            val entry = all[name] as? JsonObject
            val history = entry?.get("history") as? JsonArray
            val last = (history?.lastOrNull() as? JsonObject)?.long("delay")?.toInt()
            return ProxyNode(name, entry?.string("type") ?: "Unknown", entry?.bool("alive") ?: false,
                last?.takeIf { it > 0 }, entry?.bool("udp") ?: false, entry?.string("now")?.takeIf { entry["all"] is JsonArray })
        }
        fun group(name: String, entry: JsonObject): ProxyGroup {
            val members = (entry["all"] as JsonArray).mapNotNull { (it as? JsonPrimitive)?.contentOrNull }.map(::node)
            return ProxyGroup(name, entry.string("type") ?: "Unknown", entry.string("now")?.takeIf { it.isNotEmpty() }, members,
                entry.bool("hidden") ?: false, entry.string("testUrl")?.takeIf { it.isNotEmpty() }, entry.string("fixed")?.takeIf { it.isNotEmpty() })
        }
        val groups = all.entries.mapNotNull { (name, value) ->
            val entry = value as? JsonObject ?: return@mapNotNull null
            if (entry["all"] !is JsonArray) null else name to entry
        }
        val globalEntry = groups.firstOrNull { it.first == "GLOBAL" }
        val order = (globalEntry?.second?.get("all") as? JsonArray)?.mapNotNull { (it as? JsonPrimitive)?.contentOrNull }.orEmpty()
            .withIndex().associate { it.value to it.index }
        val ordered = groups.filter { it.first != "GLOBAL" }
            .sortedWith(compareBy<Pair<String, JsonObject>> { order[it.first] ?: Int.MAX_VALUE }.thenBy { it.first })
            .map { group(it.first, it.second) }
        return ProxySnapshot(ordered, globalEntry?.let { group(it.first, it.second) })
    }

    fun delay(text: String): Int? = parse(text).long("delay")?.toInt()?.takeIf { it > 0 }

    fun groupDelays(text: String): Map<String, Int> = parse(text).entries.mapNotNull { (name, value) ->
        (value as? JsonPrimitive)?.longOrNull?.toInt()?.takeIf { it > 0 }?.let { name to it }
    }.toMap()

    fun traffic(line: String): TrafficSample {
        val entry = parse(line)
        return TrafficSample(entry.long("up") ?: 0, entry.long("down") ?: 0, entry.long("upTotal") ?: 0, entry.long("downTotal") ?: 0)
    }

    fun log(line: String): ProxyLogEntry {
        val entry = parse(line)
        return ProxyLogEntry(entry.string("type") ?: entry.string("level") ?: "info", entry.string("payload") ?: entry.string("message") ?: "")
    }

    fun connections(text: String): ConnectionsSnapshot {
        val entry = parse(text)
        val list = (entry["connections"] as? JsonArray).orEmpty().mapNotNull { item ->
            val connection = item as? JsonObject ?: return@mapNotNull null
            val metadata = connection["metadata"] as? JsonObject ?: JsonObject(emptyMap())
            fun endpoint(ip: String, port: String): String {
                val address = metadata.string(ip).orEmpty()
                val number = metadata.string(port) ?: metadata.long(port)?.toString()
                return if (address.isEmpty()) "" else if (':' in address) "[$address]:$number" else "$address:$number"
            }
            ProxyConnection(
                id = connection.string("id") ?: return@mapNotNull null,
                network = metadata.string("network").orEmpty(),
                type = metadata.string("type").orEmpty(),
                host = metadata.string("host").orEmpty().ifEmpty { metadata.string("sniffHost").orEmpty() },
                destination = endpoint("destinationIP", "destinationPort"),
                source = endpoint("sourceIP", "sourcePort"),
                process = metadata.string("process").orEmpty(),
                chains = (connection["chains"] as? JsonArray).orEmpty().mapNotNull { (it as? JsonPrimitive)?.contentOrNull },
                rule = connection.string("rule").orEmpty(),
                rulePayload = connection.string("rulePayload").orEmpty(),
                upload = connection.long("upload") ?: 0,
                download = connection.long("download") ?: 0,
                start = connection.string("start").orEmpty(),
            )
        }
        return ConnectionsSnapshot(entry.long("uploadTotal") ?: 0, entry.long("downloadTotal") ?: 0, list)
    }

    fun providers(text: String, kind: ProviderKind): List<ProxyProvider> {
        val map = parse(text)["providers"] as? JsonObject ?: return emptyList()
        return map.entries.mapNotNull { (name, value) ->
            val entry = value as? JsonObject ?: return@mapNotNull null
            val vehicle = entry.string("vehicleType") ?: "Unknown"
            if (vehicle == "Compatible") return@mapNotNull null
            val size = when (kind) {
                ProviderKind.PROXY -> (entry["proxies"] as? JsonArray)?.size ?: 0
                ProviderKind.RULE -> entry.long("ruleCount")?.toInt() ?: 0
            }
            ProxyProvider(name, kind, vehicle, entry.string("updatedAt"), size)
        }.sortedBy { it.name }
    }

    private fun JsonObject.string(key: String): String? = (this[key] as? JsonPrimitive)?.takeIf { it.isString }?.content
    private fun JsonObject.long(key: String): Long? = (this[key] as? JsonPrimitive)?.let { if (it.isString) it.content.toLongOrNull() else it.longOrNull }
    private fun JsonObject.bool(key: String): Boolean? = (this[key] as? JsonPrimitive)?.takeIf { !it.isString }?.booleanOrNull
}
