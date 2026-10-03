package top.flysoftbeta.workflow.core.store

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.callbackFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import top.flysoftbeta.workflow.core.config.AppConfig
import top.flysoftbeta.workflow.core.config.ConfigCodec
import top.flysoftbeta.workflow.core.config.ConfigParse
import top.flysoftbeta.workflow.core.config.Launcher
import top.flysoftbeta.workflow.core.io.FileEntry
import top.flysoftbeta.workflow.core.io.FileSystem
import top.flysoftbeta.workflow.core.io.FileWatcher
import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.core.io.readText
import top.flysoftbeta.workflow.core.io.writeAtomic
import top.flysoftbeta.workflow.core.json.Json
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.Workbench
import top.flysoftbeta.workflow.core.resource.ComposerDraft
import top.flysoftbeta.workflow.core.resource.ComposerRevisionConflictException
import top.flysoftbeta.workflow.core.resource.ConflictResolution
import top.flysoftbeta.workflow.core.resource.DiskVersion
import top.flysoftbeta.workflow.core.resource.Drafts
import top.flysoftbeta.workflow.core.resource.FileDraft
import top.flysoftbeta.workflow.core.resource.ResourceRef
import top.flysoftbeta.workflow.core.session.ArchiveDecision
import top.flysoftbeta.workflow.core.session.Session
import top.flysoftbeta.workflow.core.session.SessionId
import top.flysoftbeta.workflow.core.session.SessionKind
import top.flysoftbeta.workflow.core.session.SessionPolicy
import java.io.IOException
import java.io.InputStream
import java.nio.ByteBuffer
import java.nio.charset.CharacterCodingException
import java.nio.charset.CodingErrorAction
import java.util.UUID

data class StoreOptions(
    val sessionsDebounceMs: Long = 500,
    val draftDebounceMs: Long = 400,
    /** Upper bound for how long a change can stay unwritten while edits keep coming. */
    val maxWriteDelayMs: Long = 2_000,
    val externalDebounceMs: Long = 150,
    val retryDelaysMs: List<Long> = listOf(1_000, 5_000, 15_000, 60_000),
    /** Larger files are not opened as text. */
    val maxEditableBytes: Long = 16L * 1024 * 1024,
    /** Larger files are versioned by size + mtime only. */
    val maxHashedBytes: Long = 64L * 1024 * 1024,
    /**
     * Extra save validators by workspace path (for example container.json from `core.environment`):
     * return an error message to refuse the save. config.json is always validated.
     */
    val validators: Map<String, (String) -> String?> = emptyMap(),
)

/**
 * Test-only reference engine from the accepted JVM baseline. Never packaged in production.
 *
 * All commands run one at a time on a serial actor; [state] is the only published snapshot. No
 * method blocks the caller's thread: suspend functions wait for their command, `post`-style
 * functions ([layout], [editFile]) return immediately. Persistence is debounced per file and
 * written atomically through [fs]; typing only rewrites that file's draft. External changes
 * arrive through [watcher] and are verified by re-reading versions.
 *
 * Create one per process, call [start] once, [flush] when the app goes to the background.
 */
