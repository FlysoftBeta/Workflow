package top.flysoftbeta.workflow.ui.sora

import android.content.Context
import io.github.rosemoe.sora.langs.textmate.registry.GrammarRegistry
import io.github.rosemoe.sora.langs.textmate.registry.ThemeRegistry
import io.github.rosemoe.sora.langs.textmate.registry.model.DefaultGrammarDefinition
import io.github.rosemoe.sora.langs.textmate.registry.model.ThemeModel
import org.eclipse.tm4e.core.registry.IGrammarSource
import org.eclipse.tm4e.core.registry.IThemeSource

/**
 * The sora-editor TextMate adapter kept from 0.2.1 (the editor UI itself is rebuilt by the editor
 * workstream): loads the bundled grammars and light/dark themes from `assets/textmate/` once, and maps
 * a file name to its grammar scope. Call [load] off the main thread.
 */
object SoraGrammars {
    private var loaded = false

    @Synchronized
    fun load(context: Context) {
        if (loaded) return
        val registry = GrammarRegistry.getInstance()
        listOf("json", "python", "javascript", "typescript", "markdown", "kotlin").forEach { language ->
            val scopeName = when (language) {
                "javascript" -> "source.js"
                "typescript" -> "source.ts"
                "markdown" -> "text.html.markdown"
                else -> "source.$language"
            }
            context.assets.open("textmate/$language.json").use {
                registry.loadGrammar(DefaultGrammarDefinition.withGrammarSource(IGrammarSource.fromInputStream(it, "$language.json", null), language, scopeName))
            }
        }
        listOf("light", "dark").forEach { theme ->
            context.assets.open("textmate/theme-$theme.json").use {
                ThemeRegistry.getInstance().loadTheme(ThemeModel(IThemeSource.fromInputStream(it, "theme-$theme.json", null), theme).apply { isDark = theme == "dark" })
            }
        }
        loaded = true
    }

    /** TextMate scope for [path] by extension, or null (plain text). */
    fun scope(path: String): String? = when (path.substringAfterLast('.', "").lowercase()) {
        "json", "jsonc" -> "source.json"
        "py" -> "source.python"
        "js", "jsx", "mjs" -> "source.js"
        "ts", "tsx" -> "source.ts"
        "md", "markdown" -> "text.html.markdown"
        "kt", "kts" -> "source.kotlin"
        else -> null
    }
}
