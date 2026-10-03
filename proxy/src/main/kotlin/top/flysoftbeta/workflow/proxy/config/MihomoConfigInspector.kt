package top.flysoftbeta.workflow.proxy.config

import top.flysoftbeta.workflow.proxy.controller.ProxySnapshot
import top.flysoftbeta.workflow.proxy.redact.ProxySecrets

/** The `tun:` block as Mihomo will interpret it. Unset numeric keys stay null; see the `effective*` values. */
data class TunSettings(
    val enable: Boolean = false,
    val device: String? = null,
    val stack: String? = null,
    val autoRoute: Boolean = true,
    val autoRedirect: Boolean = false,
    val autoDetectInterface: Boolean = true,
    val strictRoute: Boolean = false,
    val tableIndex: Int? = null,
    val ruleIndex: Int? = null,
    val autoRedirectInputMark: Long? = null,
    val autoRedirectOutputMark: Long? = null,
    val dnsHijack: List<String> = emptyList(),
    val includePackages: List<String> = emptyList(),
    val excludePackages: List<String> = emptyList(),
    val includeUids: Boolean = false,
    val excludeUids: Boolean = false,
    val overrideAndroidVpn: Boolean = false,
) {
    val effectiveTableIndex: Int get() = tableIndex?.takeIf { it != 0 } ?: SingTunDefaults.TABLE_INDEX
    val effectiveRuleIndex: Int get() = ruleIndex?.takeIf { it != 0 } ?: SingTunDefaults.RULE_INDEX
    val effectiveRuleRange: IntRange get() = effectiveRuleIndex..(effectiveRuleIndex + ProxySafeDefaults.RULE_SPAN)
    val effectiveInputMark: Long get() = autoRedirectInputMark?.takeIf { it != 0L } ?: SingTunDefaults.AUTO_REDIRECT_INPUT_MARK
    val effectiveOutputMark: Long get() = autoRedirectOutputMark?.takeIf { it != 0L } ?: SingTunDefaults.AUTO_REDIRECT_OUTPUT_MARK
    /** Null means Mihomo picks `Meta`, `Meta1`, … at start and the name cannot be told apart from another kernel's. */
    val explicitDevice: String? get() = device?.takeIf { it.isNotBlank() }
}

enum class IssueSeverity { INFO, WARNING, BLOCKING }

enum class IssueCode {
    CONTROLLER_MISSING, CONTROLLER_REJECTED, CONTROLLER_WITHOUT_SECRET, CONTROLLER_EXPOSED,
    ALLOW_LAN, DEFAULT_PORT, UNKNOWN_MODE,
    TUN_DEFAULT_TABLE, TUN_DEFAULT_RULE_INDEX, TUN_TABLE_RESERVED, TUN_RULE_INDEX_RANGE,
    TUN_MARK_COLLISION, TUN_AUTO_DEVICE, TUN_DEVICE_INVALID, TUN_OVERRIDES_ANDROID_VPN,
}

/** Messages are fixed text or numbers; they never contain the secret or other free-form config values. */
data class ConfigIssue(val code: IssueCode, val severity: IssueSeverity, val message: String)

data class ConfigInspection(
    /** Loopback settings the app may use, or null (missing or rejected; see [issues]). */
    val controller: ProxyControllerSettings?,
    val mode: String?,
    val ports: Map<String, Int>,
    val allowLan: Boolean,
    val tun: TunSettings,
    val proxyProviders: List<String>,
    val ruleProviders: List<String>,
    val issues: List<ConfigIssue>,
    /** `proxy-groups` as written (no selection, no provider members): shown disabled while stopped. */
    val groups: ProxySnapshot = ProxySnapshot.EMPTY,
    val secrets: ProxySecrets = ProxySecrets.empty(),
) {
    val blocking: List<ConfigIssue> get() = issues.filter { it.severity == IssueSeverity.BLOCKING }
    override fun toString(): String = "ConfigInspection(controller=$controller, mode=$mode, tun=${tun.enable}, issues=${issues.map { it.code }})"
}

/**
 * Read-only inspection of the user's configuration. Nothing is rewritten: the result only explains how
 * Mihomo will behave and what would collide with other routing owners on the device.
 */
object MihomoConfigInspector {
    private val portKeys = listOf("mixed-port", "port", "socks-port", "redir-port", "tproxy-port")
    private val knownModes = setOf("rule", "global", "direct")

