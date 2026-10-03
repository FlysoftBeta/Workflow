package top.flysoftbeta.workflow.feature.chat.transcript

import org.commonmark.ext.gfm.strikethrough.Strikethrough
import org.commonmark.ext.gfm.strikethrough.StrikethroughExtension
import org.commonmark.ext.gfm.tables.*
import org.commonmark.node.*
import org.commonmark.parser.Parser
import org.commonmark.parser.beta.*

/** Immutable rendering tree. No HTML, JavaScript, Android View or network renderer exists here. */
data class MarkdownSpan(val text: String, val bold: Boolean = false, val italic: Boolean = false,
    val strike: Boolean = false, val code: Boolean = false, val link: String? = null, val math: Boolean = false)
sealed interface MarkdownBlock {
    data class Prose(val spans: List<MarkdownSpan>, val heading: Int = 0, val indent: Int = 0,
        val marker: String? = null, val quote: Boolean = false) : MarkdownBlock
    data class Code(val text: String, val language: String, val copyText: String = text, val continuation: Boolean = false) : MarkdownBlock
    data class Formula(val latex: String) : MarkdownBlock
    data class Table(val rows: List<List<List<MarkdownSpan>>>, val alignments: List<String>) : MarkdownBlock
    data object Rule : MarkdownBlock
}

/** Custom inline parser runs before CommonMark's backslash escape parser, retaining both TeX delimiters. */
private class MathNode(val latex: String, val display: Boolean) : CustomNode()
private class MathParser : InlineContentParser {
    override fun tryParse(state: InlineParserState): ParsedInline? {
        val scanner = state.scanner()
        val begin = scanner.position()
        val opener = when (scanner.peek()) {
            '$' -> { scanner.next(); if (scanner.next('$')) "$$" else "$" }
            '\\' -> { scanner.next(); when { scanner.next('(') -> "\\("; scanner.next('[') -> "\\["; else -> return ParsedInline.none() } }
            else -> return ParsedInline.none()
        }
        val display = opener == "$$" || opener == "\\["
        if (opener == "$" && (scanner.peek().isWhitespace() || scanner.peek() == Scanner.END)) return ParsedInline.none()
        val close = when (opener) { "\\(" -> "\\)"; "\\[" -> "\\]"; else -> opener }
        val content = scanner.position()
        while (scanner.hasNext()) {
            val before = scanner.position()
            val previous = scanner.peekPreviousCodePoint()
            if (scanner.next(close)) {
                if (opener == "$" && (previous.toChar().isWhitespace() || scanner.peek().isDigit())) continue
                val latex = scanner.getSource(content, before).content
                if (latex.isNotBlank()) return ParsedInline.of(MathNode(latex, display), scanner.position())
            } else {
                scanner.next()
                // An escaped dollar is text inside math, not its closing delimiter.
                if (previous == '\\'.code && scanner.peek() == '$') scanner.next()
            }
        }
        // Open streaming delimiters stay visible, including the backslash.
        return ParsedInline.of(Text(scanner.getSource(begin, scanner.position()).content), scanner.position())
    }
    class Factory : InlineContentParserFactory {
        override fun getTriggerCharacters(): Set<Char> = setOf('$', '\\')
        override fun create(): InlineContentParser = MathParser()
    }
}

object NativeMarkdownParser {
    private val parser = Parser.builder().extensions(listOf(TablesExtension.create(), StrikethroughExtension.create()))
        .customInlineContentParserFactory(MathParser.Factory()).build()

