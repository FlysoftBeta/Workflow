package top.flysoftbeta.workflow.core.connection

import java.io.IOException
import java.io.InputStream
import java.util.concurrent.ConcurrentHashMap
import java.util.Base64
import kotlinx.coroutines.*
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.*
import top.flysoftbeta.workflow.core.config.*
import top.flysoftbeta.workflow.core.io.FileEntry
import top.flysoftbeta.workflow.core.json.Json
import top.flysoftbeta.workflow.core.layout.*
import top.flysoftbeta.workflow.core.resource.*
import top.flysoftbeta.workflow.core.session.*
import top.flysoftbeta.workflow.core.store.*

/** Transport-only implementation. Every authoritative read/mutation goes to the Rust Engine. */
class RemoteWorkspaceStore(val rpc: WorkspaceRpc, private val scope: CoroutineScope, private val clientId: String) : WorkspaceStore {
    private val mutable = MutableStateFlow(WorkspaceState())
    override val state = mutable.asStateFlow()
    private val revisions = MutableStateFlow(-1L)
    private var configuration: Map<String, Any?> = emptyMap()
    private var started = false
    @Volatile private var closed = false
    private val pending = ConcurrentHashMap.newKeySet<Deferred<*>>()
    private var watcher: Job? = null
    private val commands = Channel<suspend () -> Unit>(Channel.UNLIMITED)
    private val worker = scope.launch { for (command in commands) command() }

