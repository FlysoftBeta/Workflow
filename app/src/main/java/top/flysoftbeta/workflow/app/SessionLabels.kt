package top.flysoftbeta.workflow.app

import top.flysoftbeta.workflow.core.session.Session
import java.time.Instant
import java.time.ZoneId
import java.time.temporal.ChronoUnit

private val WEEKDAYS = arrayOf("周一", "周二", "周三", "周四", "周五", "周六", "周日")

/** "周日 14:32" within a week of [now], else "9月20日 14:32". */
fun clockLabel(time: Long, zone: ZoneId, now: Long = System.currentTimeMillis()): String {
    val at = Instant.ofEpochMilli(time).atZone(zone)
    val days = ChronoUnit.DAYS.between(at.toLocalDate(), Instant.ofEpochMilli(now).atZone(zone).toLocalDate())
    val clock = "%02d:%02d".format(at.hour, at.minute)
    return if (days in 0..6) "${WEEKDAYS[at.dayOfWeek.value - 1]} $clock" else "${at.monthValue}月${at.dayOfMonth}日 $clock"
}

/** The session chip / list label: the persistent name, or the start time of a temporary session (ui.md §2.1). */
fun sessionLabel(session: Session, zone: ZoneId, now: Long = System.currentTimeMillis()): String =
    session.name ?: clockLabel(session.createdAt, zone, now)

/** Compact relative time for list rows: 刚刚 · 5 分钟前 · 3 小时前 · 昨天 · 3 天前 · 9月20日. */
fun relativeTime(time: Long, zone: ZoneId, now: Long = System.currentTimeMillis()): String {
    val delta = (now - time).coerceAtLeast(0)
    val minutes = delta / 60_000
    val days = ChronoUnit.DAYS.between(Instant.ofEpochMilli(time).atZone(zone).toLocalDate(), Instant.ofEpochMilli(now).atZone(zone).toLocalDate())
    return when {
        minutes < 1 -> "刚刚"
        minutes < 60 -> "$minutes 分钟前"
        days == 0L -> "${minutes / 60} 小时前"
        days == 1L -> "昨天"
        days < 7 -> "$days 天前"
        else -> Instant.ofEpochMilli(time).atZone(zone).let { "${it.monthValue}月${it.dayOfMonth}日" }
    }
}
