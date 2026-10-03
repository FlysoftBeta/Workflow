package top.flysoftbeta.workflow.core.terminal

data class TerminalPatch(val reset: Boolean, val text: String)

/** Absolute UTF-16 positions let a rolling history append without replaying the VT stream. */
class TerminalDeltaTracker {
    private var initialized = false
    private var streamId: String? = null
    private var consumedEnd = 0L

    fun clear() { initialized = false; streamId = null; consumedEnd = 0 }

    fun update(id: String, start: Long, text: String): TerminalPatch? {
        require(start >= 0 && start <= Long.MAX_VALUE - text.length)
        val end = start + text.length
        val reset = !initialized || streamId != id || start > consumedEnd || end < consumedEnd
        val patch = when {
            reset -> TerminalPatch(true, text)
            end == consumedEnd -> null
            else -> TerminalPatch(false, text.substring((consumedEnd - start).toInt()))
        }
        initialized = true
        streamId = id
        consumedEnd = end
        return patch
    }
}
