package top.flysoftbeta.workflow.core.session

import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.core.resource.ResourceRef
import java.time.Instant
import java.time.ZoneId
import java.time.temporal.ChronoUnit
import java.util.Locale
import kotlin.math.ln

data class PersistentRanking(
    /** Sessions the user ordered by dragging, in that order. */
    val pinned: List<Session>,
    /** Every other live persistent session, by [SessionPolicy.score]. */
    val ranked: List<Session>,
)

/** Temporary-session timeline buckets; later buckets render fainter with less information (ui.md §4.9). */
enum class TimelineBucket(val fade: Int, val maxResources: Int) {
    TODAY(0, 3), YESTERDAY(1, 2), THIS_WEEK(2, 1), EARLIER(3, 1)
}

data class TimelineGroup(val bucket: TimelineBucket, val sessions: List<Session>)

data class SessionMatch(val session: Session, val nameMatched: Boolean, val matchedResources: List<ResourceRef>)

enum class ArchiveDecision {
    /** Save every dirty file; unsent conversation input cannot be "saved" and is kept. */
    SAVE_ALL,
    KEEP_DRAFTS,
    /** Drop file drafts and clear unsent conversation input. */
    DISCARD,
}

object SessionPolicy {
    const val DAY_MS = 86_400_000L
    const val HOUR_MS = 3_600_000L
    const val TEMPORARY_RETENTION_MS = DAY_MS
    const val PERSISTENT_RETENTION_MS = 7 * DAY_MS

    /** Recency outweighs frequency ("优先最后使用时间"). */
    const val RECENCY_WEIGHT = 0.65
    const val FREQUENCY_WEIGHT = 0.35
    /** Recency is 1 up to this age, then decays logarithmically to 0 at [RECENCY_HORIZON_MS]. */
    const val RECENCY_FULL_MS = HOUR_MS
    const val RECENCY_HORIZON_MS = 7 * DAY_MS

    // ---- Persistent ranking ----

    fun recency(ageMs: Long): Double = when {
        ageMs <= RECENCY_FULL_MS -> 1.0
        ageMs >= RECENCY_HORIZON_MS -> 0.0
        else -> 1.0 - ln(ageMs.toDouble() / RECENCY_FULL_MS) / ln(RECENCY_HORIZON_MS.toDouble() / RECENCY_FULL_MS)
    }

    /**
     * score = 0.65 * capped recency + 0.35 * frequency, where frequency is ln(1 + decayed uses)
     * normalized by the maximum among [among]. Deterministic for a given [now].
     */
    fun score(session: Session, among: List<Session>, now: Long): Double {
        val max = among.maxOfOrNull { ln(1.0 + it.usage.decayed(now)) } ?: 0.0
        val frequency = if (max <= 0.0) 0.0 else ln(1.0 + session.usage.decayed(now)) / max
        return RECENCY_WEIGHT * recency(now - session.lastUsedAt) + FREQUENCY_WEIGHT * frequency
    }

    fun persistentRanking(sessions: List<Session>, pinned: List<SessionId>, now: Long): PersistentRanking {
        val live = sessions.filter { it.kind == SessionKind.PERSISTENT && !it.isArchived }
        val byId = live.associateBy { it.id }
        val pinnedSessions = pinned.distinct().mapNotNull { byId[it] }
        val rest = live.filter { it.id !in pinned }
        val scores = rest.associate { it.id to score(it, rest, now) }
        val ranked = rest.sortedWith(compareByDescending<Session> { scores.getValue(it.id) }.thenByDescending { it.lastUsedAt }.thenBy { it.id })
        return PersistentRanking(pinnedSessions, ranked)
    }

    // ---- Temporary timeline ----

    fun bucket(time: Long, now: Long, zone: ZoneId): TimelineBucket {
        val days = ChronoUnit.DAYS.between(
            Instant.ofEpochMilli(time).atZone(zone).toLocalDate(),
            Instant.ofEpochMilli(now).atZone(zone).toLocalDate(),
        )
        return when {
            days <= 0 -> TimelineBucket.TODAY
            days == 1L -> TimelineBucket.YESTERDAY
            days < 7 -> TimelineBucket.THIS_WEEK
            else -> TimelineBucket.EARLIER
        }
    }

