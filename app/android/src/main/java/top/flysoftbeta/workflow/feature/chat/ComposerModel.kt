package top.flysoftbeta.workflow.feature.chat

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import top.flysoftbeta.workflow.core.resource.ComposerAttachment
import top.flysoftbeta.workflow.core.resource.ComposerDraft
import top.flysoftbeta.workflow.core.resource.ComposerRevisionConflictException
import top.flysoftbeta.workflow.core.store.StoreStatus
import top.flysoftbeta.workflow.core.store.WorkspaceStore

/** A pending attachment in the strip. [path] is null while it is still being imported. */
data class PendingAttachment(
    val key: String,
    val name: String,
    val path: String?,
    val image: Boolean,
    val failed: Boolean = false,
    val thumbnail: ImageBitmap? = null,
)

/**
 * The composer's text and attachments for one conversation. The text is a Working Resource: it is
 * saved to the store's composer draft (debounced, revision-checked) so it survives closing the
 * panel, switching sessions and process death (product.md §4), and is cleared only after a send is
 * acknowledged.
 */
class ComposerModel(
    private val conversationId: String,
    private val store: WorkspaceStore,
    private val services: ChatFeatureServices,
    private val scope: CoroutineScope,
) {
    var text by mutableStateOf(TextFieldValue(""))
        private set
    val attachments = mutableStateListOf<PendingAttachment>()
    /** Full authoritative baseline: unchanged content must retain its Engine-owned revision. */
    private var baseline = ComposerDraft(conversationId)
    private val persistence = Mutex()
    private var saveJob: Job? = null
    var loaded by mutableStateOf(false)
        private set

    init {
        scope.launch {
            store.state.first { it.status != StoreStatus.LOADING }
            adopt(store.state.value.composer(conversationId))
            loaded = true
        }
    }

    val hasContent: Boolean get() = text.text.isNotBlank() || attachments.any { it.path != null }
    val importing: Boolean get() = attachments.any { it.path == null && !it.failed }

    fun onTextChange(value: TextFieldValue) {
        val changed = value.text != text.text
        text = value
        if (changed) scheduleSave()
    }

    fun insertText(value: String) {
        val t = text
        val next = t.text.substring(0, t.selection.min) + value + t.text.substring(t.selection.max)
        onTextChange(TextFieldValue(next, TextRange(t.selection.min + value.length)))
    }

    fun addPaths(paths: List<String>) {
        var added = false
        for (path in paths) {
            if (attachments.any { it.path == path }) continue
            val key = "p:$path"
            attachments += PendingAttachment(key, path.substringAfterLast('/'), path, isImagePath(path))
            added = true
            if (isImagePath(path)) scope.launch {
                val bitmap = loadThumbnail(services, path, 112)
                val index = attachments.indexOfFirst { it.key == key }
                if (index >= 0 && bitmap != null) attachments[index] = attachments[index].copy(thumbnail = bitmap)
            }
        }
        if (added) scheduleSave()
    }

    /** Shows an importing chip; call [finishImport] with the resulting paths. */
    fun beginImport(label: String): String {
        val key = "i:" + System.nanoTime()
        attachments += PendingAttachment(key, label, null, false)
        return key
    }

    fun finishImport(key: String, paths: List<String>) {
        val index = attachments.indexOfFirst { it.key == key }
        if (index >= 0) attachments.removeAt(index)
        addPaths(paths)
    }

    fun failImport(key: String) {
        val index = attachments.indexOfFirst { it.key == key }
        if (index >= 0) attachments[index] = attachments[index].copy(failed = true)
    }

    fun remove(key: String) {
        val index = attachments.indexOfFirst { it.key == key }
        if (index < 0) return
        val removed = attachments.removeAt(index)
        if (removed.path != null) scheduleSave()
    }

    /** The draft as it should be stored now. */
    private fun draft(): ComposerDraft = baseline.edit(
        text = text.text,
        attachments = attachments.mapNotNull { a -> a.path?.let { path ->
            baseline.attachments.firstOrNull { it.path == path } ?: ComposerAttachment(path)
        } },
    )

    private fun scheduleSave() {
        saveJob?.cancel()
        saveJob = scope.launch {
            delay(400)
            save()
        }
    }

    /** Writes the draft now; returns what is stored (the snapshot a send acknowledges). */
    suspend fun save(): ComposerDraft {
        saveJob?.takeIf { it.isActive && it != kotlin.coroutines.coroutineContext[Job] }?.cancel()
        return persistence.withLock {
            val next = draft()
            try {
                store.editComposer(next, baseline.revision).also { baseline = it }
            } catch (conflict: ComposerRevisionConflictException) {
                // Another session edited this conversation's composer: adopt the stored version.
                store.state.value.composer(conversationId).also(::adopt)
            }
        }
    }

    /** Clears the UI for a send; [restore] puts the content back when the send failed. */
    fun takeForSend(): Pair<String, List<ComposerAttachment>> {
        val sent = text.text to attachments.mapNotNull { a -> a.path?.let { ComposerAttachment(it) } }
        text = TextFieldValue("")
        attachments.removeAll { it.path != null }
        return sent
    }

    fun restore(text: String, paths: List<String>) {
        if (this.text.text.isEmpty()) this.text = TextFieldValue(text, TextRange(text.length))
        addPaths(paths)
    }

    /** Engine acknowledges the exact submitted revision; this only refreshes the UI projection. */
    fun accepted(submitted: ComposerDraft) {
        scope.launch {
            val stored = store.state.first { it.composer(conversationId).revision > submitted.revision }.composer(conversationId)
            persistence.withLock {
                if (stored.revision >= baseline.revision) {
                    baseline = stored
                    // The user may already be writing the next turn. Preserve that local input,
                    // but derive its next save from the acknowledged authoritative baseline.
                    if (text.text.isEmpty() && attachments.none { it.path != null }) adopt(stored)
                }
            }
        }
    }

    private fun adopt(draft: ComposerDraft) {
        baseline = draft
        if (draft.text != text.text) text = TextFieldValue(draft.text, TextRange(draft.text.length))
        val keep = attachments.filter { it.path == null }
        attachments.clear()
        attachments.addAll(keep)
        addPathsSilently(draft.attachments.map { it.path })
    }

    private fun addPathsSilently(paths: List<String>) {
        val job = saveJob
        addPaths(paths)
        if (job !== saveJob) saveJob?.cancel()
    }
}
