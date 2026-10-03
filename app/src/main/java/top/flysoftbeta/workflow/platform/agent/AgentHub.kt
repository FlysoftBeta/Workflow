package top.flysoftbeta.workflow.platform.agent

import android.content.Context
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import kotlinx.coroutines.runInterruptible
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import top.flysoftbeta.workflow.agent.AgentBackend
import top.flysoftbeta.workflow.agent.AgentStateStore
import top.flysoftbeta.workflow.agent.SendMode
import top.flysoftbeta.workflow.agent.ThreadOptions
import top.flysoftbeta.workflow.agent.UnknownRequestPolicy
import top.flysoftbeta.workflow.agent.claude.AttachmentReader
import top.flysoftbeta.workflow.agent.claude.ClaudeBackend
import top.flysoftbeta.workflow.agent.claude.ClaudeConfig
import top.flysoftbeta.workflow.agent.claude.ClaudeTranscriptSource
import top.flysoftbeta.workflow.agent.codex.CodexBackend
import top.flysoftbeta.workflow.agent.codex.CodexConfig
import top.flysoftbeta.workflow.agent.model.AgentEvent
import top.flysoftbeta.workflow.agent.model.AgentState
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.ConversationEntry
import top.flysoftbeta.workflow.agent.model.LoginFlow
import top.flysoftbeta.workflow.agent.model.LoginMethod
import top.flysoftbeta.workflow.agent.model.PermissionPreset
import top.flysoftbeta.workflow.agent.model.ProcessState
import top.flysoftbeta.workflow.agent.model.RequestKey
import top.flysoftbeta.workflow.agent.model.RequestResponse
import top.flysoftbeta.workflow.agent.model.ThreadKey
import top.flysoftbeta.workflow.agent.model.TurnSettings
import top.flysoftbeta.workflow.agent.model.UserPart
import top.flysoftbeta.workflow.agent.process.ProcessLauncher
import top.flysoftbeta.workflow.core.config.BackendDefaults
import top.flysoftbeta.workflow.core.resource.ComposerAttachment
import top.flysoftbeta.workflow.core.store.WorkspaceStore

/**
 * How one backend runs inside the environment (docs/agents.md §3).
 * The environment installs guest launchers through [AgentHub.install] only when usable.
 */
data class BackendSetup(
    val launcher: ProcessLauncher,
    /** argv[0] as the launcher sees it. */
    val executable: String,
    /** Workspace root as the agent sees it (`/workspace` in the guest). */
    val workspaceRoot: String,
    /** App-owned agent home as the agent sees it (CODEX_HOME / CLAUDE_CONFIG_DIR). */
    val homeDir: String,
    val tmpDir: String,
    /** Base environment of the child (filtered again by the agent layer). */
    val env: Map<String, String>,
    /** Engine-read bytes of a guest path (attachments/history); null means absent. */
    val readFile: suspend (String, Int) -> ByteArray?,
)

/** The backend cannot run on this device yet (Claude Code before the environment exists). */
class BackendUnavailableException(val kind: BackendKind) : IllegalStateException("${kind.id} is not available")

/**
 * Process-scoped owner of the agent backends (docs/architecture.md §1: lives in AppGraph, never in
 * an Activity or ViewModel). Holds the single [AgentStateStore], the persistent conversation index
 * and the app-conversation-id → backend-thread mapping. The UI reads [state]/[conversations] and
 * commands through the suspend methods; nothing here answers a server request on its own.
 */
