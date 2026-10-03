package top.flysoftbeta.workflow.core.resource

/** One line of a diff: unchanged, only on the left (disk), or only on the right (mine). Line numbers are 1-based. */
sealed interface DiffLine {
    data class Same(val leftNo: Int, val rightNo: Int, val text: String) : DiffLine
    data class Removed(val leftNo: Int, val text: String) : DiffLine
    data class Added(val rightNo: Int, val text: String) : DiffLine
}

/**
 * Line diff for the conflict "对比" panel (Myers' O((N+M)·D) algorithm). When the files differ in more
 * than [maxEdits] lines, the result is simply "all removed, all added", which is still correct.
 */
object LineDiff {
    fun lines(text: String): List<String> = if (text.isEmpty()) emptyList() else text.split('\n')

    fun diff(left: List<String>, right: List<String>, maxEdits: Int = 4000): List<DiffLine> {
        // Trim the common prefix and suffix first: typical conflicts touch a few lines.
        var prefix = 0
        while (prefix < left.size && prefix < right.size && left[prefix] == right[prefix]) prefix++
        var suffix = 0
        while (suffix < left.size - prefix && suffix < right.size - prefix &&
            left[left.size - 1 - suffix] == right[right.size - 1 - suffix]) suffix++
        val a = left.subList(prefix, left.size - suffix)
        val b = right.subList(prefix, right.size - suffix)
        val middle = myers(a, b, maxEdits) ?: (a.map { Op.DELETE } + b.map { Op.INSERT })
        val out = ArrayList<DiffLine>(left.size + right.size)
        for (i in 0 until prefix) out += DiffLine.Same(i + 1, i + 1, left[i])
        var i = 0
        var j = 0
        for (op in middle) when (op) {
            Op.KEEP -> { out += DiffLine.Same(prefix + i + 1, prefix + j + 1, a[i]); i++; j++ }
            Op.DELETE -> { out += DiffLine.Removed(prefix + i + 1, a[i]); i++ }
            Op.INSERT -> { out += DiffLine.Added(prefix + j + 1, b[j]); j++ }
        }
        for (k in 0 until suffix) {
            val l = left.size - suffix + k
            val r = right.size - suffix + k
            out += DiffLine.Same(l + 1, r + 1, left[l])
        }
        return out
    }

    private enum class Op { KEEP, DELETE, INSERT }

    private fun myers(a: List<String>, b: List<String>, maxEdits: Int): List<Op>? {
        val n = a.size
        val m = b.size
        if (n == 0) return List(m) { Op.INSERT }
        if (m == 0) return List(n) { Op.DELETE }
        val max = minOf(n + m, maxEdits)
        val offset = max + 1
        var v = IntArray(2 * max + 3)
        val trace = ArrayList<IntArray>()
        for (d in 0..max) {
            trace += v.copyOf()
            val next = v.copyOf()
            var k = -d
            while (k <= d) {
                var x = if (k == -d || (k != d && v[offset + k - 1] < v[offset + k + 1])) v[offset + k + 1] else v[offset + k - 1] + 1
                var y = x - k
                while (x < n && y < m && a[x] == b[y]) { x++; y++ }
                next[offset + k] = x
                if (x >= n && y >= m) return backtrack(trace, d, n, m, offset)
                k += 2
            }
            v = next
        }
        return null
    }

    private fun backtrack(trace: List<IntArray>, dEnd: Int, n: Int, m: Int, offset: Int): List<Op> {
        val ops = ArrayList<Op>()
        var x = n
        var y = m
        for (d in dEnd downTo 1) {
            val v = trace[d]
            val k = x - y
            val prevK = if (k == -d || (k != d && v[offset + k - 1] < v[offset + k + 1])) k + 1 else k - 1
            val prevX = v[offset + prevK]
            val prevY = prevX - prevK
            while (x > prevX && y > prevY) { ops += Op.KEEP; x--; y-- }
            if (x == prevX) { ops += Op.INSERT; y-- } else { ops += Op.DELETE; x-- }
        }
        while (x > 0 && y > 0) { ops += Op.KEEP; x--; y-- }
        return ops.asReversed()
    }
}
