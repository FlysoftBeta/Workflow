package top.flysoftbeta.workflow.feature.terminal

import android.content.Context
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicInteger
import kotlinx.coroutines.*
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.*
import top.flysoftbeta.workflow.core.terminal.*
import top.flysoftbeta.workflow.app.AppGraph
import top.flysoftbeta.workflow.platform.engine.EngineController
import top.flysoftbeta.workflow.platform.engine.EngineTerminalBackend
import top.flysoftbeta.workflow.platform.engine.EnvironmentUnavailableException

sealed interface TerminalStatus {
    data object Starting : TerminalStatus
    data object Running : TerminalStatus
    data class Ended(val exitCode: Int) : TerminalStatus
    data class Failed(val message: String, val environment: Boolean = false) : TerminalStatus
}

/** Connection-scoped UI attachments. Engine owns IDs, process lifecycle and reference cleanup. */
class TerminalHost internal constructor(
    val backend: TerminalBackend,
    val environment: EngineController? = null,
) {
    private val scope = CoroutineScope(SupervisorJob(environment?.scope?.coroutineContext?.get(Job)) + Dispatchers.Default)
    private val sessions = ConcurrentHashMap<String, TerminalSession>()
    private val fixtureOrdinals = AtomicInteger()

    init { require(environment == null || backend is ManagedTerminalBackend) }
    fun session(id: String): TerminalSession? = sessions[id]

    suspend fun create(directory: String?): String {
        val spec = TerminalSpec(directory = directory.orEmpty())
        val process = backend.start(spec)
        val managed = process as? ManagedTerminalProcess
        // Only explicitly injected test backends use a client identity; production requires managed resources.
        val id = managed?.initial?.id ?: "fixture-${UUID.randomUUID()}"
        val session = TerminalSession(id, managed?.initial?.ordinal ?: fixtureOrdinals.incrementAndGet(), backend, scope)
        sessions[id] = session
        session.bind(process)
        return id
    }

    /** Attach to a persisted panel without restarting a live or ended terminal. */
    fun attach(id: String): TerminalSession = sessions.getOrPut(id) {
        TerminalSession(id, 0, backend, scope).also { it.attach() }
    }

    fun restart(id: String): TerminalSession = attach(id).also { it.restart() }
    fun end(id: String) { sessions[id]?.terminate() }
    private fun close() { scope.cancel(); sessions.clear() }

    companion object {
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

/** Disposable decoded stream and metadata projection for one terminal attachment. */
class TerminalSession internal constructor(
    val id: String,
    initialOrdinal: Int,
    private val backend: TerminalBackend,
    private val parent: CoroutineScope,
) {
    val paths get() = backend.paths
    @Volatile var ordinal: Int = initialOrdinal
        private set
    val scrollback = TerminalScrollback()
    private val displayLock = Any()
    private val mutableEnd = MutableStateFlow(0L)
    val outputEnd = mutableEnd.asStateFlow()
    private val mutableStatus = MutableStateFlow<TerminalStatus>(TerminalStatus.Starting)
    val status = mutableStatus.asStateFlow()
    private val mutableTitle = MutableStateFlow<String?>(null)
    val programTitle = mutableTitle.asStateFlow()
    private val mutableCustomTitle = MutableStateFlow<String?>(null)
    val customTitle = mutableCustomTitle.asStateFlow()
    private val mutableGeneration = MutableStateFlow(0L)
    val generation = mutableGeneration.asStateFlow()
    @Volatile private var process: TerminalProcess? = null
    @Volatile private var directory: String? = null
    @Volatile private var size = 24 to 80
    @Volatile private var sizeRequested = false
    private var job: Job? = null
    private val inputs = Channel<ByteArray>(64)

    init {
        parent.launch {
            for (bytes in inputs) {
                try { process?.write(bytes) }
                catch (cancelled: CancellationException) { throw cancelled }
                catch (error: Exception) { fail(error) }
            }
        }
    }

    internal fun attach() {
        job = parent.launch {
            try {
                val managed = backend as? ManagedTerminalBackend ?: error("测试终端不存在")
                consume(managed.attach(id))
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { fail(error) }
        }
    }

    internal fun bind(attached: TerminalProcess) {
        job?.cancel()
        job = parent.launch {
            try { consume(attached) }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { fail(error) }
        }
    }

    private fun fail(error: Exception) {
        mutableStatus.value = TerminalStatus.Failed(error.message ?: "终端连接失败", error is EnvironmentUnavailableException)
    }

    private suspend fun consume(attached: TerminalProcess) {
        process = attached
        if (sizeRequested) attached.resize(size.first, size.second)
        if (attached is ManagedTerminalProcess) {
            var decoder = StreamingUtf8Decoder()
            var observedGeneration = Long.MIN_VALUE
            var decoderEnded = false
            attached.frames.collect { frame ->
                synchronized(displayLock) {
                    val metadata = frame.terminal
                    if (frame.reset || metadata.generation != observedGeneration) {
                        scrollback.clear()
                        decoder = StreamingUtf8Decoder()
                        decoderEnded = false
                    }
                    observedGeneration = metadata.generation
                    // Apply metadata and its bytes under one lock; pump reads the same transaction.
                    ordinal = metadata.ordinal
                    directory = metadata.cwd
                    mutableTitle.value = metadata.title
                    mutableCustomTitle.value = metadata.customTitle
                    mutableStatus.value = when (metadata.status) {
                        "running" -> TerminalStatus.Running
                        "ended" -> TerminalStatus.Ended(metadata.exitCode ?: 0)
                        else -> TerminalStatus.Failed(metadata.error ?: "终端不可用")
                    }
                    if (!decoderEnded) {
                        scrollback.append(decoder.decode(frame.bytes))
                        if (frame.eof) {
                            scrollback.append(decoder.decode(ByteArray(0), endOfInput = true))
                            decoderEnded = true
                        }
                    } else check(frame.bytes.isEmpty()) { "已结束的终端流缺少重置标记" }
                    mutableGeneration.value = metadata.generation
                    mutableEnd.value = scrollback.endOffset
                }
            }
        } else consumeFixture(attached)
    }

    /** Explicit fixture backends keep their existing stream protocol, outside the production path. */
    private suspend fun consumeFixture(attached: TerminalProcess) {
        mutableStatus.value = TerminalStatus.Running
        val decoder = StreamingUtf8Decoder()
        val osc = OscScanner { code, payload ->
            when (code) { 0, 2 -> mutableTitle.value = payload.trim().take(80).ifEmpty { null }; 7 -> directory = OscScanner.cwdOf(payload) }
        }
        fun deliver(text: String) = synchronized(displayLock) {
            osc.feed(text); scrollback.append(text); mutableEnd.value = scrollback.endOffset
        }
        attached.output.collect { deliver(decoder.decode(it)) }
        deliver(decoder.decode(ByteArray(0), endOfInput = true))
        mutableStatus.value = TerminalStatus.Ended(attached.awaitExit())
    }

    /** Generation and slice are read together, so a reset cannot pair new metadata with old bytes. */
    fun outputSince(offset: Long, previousGeneration: Long): Pair<Long, ScrollbackSlice> = synchronized(displayLock) {
        val generation = mutableGeneration.value
        generation to scrollback.since(if (generation != previousGeneration || offset < 0) Long.MIN_VALUE else offset)
    }

    fun write(text: String) {
        if (text.isEmpty() || status.value !is TerminalStatus.Running) return
        if (inputs.trySend(text.toByteArray(Charsets.UTF_8)).isFailure) mutableStatus.value = TerminalStatus.Failed("终端输入队列已满，请稍后重试")
    }
    fun resize(rows: Int, columns: Int) {
        if (rows !in 1..1000 || columns !in 1..1000 || (sizeRequested && size == rows to columns)) return
        size = rows to columns
        sizeRequested = true
        process?.let { current -> action { current.resize(rows, columns) } }
    }
    suspend fun currentDirectory(): String? = directory ?: process?.currentDirectory()
    fun terminate() { process?.terminate() }
    fun rename(title: String?) {
        val current = process
        if (current is ManagedTerminalProcess) action { current.rename(title) }
        else if (backend !is ManagedTerminalBackend) mutableCustomTitle.value = title
    }
    fun clearScrollback() {
        val current = process
        if (current is ManagedTerminalProcess) action { current.clear() }
        else if (backend !is ManagedTerminalBackend) synchronized(displayLock) {
            scrollback.clear(); mutableGeneration.value++; mutableEnd.value = scrollback.endOffset
        }
    }
    fun restart() = action {
        val current = process
        if (backend is ManagedTerminalBackend) {
            val attached = current as? ManagedTerminalProcess ?: backend.attach(id)
            attached.restart(size.first, size.second)
            // A failed reader can be reattached after the explicit Engine restart.
            if (job?.isActive != true) attach()
        } else {
            current?.terminate()
            synchronized(displayLock) { scrollback.clear(); mutableGeneration.value++ }
            bind(backend.start(TerminalSpec(rows = size.first, columns = size.second)))
        }
    }
    private fun action(block: suspend () -> Unit) { parent.launch {
        try { block() } catch (cancelled: CancellationException) { throw cancelled } catch (error: Exception) { fail(error) }
    } }
}