    fun parse(source: String): List<MarkdownBlock> {
        val result = ArrayList<MarkdownBlock>()
        fun inlines(node: Node, style: MarkdownSpan = MarkdownSpan("")): List<MarkdownSpan> {
            val own = when (node) {
                is Text -> return listOf(style.copy(text = node.literal))
                is Code -> return listOf(style.copy(text = node.literal, code = true))
                is MathNode -> return listOf(style.copy(text = node.latex, math = true, code = node.display))
                is SoftLineBreak, is HardLineBreak -> return listOf(style.copy(text = "\n"))
                is HtmlInline -> return listOf(style.copy(text = node.literal))
                is StrongEmphasis -> style.copy(bold = true)
                is Emphasis -> style.copy(italic = true)
                is Strikethrough -> style.copy(strike = true)
                is Link -> style.copy(link = safeLink(node.destination))
                is Image -> style.copy(link = safeLink(node.destination))
                else -> style
            }
            return node.children().flatMap { inlines(it, own) }
        }
        fun prose(node: Node, indent: Int, marker: String?, quote: Boolean, heading: Int = 0) {
            var first = true
            var buffer = ArrayList<MarkdownSpan>()
            fun flush() {
                if (buffer.isEmpty()) return
                // Bound Text layout/selection work for very large single-paragraph replies.
                var chunk = ArrayList<MarkdownSpan>()
                var length = 0
                fun emit() {
                    if (chunk.isNotEmpty()) result += MarkdownBlock.Prose(chunk, heading, indent, if (first) marker else null, quote)
                    chunk = ArrayList(); length = 0; first = false
                }
                for (span in buffer) {
                    if (span.math) { if (length > 3500) emit(); chunk += span; length += span.text.length }
                    else for (part in span.text.chunked(3500)) {
                        if (length + part.length > 4000) emit()
                        chunk += span.copy(text = part); length += part.length
                    }
                }
                emit(); buffer = ArrayList()
            }
            for (span in inlines(node)) {
                if (span.math && span.code) { flush(); result += MarkdownBlock.Formula(span.text) }
                else buffer += span
            }
            flush()
        }
        fun code(text: String, language: String) {
            val chunks = text.trimEnd('\n').chunked(4000).ifEmpty { listOf("") }
            chunks.forEachIndexed { i, part -> result += MarkdownBlock.Code(part, language, text.trimEnd('\n'), i != 0) }
        }
        fun walk(node: Node, indent: Int = 0, marker: String? = null, quote: Boolean = false) {
            when (node) {
                is Paragraph -> prose(node, indent, marker, quote)
                is Heading -> prose(node, indent, marker, quote, node.level)
                is FencedCodeBlock -> code(node.literal, node.info.orEmpty().substringBefore(' '))
                is IndentedCodeBlock -> code(node.literal, "")
                is HtmlBlock -> code(node.literal, "text")
                is ThematicBreak -> result += MarkdownBlock.Rule
                is BlockQuote -> node.children().forEach { walk(it, indent, marker, true) }
                is BulletList, is OrderedList -> node.children().forEachIndexed { i, item ->
                    val label = if (node is OrderedList) "${node.markerStartNumber + i}." else "•"
                    item.children().forEachIndexed { j, child -> walk(child, indent + 1, if (j == 0) label else null, quote) }
                }
                is TableBlock -> {
                    val rows = node.children().flatMap { it.children() }
                    val alignment = rows.firstOrNull()?.children()?.map { (it as? TableCell)?.alignment?.name.orEmpty() }.orEmpty()
                    result += MarkdownBlock.Table(rows.map { row -> row.children().map { inlines(it) } }, alignment)
                }
                else -> node.children().forEach { walk(it, indent, marker, quote) }
            }
        }
        try { walk(parser.parse(source)) }
        catch (_: Exception) { return source.chunked(4000).map { MarkdownBlock.Prose(listOf(MarkdownSpan(it))) } }
        catch (_: StackOverflowError) { return source.chunked(4000).map { MarkdownBlock.Prose(listOf(MarkdownSpan(it))) } }
        return result
    }

    /** No intent/data/javascript/file URI can reach the platform. Relative links are workspace paths. */
    fun safeLink(destination: String): String? {
        val link = destination.trim()
        if (link.isEmpty() || link.any { it.code < 32 } || link.startsWith("//")) return null
        val scheme = Regex("^([A-Za-z][A-Za-z0-9+.-]*):").find(link)?.groupValues?.get(1)?.lowercase()
        return if (scheme == null || scheme in setOf("http", "https", "mailto") || Regex("^[^:]+:\\d+(:\\d+)?$").matches(link)) link else null
    }
}

private fun Node.children(): List<Node> = buildList { var n = firstChild; while (n != null) { add(n); n = n.next } }

