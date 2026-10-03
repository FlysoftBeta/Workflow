package top.flysoftbeta.workflow.platform.engine

import java.util.Base64
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.flow
import top.flysoftbeta.workflow.core.connection.WorkspaceWire
import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.core.terminal.*

/** PTY allocation and job control happen in the Rust Server; Android only displays its byte stream. */
class EngineTerminalBackend(private val controller: EngineController) : TerminalBackend {
    override val paths = ShellPaths(Guest.WORKSPACE, Guest.HOME)
    override suspend fun start(spec: TerminalSpec): TerminalProcess {
        require(WorkspacePaths.normalizeOrNull(spec.directory) == spec.directory)
        controller.awaitUsable()
        val cwd = paths.toShell(spec.directory)
        val env = spec.env + mapOf("TERM" to "xterm-256color", "COLORTERM" to "truecolor",
            "PROMPT_COMMAND" to "printf '\\033]7;file://localhost%s\\007' \"\$PWD\"")
        val result = WorkspaceWire.obj(controller.rpc.request("process.spawn", mapOf(
            "argv" to (spec.argv ?: listOf(Guest.SHELL, "--login")), "cwd" to cwd, "env" to env,
            "terminal" to true, "rows" to spec.rows, "columns" to spec.columns, "label" to "terminal",
        ), 60_000))
        val id = WorkspaceWire.string(result, "processId")
        return object : TerminalProcess {
            override val output = flow {
                var offset = 0L
                while (currentCoroutineContext().isActive) {
                    val frame = WorkspaceWire.obj(controller.rpc.request("process.read", mapOf("processId" to id,
                        "stream" to "stdout", "offset" to offset, "maxBytes" to 65_536, "waitMs" to 1000)))
                    val bytes = Base64.getDecoder().decode(frame["data"] as? String ?: "")
                    offset = WorkspaceWire.long(frame, "nextOffset")
                    if (bytes.isNotEmpty()) emit(bytes)
                    if (frame["eof"] == true) break
                }
            }
            override suspend fun write(bytes: ByteArray) {
                var offset = 0
                while (offset < bytes.size) {
                    val end = minOf(offset + 65_536, bytes.size)
                    controller.rpc.request("process.write", mapOf("processId" to id,
                        "data" to Base64.getEncoder().encodeToString(bytes.copyOfRange(offset, end))))
                    offset = end
                }
            }
            override suspend fun resize(rows: Int, columns: Int) { controller.rpc.request("process.resize", mapOf("processId" to id, "rows" to rows, "columns" to columns)) }
            override suspend fun awaitExit(): Int {
                while (true) {
                    val value = WorkspaceWire.obj(controller.rpc.request("process.wait", mapOf("processId" to id, "timeoutMs" to 1000)))
                    if (value["running"] != true) return (value["exitCode"] as? Number)?.toInt() ?: 1
                }
            }
            override fun terminate(force: Boolean) { controller.scope.launch { runCatching { controller.rpc.request("process.stop", mapOf("processId" to id, "force" to force)) } } }
            override suspend fun currentDirectory(): String = cwd // TerminalHost prefers the live OSC 7 cwd.
        }
    }
}