class ReferenceWorkspaceStore(
    private val fs: FileSystem,
    private val scope: CoroutineScope,
    /** Serial or IO dispatcher; blocking file IO runs on it. */
    private val dispatcher: CoroutineDispatcher,
    private val watcher: FileWatcher = FileWatcher.NONE,
    private val clock: () -> Long = System::currentTimeMillis,
    private val newId: () -> String = { UUID.randomUUID().toString() },
    private val options: StoreOptions = StoreOptions(),
) : WorkspaceStore {
    private val mutableState = MutableStateFlow(WorkspaceState())
    override val state: StateFlow<WorkspaceState> = mutableState.asStateFlow()

    /** Only the actor reads or writes this. */
    private var s: WorkspaceState
        get() = mutableState.value
        set(value) { mutableState.value = value }

    private val inbox = Channel<() -> Unit>(Channel.UNLIMITED)
    private var loop: Job? = null

    // ---- Lifecycle ----

    /** Starts the actor; loading runs before any other command. */
    override fun start() {
        synchronized(this) {
            if (loop != null) return
            loop = scope.launch(dispatcher) {
                runTask { load() }
                for (task in inbox) runTask(task)
            }
        }
    }

    override suspend fun awaitReady(): WorkspaceState = state.first { it.status != StoreStatus.LOADING }

    /** Writes everything pending now. Call when the app goes to the background. */
    override suspend fun flush(): Boolean = call { flushPending() }

    /** Flushes, stops watching and stops the actor. */
    override suspend fun close() {
        call {
            flushPending()
            watches.values.forEach { runCatching { it.close() } }
            watches.clear()
            listOfNotNull(debounceJob, deadlineJob, externalJob).forEach { it.cancel() }
        }
        inbox.close()
    }

    private fun runTask(task: () -> Unit) {
        try {
            task()
        } catch (error: CancellationException) {
            throw error
        } catch (_: Exception) {
            // A failed fire-and-forget command leaves the state unchanged.
        }
    }

    private suspend fun <R> call(block: () -> R): R {
        val result = CompletableDeferred<R>()
        inbox.send {
            try { result.complete(block()) } catch (error: Throwable) { result.completeExceptionally(error) }
        }
        return result.await()
    }

    private fun post(block: () -> Unit) {
        inbox.trySend(block)
    }

    // ---- Sessions ----

    /** "点击 Workbench": the active session, else the most recently used live one, else a new one. */
    override suspend fun enterWorkbench(): SessionId = call {
        val existing = s.activeSession ?: s.liveSessions.maxByOrNull { it.lastUsedAt }
        if (existing != null) { activateInternal(existing.id); existing.id } else createInternal(null, Workbench.empty())
    }

    /** A new session (temporary without [name]) that becomes active. */
    override suspend fun createSession(name: String?): SessionId = call {
        val clean = name?.trim()?.also { require(it.isNotEmpty()) { "A persistent session needs a name" } }
        createInternal(clean, Workbench.empty())
    }

    /** "在单独的会话中打开": a new temporary session showing only [target] (Solo). */
    override suspend fun openInSeparateSession(target: PanelTarget): SessionId = call { createInternal(null, Workbench.solo(target)) }

    /** Switches to [id] (restoring it if archived). */
    override suspend fun activateSession(id: SessionId): Boolean = call { activateInternal(id) }

    /** Names a session; a temporary session becomes persistent. */
    override suspend fun renameSession(id: SessionId, name: String): Boolean = call {
        val session = s.session(id) ?: return@call false
        replaceSession(session.renamed(name))
        true
    }

    override suspend fun restoreSession(id: SessionId): Boolean = activateSession(id)

    /**
     * Manual archive. Unsaved content referenced by the session requires [decision]
     * ([ArchiveOutcome.NeedsDecision] otherwise). SAVE_ALL is all-or-nothing on conflicts.
     */
    override suspend fun archiveSession(id: SessionId, decision: ArchiveDecision?): ArchiveOutcome = call { archiveInternal(id, decision) }

    /** Drag reorder in the persistent list: pins [id] at [index] of the pinned group. */
    override suspend fun pinSession(id: SessionId, index: Int): Boolean = call {
        val session = s.session(id)?.takeIf { it.kind == SessionKind.PERSISTENT } ?: return@call false
        val rest = s.pinned.filter { it != session.id }
        s = s.copy(pinned = rest.toMutableList().apply { add(index.coerceIn(0, rest.size), session.id) })
        markSessions()
        true
    }

    override suspend fun unpinSession(id: SessionId): Boolean = call {
        if (id !in s.pinned) return@call false
        s = s.copy(pinned = s.pinned - id)
        markSessions()
        true
    }

    /** Auto-archive and re-check tracked files. Call on resume and periodically (e.g. hourly). */
    override suspend fun runMaintenance() = call {
        autoArchiveInternal()
        refreshPaths(trackedPaths(), force = false)
        runCatching { trash.purge(clock()) }
        Unit
    }

    private val trash = WorkspaceTrash(fs, clock, newId)

    override suspend fun dismissNotice(id: String) = call { s = s.copy(notices = s.notices.filter { it.id != id }) }

    // ---- Layout ----

    /** Applies [op] to [sessionId]'s workbench; returns immediately. Archived sessions are read-only. */
    override fun layout(sessionId: SessionId, op: LayoutOp) = post { layoutInternal(sessionId, op) }

    /** Applies [op] to the active session. */
    override fun layout(op: LayoutOp) = post { s.activeSessionId?.let { layoutInternal(it, op) } }

    /** Like [layout] but waits and returns the resulting workbench (null for a missing or archived session). */
    override suspend fun applyLayout(sessionId: SessionId, op: LayoutOp): Workbench? = call { layoutInternal(sessionId, op) }

    // ---- Files and drafts ----

    /** Reads [path] for display. The editor shows [FileSnapshot.text] and passes [FileSnapshot.shownVersion] to [editFile]. */
    override suspend fun openFile(path: String): FileSnapshot = call {
        val clean = editablePath(path)
        val stat = fs.stat(clean)
        val tooLarge = stat != null && !stat.isDirectory && stat.size > options.maxEditableBytes
        var diskText: String? = null
        var binary = false
        val version = if (stat == null || stat.isDirectory) DiskVersion.MISSING
        else if (tooLarge) version(clean)
        else {
            val bytes = fs.readBytes(clean)
            diskText = decodeText(bytes)
            binary = diskText == null
            DiskVersion.of(bytes, stat.modifiedAt)
        }
        observe(clean, version)
        syncWatches()
        FileSnapshot(clean, version, diskText, binary, tooLarge, s.drafts[clean])
    }

    /** Editor text changed. Fire-and-forget; persistence is debounced. [shown] is the version the buffer came from. */
    override fun editFile(path: String, text: String, shown: DiskVersion) = post { editInternal(path, text, shown) }

    /** Saves the draft (after applying [text] when given). Never overwrites a file that changed on disk. */
    override suspend fun saveFile(path: String, text: String?): SaveResult = call {
        val clean = editablePath(path)
        if (text != null) editInternal(clean, text, s.drafts[clean]?.base ?: s.disk[clean] ?: version(clean))
        val draft = s.drafts[clean] ?: return@call SaveResult.Unchanged(currentVersion(clean))
        saveInternal(draft)
    }

    /** Conflict resolution: KEEP_MINE rebases the draft on the current disk version; TAKE_DISK drops it. */
    override suspend fun resolveConflict(path: String, resolution: ConflictResolution) = call {
        val clean = editablePath(path)
        val draft = s.drafts[clean] ?: return@call
        val current = currentVersion(clean)
        when (resolution) {
            ConflictResolution.KEEP_MINE -> setDraft(clean, Drafts.rebase(draft, current, clock()))
            ConflictResolution.TAKE_DISK -> setDraft(clean, null)
        }
    }

    override suspend fun discardDraft(path: String) = call { setDraft(editablePath(path), null) }

    /** Renames or moves a file or directory; drafts, panels in every session and attachments follow. */
    override suspend fun movePath(from: String, to: String): FileOpResult = fileOp {
        val source = editablePath(from)
        val target = editablePath(to)
        fs.move(source, target)
        val drafts = s.drafts.entries.associate { (path, draft) ->
            val moved = WorkspacePaths.rebase(path, source, target)
            if (moved == null) path to draft else {
                dirtyDrafts += path; dirtyDrafts += moved
                moved to draft.copy(path = moved)
            }
        }
        val disk = s.disk.entries.associate { (path, version) -> (WorkspacePaths.rebase(path, source, target) ?: path) to version }
        val composers = s.composers.mapValues { (_, composer) ->
            val attachments = composer.attachments.map { a -> WorkspacePaths.rebase(a.path, source, target)?.let { a.copy(path = it) } ?: a }
            if (attachments == composer.attachments) composer else composer.copy(attachments = attachments).also { dirtyComposers += it.conversationId }
        }
        val sessions = s.sessions.map { it.copy(workbench = it.workbench.apply(LayoutOp.RenamePath(source, target))) }
        s = s.copy(drafts = drafts, disk = disk, composers = composers, sessions = sessions)
        markSessions()
        schedule(options.draftDebounceMs)
        syncWatches()
    }

    /** Deletes a file or directory (after the UI's confirmation). Clean panels close; drafts are kept. */
    override suspend fun deletePath(path: String): FileOpResult = fileOp {
        val clean = editablePath(path)
        fs.delete(clean)
        afterRemoval(clean)
    }

    /** After [clean] disappeared: close clean panels showing it (drafts are Working Resources and stay). */
    private fun afterRemoval(clean: String) {
        val sessions = s.sessions.map { session ->
            val closing = session.workbench.panels.values.filter { panel ->
                val resource = panel.target.resource as? ResourceRef.File ?: return@filter false
                WorkspacePaths.isWithin(resource.path, clean) && resource.path !in s.drafts
            }.map { it.id }
            if (closing.isEmpty()) session else session.copy(workbench = session.workbench.apply(LayoutOp.Close(closing)))
        }
        s = s.copy(sessions = sessions)
        markSessions()
        refreshPaths(trackedPaths().filter { WorkspacePaths.isWithin(it, clean) }, force = true)
        syncWatches()
    }

    /**
     * "删除": moves [path] to `.workspace/trash` (restorable with [restoreFromTrash] until maintenance
     * purges it after [TrashPolicy.RETENTION_MS]). Panels close and drafts stay exactly as for [deletePath].
     */
    override suspend fun trashPath(path: String): TrashResult = call {
        try {
            val clean = editablePath(path)
            val entry = trash.put(clean)
            afterRemoval(clean)
            TrashResult.Trashed(entry)
        } catch (error: IOException) {
            TrashResult.Failed(error.message ?: "File operation failed")
        } catch (error: IllegalArgumentException) {
            TrashResult.Failed(error.message ?: "Invalid path")
        }
    }

    /** Undo of [trashPath]: puts the item back at its path (or a "name (n)" variant if that was taken since). */
    override suspend fun restoreFromTrash(id: String): RestoreResult = call {
        try {
            val restored = trash.restore(id)
            refreshPaths(trackedPaths().filter { WorkspacePaths.isWithin(it, restored) }, force = true)
            syncWatches()
            RestoreResult.Restored(restored)
        } catch (error: IOException) {
            RestoreResult.Failed(error.message ?: "Restore failed")
        } catch (error: IllegalArgumentException) {
            RestoreResult.Failed(error.message ?: "Restore failed")
        }
    }

    /** Removes trash entries older than [TrashPolicy.RETENTION_MS]; part of [runMaintenance]. */
    override suspend fun purgeTrash(): Int = call { runCatching { trash.purge(clock()) }.getOrDefault(0) }

    /**
     * "复制": copies a file or folder to [to] (which must not exist). Files larger than
     * [StoreOptions.maxEditableBytes] are refused; the copy is a new file, drafts do not follow.
     */
    override suspend fun copyPath(from: String, to: String): FileOpResult = fileOp {
        val source = editablePath(from)
        val target = editablePath(to)
        if (fs.stat(target) != null) throw IOException("Already exists: $target")
        if (WorkspacePaths.isWithin(target, source)) throw IOException("Cannot copy a folder into itself")
        fun copy(src: String, dst: String) {
            val stat = fs.stat(src) ?: throw IOException("Nothing to copy at $src")
            if (stat.isDirectory) {
                fs.createDirectories(dst)
                fs.list(src).forEach { copy(it.path, WorkspacePaths.child(dst, it.name)) }
            } else {
                if (stat.size > options.maxEditableBytes) throw IOException("File too large to copy: ${WorkspacePaths.name(src)}")
                fs.writeAtomic(dst, fs.readBytes(src))
            }
        }
        copy(source, target)
    }

    /** Creates a file; fails if something exists at [path]. */
    override suspend fun createFile(path: String, bytes: ByteArray): FileOpResult = importFile(path, bytes.size.toLong()) { bytes.inputStream() }

    /** Streams content into a new file (camera, gallery, device files); never overwrites. */
    override suspend fun importFile(path: String, size: Long, open: () -> InputStream): FileOpResult = fileOp {
        val clean = editablePath(path)
        if (fs.stat(clean) != null) throw IOException("Already exists: $clean")
        fs.writeAtomic(clean) { output -> open().use { input -> check(input.copyTo(output) == size) { "Import size changed" } } }
        if (clean in trackedPaths()) refreshPaths(listOf(clean), force = true)
    }

    override suspend fun createDirectory(path: String): FileOpResult = fileOp {
        val clean = editablePath(path)
        if (fs.stat(clean) != null) throw IOException("Already exists: $clean")
        fs.createDirectories(clean)
    }

    /** Explorer listing; app-internal directories are never listed, dot files only with [showHidden]. */
    override suspend fun listDirectory(path: String, showHidden: Boolean): List<FileEntry> = withContext(dispatcher) {
        val clean = WorkspacePaths.normalize(path)
        fs.list(clean).filter { entry ->
            !WorkspacePaths.isHiddenInExplorer(entry.path) && (showHidden || !entry.name.startsWith("."))
        }.sortedWith(compareByDescending<FileEntry> { it.isDirectory }.thenBy { it.name.lowercase() })
    }

    /** Emits when the direct children of [path] change (for the explorer). */
    override fun directoryChanges(path: String): Flow<Unit> = callbackFlow {
        val handle = watcher.watch(WorkspacePaths.normalize(path)) { trySend(Unit) }
        awaitClose { handle.close() }
    }

    // ---- Composer ----

    /**
     * Stores an edited composer snapshot. [expectedRevision] is the revision the snapshot was
     * derived from; a stale or regressing snapshot throws [ComposerRevisionConflictException].
     * Attachments must be workspace files outside app-internal state.
     */
    override suspend fun editComposer(draft: ComposerDraft, expectedRevision: Long): ComposerDraft = call {
        require(draft.conversationId.isNotBlank()) { "Missing conversation id" }
        draft.attachments.forEach { editablePath(it.path) }
        val current = s.composer(draft.conversationId)
        if (draft == current) return@call current
        if (current.revision != expectedRevision || draft.revision <= current.revision) throw ComposerRevisionConflictException(draft.conversationId)
        setComposer(draft)
        draft
    }

    /** After sending: clears the composer only if it still holds exactly [submitted]. */
    override suspend fun acknowledgeComposer(submitted: ComposerDraft): ComposerDraft = call {
        val current = s.composer(submitted.conversationId)
        val cleared = current.acknowledge(submitted) ?: return@call current
        setComposer(cleared)
        cleared
    }

    override suspend fun discardComposer(conversationId: String): ComposerDraft = call {
        val current = s.composer(conversationId)
        if (!current.hasContent) return@call current
        current.edit("", emptyList()).also(::setComposer)
    }

    /** User-confirmed deletion closes every reference, including archived sessions, and clears its draft. */
    override suspend fun removeConversation(conversationId: String) = call {
        val draft = s.composer(conversationId)
        if (draft.hasContent) setComposer(draft.edit("", emptyList()))
        s = s.copy(sessions = s.sessions.map { session ->
            val ids = session.workbench.panels.values.filter {
                (it.target as? PanelTarget.Conversation)?.conversationId == conversationId
            }.map { it.id }
            if (ids.isEmpty()) session else session.copy(workbench = session.workbench.apply(LayoutOp.Close(ids)))
        })
        markSessions()
        syncWatches()
    }

    // ---- Config ----

    /** Applies a settings change to config.json (unknown keys kept). Refused while the file has errors. */
    override suspend fun updateConfig(transform: (AppConfig) -> AppConfig): ConfigUpdate = call {
        s.configProblem?.let { return@call ConfigUpdate.Blocked(it) }
        try {
            val next = transform(s.config).let { it.copy(launcher = Launcher.normalize(it.launcher)) }
            writeConfig(next)
            ConfigUpdate.Updated(s.config)
        } catch (error: IllegalArgumentException) {
            ConfigUpdate.Failed(error.message ?: "Invalid configuration")
        } catch (error: IOException) {
            ConfigUpdate.Failed(error.message ?: "Write failed")
        }
    }

    // ---- Agent-owned schema, store-owned persistence ----

    /** The agent layer decodes its own index; all state-file IO still runs through this actor. */
    override suspend fun readConversationIndex(): String? = call {
        pendingConversationIndex ?: if (fs.stat(StatePaths.CONVERSATIONS) == null) null else fs.readText(StatePaths.CONVERSATIONS)
    }

    override suspend fun writeConversationIndex(text: String) = call {
        check(s.status == StoreStatus.READY) { "Workspace is not ready" }
        pendingConversationIndex = text
        schedule(options.draftDebounceMs)
    }

    /** Preserve an unreadable index before its decoder starts a fresh one. */
    override suspend fun quarantineConversationIndex() = call {
        if (fs.stat(StatePaths.CONVERSATIONS) != null) {
            val target = "${StatePaths.CORRUPT}/${clock()}-conversations.json"
            fs.createDirectories(StatePaths.CORRUPT)
            fs.move(StatePaths.CONVERSATIONS, target)
            s = s.copy(notices = s.notices + notice(NoticeKind.RECOVERED, "对话索引损坏，原文件已保留"))
        }
    }

    // =====================================================================================
    // Actor internals. Everything below runs on the actor only.
    // =====================================================================================

    private fun load() {
        try {
            fs.createDirectories(StatePaths.ROOT)
            cleanTemporaryFiles()
            val notices = mutableListOf<StoreNotice>()
            val sessions = readSessions(notices)
            val drafts = readDrafts(notices)
            val composers = readComposers(notices)
            s = s.copy(
                sessions = sessions.sessions, activeSessionId = sessions.activeSessionId, pinned = sessions.pinned,
                drafts = drafts, composers = composers, notices = notices,
            )
            loadConfig(initial = true)
            refreshPaths(s.drafts.keys.toList(), force = true)
            autoArchiveInternal()
            s = s.copy(status = StoreStatus.READY)
            syncWatches()
        } catch (error: Exception) {
            s = s.copy(status = StoreStatus.FAILED, failure = error.message ?: error.javaClass.simpleName)
        }
    }

    /** Leftovers of interrupted atomic writes; recent ones may belong to another writer's in-flight write. */
    private fun cleanTemporaryFiles() {
        val cutoff = clock() - 10 * 60_000L
        fs.list(WorkspacePaths.TMP).filter { it.modifiedAt < cutoff }.forEach { runCatching { fs.delete(it.path) } }
    }

    private fun notice(kind: NoticeKind, message: String) = StoreNotice(newId(), kind, message)

    private fun preserveCorrupt(path: String, notices: MutableList<StoreNotice>, what: String) {
        runCatching {
            val bytes = fs.readBytes(path)
            fs.writeAtomic("${StatePaths.CORRUPT}/${clock()}-${WorkspacePaths.name(path)}", bytes)
        }
        notices += notice(NoticeKind.RECOVERED, what)
    }

    private var sessionsReadOnly = false
    private var lastSessionsText: String? = null

    private fun readSessions(notices: MutableList<StoreNotice>): StateCodec.SessionsDocument {
        val empty = StateCodec.SessionsDocument(emptyList(), null, emptyList(), 0)
        for (path in listOf(StatePaths.SESSIONS, StatePaths.SESSIONS_BACKUP)) {
            if (fs.stat(path)?.isDirectory != false) continue
            try {
                val text = fs.readText(path)
                val document = StateCodec.decodeSessions(text)
                if (path == StatePaths.SESSIONS_BACKUP && fs.stat(StatePaths.SESSIONS) != null) {
                    preserveCorrupt(StatePaths.SESSIONS, notices, "sessions.json was damaged; the previous version was restored")
                }
                if (document.dropped > 0) notices += notice(NoticeKind.RECOVERED, "${document.dropped} damaged sessions were skipped")
                lastSessionsText = text
                return document
            } catch (error: UnsupportedFormatException) {
                sessionsReadOnly = true
                notices += notice(NoticeKind.NEWER_FORMAT, error.message ?: "sessions.json is from a newer version")
                return empty
            } catch (_: Exception) {
                // Try the backup next.
            }
        }
        if (fs.stat(StatePaths.SESSIONS) != null) preserveCorrupt(StatePaths.SESSIONS, notices, "sessions.json could not be read; sessions were reset")
        return empty
    }

    private fun readDrafts(notices: MutableList<StoreNotice>): Map<String, FileDraft> {
        val drafts = linkedMapOf<String, FileDraft>()
        fs.list(StatePaths.DRAFTS).filter { !it.isDirectory && it.name.endsWith(".json") }.forEach { entry ->
            try {
                val draft = StateCodec.decodeDraft(fs.readText(entry.path))
                val path = WorkspacePaths.normalize(draft.path)
                require(path.isNotEmpty() && !WorkspacePaths.isReserved(path)) { "reserved path" }
                val previous = drafts[path]
                if (previous == null || draft.editedAt > previous.editedAt) drafts[path] = draft.copy(path = path)
            } catch (_: UnsupportedFormatException) {
                // Written by a newer version: leave it alone.
            } catch (_: Exception) {
                preserveCorrupt(entry.path, notices, "A damaged draft file was set aside")
                runCatching { fs.delete(entry.path) }
            }
        }
        return drafts
    }

    private fun readComposers(notices: MutableList<StoreNotice>): Map<String, ComposerDraft> {
        val composers = linkedMapOf<String, ComposerDraft>()
        fs.list(StatePaths.COMPOSERS).filter { !it.isDirectory && it.name.endsWith(".json") }.forEach { entry ->
            try {
                val composer = StateCodec.decodeComposer(fs.readText(entry.path))
                val previous = composers[composer.conversationId]
                if (previous == null || composer.revision > previous.revision) composers[composer.conversationId] = composer
            } catch (_: UnsupportedFormatException) {
            } catch (_: Exception) {
                preserveCorrupt(entry.path, notices, "A damaged composer draft was set aside")
                runCatching { fs.delete(entry.path) }
            }
        }
        return composers
    }

    // ---- Config ----

    private var configDocument: Map<String, Any?> = emptyMap()

    private fun loadConfig(initial: Boolean) {
        val path = WorkspacePaths.CONFIG
        val stat = fs.stat(path)
        if (stat == null || stat.isDirectory) {
            if (initial && stat == null) writeConfig(AppConfig()) else {
                configDocument = emptyMap()
                s = s.copy(config = AppConfig(), configProblem = null)
                observe(path, DiskVersion.MISSING)
            }
            return
        }
        val bytes = fs.readBytes(path)
        observe(path, DiskVersion.of(bytes, stat.modifiedAt))
        val text = bytes.toString(Charsets.UTF_8)
        when (val parsed = ConfigCodec.decode(text)) {
            is ConfigParse.Ok -> {
                configDocument = parsed.document
                s = s.copy(config = parsed.config, configProblem = null)
                rememberLastGoodConfig(text)
            }
            is ConfigParse.Invalid -> {
                val fallback = if (!initial) s.config else lastGoodConfig() ?: AppConfig()
                s = s.copy(config = fallback, configProblem = parsed.message)
            }
        }
    }

    private fun lastGoodConfig(): AppConfig? = try {
        val text = fs.readText(StatePaths.CONFIG_LAST_GOOD)
        (ConfigCodec.decode(text) as? ConfigParse.Ok)?.also { configDocument = it.document }?.config
    } catch (_: Exception) { null }

    private fun rememberLastGoodConfig(text: String) {
        val previous = runCatching { fs.readText(StatePaths.CONFIG_LAST_GOOD) }.getOrNull()
        if (previous != text) runCatching { fs.writeAtomic(StatePaths.CONFIG_LAST_GOOD, text) }
    }

    private fun writeConfig(config: AppConfig) {
        val text = ConfigCodec.encode(config, configDocument)
        val parsed = ConfigCodec.decode(text) as? ConfigParse.Ok ?: throw IllegalArgumentException("Configuration does not round-trip")
        fs.writeAtomic(WorkspacePaths.CONFIG, text)
        configDocument = parsed.document
        s = s.copy(config = parsed.config, configProblem = null)
        rememberLastGoodConfig(text)
        observe(WorkspacePaths.CONFIG, currentVersion(WorkspacePaths.CONFIG))
    }

    // ---- Sessions ----

    private fun replaceSession(session: Session) {
        s = s.copy(sessions = s.sessions.map { if (it.id == session.id) session else it })
        markSessions()
    }

    private fun createInternal(name: String?, workbench: Workbench): SessionId {
        val now = clock()
        val session = Session(id = newId(), name = name, createdAt = now, workbench = workbench)
        s = s.copy(sessions = s.sessions + session, activeSessionId = session.id)
        markSessions()
        syncWatches()
        return session.id
    }

    private fun activateInternal(id: SessionId): Boolean {
        val session = s.session(id) ?: return false
        s = s.copy(
            sessions = s.sessions.map { if (it.id == id) it.copy(archivedAt = null).activated(clock()) else it },
            activeSessionId = id,
        )
        markSessions()
        syncWatches()
        return true
    }

    private fun layoutInternal(sessionId: SessionId, op: LayoutOp): Workbench? {
        val session = s.session(sessionId)?.takeIf { !it.isArchived } ?: return null
        val workbench = session.workbench.apply(op)
        if (workbench === session.workbench) return workbench
        replaceSession(session.copy(workbench = workbench).touched(clock()))
        syncWatches()
        return workbench
    }

    private fun touchSessionsReferencing(resource: ResourceRef) {
        val active = s.activeSession ?: return
        if (resource in active.resources) {
            val touched = active.touched(clock())
            if (touched != active) replaceSession(touched)
        }
    }

    private fun autoArchiveInternal() {
        val now = clock()
        val ids = SessionPolicy.autoArchiveCandidates(s.sessions, s.dirtyResources, now).map { it.id }.toSet()
        if (ids.isEmpty()) return
        s = s.copy(
            sessions = s.sessions.map { if (it.id in ids) it.copy(archivedAt = now) else it },
            activeSessionId = s.activeSessionId?.takeUnless { it in ids },
        )
        markSessions()
        syncWatches()
    }

    private fun archiveInternal(id: SessionId, decision: ArchiveDecision?): ArchiveOutcome {
        val session = s.session(id) ?: return ArchiveOutcome.NotFound
        if (session.isArchived) return ArchiveOutcome.Archived(emptyList())
        val dirty = SessionPolicy.dirtyResources(session, s.dirtyResources)
        if (dirty.isNotEmpty() && decision == null) return ArchiveOutcome.NeedsDecision(dirty)
        val files = dirty.filterIsInstance<ResourceRef.File>().map { it.path }.sorted()
        val conversations = dirty.filterIsInstance<ResourceRef.Conversation>().map { it.id }.sorted()
        val saved = mutableListOf<String>()
        when (decision) {
            ArchiveDecision.SAVE_ALL -> {
                // Validate everything before writing anything.
                val conflicts = files.filter { path ->
                    val draft = s.drafts.getValue(path)
                    val current = currentVersion(path)
                    !current.sameContent(draft.base) && !(current.exists && current.sha256 == draft.textSha256)
                }
                if (conflicts.isNotEmpty()) return ArchiveOutcome.SaveConflict(conflicts)
                files.forEach { path -> validate(path, s.drafts.getValue(path).text)?.let { return ArchiveOutcome.Invalid(path, it) } }
                for (path in files) {
                    when (val result = saveInternal(s.drafts.getValue(path))) {
                        is SaveResult.Saved, is SaveResult.Unchanged -> saved += path
                        is SaveResult.Conflict -> return ArchiveOutcome.SaveConflict(listOf(path))
                        is SaveResult.Invalid -> return ArchiveOutcome.Invalid(path, result.message)
                        is SaveResult.Failed -> return ArchiveOutcome.Failed(result.message)
                    }
                }
            }
            ArchiveDecision.DISCARD -> {
                val cleared = conversations.map { s.composer(it).edit("", emptyList()) } // checks revision overflow first
                files.forEach { setDraft(it, null) }
                cleared.forEach(::setComposer)
            }
            ArchiveDecision.KEEP_DRAFTS, null -> Unit
        }
        val now = clock()
        s = s.copy(
            sessions = s.sessions.map { if (it.id == id) it.copy(archivedAt = now) else it },
            activeSessionId = s.activeSessionId?.takeUnless { it == id },
        )
        markSessions()
        syncWatches()
        return ArchiveOutcome.Archived(saved)
    }

    // ---- Drafts ----

    private fun editablePath(raw: String): String {
        val path = WorkspacePaths.normalize(raw)
        require(path.isNotEmpty() && !WorkspacePaths.isReserved(path)) { "Not an editable workspace path: $raw" }
        return path
    }

    private fun editInternal(rawPath: String, text: String, shown: DiskVersion) {
        val path = editablePath(rawPath)
        val existing = s.drafts[path]
        val next = Drafts.edit(existing, path, text, shown, clock())
        if (next === existing) return
        if (path !in s.disk) observe(path, version(path))
        setDraft(path, next)
        touchSessionsReferencing(ResourceRef.File(path))
    }

    private fun setDraft(path: String, draft: FileDraft?) {
        if (s.drafts[path] == draft) return
        s = s.copy(drafts = if (draft == null) s.drafts - path else s.drafts + (path to draft))
        dirtyDrafts += path
        schedule(options.draftDebounceMs)
        syncWatches()
    }

    private fun setComposer(draft: ComposerDraft) {
        s = s.copy(composers = s.composers + (draft.conversationId to draft))
        dirtyComposers += draft.conversationId
        schedule(options.draftDebounceMs)
        touchSessionsReferencing(ResourceRef.Conversation(draft.conversationId))
    }

    private fun validate(path: String, text: String): String? {
        if (path == WorkspacePaths.CONFIG) (ConfigCodec.decode(text) as? ConfigParse.Invalid)?.let { return it.message }
        return options.validators[path]?.invoke(text)
    }

    private fun saveInternal(draft: FileDraft): SaveResult {
        val path = draft.path
        val current = currentVersion(path)
        if (!current.sameContent(draft.base)) {
            if (current.exists && current.sha256 == draft.textSha256) {
                setDraft(path, null)
                return SaveResult.Saved(current, draft.text)
            }
            return SaveResult.Conflict(current)
        }
        validate(path, draft.text)?.let { return SaveResult.Invalid(it) }
        try {
            fs.writeAtomic(path, draft.text)
        } catch (error: IOException) {
            return SaveResult.Failed(error.message ?: "Write failed")
        }
        val written = currentVersion(path)
        setDraft(path, null)
        if (path == WorkspacePaths.CONFIG) loadConfig(initial = false)
        return SaveResult.Saved(written, draft.text)
    }

    // ---- Disk versions and external changes ----

    private fun version(path: String): DiskVersion {
        val stat = fs.stat(path) ?: return DiskVersion.MISSING
        if (stat.isDirectory) return DiskVersion.MISSING
        if (stat.size > options.maxHashedBytes) return DiskVersion(true, stat.size, stat.modifiedAt, null)
        return DiskVersion.of(fs.readBytes(path), stat.modifiedAt)
    }

    /** Reads the version now and records it. */
    private fun currentVersion(path: String): DiskVersion = version(path).also { observe(path, it) }

    private fun observe(path: String, version: DiskVersion) {
        if (s.disk[path] != version) s = s.copy(disk = s.disk + (path to version))
    }

    /** Files whose disk versions the store follows: drafts, file panels of live sessions, config.json. */
    private fun trackedPaths(): Set<String> {
        val paths = linkedSetOf<String>()
        paths += s.drafts.keys
        s.liveSessions.forEach { session -> session.workbench.resources.forEach { if (it is ResourceRef.File) paths += it.path } }
        paths += WorkspacePaths.CONFIG
        return paths
    }

    /**
     * Re-reads [paths]. Unchanged size+mtime skips hashing unless [force]. A changed clean file
     * only updates [WorkspaceState.disk] (editors reload); a draft whose text is now on disk is
     * dropped; other drafts keep their base and show as conflicts.
     */
    private fun refreshPaths(paths: Collection<String>, force: Boolean) {
        for (path in paths) {
            val known = s.disk[path]
            val stat = try { fs.stat(path) } catch (_: Exception) { continue }
            if (!force && known != null && stat != null && !stat.isDirectory && known.exists &&
                known.size == stat.size && known.modifiedAt == stat.modifiedAt) continue
            val current = try { version(path) } catch (_: IOException) { continue }
            if (known == current) continue
            observe(path, current)
            s.drafts[path]?.let { draft -> Drafts.afterDiskChange(draft, current).let { if (it == null) setDraft(path, null) } }
            if (path == WorkspacePaths.CONFIG && known != null && !known.sameContent(current)) loadConfig(initial = false)
        }
    }

    private val watches = linkedMapOf<String, AutoCloseable>()
    private val pendingExternal = linkedSetOf<String>()
    private var externalJob: Job? = null

    private fun syncWatches() {
        val directories = (trackedPaths().map { WorkspacePaths.parent(it) } + "").toSet()
        (watches.keys - directories).forEach { directory -> watches.remove(directory)?.let { runCatching { it.close() } } }
        (directories - watches.keys).forEach { directory ->
            runCatching { watcher.watch(directory) { name -> post { onExternalChange(directory, name) } } }
                .onSuccess { watches[directory] = it }
        }
    }

    private fun onExternalChange(directory: String, name: String?) {
        val tracked = trackedPaths()
        val affected = if (name == null) tracked.filter { WorkspacePaths.parent(it) == directory }
        else listOf(if (directory.isEmpty()) name else "$directory/$name").filter { it in tracked }
        if (affected.isEmpty()) return
        pendingExternal += affected
        externalJob?.cancel()
        externalJob = scope.launch(dispatcher) {
            delay(options.externalDebounceMs)
            post {
                val due = pendingExternal.toList()
                pendingExternal.clear()
                refreshPaths(due, force = false)
            }
        }
    }

    // ---- Persistence ----

    private var sessionsDirty = false
    private val dirtyDrafts = linkedSetOf<String>()
    private val dirtyComposers = linkedSetOf<String>()
    private var pendingConversationIndex: String? = null
    private var debounceJob: Job? = null
    private var deadlineJob: Job? = null
    private var retryIndex = 0

    private fun markSessions() {
        sessionsDirty = true
        schedule(options.sessionsDebounceMs)
    }

    private fun schedule(delayMs: Long) {
        debounceJob?.cancel()
        debounceJob = scope.launch(dispatcher) { delay(delayMs); post { flushPending() } }
        if (deadlineJob == null) deadlineJob = scope.launch(dispatcher) { delay(options.maxWriteDelayMs); post { flushPending() } }
    }

    private fun flushPending(): Boolean {
        debounceJob?.cancel(); debounceJob = null
        deadlineJob?.cancel(); deadlineJob = null
        if (s.status != StoreStatus.READY) return false
        var failure: String? = null
        if (sessionsDirty && !sessionsReadOnly) {
            try { writeSessions(); sessionsDirty = false } catch (error: IOException) { failure = error.message ?: "sessions.json" }
        }
        for (path in dirtyDrafts.toList()) {
            try {
                val draft = s.drafts[path]
                if (draft == null) fs.delete(StatePaths.draft(path)) else fs.writeAtomic(StatePaths.draft(path), StateCodec.encodeDraft(draft))
                dirtyDrafts.remove(path)
            } catch (error: IOException) { failure = failure ?: (error.message ?: "draft") }
        }
        for (id in dirtyComposers.toList()) {
            try {
                s.composers[id]?.let { fs.writeAtomic(StatePaths.composer(id), StateCodec.encodeComposer(it)) }
                dirtyComposers.remove(id)
            } catch (error: IOException) { failure = failure ?: (error.message ?: "composer") }
        }
        pendingConversationIndex?.let { text ->
            try {
                fs.writeAtomic(StatePaths.CONVERSATIONS, text)
                pendingConversationIndex = null
            } catch (error: IOException) { failure = failure ?: (error.message ?: "conversations.json") }
        }
        if (failure == null) {
            retryIndex = 0
            if (s.writeError != null) s = s.copy(writeError = null)
            return true
        }
        s = s.copy(writeError = failure)
        val wait = options.retryDelaysMs[retryIndex.coerceAtMost(options.retryDelaysMs.size - 1)]
        retryIndex++
        debounceJob = scope.launch(dispatcher) { delay(wait); post { flushPending() } }
        return false
    }

    private fun writeSessions() {
        val text = StateCodec.encodeSessions(s.sessions, s.activeSessionId, s.pinned)
        if (text == lastSessionsText && fs.stat(StatePaths.SESSIONS) != null) return
        lastSessionsText?.let { previous -> fs.writeAtomic(StatePaths.SESSIONS_BACKUP, previous) }
        fs.writeAtomic(StatePaths.SESSIONS, text)
        lastSessionsText = text
    }

    private suspend fun fileOp(block: () -> Unit): FileOpResult = call {
        try { block(); FileOpResult.Done }
        catch (error: IOException) { FileOpResult.Failed(error.message ?: "File operation failed") }
        catch (error: IllegalArgumentException) { FileOpResult.Failed(error.message ?: "Invalid path") }
    }

    private companion object {
        fun decodeText(bytes: ByteArray): String? {
            if (bytes.contains(0)) return null
            return try {
                Charsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT).onUnmappableCharacter(CodingErrorAction.REPORT)
                    .decode(ByteBuffer.wrap(bytes)).toString()
            } catch (_: CharacterCodingException) { null }
        }
    }
}
