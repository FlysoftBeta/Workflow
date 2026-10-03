package top.flysoftbeta.workflow.core.terminal

import kotlinx.coroutines.flow.Flow

/**
 * What to run in a terminal. [directory] is workspace-relative ("" = the workspace root); the backend
 * maps it into its own namespace (`/workspace/...` for Engine terminals). [argv] null means the backend's interactive login shell. [env] is added on top of
 * the backend's own terminal environment (TERM, HOME, PATH, LANG are the backend's business).
 */
data class TerminalSpec(
    val directory: String = "",
    val rows: Int = 24,
    val columns: Int = 80,
    val argv: List<String>? = null,
    val env: Map<String, String> = emptyMap(),
) {
    init {
        require(rows in 1..1000 && columns in 1..1000) { "Invalid terminal size" }
        require(argv == null || argv.isNotEmpty()) { "argv must not be empty" }
    }
}

/**
 * A process attached to a pseudo terminal. Implementations are thread-safe; suspend functions never
 * block the caller's thread.
 */
interface TerminalProcess {
    /**
     * Raw PTY output. Collect it exactly once, from the moment the process starts: reading is what
     * drains the PTY. Completes after the process exited and its remaining output was read.
     */
    val output: Flow<ByteArray>

    /** Writes input bytes in call order. Fails once the process has exited. */
    suspend fun write(bytes: ByteArray)

    suspend fun resize(rows: Int, columns: Int)

    /** Waits for the exit status (0–255; 128+N for signal N). */
    suspend fun awaitExit(): Int

    /** SIGHUP/SIGTERM to the process group; [force] kills. Idempotent. */
    fun terminate(force: Boolean = false)

    /**
     * The shell's current directory in the backend's namespace (for relative links), or null when the
     * backend cannot tell. Managed resources return Engine-measured metadata.
     */
    suspend fun currentDirectory(): String?
}

/**
 * Port for terminal streams. Production uses Engine-owned managed resources; test fixtures may
 * supply an isolated process implementation.
 */
interface TerminalBackend {
    /** How paths printed by the shell map to the workspace (links, pasted paths). */
    val paths: ShellPaths

    suspend fun start(spec: TerminalSpec): TerminalProcess
}

/**
 * The shell's view of the workspace: where the workspace root and the home directory are, as absolute
 * paths the shell prints and accepts. Android shell: the app's real `files/.workspace`; Debian:
 * `/workspace` and `/home/work`.
 */
data class ShellPaths(
    /** Absolute shell path of the workspace root, without trailing '/'. */
    val workspaceRoot: String,
    /** Absolute shell path of `~`, or null when unknown. */
    val home: String?,
) {
    init { require(workspaceRoot.startsWith("/")) { "workspaceRoot must be absolute" } }

    /** Workspace-relative path of an absolute shell path, or null when it lies outside the workspace. */
    fun toWorkspace(shellPath: String): String? {
        val clean = ShellPathSyntax.normalizeAbsolute(shellPath) ?: return null
        val root = workspaceRoot.trimEnd('/').ifEmpty { "/" }
        return when {
            clean == root -> ""
            root == "/" -> clean.removePrefix("/")
            clean.startsWith("$root/") -> clean.substring(root.length + 1)
            else -> null
        }
    }

    /** Absolute shell path of a workspace-relative path. */
    fun toShell(workspacePath: String): String =
        if (workspacePath.isEmpty()) workspaceRoot else "${workspaceRoot.trimEnd('/')}/$workspacePath"
}

internal object ShellPathSyntax {
    /** Resolves "." and ".." in an absolute path; null when it is not absolute. Never climbs above "/". */
    fun normalizeAbsolute(path: String): String? {
        if (!path.startsWith("/")) return null
        val parts = ArrayList<String>()
        for (segment in path.split('/')) {
            when (segment) {
                "", "." -> Unit
                ".." -> if (parts.isNotEmpty()) parts.removeAt(parts.size - 1)
                else -> parts += segment
            }
        }
        return "/" + parts.joinToString("/")
    }
}

/** Engine terminal resources survive UI attachments; metadata is always server-authoritative. */
data class TerminalMetadata(
    val id: String,
    val ordinal: Int,
    val generation: Long,
    val cwd: String,
    val title: String?,
    val customTitle: String?,
    val status: String,
    val exitCode: Int?,
    val error: String?,
    val rows: Int,
    val columns: Int,
)

/** A metadata/output frame is applied atomically by the presentation adapter. */
data class TerminalFrame(val terminal: TerminalMetadata, val bytes: ByteArray, val reset: Boolean, val eof: Boolean)

interface ManagedTerminalProcess : TerminalProcess {
    val initial: TerminalMetadata
    /** Continues after EOF so another client's restart or rename remains observable. */
    val frames: Flow<TerminalFrame>
    suspend fun restart(rows: Int, columns: Int): TerminalMetadata
    suspend fun rename(title: String?): TerminalMetadata
    suspend fun clear(): TerminalMetadata
}

/** Engine-verified workspace target; cursor positions are zero-based. */
data class TerminalPath(val text: String, val path: String? = null, val isDirectory: Boolean = false, val line: Int? = null, val column: Int? = null)

interface ManagedTerminalBackend : TerminalBackend {
    /** A fixture may reject links; production must resolve against Engine metadata, never cached paths. */
    suspend fun resolvePaths(id: String, generation: Long, candidates: List<String>): List<TerminalPath> = candidates.map { TerminalPath(it) }
    suspend fun create(spec: TerminalSpec): ManagedTerminalProcess
    suspend fun attach(id: String, rows: Int? = null, columns: Int? = null): ManagedTerminalProcess
    override suspend fun start(spec: TerminalSpec): TerminalProcess = create(spec)
}
