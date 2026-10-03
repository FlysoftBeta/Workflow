package top.flysoftbeta.workflow.core.terminal

/** Non-character keys of the terminal's extra-keys row. */
enum class TerminalKey { ESCAPE, TAB, ENTER, BACKSPACE, DELETE, UP, DOWN, RIGHT, LEFT, HOME, END, PAGE_UP, PAGE_DOWN,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12 }

/** Encoded input for one key: [normal] and, for cursor keys, the DECCKM application-mode variant. */
data class KeySequence(val normal: String, val application: String? = null)

/** xterm-compatible key encoding for the extra-keys row and latched Ctrl/Alt (docs/ux/README.md §4.4). */
object TerminalKeys {
    private const val ESC = "\u001b"

    fun encode(key: TerminalKey, ctrl: Boolean = false, alt: Boolean = false): KeySequence {
        val modifier = 1 + (if (alt) 2 else 0) + (if (ctrl) 4 else 0)
        val modified = modifier > 1
        fun cursor(final: Char) = if (modified) KeySequence("$ESC[1;$modifier$final")
            else KeySequence("$ESC[$final", "${ESC}O$final")
        fun tilde(code: Int) = KeySequence(if (modified) "$ESC[$code;$modifier~" else "$ESC[$code~")
        fun ss3(final: Char) = KeySequence(if (modified) "$ESC[1;$modifier$final" else "${ESC}O$final")
        fun plain(text: String) = KeySequence(if (alt) ESC + text else text)
        return when (key) {
            TerminalKey.ESCAPE -> plain(ESC)
            TerminalKey.TAB -> plain("\t")
            TerminalKey.ENTER -> plain("\r")
            TerminalKey.BACKSPACE -> plain(if (ctrl) "\b" else "\u007f")
            TerminalKey.DELETE -> tilde(3)
            TerminalKey.UP -> cursor('A')
            TerminalKey.DOWN -> cursor('B')
            TerminalKey.RIGHT -> cursor('C')
            TerminalKey.LEFT -> cursor('D')
            TerminalKey.HOME -> cursor('H')
            TerminalKey.END -> cursor('F')
            TerminalKey.PAGE_UP -> tilde(5)
            TerminalKey.PAGE_DOWN -> tilde(6)
            TerminalKey.F1 -> ss3('P')
            TerminalKey.F2 -> ss3('Q')
            TerminalKey.F3 -> ss3('R')
            TerminalKey.F4 -> ss3('S')
            TerminalKey.F5 -> tilde(15)
            TerminalKey.F6 -> tilde(17)
            TerminalKey.F7 -> tilde(18)
            TerminalKey.F8 -> tilde(19)
            TerminalKey.F9 -> tilde(20)
            TerminalKey.F10 -> tilde(21)
            TerminalKey.F11 -> tilde(23)
            TerminalKey.F12 -> tilde(24)
        }
    }

    /** The control character for Ctrl+[c], or null when there is none (xterm mapping). */
    fun control(c: Char): Char? = when (c) {
        in 'a'..'z' -> (c.code - 'a'.code + 1).toChar()
        in 'A'..'Z' -> (c.code - 'A'.code + 1).toChar()
        '@', ' ', '2' -> '\u0000'
        '[', '3' -> '\u001b'
        '\\', '4' -> '\u001c'
        ']', '5' -> '\u001d'
        '^', '6' -> '\u001e'
        '_', '7', '/', '-' -> '\u001f'
        '?', '8' -> '\u007f'
        else -> null
    }

    /**
     * Applies latched modifiers to typed text (IME input or a character key): Ctrl turns a single
     * character into its control character, Alt prefixes ESC.
     */
    fun applyModifiers(text: String, ctrl: Boolean, alt: Boolean): String {
        if (text.isEmpty() || (!ctrl && !alt)) return text
        val controlled = if (ctrl && text.length == 1) control(text[0])?.toString() ?: text else text
        return if (alt) ESC + controlled else controlled
    }
}
