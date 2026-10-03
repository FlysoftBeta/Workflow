package top.flysoftbeta.workflow.agent.model

import kotlinx.serialization.Serializable
import kotlinx.serialization.SerialName

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

/**
 * The app's own list of conversations. Claude sessions are only files and Codex's `thread/list`
 * is scoped to its CODEX_HOME, so the chat list, search, rename and archive work on this index;
 * backends are told about renames/archives where they support it.
 *
 * [id] is app-owned and stable (it is the Working Resource id of the conversation); the backend
 * thread id may be null until the backend has created the thread.
 */
@Serializable
@SerialName("ConversationEntry")
data class ConversationEntry(
    val id: String,
    val backend: BackendKind,
    val backendThreadId: String?,
    val title: String?,
    val cwd: String,
    val createdAtMs: Long,
    val updatedAtMs: Long,
    val archived: Boolean = false,
    /** Source conversation id and the turn/message it was forked at, if any. */
    val forkedFrom: String? = null,
    val forkedAt: String? = null,
    /** Last used slider position, restored for the next turn. */
    val model: String? = null,
    val effort: String? = null,
    /** First user message, for search and untitled rows. */
    val preview: String? = null,
)

/** Persisted as an opaque Workspace Engine document; the agent layer only reads and proposes updates. */
interface ConversationIndex {
    val entries: StateFlow<List<ConversationEntry>>
    suspend fun upsert(entry: ConversationEntry)
    suspend fun remove(id: String)
}

/** Reference implementation for tests and hosts without persistence. */
class InMemoryConversationIndex(initial: List<ConversationEntry> = emptyList()) : ConversationIndex {
    private val state = MutableStateFlow(initial)
    override val entries: StateFlow<List<ConversationEntry>> = state.asStateFlow()
    override suspend fun upsert(entry: ConversationEntry) = state.update { list ->
        if (list.any { it.id == entry.id }) list.map { if (it.id == entry.id) entry else it } else list + entry
    }
    override suspend fun remove(id: String) = state.update { list -> list.filterNot { it.id == id } }
}

object ConversationIndexing {
    /** Chat list order: unarchived first, most recently updated first. */
    fun sorted(entries: List<ConversationEntry>): List<ConversationEntry> =
        entries.sortedWith(compareBy<ConversationEntry> { it.archived }.thenByDescending { it.updatedAtMs })

    fun search(entries: List<ConversationEntry>, query: String): List<ConversationEntry> {
        val q = query.trim().lowercase()
        if (q.isEmpty()) return sorted(entries)
        return sorted(entries.filter { e -> listOfNotNull(e.title, e.preview, e.cwd).any { it.lowercase().contains(q) } })
    }

    /**
     * Folds backend state into an index entry: backend thread id, title, preview and last used
     * model/effort. Returns the same instance when nothing changed so callers can skip writes.
     */
    fun refresh(entry: ConversationEntry, thread: ThreadState?, nowMs: Long): ConversationEntry {
        if (thread == null) return entry
        val lastTurn = thread.turns.lastOrNull()
        val firstUser = thread.turns.asSequence().flatMap { it.items.asSequence() }.filterIsInstance<UserMessageItem>().firstOrNull()?.text
        val updated = entry.copy(
            backendThreadId = entry.backendThreadId ?: thread.key.id,
            title = thread.title ?: entry.title,
            preview = entry.preview ?: firstUser?.take(200),
            model = lastTurn?.settings?.model ?: thread.settings.model ?: entry.model,
            effort = lastTurn?.settings?.effort ?: thread.settings.effort ?: entry.effort,
            archived = entry.archived || thread.archived,
        )
        return if (updated == entry) entry else updated.copy(updatedAtMs = nowMs)
    }
}