    fun inspect(text: String): ConfigInspection {
        val root = MihomoYaml.load(text)
        val issues = mutableListOf<ConfigIssue>()
        val controller = inspectController(root, issues)
        val mode = (root["mode"] as? String)?.lowercase()
        if (mode != null && mode !in knownModes) issues += ConfigIssue(IssueCode.UNKNOWN_MODE, IssueSeverity.INFO, "配置使用了应用不支持切换的模式")
        val ports = portKeys.mapNotNull { key -> integer(root[key])?.toInt()?.takeIf { it in 1..65535 }?.let { key to it } }.toMap()
        if (ports.values.any { it == 7890 }) issues += ConfigIssue(IssueCode.DEFAULT_PORT, IssueSeverity.INFO, "代理端口 7890 与其他 Clash 实例的默认值相同，可能已被占用")
        val allowLan = root["allow-lan"] == true
        if (allowLan) issues += ConfigIssue(IssueCode.ALLOW_LAN, IssueSeverity.WARNING, "allow-lan 已开启：局域网内的设备可以使用本机代理端口")
        val tun = tunSettings(root["tun"] as? Map<*, *>)
        if (tun.enable) inspectTun(tun, issues)
        return ConfigInspection(controller, mode, ports, allowLan, tun,
            providerNames(root["proxy-providers"]), providerNames(root["rule-providers"]), issues,
            ConfigGroups.preview(root), ProxySecrets.fromConfig(root))
    }

    private fun inspectController(root: Map<*, *>, issues: MutableList<ConfigIssue>): ProxyControllerSettings? {
        val configured = root["external-controller"]
        if (configured == null || (configured is String && configured.isBlank())) {
            issues += ConfigIssue(IssueCode.CONTROLLER_MISSING, IssueSeverity.WARNING, "配置没有 external-controller：启动后无法切换模式、节点或测速")
            return null
        }
        val settings = try { LocalProxyConfig.controller(root) } catch (error: IllegalArgumentException) {
            issues += ConfigIssue(IssueCode.CONTROLLER_REJECTED, IssueSeverity.WARNING, "应用不会连接此控制接口：${error.message}")
            return null
        }
        val host = runCatching { LocalProxyConfig.controllerAddress(configured as String).first }.getOrNull()
        val exposed = (configured as String).startsWith(":") || host == "0.0.0.0" || host == "::"
        if (settings != null && settings.secret.isEmpty()) {
            // The kernel runs as root: an unauthenticated controller lets any app on the device
            // reconfigure it (PATCH /configs, /upgrade, …). Refuse rather than silently expose it.
            issues += ConfigIssue(IssueCode.CONTROLLER_WITHOUT_SECRET, IssueSeverity.BLOCKING,
                "external-controller 没有设置 secret：设备上的任何应用都能控制以 Root 运行的内核。请在配置中设置 secret")
        }
        if (exposed) issues += ConfigIssue(IssueCode.CONTROLLER_EXPOSED, IssueSeverity.WARNING, "控制接口监听所有网卡，局域网内可访问；建议改为 127.0.0.1")
        return settings
    }

