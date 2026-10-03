package top.flysoftbeta.workflow.core.session

import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.Workbench
import top.flysoftbeta.workflow.core.resource.ResourceRef
import java.time.LocalDateTime
import java.time.ZoneId
import java.time.ZoneOffset

class SessionPolicyTest {
    private val day = SessionPolicy.DAY_MS
    private val hour = SessionPolicy.HOUR_MS
    private val now = 100 * day

    private fun persistent(id: String, lastUsed: Long, uses: Double = 1.0) =
        Session(id, "Name $id", createdAt = 0, lastUsedAt = lastUsed, usage = UsageStats(uses.toInt().coerceAtLeast(1), uses, lastUsed))

    @Test fun `kind follows the name and blank names are rejected`() {
        assertEquals(SessionKind.TEMPORARY, Session("a", createdAt = 0).kind)
        assertEquals(SessionKind.PERSISTENT, Session("a", createdAt = 0).renamed(" Plan ").kind)
        assertThrows(IllegalArgumentException::class.java) { Session("a", " ", createdAt = 0) }
        assertThrows(IllegalArgumentException::class.java) { Session("", createdAt = 0) }
    }

    @Test fun `recency is full for an hour, capped at seven days and monotonic`() {
        assertEquals(1.0, SessionPolicy.recency(0), 0.0)
        assertEquals(1.0, SessionPolicy.recency(hour), 0.0)
        assertEquals(0.0, SessionPolicy.recency(7 * day), 0.0)
        assertEquals(0.0, SessionPolicy.recency(30 * day), 0.0)
        val samples = (1..200).map { SessionPolicy.recency(it * hour) }
        assertTrue(samples.zipWithNext().all { (a, b) -> a >= b })
    }

    @Test fun `recency outweighs frequency but frequency breaks near ties`() {
        val recentRare = persistent("recent", now - 30 * 60_000, 1.0)
        val oldBusy = persistent("busy", now - 3 * day, 50.0)
        assertEquals(listOf("recent", "busy"), SessionPolicy.persistentRanking(listOf(oldBusy, recentRare), emptyList(), now).ranked.map { it.id })
        val a = persistent("a", now - 5 * hour, 2.0)
        val b = persistent("b", now - 6 * hour, 40.0)
        assertEquals("b", SessionPolicy.persistentRanking(listOf(a, b), emptyList(), now).ranked.first().id)
    }

    @Test fun `frequency is normalized so scaling all counts keeps the order`() {
        val base = listOf(persistent("a", now - 2 * hour, 3.0), persistent("b", now - 20 * hour, 9.0), persistent("c", now - 3 * day, 1.0))
        val scaled = base.map { it.copy(usage = it.usage.copy(frecency = it.usage.frecency * 1000)) }
        val order = SessionPolicy.persistentRanking(base, emptyList(), now).ranked.map { it.id }
        assertEquals(order, SessionPolicy.persistentRanking(scaled, emptyList(), now).ranked.map { it.id })
    }

    @Test fun `ranking is deterministic with ties and ignores temporary and archived sessions`() {
        val sessions = listOf(
            persistent("b", now - day), persistent("a", now - day),
            Session("t", createdAt = 0, lastUsedAt = now), persistent("z", now).copy(archivedAt = now),
        )
        assertEquals(listOf("a", "b"), SessionPolicy.persistentRanking(sessions, emptyList(), now).ranked.map { it.id })
        assertEquals(listOf("a", "b"), SessionPolicy.persistentRanking(sessions.reversed(), emptyList(), now).ranked.map { it.id })
    }

    @Test fun `pinned sessions keep the dragged order above the ranking`() {
        val sessions = listOf(persistent("a", now), persistent("b", now - day), persistent("c", now - 2 * day))
        val ranking = SessionPolicy.persistentRanking(sessions, listOf("c", "missing", "b"), now)
        assertEquals(listOf("c", "b"), ranking.pinned.map { it.id })
        assertEquals(listOf("a"), ranking.ranked.map { it.id })
    }

    @Test fun `usage decays and counts episodes, not rapid switches`() {
        val usage = UsageStats.startingAt(0)
        assertEquals(0.5, usage.decayed(UsageStats.HALF_LIFE_MS), 1e-9)
        val session = Session("s", createdAt = 0)
        val quick = session.touched(10 * 60_000).touched(20 * 60_000)
        assertEquals(1, quick.usage.uses)
        val later = quick.activated(20 * 60_000 + UsageStats.EPISODE_GAP_MS)
        assertEquals(2, later.usage.uses)
        assertSame(later, later.touched(0))
    }

