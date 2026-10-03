package top.flysoftbeta.workflow.core.io

/**
 * Workspace-relative paths: '/'-separated, no leading '/', no "." or ".." segments; "" is the
 * workspace root. Every path that crosses a module boundary uses this form.
 */
object WorkspacePaths {
    /** App-internal directory; hidden in the explorer and masked inside the environment. */
    const val INTERNAL = ".workspace"
    const val STATE = ".workspace/state"
    const val PROXY = ".workspace/services/proxy"
    const val TMP = ".workspace/tmp"
    const val CONFIG = ".workspace/config.json"
    const val ENVIRONMENT = ".workspace/env.json"

    /** Returns the normalized form, or throws [IllegalArgumentException] for absolute or escaping paths. */
    fun normalize(raw: String): String {
        require(!raw.startsWith("/")) { "Workspace paths must be relative" }
        require('\u0000' !in raw) { "Invalid path" }
        val parts = raw.split('/').filter { it.isNotEmpty() && it != "." }
        require(parts.none { it == ".." }) { "Path is outside the workspace" }
        return parts.joinToString("/")
    }

    fun normalizeOrNull(raw: String): String? = runCatching { normalize(raw) }.getOrNull()

    /** A normalized path that names a file or directory below the root (not the root itself). */
    fun isValidEntry(path: String): Boolean = path.isNotEmpty() && normalizeOrNull(path) == path

    fun parent(path: String): String = path.substringBeforeLast('/', "")
    fun name(path: String): String = path.substringAfterLast('/')
    fun child(directory: String, name: String): String {
        require(name.isNotEmpty() && '/' !in name && name != "." && name != "..") { "Invalid file name" }
        return if (directory.isEmpty()) name else "$directory/$name"
    }

    /** True when [path] equals [directory] or lies below it. The root contains everything. */
    fun isWithin(path: String, directory: String): Boolean =
        directory.isEmpty() || path == directory || path.startsWith("$directory/")

    /** Rewrites [path] after [from] was renamed to [to]; null when [path] is not affected. */
    fun rebase(path: String, from: String, to: String): String? = when {
        path == from -> to
        from.isNotEmpty() && path.startsWith("$from/") -> to + path.substring(from.length)
        else -> null
    }

    /**
     * App state is never a user file. The proxy directory is internal but can be
     * opened in the editor through the proxy app's explicit "edit configuration" action.
     */
    fun isReserved(path: String): Boolean =
        (isWithin(path, INTERNAL) && !isWithin(path, PROXY) && path != CONFIG && path != ENVIRONMENT) || path.isEmpty()

    /** Hidden in the explorer even when "show hidden files" is on. */
    fun isHiddenInExplorer(path: String): Boolean = isWithin(path, INTERNAL)
}
