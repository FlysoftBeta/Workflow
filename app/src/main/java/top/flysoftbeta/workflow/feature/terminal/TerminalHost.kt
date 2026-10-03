package top.flysoftbeta.workflow.feature.terminal

import android.content.Context
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicInteger
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import top.flysoftbeta.workflow.core.store.StoreStatus
import top.flysoftbeta.workflow.core.store.WorkspaceStore
import top.flysoftbeta.workflow.core.terminal.OscScanner
import top.flysoftbeta.workflow.core.terminal.StreamingUtf8Decoder
import top.flysoftbeta.workflow.core.terminal.TerminalBackend
import top.flysoftbeta.workflow.core.terminal.TerminalProcess
import top.flysoftbeta.workflow.core.terminal.TerminalScrollback
import top.flysoftbeta.workflow.core.terminal.TerminalSpec
import top.flysoftbeta.workflow.app.AppGraph
import top.flysoftbeta.workflow.platform.engine.EngineController
import top.flysoftbeta.workflow.platform.engine.EngineTerminalBackend
import top.flysoftbeta.workflow.platform.engine.EnvironmentRestartListener
import top.flysoftbeta.workflow.platform.engine.EnvironmentUnavailableException

/** Lifecycle of one terminal's process. */
sealed interface TerminalStatus {
    data object Starting : TerminalStatus
    data object Running : TerminalStatus
    data class Ended(val exitCode: Int) : TerminalStatus
    /** [environment]: the Debian environment could not start (the terminal reports the failure). */
    data class Failed(val message: String, val environment: Boolean = false) : TerminalStatus
}

/**
 * The process-wide owner of terminal processes (architecture.md §4): sessions live here, not in panels,
 * so a recreated UI reattaches to the running shell with its scrollback. Each process holds a
 * foreground-service lease (in the backend). Terminals that no live Workbench session references any
 * more (closed tab, archived session) are ended after a grace period.
 */
class TerminalHost internal constructor(
    val backend: TerminalBackend,
    /** The environment behind [backend] (null in tests with a plain backend). */
    val environment: EngineController? = null,
) : EnvironmentRestartListener {
    private val scope = CoroutineScope(SupervisorJob(environment?.scope?.coroutineContext?.get(Job)) + Dispatchers.Default)
    private val sessions = ConcurrentHashMap<String, TerminalSession>()
    private val ordinals = AtomicInteger()
    private val reaping = ConcurrentHashMap<String, Job>()
    @Volatile private var observed: WorkspaceStore? = null
    private var observer: Job? = null
    /** Terminals running in the environment when "重启环境" started, with their directory (workspace-relative). */
    private val restarting = ConcurrentHashMap<String, String>()

    init {
        environment?.addRestartListener(this)
    }

    /** Ends terminals that no live session of [store] references (called by the panel provider). */
    @Synchronized
    fun observe(store: WorkspaceStore) {
        if (observed === store) return
        observed = store
        observer?.cancel()
        observer = scope.launch {
            store.state.filter { it.status == StoreStatus.READY }.map { it.referencedTerminals }.distinctUntilChanged().collect { reap(store, it) }
        }
    }

    fun session(id: String): TerminalSession? = sessions[id]

    /** Starts a new terminal in [directory] (workspace-relative) and returns its id. */
    fun create(directory: String?): String {
        val id = "t-" + UUID.randomUUID().toString().take(8)
        val session = newSession(id)
        sessions[id] = session
        session.start(directory.orEmpty())
        return id
    }

    private fun newSession(id: String) = TerminalSession(id, ordinals.incrementAndGet(), backend, scope)

    /** Starts [id] again (after it ended, or when it is unknown after the process was recreated). */
    fun restart(id: String, directory: String = "", force: Boolean = false): TerminalSession {
        val session = sessions.getOrPut(id) { newSession(id) }
        val status = session.status.value
        if (force || !session.started || status is TerminalStatus.Ended || status is TerminalStatus.Failed) {
            session.start(directory)
        }
        return session
    }

    override suspend fun beforeRestart() {
        restarting.clear()
        for ((id, session) in sessions) {
            if (session.status.value !is TerminalStatus.Running) continue
            val cwd = session.currentDirectory()?.let { session.paths.toWorkspace(it) }.orEmpty()
            restarting[id] = if (cwd.isEmpty()) "" else runCatching {
                val parent = cwd.substringBeforeLast('/', "")
                cwd.takeIf { path -> observed?.listDirectory(parent, true)?.any { it.path == path && it.isDirectory } == true }.orEmpty()
            }.getOrDefault("")
        }
    }

    /** Terminals that "重启环境" stopped start again in the new environment, in the directory they were in. */
    override suspend fun afterRestart() {
        val due = restarting.toMap()
        restarting.clear()
        due.forEach { (id, directory) -> sessions[id]?.start(directory) }
    }

    fun end(id: String) {
        sessions.remove(id)?.terminate()
    }

    private fun close() {
        sessions.keys.toList().forEach(::end)
        scope.cancel()
    }

    private fun reap(store: WorkspaceStore, referenced: Set<String>) {
        referenced.forEach { reaping.remove(it)?.cancel() }
        sessions.keys.filter { it !in referenced && !reaping.containsKey(it) }.forEach { id ->
            reaping[id] = scope.launch {
                delay(REAP_GRACE_MS)
                val still = store.state.value.referencedTerminals
                if (id !in still) end(id)
                reaping.remove(id)
            }
        }
    }

    companion object {
        /** A terminal must stay unreferenced this long (a tab moved between sessions, a quick undo) before it ends. */
        private const val REAP_GRACE_MS = 15_000L

        @Volatile private var instance: TerminalHost? = null

        fun get(context: Context): TerminalHost = synchronized(this) {
            val engine = AppGraph.engine(context)
            instance?.takeIf { it.environment === engine } ?: run {
                instance?.close()
                TerminalHost(EngineTerminalBackend(engine), engine).also { instance = it }
            }
        }
    }
}

