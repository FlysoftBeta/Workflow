package top.flysoftbeta.workflow.feature.editor

import java.io.File
import java.security.MessageDigest
import org.eclipse.tm4e.core.internal.oniguruma.OnigString
import org.eclipse.tm4e.core.internal.oniguruma.impl.joni.JoniOnigRegExp
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.core.json.Json

/** Compile with Sora's actual regex engine, which rejects variable-length lookbehind. */
class MarkdownGrammarTest {
    private val grammar = Json.parse(File("src/main/assets/textmate/markdown.json").readText()) as Map<*, *>
    private val strikePattern = ((grammar["repository"] as Map<*, *>)["strikethrough"] as Map<*, *>)["match"] as String

    @Test fun patchedAssetMatchesItsManifest() {
        val manifest = Json.parse(File("src/main/assets/textmate/manifest.json").readText()) as List<*>
        val entry = manifest.map { it as Map<*, *> }.single { it["file"] == "markdown.json" }
        val digest = MessageDigest.getInstance("SHA-256").digest(File("src/main/assets/textmate/markdown.json").readBytes())
        assertEquals(entry["sha256"], digest.joinToString("") { "%02x".format(it) })
        assertEquals(listOf("joni-strikethrough-lookahead"), entry["patches"])
        assertNotEquals(entry["upstream_sha256"], entry["sha256"])
    }

    @Test fun allStaticMarkdownPatternsCompileInJoni() {
        var count = 0
        fun visit(value: Any?) {
            when (value) {
                is Map<*, *> -> value.forEach { (key, child) ->
                    if (key == "match" || key == "begin") {
                        JoniOnigRegExp(child as String)
                        count++
                    } else visit(child)
                }
                is List<*> -> value.forEach(::visit)
            }
        }
        visit(grammar)
        assertTrue("Expected the complete bundled grammar", count > 100)
    }

    @Test fun strikethroughPreservesDelimiterAndContentCaptures() {
        val regex = JoniOnigRegExp(strikePattern)
        for (text in listOf("~~removed~~", "~~~removed~~~", "~~_removed_~~", "~~a~b~~")) {
            val match = regex.search(OnigString.of(text), 0)
            assertNotNull(text, match)
            assertEquals(text.length, match!!.lengthAt(0))
            assertEquals(4, match.count())
            assertEquals(match.lengthAt(1), match.lengthAt(3))
            assertTrue(match.lengthAt(2) > 0)
        }
        // Test the underscore boundary before the closing delimiter. Keep the backreference in
        // a lookahead, where Joni accepts it, instead of a variable-length lookbehind.
        assertNull(regex.search(OnigString.of("~~removed_~~word"), 0))
        assertNull(regex.search(OnigString.of("\\~~removed~~"), 0))
    }
}