    @Synchronized override fun start() {
        check(!closed) { "工作区连接已关闭" }
        if (started) return
        started = true
        watcher = scope.launch {
            try {
                rpc.hello(clientId)
                accept(rpc.request("workspace.snapshot"))
                while (isActive) {
                    accept(rpc.request("workspace.watch", mapOf("afterRevision" to revisions.value, "timeoutMs" to 25_000), 30_000))
                }
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { disconnect(error.message ?: "工作区连接已断开") }
        }
    }

    @Synchronized private fun accept(value: Any?): Map<String, Any?> {
        val response = WorkspaceWire.obj(value)
        val revision = WorkspaceWire.long(response, "revision")
        check(revision >= 0) { "Workspace returned negative revision" }
        if (closed) throw IOException(state.value.failure ?: "工作区连接已关闭")
        if (revision >= revisions.value) {
            val snapshot = WorkspaceWire.state(response["state"])
            configuration = WorkspaceWire.obj(WorkspaceWire.obj(response["state"])["config"])
            mutable.value = snapshot
            revisions.value = revision
        }
        return response
    }

    override suspend fun awaitReady(): WorkspaceState = state.first { it.status != StoreStatus.LOADING }
    private suspend fun ensureReady() {
        check(!closed && rpc.failure.value == null && awaitReady().status == StoreStatus.READY) { state.value.failure ?: rpc.failure.value ?: "请先连接工作区" }
    }

    private suspend fun <T> serial(block: suspend () -> T): T {
        check(!closed) { state.value.failure ?: "工作区连接已关闭" }
        val turn = CompletableDeferred<Unit>()
        val task = scope.async {
            turn.await()
            try { block() }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { mutable.value = mutable.value.copy(writeError = error.message); throw error }
        }
        pending += task
        try {
            // A failed or cancelled request must never terminate the serial command lane.
            commands.send { turn.complete(Unit); task.join() }
            return task.await()
        } finally { pending -= task; task.cancel() }
    }
    private fun post(block: suspend () -> Unit) {
        if (closed) return
        commands.trySend {
            try { block() }
            catch (cancelled: CancellationException) { currentCoroutineContext().ensureActive(); mutable.value = mutable.value.copy(writeError = cancelled.message) }
            catch (error: Exception) { mutable.value = mutable.value.copy(writeError = error.message) }
        }
    }
    private suspend fun command(name: String, vararg args: Pair<String, Any?>): Any? {
        ensureReady()
        return accept(rpc.request("workspace.command", mapOf("name" to name, "args" to mapOf(*args))))["value"]
    }
    private suspend fun call(name: String, vararg args: Pair<String, Any?>): Any? = serial { command(name, *args) }
    override suspend fun flush() = call("flush") as? Boolean ?: false
    override suspend fun close() {
        try { if (!closed) withTimeoutOrNull(5_000) { runCatching { flush() } } }
        finally { disconnect("工作区连接已关闭") }
    }

    /** Immediately retires this projection; reconnect creates a new store from a new snapshot. */
    @Synchronized fun disconnect(message: String = "工作区连接已断开") {
        if (closed) return
        closed = true
        mutable.value = mutable.value.copy(status = StoreStatus.FAILED, failure = message)
        commands.close()
        pending.forEach { it.cancel(CancellationException(message)) }
        watcher?.cancel(); worker.cancel(); rpc.close()
    }
    override suspend fun enterWorkbench() = call("enterWorkbench") as String
    override suspend fun createSession(name: String?) = call("createSession", "name" to name) as String
    override suspend fun openInSeparateSession(target: PanelTarget) = call("openInSeparateSession", "target" to StateCodec.target(target)) as String
    override suspend fun activateSession(id: String) = call("activateSession", "id" to id) == true
    override suspend fun restoreSession(id: String) = call("restoreSession", "id" to id) == true
    override suspend fun renameSession(id: String, name: String) = call("renameSession", "id" to id, "name" to name) == true
    override suspend fun pinSession(id: String, index: Int) = call("pinSession", "id" to id, "index" to index) == true
    override suspend fun unpinSession(id: String) = call("unpinSession", "id" to id) == true
    override suspend fun runMaintenance() { call("runMaintenance") }
    override suspend fun dismissNotice(id: String) { call("dismissNotice", "id" to id) }
    override fun layout(sessionId: String, op: LayoutOp) = post { command("applyLayout", "sessionId" to sessionId, "op" to WorkspaceWire.layout(op)) }
    override fun layout(op: LayoutOp) = post { command("applyLayout", "op" to WorkspaceWire.layout(op)) }
    override suspend fun applyLayout(sessionId: String, op: LayoutOp): Workbench? =
        call("applyLayout", "sessionId" to sessionId, "op" to WorkspaceWire.layout(op))?.let { WorkspaceWire.workbench(it) }
    override suspend fun openFile(path: String) = WorkspaceWire.file(call("openFile", "path" to path))
    override fun editFile(path: String, text: String, shown: DiskVersion) = post { command("editFile", "path" to path, "text" to text, "shown" to StateCodec.version(shown)) }
    override suspend fun resolveConflict(path: String, resolution: ConflictResolution) { call("resolveConflict", "path" to path, "resolution" to resolution.name.lowercase()) }
    override suspend fun discardDraft(path: String) { call("discardDraft", "path" to path) }
    override suspend fun saveFile(path: String, text: String?): SaveResult = try {
        val result = WorkspaceWire.obj(call("saveFile", "path" to path, "text" to text))
        when (result["kind"]) {
            "saved" -> SaveResult.Saved(WorkspaceWire.version(result["version"]), WorkspaceWire.string(result, "text"))
            "unchanged" -> SaveResult.Unchanged(WorkspaceWire.version(result["version"]))
            "conflict" -> SaveResult.Conflict(WorkspaceWire.version(result["disk"]))
            "invalid" -> SaveResult.Invalid(message(result))
            "failed" -> SaveResult.Failed(message(result))
            else -> throw IOException("Unknown save result")
        }
    } catch (e: Exception) { if (e is CancellationException) throw e; SaveResult.Failed(e.message ?: "保存失败") }

    override suspend fun archiveSession(id: String, decision: ArchiveDecision?): ArchiveOutcome {
        val result = WorkspaceWire.obj(call("archiveSession", "id" to id, "decision" to decision?.name?.lowercase()))
        fun strings(key: String) = (result[key] as? List<*>).orEmpty().map { it as String }
        return when (result["kind"]) {
            "archived" -> ArchiveOutcome.Archived(strings("savedPaths"))
            "needsDecision" -> ArchiveOutcome.NeedsDecision((result["resources"] as? List<*>).orEmpty().map(WorkspaceWire::resource).toSet())
            "saveConflict" -> ArchiveOutcome.SaveConflict(strings("paths"))
            "invalid" -> ArchiveOutcome.Invalid(WorkspaceWire.string(result, "path"), message(result))
            "failed" -> ArchiveOutcome.Failed(message(result))
            "notFound" -> ArchiveOutcome.NotFound
            else -> throw IOException("Unknown archive result")
        }
    }

    private suspend fun fileOperation(name: String, vararg args: Pair<String, Any?>): FileOpResult = try { fileResult(call(name, *args)) }
        catch (e: Exception) { if (e is CancellationException) throw e; FileOpResult.Failed(e.message ?: "文件操作失败") }
    private fun fileResult(value: Any?): FileOpResult = WorkspaceWire.obj(value).let {
        when (it["kind"]) { "done" -> FileOpResult.Done; "failed" -> FileOpResult.Failed(message(it)); else -> throw IOException("Unknown file result") }
    }
    override suspend fun movePath(from: String, to: String) = fileOperation("movePath", "from" to from, "to" to to)
    override suspend fun copyPath(from: String, to: String) = fileOperation("copyPath", "from" to from, "to" to to)
    override suspend fun deletePath(path: String) = fileOperation("deletePath", "path" to path)
    override suspend fun createDirectory(path: String) = fileOperation("createDirectory", "path" to path)
    override suspend fun createFile(path: String, bytes: ByteArray) = importFile(path, bytes.size.toLong()) { bytes.inputStream() }
    override suspend fun trashPath(path: String): TrashResult = WorkspaceWire.obj(call("trashPath", "path" to path)).let { result ->
        if (result["kind"] == "failed") TrashResult.Failed(message(result)) else {
            check(result["kind"] == "trashed")
            val entry = WorkspaceWire.obj(result["entry"])
            TrashResult.Trashed(TrashEntry(WorkspaceWire.string(entry, "id"), WorkspaceWire.string(entry, "path"), WorkspaceWire.long(entry, "trashedAt"), entry["isDirectory"] == true))
        }
    }
    override suspend fun restoreFromTrash(id: String): RestoreResult = WorkspaceWire.obj(call("restoreFromTrash", "id" to id)).let {
        when (it["kind"]) { "restored" -> RestoreResult.Restored(WorkspaceWire.string(it, "path")); "failed" -> RestoreResult.Failed(message(it)); else -> throw IOException("Unknown restore result") }
    }
    override suspend fun purgeTrash() = (call("purgeTrash") as Number).toInt()
    override suspend fun listDirectory(path: String, showHidden: Boolean): List<FileEntry> =
        (call("listDirectory", "path" to path, "showHidden" to showHidden) as List<*>).map { item ->
            val entry = WorkspaceWire.obj(item)
            FileEntry(WorkspaceWire.string(entry, "path"), entry["isDirectory"] == true, WorkspaceWire.long(entry, "size"), WorkspaceWire.long(entry, "modifiedAt"))
        }
    override fun directoryChanges(path: String): Flow<Unit> = flow {
        // Engine watches disk changes; a bounded poll also covers filesystem-only changes without UI mutations.
        while (currentCoroutineContext().isActive) { delay(1_000); if (state.value.status == StoreStatus.READY) emit(Unit) }
    }

    override suspend fun editComposer(draft: ComposerDraft, expectedRevision: Long) = WorkspaceWire.composer(call("editComposer", "draft" to WorkspaceWire.encode(draft), "expectedRevision" to expectedRevision))
    override suspend fun acknowledgeComposer(submitted: ComposerDraft) = WorkspaceWire.composer(call("acknowledgeComposer", "submitted" to WorkspaceWire.encode(submitted)))
    override suspend fun discardComposer(conversationId: String) = WorkspaceWire.composer(call("discardComposer", "conversationId" to conversationId))
    override suspend fun removeConversation(conversationId: String) { call("removeConversation", "conversationId" to conversationId) }

    override suspend fun updateConfig(transform: (AppConfig) -> AppConfig): ConfigUpdate = serial {
        repeat(5) {
            ensureReady()
            state.value.configProblem?.let { problem -> return@serial ConfigUpdate.Blocked(problem) }
            val (config, original, revision) = synchronized(this@RemoteWorkspaceStore) { Triple(state.value.config, configuration, revisions.value) }
            val document = Json.parse(ConfigCodec.encode(transform(config), original))
            val result = try { WorkspaceWire.obj(command("updateConfig", "config" to document, "expectedRevision" to revision)) }
            catch (conflict: WorkspaceRpcException) {
                if ((conflict.data as? Map<*, *>)?.get("kind") != "conflict") throw conflict
                accept(rpc.request("workspace.snapshot")); return@repeat
            }
            if (result["kind"] == "conflict") return@repeat // command already accepted the competing authoritative snapshot.
            return@serial when (result["kind"]) {
                "updated" -> ConfigUpdate.Updated((ConfigCodec.decode(Json.stringify(result["config"])) as ConfigParse.Ok).config)
                "blocked" -> ConfigUpdate.Blocked(result["problem"] as? String ?: "配置文件无效")
                "failed" -> ConfigUpdate.Failed(message(result))
                else -> throw IOException("Unknown config result")
            }
        }
        ConfigUpdate.Failed("配置正在被其他客户端修改，请重试")
    }

    override suspend fun readConversationIndex(): String? = serial {
        ensureReady()
        WorkspaceWire.obj(rpc.request("documents.read", documentKey))["document"] as? String
    }
    override suspend fun writeConversationIndex(text: String) { serial { ensureReady(); rpc.request("documents.write", documentKey + ("document" to text)) }; Unit }
    override suspend fun quarantineConversationIndex() { serial { ensureReady(); rpc.request("documents.quarantine", documentKey) }; Unit }

    override suspend fun importFile(path: String, size: Long, open: () -> InputStream): FileOpResult = withContext(Dispatchers.IO) {
        var uploadId: String? = null
        try {
            require(size >= 0) { "Import size must be nonnegative" }
            ensureReady()
            val id = WorkspaceWire.string(WorkspaceWire.obj(rpc.request("files.upload.begin", mapOf("path" to path, "size" to size))), "uploadId")
            uploadId = id
            open().use { input ->
                val buffer = ByteArray(65_536)
                var offset = 0L
                while (offset < size) {
                    currentCoroutineContext().ensureActive()
                    val length = runInterruptible { input.read(buffer, 0, minOf(buffer.size.toLong(), size - offset).toInt()) }
                    check(length > 0) { "Import source ended before its declared size" }
                    val data = Base64.getEncoder().encodeToString(buffer.copyOf(length))
                    val result = rpc.request("files.upload.chunk", mapOf("uploadId" to id, "offset" to offset, "data" to data))
                    val next = WorkspaceWire.long(WorkspaceWire.obj(result), "nextOffset")
                    check(next == offset + length) { "Upload offset mismatch" }
                    offset = next
                }
                check(runInterruptible { input.read() } == -1) { "Import source exceeds its declared size" }
            }
            currentCoroutineContext().ensureActive()
            val result = fileResult(rpc.request("files.upload.commit", mapOf("uploadId" to id)))
            if (result == FileOpResult.Done) uploadId = null
            accept(rpc.request("workspace.snapshot"))
            result
        } catch (e: Exception) {
            if (e is CancellationException) throw e
            FileOpResult.Failed(e.message ?: "导入失败")
        } finally {
            uploadId?.let { id ->
                withContext(NonCancellable) { runCatching { rpc.request("files.upload.cancel", mapOf("uploadId" to id), 5_000) } }
            }
        }
    }

    suspend fun readBytes(path: String, maxBytes: Int): ByteArray {
        require(maxBytes >= 0) { "Read limit must be nonnegative" }
        ensureReady()
        val output = java.io.ByteArrayOutputStream()
        var offset = 0L
        do {
            val result = WorkspaceWire.obj(rpc.request("files.read", mapOf("path" to path, "offset" to offset, "length" to minOf(65_536L, maxBytes.toLong() - output.size() + 1))))
            val bytes = Base64.getDecoder().decode(WorkspaceWire.string(result, "data"))
            require(output.size().toLong() + bytes.size <= maxBytes.toLong()) { "文件超过大小限制" }
            output.write(bytes)
            val next = WorkspaceWire.long(result, "nextOffset")
            check(next == offset + bytes.size) { "File read offset mismatch" }
            if (result["eof"] == true) break
            check(next > offset) { "File read made no progress" }; offset = next
        } while (true)
        return output.toByteArray()
    }

    private fun message(value: Map<String, Any?>) = value["message"] as? String ?: "工作区操作失败"
    private val documentKey = mapOf("namespace" to "chat", "key" to "conversations")
}
