package top.flysoftbeta.workflow.core.session

import top.flysoftbeta.workflow.core.layout.Workbench
import top.flysoftbeta.workflow.core.resource.ResourceRef
import kotlin.math.pow

typealias SessionId = String

enum class SessionKind { TEMPORARY, PERSISTENT }

/**
 * Usage for ranking. A "use" is an episode: activating a session, or interacting with it after at
 * least [EPISODE_GAP_MS] of not using it. Rapid switching therefore does not inflate frequency.
 * [frecency] is an exponentially decayed use count (half-life [HALF_LIFE_MS]) as of [frecencyAt].
 */
data class UsageStats(val uses: Int = 1, val frecency: Double = 1.0, val frecencyAt: Long = 0) {
    fun decayed(now: Long): Double {
        val age = (now - frecencyAt).coerceAtLeast(0).toDouble()
        return frecency * 0.5.pow(age / HALF_LIFE_MS)
    }

    fun recordUse(now: Long): UsageStats = UsageStats(
        uses = if (uses == Int.MAX_VALUE) uses else uses + 1,
        frecency = decayed(now) + 1.0,
        frecencyAt = maxOf(now, frecencyAt),
    )

    companion object {
        const val EPISODE_GAP_MS = 30L * 60 * 1000
        const val HALF_LIFE_MS = 7L * 86_400_000
        fun startingAt(now: Long) = UsageStats(1, 1.0, now)
    }
}

/**
 * A coherent stretch of work: its workbench (paradigms, stacks, panels, focus, reading positions)
 * and usage. Never holds unsaved content: that is a Working Resource shared by all sessions.
 * A session is persistent exactly when it has a [name]; naming a temporary session converts it.
 */
data class Session(
    val id: SessionId,
    val name: String? = null,
    val createdAt: Long,
    val lastUsedAt: Long = createdAt,
    val usage: UsageStats = UsageStats.startingAt(createdAt),
    val archivedAt: Long? = null,
    val workbench: Workbench = Workbench.empty(),
) {
    init {
        require(id.isNotBlank()) { "Session id must not be blank" }
        require(name == null || name.isNotBlank()) { "A persistent session needs a name" }
    }

    val kind: SessionKind get() = if (name == null) SessionKind.TEMPORARY else SessionKind.PERSISTENT
    val isArchived: Boolean get() = archivedAt != null
    val resources: Set<ResourceRef> get() = workbench.resources

    /** An interaction: counts as a new use after a pause of at least [UsageStats.EPISODE_GAP_MS]. */
    fun touched(now: Long): Session {
        if (now <= lastUsedAt) return this
        val usage = if (now - lastUsedAt >= UsageStats.EPISODE_GAP_MS) usage.recordUse(now) else usage
        return copy(lastUsedAt = now, usage = usage)
    }

    /** Entering the session (switch, restore, return from the Launcher): the same episode rule. */
    fun activated(now: Long): Session = touched(now)

    fun renamed(name: String): Session = copy(name = name.trim().also { require(it.isNotEmpty()) { "A persistent session needs a name" } })
}
