package top.flysoftbeta.workflow.proxy.runtime

import top.flysoftbeta.workflow.proxy.config.ConfigInspection
import top.flysoftbeta.workflow.proxy.controller.DelayResult
import top.flysoftbeta.workflow.proxy.controller.ProxyMode
import top.flysoftbeta.workflow.proxy.controller.ProxyProvider
import top.flysoftbeta.workflow.proxy.controller.ProxySnapshot
import top.flysoftbeta.workflow.proxy.controller.TrafficSample
import top.flysoftbeta.workflow.proxy.network.ProxyConflict

enum class ProxyPhase {
    STOPPED,
    STARTING,
    RUNNING,
    STOPPING,
    /** A start was refused because another VPN/TUN owns routing; see [ProxyState.conflict]. Nothing was started. */
    CONFLICT,
    /** Start failed, the kernel exited unexpectedly, or a stop could not be confirmed ([ProxyState.stopUnconfirmed]). */
    ERROR,
}

/** GRANTED/DENIED only after an actual `su` check returned uid 0 or failed; never inferred from an installed su. */
enum class RootStatus { UNKNOWN, GRANTED, DENIED }

data class ProxyCapabilities(
    /** Packaged Mihomo exists and is executable for this ABI. */
    val kernel: Boolean = false,
    val guardian: Boolean = false,
    val root: RootStatus = RootStatus.UNKNOWN,
)

data class ProxyConfigState(
    val exists: Boolean = false,
    /** Null when missing or unparseable. */
    val inspection: ConfigInspection? = null,
    /** Fixed-text parse error. */
    val error: String? = null,
)

/** [up]: the configured device exists and is up; [routed]: our policy rules were observed via read-only `ip rule`. */
data class TunStatus(val device: String?, val up: Boolean, val routed: Boolean?)

data class ProxyConfigCheck(val valid: Boolean, val output: String)

/** Everything a proxy screen needs. Immutable; published through [ProxyRuntime.state]. */
data class ProxyState(
    val phase: ProxyPhase = ProxyPhase.STOPPED,
    /** Current step while STARTING/STOPPING (e.g. "正在请求 Root 授权"). */
    val progress: String? = null,
    /** Lifecycle error text (secret-redacted). Controller action errors are returned to the caller instead. */
    val error: String? = null,
    /** A previous kernel may still exist: the guardian channel was lost before an exit was confirmed. */
    val stopUnconfirmed: Boolean = false,
    val capabilities: ProxyCapabilities = ProxyCapabilities(),
    val config: ProxyConfigState = ProxyConfigState(),
    /** Latest root-free detection; informational unless [phase] is CONFLICT. */
    val conflict: ProxyConflict = ProxyConflict(),
    val pid: Int? = null,
    val version: String? = null,
    val controllerReachable: Boolean = false,
    val tun: TunStatus? = null,
    /** Live mode while running, otherwise the configured one. */
    val mode: ProxyMode? = null,
    /** Live groups while [live]; otherwise the last live groups for an unchanged config, else config.yaml's `proxy-groups`. */
    val groups: ProxySnapshot = ProxySnapshot.EMPTY,
    /** Latest delay per node name. */
    val delays: Map<String, DelayResult> = emptyMap(),
    /** Group or node names with a delay test in flight. */
    val testing: Set<String> = emptySet(),
    val traffic: TrafficSample? = null,
    val providers: List<ProxyProvider> = emptyList(),
) {
    val running: Boolean get() = phase == ProxyPhase.RUNNING
    /** [groups] and [mode] come from the running kernel and can be changed; otherwise they are read from config.yaml. */
    val live: Boolean get() = running && controllerReachable
    val busy: Boolean get() = phase == ProxyPhase.STARTING || phase == ProxyPhase.STOPPING
    val canStart: Boolean get() = !busy && phase != ProxyPhase.RUNNING && capabilities.kernel && capabilities.guardian &&
        config.exists && config.inspection != null && config.inspection.blocking.isEmpty()
}
