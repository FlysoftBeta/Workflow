package top.flysoftbeta.workflow.core.terminal

import top.flysoftbeta.workflow.core.io.WorkspacePaths

/** A `path[:line[:col]]` reference printed in a terminal; [line] and [column] are 1-based. */
data class FileReference(val path: String, val line: Int? = null, val column: Int? = null)

/** A resolved local link: a workspace path, the 0-based cursor, and whether it is (believed to be) a folder. */
data class ResolvedFileLink(val path: String, val line: Int? = null, val column: Int? = null)

/**
 * Terminal link text (docs/ui.md §4.4): URLs go to the system browser; local paths, absolute or relative to
 * the shell's current directory, optionally with `:line[:col]` (also `(line,col)`), open in the editor.
 * Detection of candidates happens in the terminal page; this resolves one candidate.
 */
object TerminalLinks {
    private val URL = Regex("^(?:https?|ftp)://[^\\s]+$", RegexOption.IGNORE_CASE)
    private val COLON_SUFFIX = Regex("^(.+?):(\\d+)(?::(\\d+))?:?$")
    private val PAREN_SUFFIX = Regex("^(.+?)\\((\\d+)(?:,\\s*(\\d+))?\\)$")
    private const val LEADING = "(['\"<`["
    private const val TRAILING = ")]'\">`,;."

    fun isUrl(text: String): Boolean = URL.matches(text.trim())

    /** Strips surrounding quotes/brackets and trailing punctuation, then splits a line/column suffix. */
    fun parse(raw: String): FileReference? {
        var text = raw.trim().trimStart { it in LEADING }
        // Trailing punctuation, but keep a closing ')' that belongs to a "(line,col)" suffix.
        while (text.isNotEmpty() && text.last() in TRAILING && !(text.last() == ')' && PAREN_SUFFIX.matches(text))) text = text.dropLast(1)
        if (text.startsWith("file://")) text = text.removePrefix("file://").let { if (it.startsWith("/")) it else it.substringAfter('/', "").let { p -> "/$p" } }
        if (text.isEmpty() || isUrl(text)) return null
        PAREN_SUFFIX.matchEntire(text)?.let { m ->
            return FileReference(m.groupValues[1], m.groupValues[2].toIntOrNull(), m.groupValues[3].toIntOrNull())
        }
        COLON_SUFFIX.matchEntire(text)?.let { m ->
            return FileReference(m.groupValues[1], m.groupValues[2].toIntOrNull(), m.groupValues[3].toIntOrNull())
        }
        return FileReference(text)
    }

    /**
     * Workspace paths [reference] may name, most likely first: `~` expands to the shell's home, relative
     * paths resolve against [cwd] (the shell's current directory, absolute in the shell's namespace; the
     * workspace root when unknown), and a git diff prefix (`a/`, `b/`) is also tried. Paths outside the
     * workspace or app-internal ones are dropped. The caller opens the first that exists.
     */
    fun candidates(reference: FileReference, cwd: String?, paths: ShellPaths): List<String> {
        val text = reference.path
        val base = cwd?.takeIf { it.startsWith("/") } ?: paths.workspaceRoot
        val absolutes = buildList {
            when {
                text == "~" || text.startsWith("~/") -> paths.home?.let { add(it.trimEnd('/') + text.removePrefix("~")) }
                text.startsWith("/") -> add(text)
                else -> {
                    add("$base/$text")
                    if (text.startsWith("a/") || text.startsWith("b/")) add("$base/${text.substring(2)}")
                    if (base != paths.workspaceRoot) add("${paths.workspaceRoot}/$text")
                }
            }
        }
        return absolutes.mapNotNull { paths.toWorkspace(it) }
            .filter { it.isNotEmpty() && WorkspacePaths.normalizeOrNull(it) == it && !WorkspacePaths.isReserved(it) }
            .distinct()
    }

    /** 0-based cursor for a 1-based reference (line/column 0 or missing are treated as 1). */
    fun cursor(reference: FileReference): Pair<Int, Int>? {
        val line = reference.line ?: return null
        return (line - 1).coerceAtLeast(0) to ((reference.column ?: 1) - 1).coerceAtLeast(0)
    }
}

/** Quoting of paths pasted into a POSIX shell (drag and drop, "在终端中粘贴路径"). */
object ShellQuote {
    private val SAFE = Regex("^[A-Za-z0-9_./+@%=:,-]+$")

    /** [value] unchanged when it has only safe characters, else single-quoted with `'\''` for quotes. */
    fun quote(value: String): String = when {
        value.isEmpty() -> "''"
        SAFE.matches(value) && !value.startsWith("-") && !value.startsWith("~") -> value
        else -> "'" + value.replace("'", "'\\''") + "'"
    }

    /**
     * The text pasted for dropped workspace paths: each as the shell sees it, relative to [cwd] when it lies
     * below it (else absolute), quoted, separated and followed by one space.
     */
    fun pasteText(workspacePaths: List<String>, cwd: String?, paths: ShellPaths): String =
        workspacePaths.joinToString(" ", postfix = " ") { quote(shellPath(it, cwd, paths)) }

    fun shellPath(workspacePath: String, cwd: String?, paths: ShellPaths): String {
        val absolute = paths.toShell(workspacePath)
        val base = cwd?.let { ShellPathSyntax.normalizeAbsolute(it) }?.trimEnd('/') ?: return absolute
        return when {
            absolute == base -> "."
            base.isEmpty() -> absolute
            absolute.startsWith("$base/") -> absolute.substring(base.length + 1)
            else -> absolute
        }
    }
}
