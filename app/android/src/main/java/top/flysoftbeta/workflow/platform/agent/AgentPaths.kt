package top.flysoftbeta.workflow.platform.agent

import top.flysoftbeta.workflow.core.io.WorkspacePaths

/**
 * Translates workspace-relative file paths to the sole production agent root, inside the container.
 * Agent homes are visible workspace configuration below `.workspace/agents/<id>`, but agents see them
 * only at their guest homes (Environment masks that tree below `/workspace`), so both directions map
 * between the two. The Engine's FileWork allowlist still decides what may be opened or attached.
 */
class AgentPaths(agentRoot: String = GUEST_ROOT) {
    init { require(agentRoot.trimEnd('/') == GUEST_ROOT) { "Agents require the container workspace" } }
    val root: String = GUEST_ROOT
    private val roots = listOf(root)

    /** Agent-visible path of a workspace-relative path ("" = the root). */
    fun toAgent(relative: String): String {
        val clean = WorkspacePaths.normalize(relative)
        HOMES.entries.firstOrNull { (_, visible) -> WorkspacePaths.isWithin(clean, visible) }?.let { (guest, visible) ->
            return guest + clean.removePrefix(visible)
        }
        return if (clean.isEmpty()) root else "$root/$clean"
    }

    /**
     * Workspace-relative path of an agent path, or null when it is outside the workspace or names
     * app-internal state. Relative inputs are taken relative to the workspace root (the agent's cwd).
     */
    fun toWorkspace(agentPath: String): String? {
        var path = agentPath.trim()
        if (path.startsWith("file://")) path = path.removePrefix("file://")
        path = percentDecode(path)
        if (path.startsWith("/")) {
            HOMES.entries.firstOrNull { (guest, _) -> path == guest || path.startsWith("$guest/") }?.let { (guest, visible) ->
                return WorkspacePaths.normalizeOrNull(path.removePrefix(guest).trimStart('/'))?.let { rest ->
                    if (rest.isEmpty()) visible else "$visible/$rest"
                }
            }
        }
        val relative = if (path.startsWith("/")) {
            val base = roots.firstOrNull { path == it || path.startsWith("$it/") } ?: return null
            path.removePrefix(base).trimStart('/')
        } else path.removePrefix("./")
        val clean = WorkspacePaths.normalizeOrNull(relative) ?: return null
        if (clean == WorkspacePaths.INTERNAL || clean.startsWith(WorkspacePaths.INTERNAL + "/")) return null
        return clean
    }

    /** Prefixes the transcript page treats as the workspace root (for images). */
    val prefixes: List<String> get() = roots.map { "$it/" }

    /** A link target from rendered Markdown: path plus optional `:line[:col]`, `#L12` or `#L12C3`. */
    data class Link(val path: String, val line: Int?, val column: Int?)

    fun parseLink(raw: String, line: Int? = null, column: Int? = null): Link? {
        var path = raw.trim()
        if (path.isEmpty() || path.length > 4096) return null
        var l = line
        var c = column
        val hash = HASH_LINE.find(path)
        if (hash != null) {
            l = l ?: hash.groupValues[1].toIntOrNull()
            c = c ?: hash.groupValues[2].toIntOrNull()
            path = path.substring(0, hash.range.first)
        } else if ('#' in path) {
            path = path.substringBefore('#')
        }
        val suffix = COLON_LINE.find(path)
        if (suffix != null) {
            l = l ?: suffix.groupValues[1].toIntOrNull()
            c = c ?: suffix.groupValues[2].toIntOrNull()
            path = path.substring(0, suffix.range.first)
        }
        val relative = toWorkspace(path) ?: return null
        if (relative.isEmpty()) return null
        return Link(relative, l?.takeIf { it > 0 }, c?.takeIf { it > 0 })
    }

    private fun percentDecode(value: String): String =
        if ('%' !in value) value else runCatching { java.net.URLDecoder.decode(value.replace("+", "%2B"), "UTF-8") }.getOrDefault(value)

    companion object {
        const val GUEST_ROOT = "/workspace"
        /** Guest agent homes and their visible workspace paths, matching Environment's agent homes. */
        private val HOMES = mapOf(
            "/home/work/.codex" to ".workspace/agents/codex",
            "/home/work/.claude" to ".workspace/agents/claude",
        )
        private val HASH_LINE = Regex("#L(\\d+)(?:C(\\d+))?(?:-L?\\d+)?$")
        private val COLON_LINE = Regex(":(\\d+)(?::(\\d+))?$")
    }
}