    @Test fun `timeline buckets use calendar days in the given zone`() {
        val zone = ZoneId.of("Asia/Shanghai")
        fun at(day: Int, hour: Int) = LocalDateTime.of(2026, 9, day, hour, 0).atZone(zone).toInstant().toEpochMilli()
        val nowAt = at(27, 9)
        assertEquals(TimelineBucket.TODAY, SessionPolicy.bucket(at(27, 0), nowAt, zone))
        assertEquals(TimelineBucket.TODAY, SessionPolicy.bucket(at(28, 0), nowAt, zone))
        assertEquals(TimelineBucket.YESTERDAY, SessionPolicy.bucket(at(26, 23), nowAt, zone))
        assertEquals(TimelineBucket.THIS_WEEK, SessionPolicy.bucket(at(21, 1), nowAt, zone))
        assertEquals(TimelineBucket.EARLIER, SessionPolicy.bucket(at(20, 23), nowAt, zone))
        // One instant, different local days.
        assertEquals(TimelineBucket.YESTERDAY, SessionPolicy.bucket(at(26, 23), nowAt, ZoneOffset.ofHours(8)))
        assertTrue(TimelineBucket.entries.zipWithNext().all { (a, b) -> a.fade < b.fade && a.maxResources >= b.maxResources })
    }

    @Test fun `timeline groups temporary sessions newest first`() {
        val zone = ZoneOffset.UTC
        val now = this.now + 12 * hour
        val sessions = listOf(
            Session("old", createdAt = 0, lastUsedAt = now - 10 * day), Session("today", createdAt = 0, lastUsedAt = now - hour),
            Session("yesterday", createdAt = 0, lastUsedAt = now - day), Session("today2", createdAt = 0, lastUsedAt = now - 2 * hour),
            persistent("p", now), Session("gone", createdAt = 0, lastUsedAt = now, archivedAt = now),
        )
        val groups = SessionPolicy.timeline(sessions, now, zone)
        assertEquals(listOf(TimelineBucket.TODAY, TimelineBucket.YESTERDAY, TimelineBucket.EARLIER), groups.map { it.bucket })
        assertEquals(listOf("today", "today2"), groups.first().sessions.map { it.id })
        assertEquals(listOf("gone"), SessionPolicy.timeline(sessions, now, zone, archived = true).single().sessions.map { it.id })
    }

    @Test fun `search matches names and resources with every token`() {
        val files = Workbench.empty().apply(LayoutOp.Open(PanelTarget.File("src/Theme.kt"))).apply(LayoutOp.showConversation("c1"))
        val sessions = listOf(
            Session("a", "Design notes", createdAt = 0, lastUsedAt = 5),
            Session("b", createdAt = 0, lastUsedAt = 9, workbench = files),
            Session("c", "Theme work", createdAt = 0, lastUsedAt = 1, archivedAt = 2),
        )
        val titles = { ref: ResourceRef -> if (ref == ResourceRef.Conversation("c1")) "Migration plan" else null }
        assertEquals(listOf("c", "b"), SessionPolicy.search(sessions, "theme", titles).map { it.session.id })
        assertEquals(listOf(ResourceRef.File("src/Theme.kt")), SessionPolicy.search(sessions, "THEME.kt", titles).single().matchedResources)
        assertEquals(listOf("b"), SessionPolicy.search(sessions, "migration theme", titles).map { it.session.id })
        assertTrue(SessionPolicy.search(sessions, "nothing", titles).isEmpty())
        assertEquals(listOf("b"), SessionPolicy.search(sessions, "theme", titles, includeArchived = false).map { it.session.id })
        assertEquals(listOf("b", "a", "c"), SessionPolicy.search(sessions, "  ", titles).map { it.session.id })
    }

    @Test fun `protection picks the newest live reference per dirty resource`() {
        val w = Workbench.empty().apply(LayoutOp.Open(PanelTarget.File("a")))
        val sessions = listOf(
            Session("old", createdAt = 0, lastUsedAt = 1, workbench = w),
            Session("new", createdAt = 0, lastUsedAt = 2, workbench = w),
            Session("archived", createdAt = 0, lastUsedAt = 3, archivedAt = 4, workbench = w),
        )
        assertEquals(setOf("new"), SessionPolicy.protectedSessions(sessions, setOf(ResourceRef.File("a"))))
        assertEquals(emptySet<String>(), SessionPolicy.protectedSessions(sessions, emptySet()))
        assertEquals(listOf("old"), SessionPolicy.autoArchiveCandidates(sessions, setOf(ResourceRef.File("a")), 10 * day).map { it.id })
        assertEquals(setOf(ResourceRef.File("a")), SessionPolicy.dirtyResources(sessions[0], setOf(ResourceRef.File("a"), ResourceRef.File("b"))))
    }
}