    /** Temporary sessions, most recently used first, grouped by calendar-day age of last use. */
    fun timeline(sessions: List<Session>, now: Long, zone: ZoneId, archived: Boolean = false): List<TimelineGroup> =
        sessions.filter { it.kind == SessionKind.TEMPORARY && it.isArchived == archived }
            .sortedWith(compareByDescending<Session> { it.lastUsedAt }.thenBy { it.id })
            .groupBy { bucket(it.lastUsedAt, now, zone) }
            .toSortedMap()
            .map { (bucket, list) -> TimelineGroup(bucket, list) }

    // ---- Archive ----

    /**
     * For every dirty resource, only the most recently used live session that references it is
     * protected (ties: later creation, then larger id). Older references may still be archived.
     */
    fun protectedSessions(sessions: List<Session>, dirty: Set<ResourceRef>): Set<SessionId> {
        if (dirty.isEmpty()) return emptySet()
        val live = sessions.filter { !it.isArchived }
        return dirty.mapNotNull { resource ->
            live.filter { resource in it.resources }
                .maxWithOrNull(compareBy<Session> { it.lastUsedAt }.thenBy { it.createdAt }.thenBy { it.id })?.id
        }.toSet()
    }

    fun retention(session: Session): Long =
        if (session.kind == SessionKind.TEMPORARY) TEMPORARY_RETENTION_MS else PERSISTENT_RETENTION_MS

    fun autoArchiveCandidates(sessions: List<Session>, dirty: Set<ResourceRef>, now: Long): List<Session> {
        val protected = protectedSessions(sessions, dirty)
        return sessions.filter { !it.isArchived && it.id !in protected && now - it.lastUsedAt >= retention(it) }
    }

    /** Dirty resources a manual archive must decide about. */
    fun dirtyResources(session: Session, dirty: Set<ResourceRef>): Set<ResourceRef> = session.resources.intersect(dirty)

    // ---- Search ----

    /**
     * Case-insensitive search over session names and referenced resources. Every whitespace-
     * separated token must match the name or one resource (file path/name, or the title that
     * [titleOf] returns, e.g. a conversation title). Name matches first, live before archived,
     * then most recently used.
     */
    fun search(
        sessions: List<Session>,
        query: String,
        titleOf: (ResourceRef) -> String? = { null },
        includeArchived: Boolean = true,
    ): List<SessionMatch> {
        val tokens = query.lowercase(Locale.ROOT).split(Regex("\\s+")).filter { it.isNotEmpty() }
        val candidates = sessions.filter { includeArchived || !it.isArchived }
        if (tokens.isEmpty()) return candidates.sortedByDescending { it.lastUsedAt }.map { SessionMatch(it, false, emptyList()) }
        return candidates.mapNotNull { session ->
            val name = session.name?.lowercase(Locale.ROOT).orEmpty()
            val resources = session.workbench.primaryResources.map { ref -> ref to searchText(ref, titleOf) }
            val matched = mutableSetOf<ResourceRef>()
            var nameMatched = false
            for (token in tokens) {
                val inName = token in name
                val hits = resources.filter { (_, text) -> token in text }.map { it.first }
                if (!inName && hits.isEmpty()) return@mapNotNull null
                nameMatched = nameMatched || inName
                matched += hits
            }
            SessionMatch(session, nameMatched, resources.map { it.first }.filter { it in matched })
        }.sortedWith(
            compareByDescending<SessionMatch> { it.nameMatched }
                .thenBy { it.session.isArchived }
                .thenByDescending { it.session.lastUsedAt }
                .thenBy { it.session.id },
        )
    }

    private fun searchText(ref: ResourceRef, titleOf: (ResourceRef) -> String?): String = when (ref) {
        is ResourceRef.File -> listOfNotNull(ref.path, WorkspacePaths.name(ref.path), titleOf(ref))
        is ResourceRef.Conversation -> listOfNotNull(titleOf(ref))
    }.joinToString("\n").lowercase(Locale.ROOT)
}
