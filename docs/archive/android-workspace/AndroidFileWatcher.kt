package top.flysoftbeta.workflow.platform.workspace

import android.os.FileObserver
import top.flysoftbeta.workflow.core.io.FileWatcher
import java.io.File
import java.util.concurrent.CopyOnWriteArrayList

/**
 * [FileWatcher] over inotify ([FileObserver]). Watching the parent directory keeps atomic
 * replacements visible. Callbacks arrive on the observer thread; the store re-posts them.
 *
 * One [FileObserver] per directory, shared by all listeners: up to API 28 FileObserver keeps a single
 * observer per inotify watch descriptor, so a second observer of the same directory (the store tracks
 * a file's parent, the explorer lists it) silently replaced the first, and stopping either removed the
 * kernel watch for both.
 */
class AndroidFileWatcher(private val root: File) : FileWatcher {
    private class Shared(val observer: FileObserver, val listeners: CopyOnWriteArrayList<(String?) -> Unit>)

    private val shared = HashMap<String, Shared>()

    override fun watch(directory: String, onChange: (name: String?) -> Unit): AutoCloseable {
        val path = if (directory.isEmpty()) root.path else File(root, directory).path
        synchronized(shared) {
            val entry = shared.getOrPut(path) {
                val listeners = CopyOnWriteArrayList<(String?) -> Unit>()
                @Suppress("DEPRECATION") // The File-based constructor needs API 29; minSdk is 28.
                val observer = object : FileObserver(path, EVENTS) {
                    override fun onEvent(event: Int, name: String?) {
                        if ((event and ALL_EVENTS) == 0) return
                        listeners.forEach { runCatching { it(name) } }
                    }
                }
                Shared(observer, listeners).also { observer.startWatching() }
            }
            entry.listeners += onChange
        }
        // Holding the observer in the shared map keeps it reachable (an unreferenced observer stops).
        return AutoCloseable {
            synchronized(shared) {
                val entry = shared[path] ?: return@synchronized
                entry.listeners.remove(onChange)
                if (entry.listeners.isEmpty()) {
                    shared.remove(path)
                    entry.observer.stopWatching()
                }
            }
        }
    }

    private companion object {
        const val EVENTS = FileObserver.CLOSE_WRITE or FileObserver.MOVED_TO or FileObserver.MOVED_FROM or
            FileObserver.CREATE or FileObserver.DELETE or FileObserver.DELETE_SELF or FileObserver.MOVE_SELF or
            FileObserver.ATTRIB
    }
}
