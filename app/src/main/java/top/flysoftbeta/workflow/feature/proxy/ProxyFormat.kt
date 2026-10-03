package top.flysoftbeta.workflow.feature.proxy

import top.flysoftbeta.workflow.proxy.controller.DelayResult
import top.flysoftbeta.workflow.proxy.controller.ProxyGroup
import top.flysoftbeta.workflow.proxy.controller.ProxyMode
import top.flysoftbeta.workflow.proxy.controller.ProxyNode
import top.flysoftbeta.workflow.proxy.controller.ProxySnapshot
import java.util.Locale

/** Pure presentation rules of the proxy panels (unit-tested without Android). */
internal object ProxyFormat {
    /** docs/ui.md §4.12: < 300ms success, < 800ms warning, slower or failed error. */
    enum class Tier { GOOD, FAIR, BAD }

    data class Latency(val text: String, val tier: Tier)

    fun latency(result: DelayResult?, history: Int?): Latency? = when (result) {
        is DelayResult.Success -> ms(result.ms)
        DelayResult.Timeout -> Latency("超时", Tier.BAD)
        is DelayResult.Failed -> Latency("失败", Tier.BAD)
        null -> history?.let(::ms)
    }

    private fun ms(value: Int) = Latency(value.toString(), when {
        value < 300 -> Tier.GOOD
        value < 800 -> Tier.FAIR
        else -> Tier.BAD
    })

    /** "12 KB/s", "1.2 MB/s"; one decimal below 10 units, none above. */
    fun rate(bytesPerSecond: Long): String = size(bytesPerSecond) + "/s"

    fun size(bytes: Long): String {
        if (bytes < 1024) return "$bytes B"
        val units = listOf("KB", "MB", "GB", "TB")
        var value = bytes / 1024.0
        var index = 0
        while (value >= 1024 && index < units.lastIndex) { value /= 1024; index++ }
        return if (value < 10) String.format(Locale.ROOT, "%.1f %s", value, units[index]) else "${value.toLong()} ${units[index]}"
    }

    /** Groups to list for [mode]. Direct mode lists the rule groups too (dimmed by the UI): they still apply after a switch back. */
    fun visibleGroups(snapshot: ProxySnapshot, mode: ProxyMode?): List<ProxyGroup> =
        if (mode == ProxyMode.GLOBAL) listOfNotNull(snapshot.global) else snapshot.groups.filterNot { it.hidden }

    /** Selector and url-test/fallback groups accept a choice (the latter as a pin until the next test). */
    fun canSelect(group: ProxyGroup): Boolean = group.selectable

    /** Manual groups vs. automatically tested ones (the only distinction the header shows). */
    fun automatic(group: ProxyGroup): Boolean = group.type == "URLTest" || group.type == "Fallback" || group.type == "LoadBalance"

    /** Nodes that can be latency-tested on their own (built-ins like DIRECT/REJECT and nested groups are skipped). */
    fun testable(node: ProxyNode): Boolean = node.now == null && node.type != "Reject" && node.type != "Pass"

    /** Columns of the node grid: chips at least [minChip] wide, [gap] apart. */
    fun columns(width: Float, minChip: Float, gap: Float): Int = maxOf(1, ((width + gap) / (minChip + gap)).toInt())

    /** One line of runtime.log. Mihomo writes logrus text: `time="…" level=info msg="…"`. */
    data class LogLine(val time: String?, val level: String?, val message: String)

    private val logrus = Regex("""^time="([^"]*)" level=(\w+) msg="(.*)"$""")

    fun parseLog(line: String): LogLine {
        val match = logrus.matchEntire(line) ?: return LogLine(null, null, line)
        val (time, level, message) = match.destructured
        // RFC 3339 → HH:mm:ss
        val clock = time.substringAfter('T', "").take(8).takeIf { it.length == 8 }
        return LogLine(clock, level.lowercase(), message.replace("\\\"", "\"").replace("\\\\", "\\"))
    }
}
