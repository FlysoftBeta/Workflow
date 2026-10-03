package top.flysoftbeta.workflow.core.terminal

/** Output since an offset: [reset] means the reader fell behind the retained window and must redraw [text]. */
data class ScrollbackSlice(val text: String, val reset: Boolean, val endOffset: Long)

/**
 * The retained decoded output of one terminal, so a recreated view can replay it (reattach with
 * scrollback). Offsets are absolute UTF-16 positions in the whole stream. When the buffer exceeds
 * [capacity] it drops old output at a line boundary, which keeps escape sequences of the retained part
 * intact in practice. Thread-safe.
 */
class TerminalScrollback(private val capacity: Int = 1_000_000) {
    private val buffer = StringBuilder()
    private var start = 0L

    init { require(capacity >= 1024) }

    @get:Synchronized val startOffset: Long get() = start
    @get:Synchronized val endOffset: Long get() = start + buffer.length

    @Synchronized
    fun append(text: String) {
        if (text.isEmpty()) return
        buffer.append(text)
        // Trim in batches (amortized) down to the capacity.
        if (buffer.length > capacity + capacity / 4) {
            var cut = buffer.length - capacity
            val newline = buffer.indexOf("\n", cut)
            cut = if (newline in cut until (cut + 8192).coerceAtMost(buffer.length)) newline + 1 else cut
            if (cut < buffer.length && buffer[cut].isLowSurrogate()) cut++
            buffer.delete(0, cut)
            start += cut
        }
    }

    /** Everything after [offset]; a reset with the whole window when [offset] is outside it. */
    @Synchronized
    fun since(offset: Long): ScrollbackSlice {
        val end = start + buffer.length
        return if (offset < start || offset > end) ScrollbackSlice(buffer.toString(), true, end)
        else ScrollbackSlice(buffer.substring((offset - start).toInt()), false, end)
    }

    /** "清屏": forget the retained output (the view clears itself). */
    @Synchronized
    fun clear() {
        start += buffer.length
        buffer.setLength(0)
    }
}