/** One terminal: its process, retained output, title and directory. Thread-safe. */
class TerminalSession internal constructor(
    val id: String,
    val ordinal: Int,
    private val backend: TerminalBackend,
    private val parent: CoroutineScope,
) {
    /** How this terminal's shell names workspace paths. */
    val paths get() = backend.paths
    val scrollback = TerminalScrollback()
    private val mutableEnd = MutableStateFlow(0L)
    /** End offset of the retained output; changes whenever output arrives (conflated). */
    val outputEnd: StateFlow<Long> = mutableEnd.asStateFlow()
    private val mutableStatus = MutableStateFlow<TerminalStatus>(TerminalStatus.Starting)
    val status: StateFlow<TerminalStatus> = mutableStatus.asStateFlow()
    private val mutableTitle = MutableStateFlow<String?>(null)
    /** Title set by the program (OSC 0/2). */
    val programTitle: StateFlow<String?> = mutableTitle.asStateFlow()
    /** Title chosen with "重命名"; wins over [programTitle]. */
    val customTitle = MutableStateFlow<String?>(null)
    /** Increments on restart, so views reset instead of appending. */
    private val mutableGeneration = MutableStateFlow(0)
    val generation: StateFlow<Int> = mutableGeneration.asStateFlow()

    @Volatile private var process: TerminalProcess? = null
    @Volatile private var oscDirectory: String? = null
    @Volatile private var size = 24 to 80
    private var job: Job? = null

    /** False until the first [start] (an id known from a session but whose process died with the app). */
    @Volatile var started = false
        private set

    internal fun start(directory: String) {
        started = true
        process?.terminate()
        process = null
        job?.cancel()
        scrollback.clear()
        mutableEnd.value = scrollback.endOffset
        mutableGeneration.value++
        mutableStatus.value = TerminalStatus.Starting
        oscDirectory = null
        val input = Channel<ByteArray>(Channel.UNLIMITED)
        inputs = input
        job = parent.launch {
            val spec = TerminalSpec(directory = directory, rows = size.first, columns = size.second)
            val proc = try {
                backend.start(spec)
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (unavailable: EnvironmentUnavailableException) {
                mutableStatus.value = TerminalStatus.Failed(unavailable.message ?: "环境不可用", environment = true)
                return@launch
            }
            catch (error: Exception) {
                mutableStatus.value = TerminalStatus.Failed(error.message ?: "终端无法启动")
                return@launch
            }
            process = proc
            // A resize that arrived while the process was starting.
            val requested = size
            if (requested != spec.rows to spec.columns) runCatching { proc.resize(requested.first, requested.second) }
            mutableStatus.value = TerminalStatus.Running
            launch { for (bytes in input) runCatching { proc.write(bytes) } }
            val decoder = StreamingUtf8Decoder()
            val osc = OscScanner { code, payload ->
                when (code) {
                    0, 2 -> mutableTitle.value = payload.trim().take(80).ifEmpty { null }
                    7 -> oscDirectory = OscScanner.cwdOf(payload)
                }
            }
            fun deliver(text: String) {
                if (text.isEmpty()) return
                osc.feed(text)
                scrollback.append(text)
                mutableEnd.value = scrollback.endOffset
            }
            try {
                proc.output.collect { bytes -> deliver(decoder.decode(bytes)) }
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (_: Exception) { }
            runCatching { deliver(decoder.decode(ByteArray(0), endOfInput = true)) }
            val code = proc.awaitExit()
            input.close()
            mutableStatus.value = TerminalStatus.Ended(code)
        }
    }

    @Volatile private var inputs: Channel<ByteArray>? = null

    /** Queues input in order; ignored when the process is not running. */
    fun write(text: String) {
        if (text.isEmpty() || status.value !is TerminalStatus.Running) return
        inputs?.trySend(text.toByteArray(Charsets.UTF_8))
    }

    fun resize(rows: Int, columns: Int) {
        if (rows !in 1..1000 || columns !in 1..1000 || size == rows to columns) return
        size = rows to columns
        val current = process ?: return
        parent.launch { runCatching { current.resize(rows, columns) } }
    }

    /** The shell's directory (OSC 7 when the shell reports it, else the backend's view), shell namespace. */
    suspend fun currentDirectory(): String? = oscDirectory ?: process?.currentDirectory()

    fun terminate() {
        process?.terminate()
    }

    /** "清屏": the retained output is dropped too, so a reattached view does not replay it. */
    fun clearScrollback() {
        scrollback.clear()
        mutableEnd.value = scrollback.endOffset
    }
}
