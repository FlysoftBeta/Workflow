package top.flysoftbeta.workflow.proxy.network

import top.flysoftbeta.workflow.proxy.config.TunSettings

/** A root-free interface view. A point-to-point TUN can remain owned while its carrier is temporarily down. */
data class NetInterface(val name: String, val up: Boolean, val tun: Boolean? = null, val pointToPoint: Boolean = false)

/** One `ip rule show` line. [table] is the `lookup` target (name or number), [goto] the `goto` target. */
data class IpRule(val priority: Int, val table: String?, val goto: Int?, val iif: String?, val raw: String)

/**
 * What else routes traffic on this device. Detection never needs root; [ruleCollisions] is filled only
 * during a start the user already authorized (read-only `ip rule show`).
 */
data class ProxyConflict(
    /** Android reports a VpnService network (TRANSPORT_VPN), e.g. CMFA or another VPN app. */
    val vpnActive: Boolean = false,
    /** Up or point-to-point TUN-like interfaces that this app did not create in the current run. */
    val foreignInterfaces: List<String> = emptyList(),
    /** Our configured `tun.device` already exists although we are not running it. */
    val deviceTaken: Boolean = false,
    /** Policy rules owned by someone else inside our rule range or pointing at our table. */
    val ruleCollisions: List<String> = emptyList(),
) {
    val any: Boolean get() = vpnActive || foreignInterfaces.isNotEmpty() || deviceTaken || ruleCollisions.isNotEmpty()
    /** Collisions that would corrupt the other owner's routing: never overridable. */
    val hard: Boolean get() = deviceTaken || ruleCollisions.isNotEmpty()

    fun describe(): String = buildList {
        if (vpnActive) add("系统中有其他 VPN 正在运行")
        if (foreignInterfaces.isNotEmpty()) add("检测到其他 TUN 网卡：" + foreignInterfaces.joinToString("、"))
        if (deviceTaken) add("配置的 TUN 网卡名已被占用")
        if (ruleCollisions.isNotEmpty()) add("路由规则与其他 TUN 冲突（${ruleCollisions.size} 条）")
    }.joinToString("；")
}

object TunConflictDetector {
    /** Interface names used by VPN apps, Mihomo/sing-box/Clash, WireGuard and similar tunnels. */
    private val tunnelName = Regex("^(tun|tap|utun|Meta|clash|mihomo|sing|box|wg|tailscale|zt|ppp|ipsec|nekoray|v2ray|xray)[A-Za-z0-9_.-]*$")
    private val linkLine = Regex("^\\d+: ([^:]+): <([^>]*)>.*$")

    /** `ip -o link show` retains carrierless/addressless interfaces Android java.net may omit. */
    fun parseLinks(text: String): List<NetInterface> = text.lineSequence().filter { it.isNotBlank() }.map { line ->
        val match = requireNotNull(linkLine.matchEntire(line.trim())) { "无法解析系统网卡状态" }
        val flags = match.groupValues[2].split(',').toSet()
        NetInterface(match.groupValues[1].substringBefore('@'), up = "UP" in flags, pointToPoint = "POINTOPOINT" in flags)
    }.toList().also { require(it.isNotEmpty()) { "系统网卡状态为空" } }

    /**
     * [ownDevice]: the interface this app's kernel is running right now (excluded), or null when stopped.
     * [configuredDevice]: `tun.device` from the config, checked for a name clash while stopped.
     */
    fun detect(interfaces: List<NetInterface>, vpnActive: Boolean, ownDevice: String?, configuredDevice: String?): ProxyConflict {
        // Android's NetworkInterface.isUp can become false for an administratively UP TUN with
        // NO-CARRIER. A paused point-to-point tunnel still belongs to its existing owner.
        val foreign = interfaces.filter { (it.up || it.pointToPoint) && it.name != ownDevice &&
            (it.tun == true || (it.tun == null && tunnelName.matches(it.name))) }
            .map { it.name }.distinct().sorted()
        val deviceTaken = ownDevice == null && configuredDevice != null && interfaces.any { it.name == configuredDevice }
        return ProxyConflict(vpnActive, foreign.filter { it != configuredDevice || !deviceTaken }, deviceTaken)
    }

    /** Never infer ownership of existing rules from familiar priorities or tables. */
    @Suppress("UNUSED_PARAMETER")
    fun ruleCollisions(tun: TunSettings, rules: List<IpRule>, foreignInterfaces: List<String> = emptyList()): List<String> =
        rules.filter { it.priority in tun.effectiveRuleRange || it.table == tun.effectiveTableIndex.toString() }
            .map { it.raw }.distinct()

    /** Parses `ip rule show` / `ip -6 rule show` output; unparseable lines are skipped. */
    fun parseRules(text: String): List<IpRule> = text.lineSequence().mapNotNull { line ->
        val trimmed = line.trim()
        val colon = trimmed.indexOf(':')
        if (colon <= 0) return@mapNotNull null
        val priority = trimmed.substring(0, colon).toIntOrNull() ?: return@mapNotNull null
        val words = trimmed.substring(colon + 1).trim().split(Regex("\\s+"))
        fun after(keyword: String) = words.indexOf(keyword).takeIf { it >= 0 }?.let { words.getOrNull(it + 1) }
        IpRule(priority, after("lookup") ?: after("table"), after("goto")?.toIntOrNull(), after("iif"), "$priority: ${words.joinToString(" ")}")
    }.toList()
}
