package top.flysoftbeta.workflow.proxy.config

import top.flysoftbeta.workflow.proxy.controller.ProxyGroup
import top.flysoftbeta.workflow.proxy.controller.ProxyNode
import top.flysoftbeta.workflow.proxy.controller.ProxySnapshot

/**
 * The proxy groups as written in `proxy-groups`, for showing them (disabled) while the kernel is stopped.
 * Only what the file itself says: members from `proxies`, plus every proxy for `include-all(-proxies)`
 * (narrowed by `filter` / `exclude-filter` / `exclude-type` when those are plain regular expressions).
 * Provider members (`use`, `include-all-providers`) are unknown until the kernel loads them. The current
 * selection is unknown too (Mihomo restores it from its own cache), so `now` stays null.
 */
internal object ConfigGroups {
    private val builtIns = listOf("DIRECT", "REJECT")
    private val adapterTypes = mapOf(
        "select" to "Selector", "url-test" to "URLTest", "fallback" to "Fallback",
        "load-balance" to "LoadBalance", "relay" to "Relay",
    )

    fun preview(root: Map<*, *>): ProxySnapshot {
        val proxies = (root["proxies"] as? List<*>).orEmpty().mapNotNull { entry ->
            val map = entry as? Map<*, *> ?: return@mapNotNull null
            val name = map["name"] as? String ?: return@mapNotNull null
            name to ((map["type"] as? String)?.lowercase() ?: "")
        }
        val proxyTypes = proxies.toMap()
        val entries = (root["proxy-groups"] as? List<*>).orEmpty().mapNotNull { it as? Map<*, *> }
            .mapNotNull { map -> (map["name"] as? String)?.takeIf { it.isNotBlank() }?.let { it to map } }
            .distinctBy { it.first }
        val groupTypes = entries.associate { (name, map) -> name to (adapterTypes[(map["type"] as? String)?.lowercase()] ?: "Unknown") }

        fun node(name: String) = ProxyNode(
            name = name,
            type = groupTypes[name] ?: builtInType(name) ?: proxyTypes[name]?.let(::adapterName) ?: "Unknown",
            alive = true, delayMs = null, udp = false,
        )

        val groups = entries.map { (name, map) ->
            val listed = strings(map["proxies"])
            val includeAll = map["include-all"] == true || map["include-all-proxies"] == true
            val filter = pattern(map["filter"])
            val exclude = pattern(map["exclude-filter"])
            val excludeTypes = (map["exclude-type"] as? String)?.split('|')?.map { it.trim().lowercase() }?.filter { it.isNotEmpty() }.orEmpty()
            val included = if (!includeAll) emptyList() else proxies.filter { (proxy, type) ->
                (filter == null || filter.containsMatchIn(proxy)) && (exclude == null || !exclude.containsMatchIn(proxy)) &&
                    adapterName(type).lowercase() !in excludeTypes
            }.map { it.first }
            val members = (listed + included).distinct().filter { it != name }
            ProxyGroup(name, groupTypes.getValue(name), null, members.map(::node), map["hidden"] == true,
                (map["url"] as? String)?.takeIf { it.startsWith("http://") || it.startsWith("https://") }, null)
        }
        // Mihomo's GLOBAL lists the built-ins, then every proxy, then every group, in file order.
        val globalMembers = (builtIns + proxies.map { it.first } + groups.map { it.name }).distinct()
        val global = ProxyGroup("GLOBAL", "Selector", null, globalMembers.map(::node), false, null, null)
        return ProxySnapshot(groups, global)
    }

    private fun strings(value: Any?): List<String> = (value as? List<*>).orEmpty().mapNotNull { (it as? String) ?: (it as? Number)?.toString() }

    /** Go RE2 and java.util.regex agree on ordinary patterns; anything else is ignored rather than guessed. */
    private fun pattern(value: Any?): Regex? = (value as? String)?.takeIf { it.isNotEmpty() && it.length <= 1024 }
        ?.let { runCatching { Regex(it) }.getOrNull() }

    private fun builtInType(name: String): String? = when (name) {
        "DIRECT" -> "Direct"
        "REJECT", "REJECT-DROP" -> "Reject"
        "PASS" -> "Pass"
        else -> null
    }

    private fun adapterName(type: String): String = when (type) {
        "ss" -> "Shadowsocks"
        "ssr" -> "ShadowsocksR"
        "vmess" -> "Vmess"
        "vless" -> "Vless"
        "trojan" -> "Trojan"
        "hysteria" -> "Hysteria"
        "hysteria2" -> "Hysteria2"
        "tuic" -> "Tuic"
        "wireguard" -> "WireGuard"
        "socks5" -> "Socks5"
        "http" -> "Http"
        "snell" -> "Snell"
        "ssh" -> "Ssh"
        "anytls" -> "AnyTLS"
        "direct" -> "Direct"
        else -> "Unknown"
    }
}
