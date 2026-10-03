package top.flysoftbeta.workflow.proxy.redact

import java.net.URLEncoder

/** Values from credentials and subscription URLs; never render their collection in diagnostics. */
class ProxySecrets private constructor(private val values: List<String>) {
    fun redact(text: String): String = values.fold(text) { output, value -> output.replace(value, REDACTED) }
    fun stream(): ProxyLogRedactor = ProxyLogRedactor(values)
    override fun toString(): String = "ProxySecrets([hidden])"

    companion object {
        private val keys = setOf("secret", "password", "passwd", "token", "uuid", "private-key", "client-key", "preshared-key", "authorization", "authentication")
        fun empty() = ProxySecrets(emptyList())
        fun fromConfig(root: Map<*, *>): ProxySecrets {
            val found = linkedSetOf<String>()
            val visited = java.util.IdentityHashMap<Any, Boolean>()
            fun visit(value: Any?, key: String = "") {
                if ((value is Map<*, *> || value is List<*>) && visited.put(value, true) != null) return
                when (value) {
                    is Map<*, *> -> value.forEach { (name, child) -> visit(child, name?.toString()?.lowercase().orEmpty()) }
                    is List<*> -> value.forEach { visit(it, key) }
                    is String -> if (value.isNotEmpty() && (key in keys || key.endsWith("-token") || key == "url")) {
                        found += value
                        found += URLEncoder.encode(value, "UTF-8")
                        // Diagnostics may print the URL's query separately from its host/path.
                        if (key == "url") runCatching {
                            val uri = java.net.URI(value)
                            uri.rawUserInfo?.takeIf { it.isNotEmpty() }?.let { found += it }
                            uri.rawQuery?.split('&')?.forEach { part ->
                                part.substringAfter('=', "").takeIf { it.isNotEmpty() }?.let {
                                    found += it
                                    found += java.net.URLDecoder.decode(it, "UTF-8")
                                }
                            }
                        }
                    }
                }
            }
            visit(root)
            return ProxySecrets(found.filter { it.isNotEmpty() }.sortedByDescending { it.length })
        }
    }
}

/** Avoid retaining a complete credential in logs or diagnostics, including across read chunks. */
class ProxyLogRedactor internal constructor(private val secrets: List<String>) {
    constructor(secret: String) : this(listOf(secret, URLEncoder.encode(secret, "UTF-8")).filter { it.isNotEmpty() }.distinct().sortedByDescending { it.length })
    private var pending = ""
    private val longest = secrets.maxOfOrNull { it.length } ?: 0

    fun append(chunk: String, finished: Boolean = false): String {
        if (secrets.isEmpty()) return chunk
        val combined = pending + chunk
        var boundary = if (finished) combined.length else (combined.length - longest + 1).coerceAtLeast(0)
        // A match crossing the output boundary must be emitted as one redacted unit. Extending for
        // one secret can expose another crossing match, so compute a fixed point before splitting.
        do {
            val before = boundary
            for (secret in secrets) {
                var search = 0
                while (search < boundary) {
                    val match = combined.indexOf(secret, search)
                    if (match < 0 || match >= boundary) break
                    boundary = maxOf(boundary, match + secret.length)
                    search = match + secret.length
                }
            }
        } while (boundary > before)
        val safe = secrets.fold(combined.substring(0, boundary)) { output, secret -> output.replace(secret, REDACTED) }
        pending = combined.substring(boundary)
        return safe
    }
}

const val REDACTED = "[已隐藏]"

/** Removes every occurrence of [secret] (and its percent-encoded form) from diagnostic text. */
fun redactSecret(text: String, secret: String): String {
    if (secret.isEmpty()) return text
    var result = text.replace(secret, REDACTED)
    val encoded = URLEncoder.encode(secret, "UTF-8")
    if (encoded != secret) result = result.replace(encoded, REDACTED)
    return result
}
