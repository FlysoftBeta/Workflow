package top.flysoftbeta.workflow.proxy.config

/**
 * Values used by the starter template, chosen so a second Mihomo (for example ClashMetaForAndroid or a
 * Magisk box module left at sing-tun defaults) and Android's netd keep their own routing state.
 *
 * - sing-tun installs policy rules at `ruleIndex .. ruleIndex + 10` (the last one is a `nop` target).
 *   Android 9 netd uses priorities 0 and 10000..32000. 9500..9510 is between a default sing-tun
 *   (9000..9010) and netd; because it comes *after* 9000, an already running default TUN keeps precedence.
 * - Tables: netd uses 97/98/99 and 1000 + ifindex; sing-tun defaults to 2022.
 * - netd owns fwmark bits 0..20 (netId, explicit, protected-from-VPN, permission). Auto-redirect marks
 *   therefore live in bits 28/29 only.
 * - Ports differ from the Clash defaults 7890/9090.
 */
object ProxySafeDefaults {
    const val TUN_DEVICE = "workflow-tun"
    const val TABLE_INDEX = 9500
    const val RULE_INDEX = 9500
    /** sing-tun uses `ruleIndex .. ruleIndex + RULE_SPAN` (inclusive). */
    const val RULE_SPAN = 10
    const val AUTO_REDIRECT_INPUT_MARK = 0x10000000L
    const val AUTO_REDIRECT_OUTPUT_MARK = 0x20000000L
    const val MIXED_PORT = 17890
    const val CONTROLLER_PORT = 19090
}

/** sing-tun v0.4.24 values applied when the configuration leaves a key unset or 0. */
object SingTunDefaults {
    const val TABLE_INDEX = 2022
    const val RULE_INDEX = 9000
    const val AUTO_REDIRECT_INPUT_MARK = 0x2023L
    const val AUTO_REDIRECT_OUTPUT_MARK = 0x2024L
    const val FALLBACK_RULE_INDEX = 32768
    /** Mihomo's own device name when `tun.device` is empty (auto-numbered Meta, Meta1, …). */
    const val DEVICE_PREFIX = "Meta"
}

/** Routing state owned by Android 9 netd; values here must never be claimed by our TUN. */
object AndroidNetdRouting {
    val TABLES: Set<Int> = setOf(97, 98, 99)
    /** netd uses `1000 + ifindex` per interface. */
    val INTERFACE_TABLES: IntRange = 1000..1999
    /** Lowest priority netd uses besides the `local` rule at 0. */
    const val FIRST_RULE_PRIORITY = 10000
    const val UNREACHABLE_RULE_PRIORITY = 32000
    const val FWMARK_MASK = 0x1fffffL
}