    private fun inspectTun(tun: TunSettings, issues: MutableList<ConfigIssue>) {
        tun.explicitDevice?.let { device ->
            if (!device.matches(Regex("[A-Za-z0-9_.-]{1,15}")) || device == "." || device == "..") {
                issues += ConfigIssue(IssueCode.TUN_DEVICE_INVALID, IssueSeverity.BLOCKING,
                    "TUN 网卡名须为 1–15 个英文字母、数字、下划线、点或短横线，且不能是 . 或 ..")
            }
        }
        if (tun.tableIndex == null || tun.tableIndex == 0 || tun.effectiveTableIndex == SingTunDefaults.TABLE_INDEX) {
            issues += ConfigIssue(IssueCode.TUN_DEFAULT_TABLE, IssueSeverity.WARNING,
                "TUN 使用默认路由表 ${SingTunDefaults.TABLE_INDEX}，会与其他 Mihomo/CMFA 冲突；建议 iproute2-table-index: ${ProxySafeDefaults.TABLE_INDEX}")
        }
        if (tun.ruleIndex == null || tun.ruleIndex == 0 || tun.effectiveRuleIndex == SingTunDefaults.RULE_INDEX) {
            issues += ConfigIssue(IssueCode.TUN_DEFAULT_RULE_INDEX, IssueSeverity.WARNING,
                "TUN 使用默认规则序号 ${SingTunDefaults.RULE_INDEX}，会与其他 Mihomo/CMFA 冲突；建议 iproute2-rule-index: ${ProxySafeDefaults.RULE_INDEX}")
        }
        val table = tun.effectiveTableIndex
        if (table in AndroidNetdRouting.TABLES || table in AndroidNetdRouting.INTERFACE_TABLES || table <= 0 || table in 253..255) {
            issues += ConfigIssue(IssueCode.TUN_TABLE_RESERVED, IssueSeverity.BLOCKING, "路由表 $table 由系统使用，请改用其他 iproute2-table-index")
        }
        val range = tun.effectiveRuleRange
        if (range.first <= 0 || range.last >= AndroidNetdRouting.FIRST_RULE_PRIORITY) {
            issues += ConfigIssue(IssueCode.TUN_RULE_INDEX_RANGE, IssueSeverity.BLOCKING,
                "规则序号 ${range.first}–${range.last} 与系统 netd 规则交错，请使用 1–${AndroidNetdRouting.FIRST_RULE_PRIORITY - ProxySafeDefaults.RULE_SPAN - 1} 之间的值")
        }
        if (tun.autoRedirect) {
            val marks = listOf(tun.effectiveInputMark, tun.effectiveOutputMark)
            if (marks.any { it and AndroidNetdRouting.FWMARK_MASK != 0L } || marks[0] == marks[1] ||
                marks.any { it == SingTunDefaults.AUTO_REDIRECT_INPUT_MARK || it == SingTunDefaults.AUTO_REDIRECT_OUTPUT_MARK }) {
                issues += ConfigIssue(IssueCode.TUN_MARK_COLLISION, IssueSeverity.WARNING,
                    "auto-redirect 标记与系统 fwmark 或其他 Mihomo 默认值重叠；建议 0x%x / 0x%x".format(
                        ProxySafeDefaults.AUTO_REDIRECT_INPUT_MARK, ProxySafeDefaults.AUTO_REDIRECT_OUTPUT_MARK))
            }
        }
        if (tun.explicitDevice == null) {
            issues += ConfigIssue(IssueCode.TUN_AUTO_DEVICE, IssueSeverity.INFO,
                "未设置 tun.device：网卡名与其他 Mihomo 相同（Meta…），无法区分；建议 device: ${ProxySafeDefaults.TUN_DEVICE}")
        }
        if (tun.overrideAndroidVpn) {
            issues += ConfigIssue(IssueCode.TUN_OVERRIDES_ANDROID_VPN, IssueSeverity.WARNING, "override-android-vpn 会接管系统 VPN 的流量")
        }
    }

    private fun tunSettings(block: Map<*, *>?): TunSettings {
        if (block == null) return TunSettings()
        fun bool(key: String, default: Boolean) = block[key] as? Boolean ?: default
        fun strings(key: String) = (block[key] as? List<*>)?.mapNotNull { (it as? String) ?: (it as? Number)?.toString() }.orEmpty()
        return TunSettings(
            enable = bool("enable", false),
            device = block["device"] as? String,
            stack = (block["stack"] as? String)?.lowercase(),
            autoRoute = bool("auto-route", true),
            autoRedirect = bool("auto-redirect", false),
            autoDetectInterface = bool("auto-detect-interface", true),
            strictRoute = bool("strict-route", false),
            tableIndex = integer(block["iproute2-table-index"])?.toInt(),
            ruleIndex = integer(block["iproute2-rule-index"])?.toInt(),
            autoRedirectInputMark = integer(block["auto-redirect-input-mark"]),
            autoRedirectOutputMark = integer(block["auto-redirect-output-mark"]),
            dnsHijack = strings("dns-hijack"),
            includePackages = strings("include-package"),
            excludePackages = strings("exclude-package"),
            includeUids = block["include-uid"] is List<*> || block["include-uid-range"] is List<*>,
            excludeUids = block["exclude-uid"] is List<*> || block["exclude-uid-range"] is List<*>,
            overrideAndroidVpn = bool("override-android-vpn", false),
        )
    }

    private fun providerNames(value: Any?): List<String> = (value as? Map<*, *>)?.keys?.mapNotNull { it as? String }.orEmpty()

    /** YAML 1.1 ints (decimal, 0x hex, 0o/0 octal) as SnakeYAML resolves them; out-of-range values are ignored. */
    private fun integer(value: Any?): Long? = when (value) {
        is Int -> value.toLong()
        is Long -> value
        is java.math.BigInteger -> if (value.bitLength() < 63) value.toLong() else null
        else -> null
    }
}
