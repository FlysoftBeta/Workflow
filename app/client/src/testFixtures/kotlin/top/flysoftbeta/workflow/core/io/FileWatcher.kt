package top.flysoftbeta.workflow.core.io

/**
 * Port for external change notifications (terminal commands, agents, other apps).
 * Watching is non-recursive and per directory, so atomic replacement of a watched file keeps
 * being observed. Events are hints only: the store re-reads versions before acting.
 */
fun interface FileWatcher {
    /**
     * Starts watching [directory] (workspace-relative, "" = root). [onChange] receives the changed
     * child name, or null when unknown, and may be called on any thread. Closing stops the watch.
     */
    fun watch(directory: String, onChange: (name: String?) -> Unit): AutoCloseable

    companion object {
        val NONE = FileWatcher { _, _ -> AutoCloseable { } }
    }
}

/** A [FileWatcher] driven by hand, for tests and hosts without notifications. */
class ManualFileWatcher : FileWatcher {
    private val listeners = mutableMapOf<String, MutableList<(String?) -> Unit>>()

    val watchedDirectories: Set<String> @Synchronized get() = listeners.filterValues { it.isNotEmpty() }.keys.toSet()

    @Synchronized
    override fun watch(directory: String, onChange: (name: String?) -> Unit): AutoCloseable {
        listeners.getOrPut(directory) { mutableListOf() } += onChange
        return AutoCloseable { synchronized(this) { listeners[directory]?.remove(onChange) } }
    }

    /** Notifies the watchers of the parent directory of [path]. */
    fun changed(path: String) {
        val callbacks = synchronized(this) { listeners[WorkspacePaths.parent(path)]?.toList().orEmpty() }
        callbacks.forEach { it(WorkspacePaths.name(path)) }
    }
}
