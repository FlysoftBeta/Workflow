package top.flysoftbeta.workflow.platform.pty

import android.system.Os
import java.io.IOException
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.flowOn
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import top.flysoftbeta.workflow.core.terminal.TerminalProcess

/**
 * A process on the JNI PTY (app/native/pty). The native handle is released only after both the exit status
 * was collected and the output was drained, so reads, writes and resizes never touch freed memory.
 * [onFinished] runs once, after release (the runtime service lease is dropped there).
 */
internal class PtyTerminalProcess(
    private val handle: Long,
    val pid: Int,
    private val onFinished: () -> Unit,
) : TerminalProcess {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val exit = CompletableDeferred<Int>()
    private val drained = CompletableDeferred<Unit>()
    private val released = AtomicBoolean(false)
    private val collected = AtomicBoolean(false)
    private val collecting = CompletableDeferred<Unit>()
    /** Guards the handle against release while a write/resize is in flight. */
    private val lock = Any()
    private val writes = Mutex()

    init {
        scope.launch {
            val code = try { NativePty.waitFor(handle) } catch (_: Throwable) { 255 }
            exit.complete(code)
        }
        scope.launch {
            exit.await()
            // The reader owns the handle while it runs; a process whose output is never collected is
            // released after a grace period.
            if (kotlinx.coroutines.withTimeoutOrNull(5_000) { collecting.await() } != null) drained.await()
            releaseOnce()
        }
    }

    override val output: Flow<ByteArray> = flow {
        check(collected.compareAndSet(false, true)) { "PTY output can be collected once" }
        collecting.complete(Unit)
        try {
            var drainDeadline = Long.MAX_VALUE
            while (true) {
                if (released.get()) break
                val bytes = NativePty.read(handle, 100) ?: break
                if (bytes.isNotEmpty()) emit(bytes)
                if (exit.isCompleted && drainDeadline == Long.MAX_VALUE) drainDeadline = System.nanoTime() + 500_000_000L
                if (exit.isCompleted && (bytes.isEmpty() || System.nanoTime() > drainDeadline)) break
            }
        } finally {
            drained.complete(Unit)
        }
    }.flowOn(Dispatchers.IO)

    override suspend fun write(bytes: ByteArray) {
        // The mutex is joined on the caller's (ordered) dispatcher before switching to the IO pool.
        writes.withLock {
            withContext(Dispatchers.IO) {
                var offset = 0
                while (offset < bytes.size) {
                    val chunk = bytes.copyOfRange(offset, (offset + 65536).coerceAtMost(bytes.size))
                    val written = synchronized(lock) {
                        if (released.get() || exit.isCompleted) throw IOException("终端进程已结束")
                        NativePty.write(handle, chunk)
                    }
                    if (written != chunk.size) throw IOException("终端输入未完整写入")
                    offset += written
                }
            }
        }
    }

    override suspend fun resize(rows: Int, columns: Int) = withContext(Dispatchers.IO) {
        require(rows in 1..1000 && columns in 1..1000) { "Invalid terminal size" }
        synchronized(lock) { if (!released.get() && !exit.isCompleted) NativePty.resize(handle, rows, columns) }
    }

    override suspend fun awaitExit(): Int = exit.await()

    override fun terminate(force: Boolean) {
        if (exit.isCompleted) return
        scope.launch {
            synchronized(lock) { if (!released.get()) NativePty.stop(handle, if (force) 0 else 600) }
        }
    }

    override suspend fun currentDirectory(): String? = withContext(Dispatchers.IO) {
        if (exit.isCompleted) null else runCatching { Os.readlink("/proc/$pid/cwd") }.getOrNull()
    }

    private suspend fun releaseOnce() = withContext(NonCancellable) {
        synchronized(lock) {
            if (!released.compareAndSet(false, true)) return@withContext
            runCatching { NativePty.release(handle) }
        }
        runCatching(onFinished)
    }
}
