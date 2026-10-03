package top.flysoftbeta.workflow.agent.model

/**
 * Immutable text that grows by appending streamed deltas in O(1).
 *
 * The reducer produces a new state for every delta; copying a whole message or command output on
 * each token would be quadratic. A [StreamText] is a persistent chain of chunks that is collapsed
 * every [COMPACT_EVERY] appends and materialised (and cached) on the first [toString] call.
 * Equality is by content.
 */
class StreamText private constructor(
    private val previous: StreamText?,
    private val chunk: String,
    /** Total number of chars. */
    val length: Int,
    private val depth: Int,
) {
    @Volatile private var materialised: String? = if (previous == null) chunk else null

    val isEmpty: Boolean get() = length == 0

    fun append(delta: String): StreamText = when {
        delta.isEmpty() -> this
        length == 0 -> of(delta)
        depth >= COMPACT_EVERY -> StreamText(null, toString() + delta, length + delta.length, 0)
        else -> StreamText(this, delta, length + delta.length, depth + 1)
    }

    /** Keeps only the last [maxChars] characters. */
    fun takeLast(maxChars: Int): StreamText = if (length <= maxChars) this else of(toString().takeLast(maxChars))

    override fun toString(): String {
        materialised?.let { return it }
        val chunks = ArrayList<String>(depth + 1)
        var node: StreamText? = this
        while (node != null) {
            val cached = node.materialised
            if (cached != null) { chunks.add(cached); break }
            chunks.add(node.chunk)
            node = node.previous
        }
        val builder = StringBuilder(length)
        for (i in chunks.indices.reversed()) builder.append(chunks[i])
        return builder.toString().also { materialised = it }
    }

    override fun equals(other: Any?): Boolean =
        this === other || (other is StreamText && other.length == length && other.toString() == toString())

    override fun hashCode(): Int = toString().hashCode()

    companion object {
        private const val COMPACT_EVERY = 256
        val EMPTY = StreamText(null, "", 0, 0)
        fun of(text: String?): StreamText = if (text.isNullOrEmpty()) EMPTY else StreamText(null, text, text.length, 0)
    }
}
