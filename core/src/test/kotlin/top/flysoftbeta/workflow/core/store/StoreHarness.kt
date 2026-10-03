package top.flysoftbeta.workflow.core.store

import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import top.flysoftbeta.workflow.core.io.FileSystem
import top.flysoftbeta.workflow.core.io.ManualFileWatcher
import top.flysoftbeta.workflow.core.io.MemoryFileSystem
import top.flysoftbeta.workflow.core.io.writeAtomic

/** A store over an in-memory workspace with a manual clock, ids and watcher. */
@OptIn(ExperimentalCoroutinesApi::class)
internal class StoreHarness(
    private val test: TestScope,
    val memory: MemoryFileSystem? = null,
    fileSystem: FileSystem? = null,
) {
    var now = 1_000_000_000L
    val fs: FileSystem = fileSystem ?: memory ?: MemoryFileSystem { now }
    val mem: MemoryFileSystem get() = fs as MemoryFileSystem
    val watcher = ManualFileWatcher()
    private var ids = 0

    fun store(options: StoreOptions = StoreOptions()): WorkspaceStore =
        ReferenceWorkspaceStore(fs, test.backgroundScope, StandardTestDispatcher(test.testScheduler), watcher, { now }, { "id${++ids}" }, options)
            .also { it.start() }

    suspend fun ready(options: StoreOptions = StoreOptions()): WorkspaceStore = store(options).also { it.awaitReady() }

    /** Simulates a terminal or agent writing a file, and the watcher noticing. */
    fun external(path: String, text: String?) {
        now += 1_000
        if (text == null) fs.delete(path) else fs.writeAtomic(path, text)
        watcher.changed(path)
    }

    /** Runs every pending task, including delayed background ones (debounce, retries, watcher). */
    fun idle() {
        test.testScheduler.advanceTimeBy(120_000)
        test.testScheduler.runCurrent()
    }

    fun writes(path: String): Int = mem.writeLog.count { it == path }
}
