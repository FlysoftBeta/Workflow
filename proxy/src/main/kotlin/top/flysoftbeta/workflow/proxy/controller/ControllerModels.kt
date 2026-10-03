package top.flysoftbeta.workflow.proxy.controller

/** The three modes the app switches between. Mihomo's own value (lower case) is [wire]. */
enum class ProxyMode(val wire: String) {
    RULE("rule"), GLOBAL("global"), DIRECT("direct");
    companion object {
        fun parse(value: String?): ProxyMode? = entries.firstOrNull { it.wire.equals(value, ignoreCase = true) }
    }
}

/** One member of a group as shown in a node list. [delayMs] is the kernel's last recorded delay (null: none/failed). */
data class ProxyNode(
    val name: String,
    val type: String,
    val alive: Boolean,
    val delayMs: Int?,
    val udp: Boolean,
    /** For a nested group: its current selection. */
    val now: String? = null,
)

data class ProxyGroup(
    val name: String,
    /** Mihomo adapter type: Selector, URLTest, Fallback, LoadBalance, Relay. */
    val type: String,
    val now: String?,
    val members: List<ProxyNode>,
    val hidden: Boolean,
    /** Group-specific test URL, if configured. */
    val testUrl: String?,
    /** A url-test/fallback group pinned by the user; cleared by a group delay test. */
    val fixed: String?,
) {
    /** Only Selector groups accept `PUT /proxies/{name}`; url-test/fallback accept a pin but re-test clears it. */
    val selectable: Boolean get() = type == "Selector" || type == "URLTest" || type == "Fallback"
}

/** Groups in configuration order (as `GLOBAL.all` lists them), plus `GLOBAL` itself separately. */
data class ProxySnapshot(val groups: List<ProxyGroup>, val global: ProxyGroup?) {
    fun group(name: String): ProxyGroup? = groups.firstOrNull { it.name == name } ?: global?.takeIf { it.name == name }
    /** What the UI should list for [mode]: GLOBAL only in global mode, nothing in direct mode. */
    fun visibleGroups(mode: ProxyMode?): List<ProxyGroup> = when (mode) {
        ProxyMode.GLOBAL -> listOfNotNull(global)
        ProxyMode.DIRECT -> emptyList()
        else -> groups.filterNot { it.hidden }
    }
    companion object { val EMPTY = ProxySnapshot(emptyList(), null) }
}

sealed interface DelayResult {
    data class Success(val ms: Int) : DelayResult
    /** The kernel's own timeout elapsed (HTTP 408/504). */
    data object Timeout : DelayResult
    /** Any other failure; [message] is fixed text or the redacted kernel message. */
    data class Failed(val message: String) : DelayResult
}

data class TrafficSample(val up: Long, val down: Long, val upTotal: Long, val downTotal: Long)

enum class ProxyLogLevel(val wire: String) { DEBUG("debug"), INFO("info"), WARNING("warning"), ERROR("error"), SILENT("silent") }

/** A kernel log line; [payload] is secret-redacted. */
data class ProxyLogEntry(val level: String, val payload: String)

/** Read-only view of one tracked connection. */
data class ProxyConnection(
    val id: String,
    val network: String,
    val type: String,
    val host: String,
    val destination: String,
    val source: String,
    val process: String,
    val chains: List<String>,
    val rule: String,
    val rulePayload: String,
    val upload: Long,
    val download: Long,
    val start: String,
)

data class ConnectionsSnapshot(val uploadTotal: Long, val downloadTotal: Long, val connections: List<ProxyConnection>)

enum class ProviderKind { PROXY, RULE }

/** A refreshable provider from the user's config. Compatible (per-group) providers are excluded. */
data class ProxyProvider(
    val name: String,
    val kind: ProviderKind,
    /** HTTP, File or Inline. */
    val vehicleType: String,
    val updatedAt: String?,
    /** Proxy count for proxy providers, rule count for rule providers. */
    val size: Int,
) {
    val refreshable: Boolean get() = vehicleType == "HTTP" || vehicleType == "File"
}

data class ProviderRefresh(val provider: ProxyProvider, val error: String?)
