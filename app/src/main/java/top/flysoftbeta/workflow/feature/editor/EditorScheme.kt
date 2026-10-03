package top.flysoftbeta.workflow.feature.editor

import android.content.res.AssetManager
import android.graphics.Typeface
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import io.github.rosemoe.sora.langs.textmate.TextMateColorScheme
import io.github.rosemoe.sora.langs.textmate.registry.ThemeRegistry
import io.github.rosemoe.sora.widget.schemes.EditorColorScheme
import top.flysoftbeta.workflow.ui.design.theme.MonoFontAssets
import top.flysoftbeta.workflow.ui.design.theme.WorkflowDarkColorScheme
import top.flysoftbeta.workflow.ui.design.theme.WorkflowLightColorScheme

/**
 * sora colors (docs/ui.md §4.3): TextMate token colors from `assets/textmate/theme-{light,dark}.json`
 * (JSON keys and values differ), editor chrome from the design system: gutter numbers onSurfaceVariant,
 * current line surfaceContainerLow, selection primary 25%. The TextMate theme registry is global, so
 * [applyTheme] switches every open editor.
 */
internal class WorkflowEditorScheme(registry: ThemeRegistry) : TextMateColorScheme(registry, registry.currentThemeModel) {
    override fun applyDefault() {
        super.applyDefault()
        val c = if (isDark) WorkflowDarkColorScheme else WorkflowLightColorScheme
        fun set(id: Int, color: Color) = setColor(id, color.toArgb())
        set(WHOLE_BACKGROUND, c.surface)
        set(LINE_NUMBER_BACKGROUND, c.surface)
        set(LINE_NUMBER, c.onSurfaceVariant.copy(alpha = 0.7f))
        set(LINE_NUMBER_CURRENT, c.onSurface)
        set(LINE_DIVIDER, Color.Transparent)
        set(CURRENT_LINE, c.surfaceContainerLow)
        set(TEXT_NORMAL, c.onSurface)
        set(SELECTED_TEXT_BACKGROUND, c.primary.copy(alpha = 0.25f))
        set(SELECTION_INSERT, c.primary)
        set(SELECTION_HANDLE, c.primary)
        set(MATCHED_TEXT_BACKGROUND, c.tertiaryContainer)
        set(SCROLL_BAR_THUMB, c.onSurfaceVariant.copy(alpha = 0.3f))
        set(SCROLL_BAR_THUMB_PRESSED, c.primary)
        set(SCROLL_BAR_TRACK, Color.Transparent)
        set(BLOCK_LINE, c.outlineVariant)
        set(BLOCK_LINE_CURRENT, c.outline)
        set(SIDE_BLOCK_LINE, c.outlineVariant)
        set(NON_PRINTABLE_CHAR, c.outlineVariant)
        set(HIGHLIGHTED_DELIMITERS_FOREGROUND, c.primary)
        set(HIGHLIGHTED_DELIMITERS_UNDERLINE, c.primary)
        set(HIGHLIGHTED_DELIMITERS_BACKGROUND, Color.Transparent)
        set(TEXT_ACTION_WINDOW_BACKGROUND, c.surfaceContainerHigh)
        set(TEXT_ACTION_WINDOW_ICON_COLOR, c.onSurface)
        set(COMPLETION_WND_BACKGROUND, c.surfaceContainerHigh)
        set(COMPLETION_WND_CORNER, c.outlineVariant)
        set(COMPLETION_WND_TEXT_PRIMARY, c.onSurface)
        set(COMPLETION_WND_TEXT_SECONDARY, c.onSurfaceVariant)
        set(COMPLETION_WND_ITEM_CURRENT, c.secondaryContainer)
        set(UNDERLINE, c.primary)
        set(HARD_WRAP_MARKER, c.outlineVariant)
    }

    companion object {
        fun applyTheme(dark: Boolean) {
            val registry = ThemeRegistry.getInstance()
            val name = if (dark) "dark" else "light"
            if (registry.currentThemeModel?.name != name) runCatching { registry.setTheme(name) }
        }

        @Volatile private var mono: Typeface? = null

        fun typeface(assets: AssetManager): Typeface = mono ?: synchronized(this) {
            mono ?: runCatching { Typeface.createFromAsset(assets, MonoFontAssets.REGULAR) }.getOrDefault(Typeface.MONOSPACE).also { mono = it }
        }

    }
}
