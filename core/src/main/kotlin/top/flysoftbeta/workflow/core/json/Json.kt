package top.flysoftbeta.workflow.core.json

/** Small strict JSON codec shared by disk state and declarative configuration, with no Android dependency. */
object Json {
    fun parse(text: String): Any? = Reader(text).read()

    /**
     * [pretty] writes 2-space indented output (every JSON file visible in the workspace uses it).
     * A [Map] value of [Omit] drops that key, so merged documents can remove optional known keys.
     */
    fun stringify(value: Any?, pretty: Boolean): String =
        if (!pretty) stringify(value) else StringBuilder().also { writePretty(it, value, 0) }.toString()

    private fun writePretty(out: StringBuilder, value: Any?, indent: Int) {
        fun newline(level: Int) { out.append('\n'); repeat(level) { out.append("  ") } }
        when (value) {
            is Map<*, *> -> {
                val entries = value.entries.filter { it.value !== Omit }
                if (entries.isEmpty()) { out.append("{}"); return }
                out.append('{')
                entries.forEachIndexed { index, (key, item) ->
                    require(key is String)
                    if (index > 0) out.append(',')
                    newline(indent + 1)
                    out.append(stringify(key)).append(": ")
                    writePretty(out, item, indent + 1)
                }
                newline(indent)
                out.append('}')
            }
            is Iterable<*> -> {
                val items = value.toList()
                if (items.isEmpty()) { out.append("[]"); return }
                // Short scalar arrays stay on one line.
                if (items.all { it == null || it is String || it is Number || it is Boolean } && items.sumOf { stringify(it).length + 2 } <= 80) {
                    out.append(items.joinToString(", ", "[", "]") { stringify(it) })
                    return
                }
                out.append('[')
                items.forEachIndexed { index, item ->
                    if (index > 0) out.append(',')
                    newline(indent + 1)
                    writePretty(out, item, indent + 1)
                }
                newline(indent)
                out.append(']')
            }
            else -> out.append(stringify(value))
        }
    }

    /** Marker value: the key is left out when the map is written. */
    object Omit

    fun stringify(value: Any?): String = when (value) {
        null -> "null"
        is String -> buildString {
            append('"')
            value.forEach { char -> when (char) {
                '"' -> append("\\\"")
                '\\' -> append("\\\\")
                '\n' -> append("\\n")
                '\r' -> append("\\r")
                '\t' -> append("\\t")
                '\b' -> append("\\b")
                '\u000C' -> append("\\f")
                else -> if (char.code < 32) append("\\u%04x".format(char.code)) else append(char)
            } }
            append('"')
        }
        is Boolean, is Int, is Long -> value.toString()
        is Number -> value.toDouble().also { require(it.isFinite()) }.toString()
        is Map<*, *> -> value.entries.filter { it.value !== Omit }.joinToString(",", "{", "}") { (key, item) ->
            require(key is String)
            stringify(key) + ":" + stringify(item)
        }
        is Iterable<*> -> value.joinToString(",", "[", "]") { stringify(it) }
        else -> error("Unsupported JSON value: ${value.javaClass.name}")
    }

    private class Reader(private val input: String) {
        private var offset = 0
        private var depth = 0
        fun read(): Any? {
            val result = value()
            whitespace()
            require(offset == input.length) { "Trailing JSON at $offset" }
            return result
        }
        private fun whitespace() { while (offset < input.length && input[offset] in " \t\r\n") offset++ }
        private fun value(): Any? {
            whitespace()
            require(offset < input.length) { "Unexpected end of JSON" }
            require(++depth <= 100) { "JSON nesting exceeds 100" }
            try {
                return when (input[offset]) {
                    '{' -> objectValue()
                    '[' -> arrayValue()
                    '"' -> stringValue()
                    't' -> literal("true", true)
                    'f' -> literal("false", false)
                    'n' -> literal("null", null)
                    else -> numberValue()
                }
            } finally { depth-- }
        }
        private fun take(char: Char): Boolean {
            whitespace()
            if (offset < input.length && input[offset] == char) { offset++; return true }
            return false
        }
        private fun expect(char: Char) { require(take(char)) { "Expected $char at $offset" } }
        private fun objectValue(): Map<String, Any?> {
            expect('{')
            val result = linkedMapOf<String, Any?>()
            if (take('}')) return result
            do {
                whitespace()
                val key = stringValue()
                require(!result.containsKey(key)) { "Duplicate JSON key: $key" }
                expect(':')
                result[key] = value()
            } while (take(','))
            expect('}')
            return result
        }
        private fun arrayValue(): List<Any?> {
            expect('[')
            val result = mutableListOf<Any?>()
            if (take(']')) return result
            do { result += value() } while (take(','))
            expect(']')
            return result
        }
        private fun stringValue(): String {
            expect('"')
            return buildString {
                while (offset < input.length) {
                    val char = input[offset++]
                    if (char == '"') return@buildString
                    require(char.code >= 32) { "Unescaped control character" }
                    if (char != '\\') append(char) else {
                        require(offset < input.length) { "Unfinished escape" }
                        when (val escape = input[offset++]) {
                            '"', '\\', '/' -> append(escape)
                            'b' -> append('\b')
                            'f' -> append('\u000C')
                            'n' -> append('\n')
                            'r' -> append('\r')
                            't' -> append('\t')
                            'u' -> {
                                require(offset + 4 <= input.length)
                                append(input.substring(offset, offset + 4).toInt(16).toChar())
                                offset += 4
                            }
                            else -> error("Invalid escape $escape")
                        }
                    }
                }
                error("Unterminated string")
            }
        }
        private fun literal(token: String, result: Any?): Any? {
            require(input.startsWith(token, offset)) { "Invalid value at $offset" }
            offset += token.length
            return result
        }
        private fun numberValue(): Number {
            val start = offset
            if (offset < input.length && input[offset] == '-') offset++
            require(offset < input.length && input[offset].isDigit())
            if (input[offset] == '0') offset++ else while (offset < input.length && input[offset].isDigit()) offset++
            if (offset < input.length && input[offset] == '.') {
                offset++
                require(offset < input.length && input[offset].isDigit())
                while (offset < input.length && input[offset].isDigit()) offset++
            }
            if (offset < input.length && input[offset] in "eE") {
                offset++
                if (offset < input.length && input[offset] in "+-") offset++
                require(offset < input.length && input[offset].isDigit())
                while (offset < input.length && input[offset].isDigit()) offset++
            }
            val token = input.substring(start, offset)
            return token.toLongOrNull() ?: token.toDouble().also { require(it.isFinite()) }
        }
    }
}

@Suppress("UNCHECKED_CAST")
internal fun Any?.jsonObject(): Map<String, Any?> = this as? Map<String, Any?> ?: error("Expected JSON object")
internal fun Map<String, Any?>.str(key: String, default: String = ""): String = this[key] as? String ?: default
internal fun Map<String, Any?>.long(key: String, default: Long = 0): Long = (this[key] as? Number)?.toLong() ?: default
internal fun Map<String, Any?>.bool(key: String, default: Boolean = false): Boolean = this[key] as? Boolean ?: default
internal fun Map<String, Any?>.list(key: String): List<Any?> = this[key] as? List<*> ?: emptyList()
internal inline fun <reified T : Enum<T>> Map<String, Any?>.enum(key: String, default: T): T =
    this[key]?.let { enumValueOf<T>(it as String) } ?: default