class AgentHub(
    private val appContext: Context,
    private val store: WorkspaceStore,
    private val scope: CoroutineScope,
    private val io: CoroutineDispatcher,
) {
    val events = AgentStateStore()
    val state: StateFlow<AgentState> get() = events.state
    val index = ConversationIndexFile(store, scope, io)
    val conversations: StateFlow<List<ConversationEntry>> get() = index.entries

    private val setups = MutableStateFlow<Map<BackendKind, BackendSetup>>(emptyMap())
    /**
     * Backends whose installed launcher cannot run here: the process was killed by the app sandbox
     * (SIGSYS from Android's seccomp filter) before its handshake. Cleared by [install].
     */
    private val unsupported = MutableStateFlow<Set<BackendKind>>(emptySet())
    private val readySeen = ConcurrentHashMap.newKeySet<BackendKind>()
    /** Backends that can run on this device now (an actual start result, not just an installed launcher). */
    val available: StateFlow<Set<BackendKind>> = combine(setups, unsupported) { s, u -> s.keys - u }
        .stateIn(scope, SharingStarted.Eagerly, emptySet())
    private val backends = ConcurrentHashMap<BackendKind, AgentBackend>()
    private val lifecycle = Mutex()
    /** Threads loaded into the current backend process (resume + history done). */
    private val resumed = ConcurrentHashMap.newKeySet<ThreadKey>()
    private val threadLocks = ConcurrentHashMap<String, Mutex>()
    private val dirtyThreads = Channel<ThreadKey>(Channel.UNLIMITED)

    /** Some request waits for the user (Workbench tile / ◨ dot). */
    val attention: StateFlow<Boolean> = state.map { s -> s.requests.values.any { it.status.isOpen } }
        .distinctUntilChanged().stateIn(scope, SharingStarted.Eagerly, false)

    fun start() {
        index.start()
        events.addListener(::observe)
        scope.launch { maintainIndex() }
    }

    // ------------------------------------------------------------------ backends

    /** Installs (or removes) how [kind] runs. A running backend of that kind is stopped first. */
    fun install(kind: BackendKind, setup: BackendSetup?) {
        scope.launch { replaceBackend(kind, setup) }
    }

    /** Awaited by environment restart so backend removal cannot race its reinstallation. */
    suspend fun replaceBackend(kind: BackendKind, setup: BackendSetup?) {
        lifecycle.withLock {
            backends.remove(kind)?.let { runCatching { it.stop() } }
            resumed.removeAll { it.backend == kind }
            readySeen.remove(kind)
            unsupported.value = unsupported.value - kind
            setups.value = if (setup == null) setups.value - kind else setups.value + (kind to setup)
        }
    }

    fun paths(kind: BackendKind): AgentPaths? = setups.value[kind]?.let { AgentPaths(it.workspaceRoot) }

    /** Starts the backend process if needed and returns it (idempotent). */
    suspend fun connect(kind: BackendKind): AgentBackend {
        val backend = lifecycle.withLock {
            backends[kind] ?: create(kind, setups.value[kind] ?: throw BackendUnavailableException(kind)).also { backends[kind] = it }
        }
        backend.start()
        return backend
    }

    /** Starts [kind] in the background when it is not running (conversation shown, login screen). */
    fun warmUp(kind: BackendKind) {
        if (kind !in setups.value) return
        val process = state.value.backend(kind).process
        if (process is ProcessState.Ready || process is ProcessState.Starting) return
        scope.launch { runCatching { connect(kind) } }
    }

    private fun create(kind: BackendKind, setup: BackendSetup): AgentBackend {
        val permissions = defaultPermissions()
        return when (kind) {
            BackendKind.CODEX -> CodexBackend(
                setup.launcher,
                CodexConfig(
                    executable = setup.executable,
                    codexHome = setup.homeDir,
                    cwd = setup.workspaceRoot,
                    env = setup.env,
                    clientName = "workflow",
                    clientTitle = "Workflow",
                    clientVersion = versionName(),
                    defaultPermissions = permissions,
                    unknownRequests = UnknownRequestPolicy.ASK_USER,
                ),
                events, scope,
            )
            BackendKind.CLAUDE -> ClaudeBackend(
                setup.launcher,
                ClaudeConfig(
                    executable = setup.executable,
                    configDir = setup.homeDir,
                    cwd = setup.workspaceRoot,
                    tmpDir = setup.tmpDir,
                    env = setup.env,
                    defaultPermissions = permissions,
                    unknownRequests = UnknownRequestPolicy.ASK_USER,
                ),
                events, scope,
                attachments = AttachmentReader { path, maxBytes ->
                    setup.readFile(path, maxBytes.coerceAtMost(Int.MAX_VALUE.toLong()).toInt()) ?: throw java.io.FileNotFoundException(path)
                },
                transcripts = ClaudeTranscriptSource { path ->
                    setup.readFile(path, 16 * 1024 * 1024)?.toString(Charsets.UTF_8)?.lineSequence()?.toList()
                },
            )
        }
    }

    private fun versionName(): String = runCatching {
        appContext.packageManager.getPackageInfo(appContext.packageName, 0).versionName
    }.getOrNull() ?: "0"

    // ------------------------------------------------------------------ conversations

    fun entry(id: String): ConversationEntry? = conversations.value.firstOrNull { it.id == id }

    fun threadKey(entry: ConversationEntry): ThreadKey? = entry.backendThreadId?.let { ThreadKey(entry.backend, it) }

    fun defaultBackend(): BackendKind = BackendKind.of(store.state.value.config.agent.backend) ?: BackendKind.CODEX

    /** Creates a new, still thread-less conversation on [backend] (default: the last chosen one). */
    suspend fun newConversation(backend: BackendKind? = null, id: String = UUID.randomUUID().toString()): String {
        val kind = backend ?: defaultBackend()
        val remembered = store.state.value.config.agent.backends[kind.id]
        val now = System.currentTimeMillis()
        index.upsert(ConversationEntry(
            id = id, backend = kind, backendThreadId = null, title = null, cwd = paths(kind)?.root ?: "",
            createdAtMs = now, updatedAtMs = now, model = remembered?.model, effort = remembered?.effort,
        ))
        return id
    }

    /** The entry of [id]; an id the index does not know (older sessions) becomes a new conversation. */
    suspend fun ensureConversation(id: String): ConversationEntry {
        index.awaitLoaded()
        entry(id)?.let { return it }
        newConversation(id = id)
        return entry(id)!!
    }

    /** Switches a conversation that has no thread yet to another backend and remembers the choice. */
    suspend fun setBackend(id: String, kind: BackendKind) {
        val remembered = store.state.value.config.agent.backends[kind.id]
        index.update(id) { e ->
            if (e.backendThreadId != null) e else e.copy(backend = kind, model = remembered?.model, effort = remembered?.effort, cwd = paths(kind)?.root ?: e.cwd)
        }
        store.updateConfig { it.copy(agent = it.agent.copy(backend = kind.id)) }
    }

    /**
     * Makes [id]'s thread live in its backend: starts the process and resumes the thread with its
     * history once per process. No-op for a conversation without a thread yet.
     */
    suspend fun open(id: String) {
        val entry = ensureConversation(id)
        if (entry.backend !in setups.value) return
        val backend = connect(entry.backend)
        val key = threadKey(entry) ?: return
        lock(id).withLock {
            if (key in resumed) return
            if (!loggedIn(entry.backend)) return
            backend.resumeThread(key.id, options(entry))
            resumed += key
        }
    }

    private fun loggedIn(kind: BackendKind): Boolean =
        state.value.backend(kind).account.state != top.flysoftbeta.workflow.agent.model.LoginState.LOGGED_OUT

    suspend fun loadEarlier(id: String) {
        val entry = entry(id) ?: return
        val key = threadKey(entry) ?: return
        val cursor = state.value.thread(key)?.historyCursor ?: return
        connect(entry.backend).loadHistory(key.id, cursor)
    }

    /** Sends a user message; creates the backend thread on the first message. Returns the client message id. */
    suspend fun send(id: String, text: String, attachments: List<ComposerAttachment>, settings: TurnSettings, mode: SendMode): String {
        val entry = ensureConversation(id)
        val backend = connect(entry.backend)
        val paths = paths(entry.backend) ?: throw BackendUnavailableException(entry.backend)
        val parts = buildList {
            if (text.isNotBlank()) add(UserPart.Text(text))
            attachments.forEach { a ->
                val path = paths.toAgent(a.path)
                val mime = a.mimeType ?: guessMime(a.path)
                add(if (mime?.startsWith("image/") == true) UserPart.Image(path, mime) else UserPart.File(path, mime))
            }
        }
        require(parts.isNotEmpty()) { "Nothing to send" }
        val threadId = lock(id).withLock {
            val current = entry(id) ?: entry
            val existing = threadKey(current)
            if (existing == null) {
                val created = backend.startThread(options(current, settings))
                val key = ThreadKey(current.backend, created)
                resumed += key
                index.update(id) { it.copy(backendThreadId = created, updatedAtMs = System.currentTimeMillis(), preview = it.preview ?: text.take(200)) }
                created
            } else {
                if (existing !in resumed) {
                    backend.resumeThread(existing.id, options(current))
                    resumed += existing
                }
                existing.id
            }
        }
        return backend.send(threadId, parts, settings, mode)
    }

    suspend fun interrupt(id: String) {
        val entry = entry(id) ?: return
        val key = threadKey(entry) ?: return
        connect(entry.backend).interrupt(key.id)
    }

    suspend fun cancelQueued(id: String, clientMessageId: String) {
        val entry = entry(id) ?: return
        val key = threadKey(entry) ?: return
        connect(entry.backend).cancelQueued(key.id, clientMessageId)
    }

    /** Answers a server request with the user's explicit choice. */
    suspend fun respond(key: RequestKey, response: RequestResponse) {
        val backend = backends[key.backend] ?: throw IllegalStateException("backend not running")
        backend.respond(key, response)
    }

    suspend fun rename(id: String, title: String) {
        val clean = title.trim().take(200)
        val entry = index.update(id) { it.copy(title = clean.ifEmpty { null }) } ?: return
        val key = threadKey(entry) ?: return
        if (clean.isNotEmpty() && backends[entry.backend] != null) runCatching { backends[entry.backend]!!.rename(key.id, clean) }
    }

    suspend fun archive(id: String, archived: Boolean) {
        val entry = index.update(id) { it.copy(archived = archived, updatedAtMs = System.currentTimeMillis()) } ?: return
        val key = threadKey(entry) ?: return
        backends[entry.backend]?.let { backend -> runCatching { backend.archive(key.id, archived) } }
    }

    /** Called only after the UI explicitly confirms deletion, including the composer draft. */
    suspend fun deleteConversation(id: String) {
        val entry = entry(id) ?: return
        val key = threadKey(entry)
        if (key != null) {
            check(state.value.thread(key)?.activeTurn == null) { "请先停止正在进行的回答" }
            connect(entry.backend).delete(key.id)
            resumed.remove(key)
        }
        store.removeConversation(id)
        index.remove(id)
    }

    suspend fun compact(id: String) {
        val entry = entry(id) ?: return
        val key = threadKey(entry) ?: return
        check(state.value.thread(key)?.activeTurn == null) { "请先停止正在进行的回答" }
        open(id)
        connect(entry.backend).compact(key.id)
    }

    suspend fun refreshUsage(kind: BackendKind) = connect(kind).refreshRateLimits()

    /** Explicit advanced-console requests keep the backend's approval policy checks. */
    suspend fun rawRequest(id: String, method: String, params: kotlinx.serialization.json.JsonElement?): kotlinx.serialization.json.JsonElement {
        val entry = ensureConversation(id)
        open(id)
        return connect(entry.backend).rawRequest(method.trim().also { require(it.isNotEmpty()) }, params, entry.backendThreadId)
    }

    /** Branches [id] after [atTurnId] (null = whole conversation) into a new conversation; returns its id. */
    suspend fun fork(id: String, atTurnId: String?): String {
        val entry = entry(id) ?: throw IllegalArgumentException("unknown conversation")
        val key = threadKey(entry) ?: throw IllegalStateException("nothing to fork yet")
        val backend = connect(entry.backend)
        val created = backend.forkThread(key.id, atTurnId, options(entry))
        val newKey = ThreadKey(entry.backend, created)
        resumed += newKey
        val now = System.currentTimeMillis()
        val newId = UUID.randomUUID().toString()
        index.upsert(entry.copy(
            id = newId, backendThreadId = created, title = entry.title?.let { "$it（分叉）" }, createdAtMs = now, updatedAtMs = now,
            archived = false, forkedFrom = entry.id, forkedAt = atTurnId,
        ))
        return newId
    }

    suspend fun setPermissions(id: String, preset: PermissionPreset) {
        store.updateConfig { it.copy(agent = it.agent.copy(permissions = preset.name.lowercase())) }
        val entry = entry(id) ?: return
        val key = threadKey(entry) ?: return
        backends[entry.backend]?.setPermissions(key.id, preset)
    }

    fun permissions(): PermissionPreset = defaultPermissions()

    private fun defaultPermissions(): PermissionPreset =
        store.state.value.config.agent.permissions?.let { value -> PermissionPreset.entries.firstOrNull { it.name.equals(value, ignoreCase = true) } }
            ?: PermissionPreset.ASK

    /** Remembers the slider position for this conversation and as the backend default. */
    suspend fun rememberSelection(id: String, model: String?, effort: String?) {
        val entry = index.update(id) { it.copy(model = model, effort = effort) } ?: return
        store.updateConfig { config ->
            config.copy(agent = config.agent.copy(backends = config.agent.backends + (entry.backend.id to BackendDefaults(model, effort))))
        }
    }

    // ------------------------------------------------------------------ account

    suspend fun login(kind: BackendKind, method: LoginMethod, secret: String? = null): LoginFlow = connect(kind).login(method, secret)

    suspend fun cancelLogin(kind: BackendKind, loginId: String) = connect(kind).cancelLogin(loginId)

    suspend fun logout(kind: BackendKind) = connect(kind).logout()

    suspend fun refreshAccount(kind: BackendKind) = connect(kind).refreshAccount()

    fun loginMethods(kind: BackendKind): List<LoginMethod> = when (kind) {
        BackendKind.CODEX -> listOf(LoginMethod.CODEX_DEVICE_CODE, LoginMethod.CODEX_BROWSER, LoginMethod.CODEX_API_KEY)
        BackendKind.CLAUDE -> listOf(LoginMethod.CLAUDE_TERMINAL_LOGIN, LoginMethod.CLAUDE_SETUP_TOKEN, LoginMethod.CLAUDE_API_KEY)
    }

    suspend fun flush() = index.flush()

    // ------------------------------------------------------------------ internals

    private fun options(entry: ConversationEntry, settings: TurnSettings = TurnSettings()) = ThreadOptions(
        cwd = paths(entry.backend)?.root ?: entry.cwd,
        settings = settings.copy(permissions = settings.permissions ?: defaultPermissions()),
    )

    private fun lock(id: String) = threadLocks.getOrPut(id) { Mutex() }

    /** Called synchronously for every reduced event: cheap bookkeeping only. */
    private fun observe(event: AgentEvent) {
        when (event) {
            is AgentEvent.ProcessChanged -> if (event.state is ProcessState.Ready) {
                readySeen += event.backend
            } else if (event.state is ProcessState.Exited || event.state is ProcessState.Failed) {
                if ((event.state as? ProcessState.Exited)?.exitCode == SIGSYS_EXIT && event.backend !in readySeen) {
                    // Killed by the app's seccomp filter (e.g. x86_64 musl `readlink`): this launcher cannot
                    // run here; the environment engine's launcher is required (docs/report/initial/w7-chat.md).
                    unsupported.value = unsupported.value + event.backend
                }
                resumed.removeAll { it.backend == event.backend }
                val tail = (event.state as? ProcessState.Exited)?.stderrTail ?: (event.state as ProcessState.Failed).stderrTail
                val code = (event.state as? ProcessState.Exited)?.exitCode
                // Exit status always; the stderr tail only in debuggable builds (it may name hosts or paths).
                android.util.Log.w(TAG, "${event.backend.id} stopped (exit=$code)" + if (debuggable) ": ${tail.takeLast(600)}" else "")
            }
            is AgentEvent.ThreadUpserted -> dirtyThreads.trySend(ThreadKey(event.backend, event.threadId))
            is AgentEvent.ThreadRenamed -> dirtyThreads.trySend(ThreadKey(event.backend, event.threadId))
            is AgentEvent.TurnSubmitted -> dirtyThreads.trySend(ThreadKey(event.backend, event.threadId))
            is AgentEvent.TurnCompleted -> dirtyThreads.trySend(ThreadKey(event.backend, event.threadId))
            is AgentEvent.HistoryLoaded -> dirtyThreads.trySend(ThreadKey(event.backend, event.threadId))
            else -> Unit
        }
    }

    /** Folds thread titles, previews and activity times into the index, coalesced. */
    private suspend fun maintainIndex() {
        index.awaitLoaded()
        val batch = HashSet<ThreadKey>()
        for (first in dirtyThreads) {
            batch += first
            delay(250)
            while (true) batch += dirtyThreads.tryReceive().getOrNull() ?: break
            val snapshot = state.value
            val now = System.currentTimeMillis()
            for (key in batch) {
                index.refreshThread(key, snapshot.thread(key), now)
            }
            batch.clear()
        }
    }

    private val debuggable = (appContext.applicationInfo.flags and android.content.pm.ApplicationInfo.FLAG_DEBUGGABLE) != 0

    companion object {
        private const val TAG = "WorkflowAgent"
        /** 128 + SIGSYS: what Process.waitFor reports for a seccomp kill. */
        const val SIGSYS_EXIT = 128 + 31
        private val MIME = mapOf(
            "png" to "image/png", "jpg" to "image/jpeg", "jpeg" to "image/jpeg", "gif" to "image/gif", "webp" to "image/webp",
            "bmp" to "image/bmp", "pdf" to "application/pdf", "md" to "text/markdown", "txt" to "text/plain",
        )

        fun guessMime(path: String): String? = MIME[path.substringAfterLast('.', "").lowercase()]
    }
}
