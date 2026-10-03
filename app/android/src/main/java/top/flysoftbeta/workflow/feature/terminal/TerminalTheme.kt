package top.flysoftbeta.workflow.feature.terminal

import androidx.compose.material3.ColorScheme
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import org.json.JSONObject

/**
 * xterm colors from the design system (docs/ux/README.md §1.3): background / foreground / cursor / selection are
 * surface / onSurface / primary / primary 30%; two ANSI-16 sets, the light one darkened so every color
 * reaches ≥ 4.5:1 on the light surface (checked when chosen: the lowest is bright cyan at 4.7:1).
 */
internal object TerminalTheme {
    private val light = listOf(
        "black" to "#1B211F", "red" to "#B3261E", "green" to "#1B6E20", "yellow" to "#7A5A00",
        "blue" to "#1A5FB4", "magenta" to "#8E3C99", "cyan" to "#00697A", "white" to "#5F6B67",
        "brightBlack" to "#4A5652", "brightRed" to "#C2352B", "brightGreen" to "#2E7D32", "brightYellow" to "#8A6500",
        "brightBlue" to "#2B6CC4", "brightMagenta" to "#9C4DAE", "brightCyan" to "#007B8F", "brightWhite" to "#3C4A44",
    )
    private val dark = listOf(
        "black" to "#3A403E", "red" to "#F28B82", "green" to "#8ED888", "yellow" to "#F7D774",
        "blue" to "#8AB4F8", "magenta" to "#D7AEFB", "cyan" to "#78D9EC", "white" to "#DDE4E1",
        "brightBlack" to "#84948E", "brightRed" to "#FFA8A0", "brightGreen" to "#A8F0A1", "brightYellow" to "#FFE79A",
        "brightBlue" to "#AECBFA", "brightMagenta" to "#E9C9FF", "brightCyan" to "#A1ECFA", "brightWhite" to "#FFFFFF",
    )

    fun json(colors: ColorScheme, isDark: Boolean): JSONObject = JSONObject().apply {
        put("background", css(colors.surface))
        put("foreground", css(colors.onSurface))
        put("cursor", css(colors.primary))
        put("cursorAccent", css(colors.surface))
        put("selectionBackground", css(colors.primary.copy(alpha = 0.3f)))
        put("scrollbarSliderBackground", css(colors.onSurfaceVariant.copy(alpha = 0.25f)))
        put("scrollbarSliderHoverBackground", css(colors.onSurfaceVariant.copy(alpha = 0.4f)))
        put("scrollbarSliderActiveBackground", css(colors.onSurfaceVariant.copy(alpha = 0.5f)))
        (if (isDark) dark else light).forEach { (name, value) -> put(name, value) }
    }

    fun link(colors: ColorScheme): String = css(colors.primary)

    private fun css(color: Color): String {
        val argb = color.toArgb()
        val alpha = (argb ushr 24) and 0xff
        val rgb = String.format("#%06X", argb and 0xffffff)
        return if (alpha == 0xff) rgb else rgb + String.format("%02X", alpha)
    }
}