/** Per-controller bounded cache; only changed messages are parsed on the Default dispatcher. */
class MarkdownCache(private val maxCharacters: Int = 2_000_000) {
    private data class Entry(val source: String, val blocks: List<MarkdownBlock>)
    private val entries = LinkedHashMap<String, Entry>(16, .75f, true)
    private var size = 0
    fun parse(key: String, source: String): List<MarkdownBlock> {
        entries[key]?.takeIf { it.source == source }?.let { return it.blocks }
        val parsed = NativeMarkdownParser.parse(source)
        entries.remove(key)?.let { size -= it.source.length }
        entries[key] = Entry(source, parsed); size += source.length
        while (size > maxCharacters && entries.size > 1) {
            val eldest = entries.entries.first(); size -= eldest.value.source.length; entries.remove(eldest.key)
        }
        return parsed
    }
}

/** A deliberately bounded math language: no I/O, user macros, dynamic commands or dimension primitives. */
object MathSafety {
    private val commands = ("frac dfrac tfrac sqrt binom dbinom tbinom over atop choose boxed cancel overset underset substack overbrace underbrace overrightarrow overleftarrow " +
        "left right middle big Big bigg Bigg bigl bigr Bigl Bigr biggl biggr Biggl Biggr " +
        "sum prod coprod bigcup bigcap bigsqcup bigvee bigwedge bigoplus bigotimes biguplus opulus oplus otimes odot oslash ominus int iint iiint oint lim limits nolimits sin cos tan cot sec csc arcsin arccos arctan " +
        "sinh cosh tanh log ln exp min max sup inf det gcd Pr mod bmod pmod operatorname text textrm textbf textit " +
        "mathrm mathbf mathit mathsf mathtt mathcal mathbb mathfrak boldsymbol underline overline vec hat widehat tilde widetilde bar dot ddot " +
        "alpha beta gamma delta epsilon varepsilon zeta eta theta vartheta iota kappa lambda mu nu xi omicron pi varpi rho varrho sigma varsigma tau upsilon phi varphi chi psi omega " +
        "Gamma Delta Theta Lambda Xi Pi Sigma Upsilon Phi Psi Omega " +
        "infty partial nabla ell hbar imath jmath Re Im wp emptyset varnothing forall exists neg land lor lnot " +
        "not in notin ni subset supset subseteq supseteq cup cap setminus times div cdot pm mp ast star circ bullet " +
        "le leq ge geq ne neq approx sim simeq equiv propto cong ll gg prec succ preceq succeq " +
        "to gets mapsto rightarrow leftarrow leftrightarrow Rightarrow Leftarrow Leftrightarrow longrightarrow longleftarrow Longrightarrow Longleftarrow " +
        "uparrow downarrow updownarrow Uparrow Downarrow Updownarrow ldots cdots vdots ddots dots " +
        "langle rangle lbrace rbrace lvert rvert lVert rVert vert Vert backslash degree prime angle triangle perp parallel " +
        "quad qquad enspace thinspace negthinspace displaystyle textstyle scriptstyle scriptscriptstyle begin end").split(' ').toSet()
    private val environments = setOf("matrix", "pmatrix", "bmatrix", "Bmatrix", "vmatrix", "Vmatrix", "smallmatrix", "cases", "aligned", "align", "align*", "gathered", "array")
    fun accepts(source: String): Boolean {
        if (source.length !in 1..4096 || source.count { it == '\\' } > 256 || source.count { it == '&' } > 64) return false
        var depth = 0
        source.forEachIndexed { index, c ->
            var slashes = 0; var before = index - 1
            while (before >= 0 && source[before--] == '\\') slashes++
            if (slashes % 2 == 0) {
                if (c == '{' && ++depth > 32) return false
                if (c == '}' && --depth < 0) return false
            }
        }
        if (depth != 0) return false
        var cursor = 0
        while (cursor < source.length) {
            if (source[cursor++] != '\\' || cursor >= source.length) continue
            val start = cursor
            if (source[cursor].isLetter()) {
                while (cursor < source.length && source[cursor].isLetter()) cursor++
                if (source.substring(start, cursor) !in commands) return false
            } else cursor++
        }
        if (Regex("\\\\(?:begin|end)\\s*\\{([^}]+)\\}").findAll(source).any { it.groupValues[1] !in environments }) return false
        // Array preambles cannot inject repeat-count or dimension syntax.
        if (Regex("\\\\begin\\s*\\{array\\}\\s*\\{([^}]+)\\}").findAll(source).any { !it.groupValues[1].matches(Regex("[lcr| ]{1,16}")) }) return false
        return !source.contains('ˆ') && !source.contains("^^")
    }
}
