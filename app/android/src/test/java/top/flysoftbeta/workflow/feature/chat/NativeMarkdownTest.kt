package top.flysoftbeta.workflow.feature.chat

import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.feature.chat.transcript.*

class NativeMarkdownTest {
    private fun prose(source: String) = NativeMarkdownParser.parse(source).filterIsInstance<MarkdownBlock.Prose>().flatMap { it.spans }
    @Test fun allFourMathDelimitersSurviveEveryStreamingBoundary() {
        val text = "Inline \$x^2\$ and \\(y+1\\).\n\n\$\$\\frac{1}{2}\$\$\n\n\\[\\sum_{i=0}^n i\\]"
        for (end in 1..text.length) assertNotNull(NativeMarkdownParser.parse(text.take(end)))
        val blocks = NativeMarkdownParser.parse(text)
        assertEquals(listOf("x^2", "y+1"), prose(text).filter { it.math }.map { it.text })
        assertEquals(listOf("\\frac{1}{2}", "\\sum_{i=0}^n i"), blocks.filterIsInstance<MarkdownBlock.Formula>().map { it.latex })
        assertTrue(prose("unfinished \\(x^").joinToString("") { it.text }.contains("\\(x^"))
    }
    @Test fun codeEscapesCurrencyAndHtmlNeverBecomeExecutable() {
        val text = "```kotlin\nval price = \"\$5\"\n```\n\n`\$x\$` and \\\$x\\\$ and \$5 or \$10.\n\n<script>alert(1)</script>"
        val blocks = NativeMarkdownParser.parse(text)
        assertEquals("val price = \"\$5\"", blocks.filterIsInstance<MarkdownBlock.Code>().first().text)
        assertTrue(prose(text).none { it.math })
        assertNull(NativeMarkdownParser.safeLink("javascript:alert(1)"))
        assertNull(NativeMarkdownParser.safeLink("file:///private/data"))
        assertNull(NativeMarkdownParser.safeLink("intent://x"))
        assertEquals("README.md:3", NativeMarkdownParser.safeLink("README.md:3"))
    }
    @Test fun richMarkdownAndTableAlignment() {
        val source = "**strong** *em* ~~gone~~ [file](app/Main.kt:3)\n\n> quote\n\n1. first\n2. second\n\n|a|b|c|\n|:---|:---:|---:|\n|left|center|right|"
        val blocks = NativeMarkdownParser.parse(source)
        val spans = prose(source)
        assertTrue(spans.any { it.bold && it.text == "strong" })
        assertTrue(spans.any { it.italic && it.text == "em" })
        assertTrue(spans.any { it.strike && it.text == "gone" })
        assertTrue(blocks.filterIsInstance<MarkdownBlock.Prose>().any { it.quote })
        assertEquals(listOf("1.", "2."), blocks.filterIsInstance<MarkdownBlock.Prose>().mapNotNull { it.marker })
        assertEquals(listOf("LEFT", "CENTER", "RIGHT"), blocks.filterIsInstance<MarkdownBlock.Table>().single().alignments)
    }
    @Test fun dangerousOrExcessiveMathNeverReachesNativeParser() {
        listOf("\\includegraphics{https://evil.test/a}", "\\input{/etc/passwd}", "\\newcommand{\\a}{\\a}\\a", "\\csname input\\endcsname", "\\rule{999999em}{999999em}", "{".repeat(33)+"x"+"}".repeat(33), "x".repeat(4097)).forEach { assertFalse(it, MathSafety.accepts(it)) }
        assertTrue(MathSafety.accepts("\\frac{1}{2}+\\sqrt{x^2}+\\sum_{i=1}^n i"))
        assertTrue(MathSafety.accepts("\\left\\{x\\right."))
        assertTrue(MathSafety.accepts("\\begin{pmatrix}a&b\\\\c&d\\end{pmatrix}"))
    }
    @Test fun longAnswersUseBoundedBlocksAndCacheUnchangedMessages() {
        val text = "A long answer. ".repeat(10000)
        val cache = MarkdownCache()
        val first = cache.parse("m1", text)
        assertTrue(first.size > 20)
        assertTrue(first.filterIsInstance<MarkdownBlock.Prose>().all { it.spans.sumOf { span -> span.text.length } <= 4000 })
        assertSame(first, cache.parse("m1", text))
        assertEquals(text.trimEnd(), first.filterIsInstance<MarkdownBlock.Prose>().flatMap { it.spans }.joinToString("") { it.text })
    }
}
