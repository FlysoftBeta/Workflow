package top.flysoftbeta.workflow.engine.chat

import java.io.IOException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonArray
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import top.flysoftbeta.workflow.agent.json.arr
import top.flysoftbeta.workflow.agent.json.bool
import top.flysoftbeta.workflow.agent.json.encode
import top.flysoftbeta.workflow.agent.json.get
import top.flysoftbeta.workflow.agent.json.long
import top.flysoftbeta.workflow.agent.json.parseJson
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.ConversationEntry
import top.flysoftbeta.workflow.agent.model.ConversationIndex

/** `.workspace/documents/chat/conversations.json` format 1 (pure; unit-tested). */
object ConversationIndexCodec {
    const val FORMAT = 1

    fun encode(entries: List<ConversationEntry>): String = buildJsonObject {
        put("format", FORMAT)
        put("conversations", buildJsonArray {
            entries.forEach { e ->
                add(buildJsonObject {
                    put("id", e.id)
                    put("backend", e.backend.id)
                    e.backendThreadId?.let { put("thread", it) }
                    e.title?.let { put("title", it) }
                    put("cwd", e.cwd)
                    put("createdAt", e.createdAtMs)
                    put("updatedAt", e.updatedAtMs)
                    if (e.archived) put("archived", true)
                    e.forkedFrom?.let { put("forkedFrom", it) }
                    e.forkedAt?.let { put("forkedAt", it) }
                    e.model?.let { put("model", it) }
                    e.effort?.let { put("effort", it) }
                    e.preview?.let { put("preview", it) }
                })
            }
        })
    }.encode()

    /** Throws on a malformed document or a newer format (the caller keeps the file untouched). */
    fun decode(text: String): List<ConversationEntry> {
        val root = parseJson(text) as? JsonObject ?: throw IOException("not an object")
        val format = root["format"].long ?: throw IOException("missing format")
        if (format > FORMAT) throw NewerFormatException(format)
        return (root["conversations"].arr ?: JsonArray(emptyList())).mapNotNull { item ->
            val id = item["id"].str?.takeIf { it.isNotBlank() } ?: return@mapNotNull null
            val backend = item["backend"].str?.let(BackendKind::of) ?: return@mapNotNull null
            ConversationEntry(
                id = id,
                backend = backend,
                backendThreadId = item["thread"].str,
                title = item["title"].str,
                cwd = item["cwd"].str ?: "",
                createdAtMs = item["createdAt"].long ?: 0,
                updatedAtMs = item["updatedAt"].long ?: 0,
                archived = item["archived"].bool == true,
                forkedFrom = item["forkedFrom"].str,
                forkedAt = item["forkedAt"].str,
                model = item["model"].str,
                effort = item["effort"].str,
                preview = item["preview"].str,
            )
        }.distinctBy { it.id }
    }

    class NewerFormatException(val format: Long) : IOException("conversations.json format $format is newer than $FORMAT")
}

/** Typed conversation index. WorkspaceStore serializes, debounces and retries every disk write. */
class ConversationIndexFile(
    private val store: top.flysoftbeta.workflow.core.store.WorkspaceStore,
    private val scope: CoroutineScope,
    private val io: CoroutineDispatcher,
) : ConversationIndex {
    private val mutable = MutableStateFlow<List<ConversationEntry>>(emptyList())
    override val entries: StateFlow<List<ConversationEntry>> = mutable.asStateFlow()
    private val loaded = MutableStateFlow(false)
    @Volatile private var loadError: Exception? = null
    private val editLock = Mutex()
    private var started = false

    @Synchronized fun start() {
        if (started) return
        started = true
        scope.launch(io) {
            try {
                val text = store.readConversationIndex()
                mutable.value = if (text == null) emptyList() else try {
                    ConversationIndexCodec.decode(text)
                } catch (newer: ConversationIndexCodec.NewerFormatException) {
                    throw newer
                } catch (_: Exception) {
                    store.quarantineConversationIndex()
                    emptyList()
                }
            } catch (error: Exception) {
                if (error is kotlinx.coroutines.CancellationException) throw error
                loadError = error
            } finally {
                loaded.value = true
            }
        }
    }

    suspend fun awaitLoaded() {
        loaded.first { it }
        loadError?.let { throw it }
    }

    override suspend fun upsert(entry: ConversationEntry) {
        awaitLoaded()
        editLock.withLock {
            val list = mutable.value
            publish(if (list.any { it.id == entry.id }) list.map { if (it.id == entry.id) entry else it } else list + entry)
        }
    }

    /** Serial transforms cannot overwrite a concurrent rename, archive, or model selection. */
    suspend fun update(id: String, transform: (ConversationEntry) -> ConversationEntry): ConversationEntry? {
        awaitLoaded()
        return editLock.withLock {
            val old = mutable.value.firstOrNull { it.id == id } ?: return@withLock null
            val updated = transform(old)
            publish(mutable.value.map { if (it.id == id) updated else it })
            updated
        }
    }

    suspend fun refreshThread(key: top.flysoftbeta.workflow.agent.model.ThreadKey,
        thread: top.flysoftbeta.workflow.agent.model.ThreadState?, now: Long) {
        awaitLoaded()
        val id = entries.value.firstOrNull { it.backend == key.backend && it.backendThreadId == key.id }?.id ?: return
        update(id) { current ->
            if (current.backend != key.backend || current.backendThreadId != key.id) current
            else {
                val refreshed = top.flysoftbeta.workflow.agent.model.ConversationIndexing.refresh(current, thread, now)
                refreshed.copy(title = current.title ?: refreshed.title, archived = current.archived)
            }
        }
    }

    override suspend fun remove(id: String) {
        awaitLoaded()
        editLock.withLock { publish(mutable.value.filterNot { it.id == id }) }
    }

    private suspend fun publish(entries: List<ConversationEntry>) {
        if (entries == mutable.value) return
        store.writeConversationIndex(ConversationIndexCodec.encode(entries))
        mutable.value = entries
    }

    suspend fun flush() { store.flush() }
}
