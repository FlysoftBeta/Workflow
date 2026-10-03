package top.flysoftbeta.workflow.core.io

/**
 * Rules for single path segments (file and folder names) used by the explorer (create, rename,
 * duplicate), the importer (camera, gallery, device files, dropped content) and trash restore.
 */
object FileNames {
    /** Linux NAME_MAX, in UTF-8 bytes. */
    const val MAX_BYTES = 255

    /** Multi-part extensions kept together when a collision suffix is inserted ("a (1).tar.gz"). */
    private val COMPOUND = listOf(".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst", ".d.ts")

    /**
     * Why [name] cannot be used as a name in [directory] (short, user-facing), or null when valid.
     * App-internal names are refused at the workspace root.
     */
    fun problem(name: String, directory: String = "x"): String? = when {
        name.isEmpty() -> "名称不能为空"
        name.isBlank() -> "名称不能只有空格"
        name == "." || name == ".." -> "名称无效"
        '/' in name -> "不能包含 /"
        name.any { it == '\u0000' } -> "名称无效"
        name.toByteArray(Charsets.UTF_8).size > MAX_BYTES -> "名称过长"
        directory.isEmpty() && name == WorkspacePaths.INTERNAL -> "该名称由应用保留"
        else -> null
    }

    /**
     * A safe name from an untrusted display name (content providers, drag and drop): path separators
     * and control characters become `_`, surrounding whitespace is dropped, "." / ".." / empty fall back
     * to [fallback], and the result is shortened to [MAX_BYTES] keeping the extension.
     */
    fun sanitize(raw: String?, fallback: String): String {
        val cleaned = raw.orEmpty()
            .substringAfterLast('/')
            .map { c -> if (c == '\\' || c.code < 0x20 || c.code == 0x7f) '_' else c }
            .joinToString("")
            .trim()
        val name = if (cleaned.isEmpty() || cleaned == "." || cleaned == ".." || cleaned.all { it == '.' }) fallback else cleaned
        return truncate(name)
    }

    /** Splits "report.final.pdf" into ("report.final", ".pdf"); dot files and names without a dot have no extension. */
    fun splitExtension(name: String): Pair<String, String> {
        val lower = name.lowercase()
        COMPOUND.firstOrNull { lower.endsWith(it) && name.length > it.length }?.let {
            return name.dropLast(it.length) to name.takeLast(it.length)
        }
        val dot = name.lastIndexOf('.')
        return if (dot <= 0 || dot == name.length - 1) name to "" else name.substring(0, dot) to name.substring(dot)
    }

    /** The collision variants of [name]: "a.txt", "a (1).txt", "a (2).txt", … */
    fun variants(name: String): Sequence<String> {
        val (base, extension) = splitExtension(name)
        return sequenceOf(name) + generateSequence(1) { it + 1 }.map { truncate("$base ($it)$extension") }
    }

    /** The first variant of [name] not in [existing] (compared exactly; the workspace file system is case-sensitive). */
    fun unique(name: String, existing: Set<String>): String = variants(name).first { it !in existing }

    /** Shortens [name] to [MAX_BYTES] UTF-8 bytes, keeping the extension and never splitting a surrogate pair. */
    fun truncate(name: String): String {
        if (name.toByteArray(Charsets.UTF_8).size <= MAX_BYTES) return name
        val (base, extension) = splitExtension(name)
        val keepExtension = extension.takeIf { it.toByteArray(Charsets.UTF_8).size <= 32 }.orEmpty()
        val budget = MAX_BYTES - keepExtension.toByteArray(Charsets.UTF_8).size
        val source = if (keepExtension.isEmpty()) name else base
        val out = StringBuilder()
        var bytes = 0
        var i = 0
        while (i < source.length) {
            val codePoint = source.codePointAt(i)
            val chunk = String(Character.toChars(codePoint))
            val size = chunk.toByteArray(Charsets.UTF_8).size
            if (bytes + size > budget) break
            out.append(chunk)
            bytes += size
            i += Character.charCount(codePoint)
        }
        return out.toString() + keepExtension
    }
}
