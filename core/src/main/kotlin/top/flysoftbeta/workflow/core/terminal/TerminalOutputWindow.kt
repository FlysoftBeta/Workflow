package top.flysoftbeta.workflow.core.terminal

/** UTF-16 code-unit offsets match Kotlin String indices and JavaScript String.length. */
data class TerminalOutputWindow(val text: String, val startOffset: Long) {
    val endOffset: Long get() = startOffset + text.length

    fun append(chunk: String, maxChars: Int): TerminalOutputWindow {
        require(maxChars > 0 && startOffset >= 0)
        if (chunk.isEmpty()) return this
        val next = text + chunk
        var dropped = (next.length - maxChars).coerceAtLeast(0)
        // Never expose the low half of a surrogate pair at the beginning of a retained window.
        if (dropped < next.length && next[dropped].isLowSurrogate()) dropped++
        return TerminalOutputWindow(next.substring(dropped), Math.addExact(startOffset, dropped.toLong()))
    }
}
