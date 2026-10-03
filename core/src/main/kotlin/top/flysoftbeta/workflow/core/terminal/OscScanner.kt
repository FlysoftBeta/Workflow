package top.flysoftbeta.workflow.core.terminal

/**
 * Watches decoded terminal output for OSC sequences (`ESC ] code ; text BEL` or `… ESC \`) without
 * altering the stream, so tab titles (OSC 0/2) and the shell's directory (OSC 7) are known even while no
 * view renders the terminal. Sequences may be split across chunks. Not thread-safe: feed from one reader.
 */
class OscScanner(private val onOsc: (code: Int, payload: String) -> Unit) {
    private enum class State { TEXT, ESCAPE, OSC, OSC_ESCAPE }

    private var state = State.TEXT
    private val payload = StringBuilder()

    fun feed(text: String) {
        for (c in text) {
            state = when (state) {
                State.TEXT -> if (c == ESC) State.ESCAPE else State.TEXT
                State.ESCAPE -> when (c) {
                    ']' -> { payload.setLength(0); State.OSC }
                    ESC -> State.ESCAPE
                    else -> State.TEXT
                }
                State.OSC -> when (c) {
                    BEL -> { finish(); State.TEXT }
                    ESC -> State.OSC_ESCAPE
                    '\u009c' -> { finish(); State.TEXT }
                    else -> {
                        if (payload.length < MAX) payload.append(c)
                        State.OSC
                    }
                }
                State.OSC_ESCAPE -> if (c == '\\') { finish(); State.TEXT } else if (c == ']') { payload.setLength(0); State.OSC } else State.TEXT
            }
        }
    }

    private fun finish() {
        val text = payload.toString()
        payload.setLength(0)
        val separator = text.indexOf(';')
        val code = (if (separator < 0) text else text.substring(0, separator)).toIntOrNull() ?: return
        onOsc(code, if (separator < 0) "" else text.substring(separator + 1))
    }

    companion object {
        private const val ESC = '\u001b'
        private const val BEL = '\u0007'
        private const val MAX = 4096

        /** The absolute path of an OSC 7 `file://host/path` payload (percent-decoded), or null. */
        fun cwdOf(payload: String): String? {
            if (!payload.startsWith("file://")) return null
            val rest = payload.removePrefix("file://")
            val path = rest.substring(rest.indexOf('/').takeIf { it >= 0 } ?: return null)
            return runCatching { percentDecode(path) }.getOrNull()?.takeIf { it.startsWith("/") }
        }

        private fun percentDecode(value: String): String {
            val bytes = java.io.ByteArrayOutputStream()
            val plain = StringBuilder()
            fun flush() { if (plain.isNotEmpty()) { bytes.write(plain.toString().toByteArray(Charsets.UTF_8)); plain.setLength(0) } }
            var i = 0
            while (i < value.length) {
                val c = value[i]
                if (c == '%' && i + 2 < value.length) {
                    flush()
                    bytes.write(value.substring(i + 1, i + 3).toInt(16)); i += 3
                } else {
                    plain.append(c); i++
                }
            }
            flush()
            return bytes.toString(Charsets.UTF_8.name())
        }
    }
}
