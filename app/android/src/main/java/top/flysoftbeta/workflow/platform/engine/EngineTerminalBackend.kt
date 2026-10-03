package top.flysoftbeta.workflow.platform.engine

import java.util.Base64
import kotlinx.serialization.json.*
import top.flysoftbeta.workflow.client.protocol.EngineMethods
import top.flysoftbeta.workflow.client.protocol.TerminalResolvePathsParams
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.transformWhile
import top.flysoftbeta.workflow.core.connection.WorkspaceWire
import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.core.terminal.*

/** Attaches UI byte streams to Engine-owned terminal resources; never owns their lifecycle. */
class EngineTerminalBackend(private val controller: EngineController) : ManagedTerminalBackend {
    override val paths = ShellPaths(Guest.WORKSPACE, Guest.HOME)

    override suspend fun create(spec: TerminalSpec): ManagedTerminalProcess {
        require(WorkspacePaths.normalizeOrNull(spec.directory) == spec.directory)
        require(spec.argv == null && spec.env.isEmpty()) { "Engine chooses the terminal shell and environment" }
        controller.awaitUsable()
        return Attachment(metadata(controller.rpc.request("terminal.create", mapOf(
            "directory" to spec.directory, "rows" to spec.rows, "columns" to spec.columns,
        ), 60_000)))
    }

    override suspend fun attach(id: String, rows: Int?, columns: Int?): ManagedTerminalProcess {
        val args = mutableMapOf<String, Any?>("terminalId" to id)
        rows?.let { args["rows"] = it }; columns?.let { args["columns"] = it }
        return Attachment(metadata(controller.rpc.request("terminal.attach", args, 60_000)))
    }

    override suspend fun resolvePaths(id: String, generation: Long, candidates: List<String>): List<TerminalPath> {
        require(candidates.size <= 128)
        val response = controller.rpc.request(EngineMethods.terminal_resolvePaths, TerminalResolvePathsParams.decode(buildJsonObject {
            put("terminalId", id); put("generation", generation)
            put("candidates", JsonArray(candidates.map(::JsonPrimitive)))
        }))
        check(response.terminalId == id && response.generation.longValueExact() == generation) { "Stale terminal path result" }
        check(response.paths.map { it.text } == candidates) { "Engine reordered terminal candidates" }
        return response.paths.map { TerminalPath(it.text, it.path, it.kind?.value == "directory", it.line?.intValueExact(), it.column?.intValueExact()) }
    }

    private inner class Attachment(override val initial: TerminalMetadata) : ManagedTerminalProcess {
        @Volatile private var latest = initial
        override val frames = flow {
            var generation = initial.generation
            var offset = 0L
            var first = true
            while (currentCoroutineContext().isActive) {
                val frame = WorkspaceWire.obj(controller.rpc.request("terminal.read", mapOf(
                    "terminalId" to initial.id, "generation" to generation, "offset" to offset,
                    "maxBytes" to 65_536, "waitMs" to 1000,
                )))
                val next = metadata(frame["terminal"])
                val bytes = Base64.getDecoder().decode(frame["data"] as? String ?: "")
                val start = WorkspaceWire.long(frame, "startOffset")
                val end = WorkspaceWire.long(frame, "nextOffset")
                check(bytes.size <= 65_536 && end == start + bytes.size) { "终端输出偏移无效" }
                val reset = first || frame["reset"] == true || next.generation != generation || start != offset
                generation = next.generation; offset = end; first = false
                latest = next
                val eof = frame["eof"] == true
                emit(TerminalFrame(next, bytes, reset, eof))
                if (eof) delay(250)
            }
        }
        @OptIn(kotlinx.coroutines.ExperimentalCoroutinesApi::class)
        override val output = frames.transformWhile { frame ->
            if (frame.bytes.isNotEmpty()) emit(frame.bytes)
            !frame.eof
        }
        override suspend fun write(bytes: ByteArray) {
            var offset = 0
            while (offset < bytes.size) {
                val end = minOf(offset + 65_536, bytes.size)
                controller.rpc.request("terminal.write", mapOf("terminalId" to initial.id,
                    "data" to Base64.getEncoder().encodeToString(bytes.copyOfRange(offset, end))))
                offset = end
            }
        }
        override suspend fun resize(rows: Int, columns: Int) {
            controller.rpc.request("terminal.resize", mapOf("terminalId" to initial.id, "rows" to rows, "columns" to columns))
        }
        override suspend fun awaitExit(): Int {
            while (true) {
                val value = WorkspaceWire.obj(controller.rpc.request("terminal.wait", mapOf("terminalId" to initial.id, "timeoutMs" to 1000)))
                if (value["running"] != true) return (value["exitCode"] as? Number)?.toInt() ?: 1
            }
        }
        override fun terminate(force: Boolean) {
            controller.scope.launch { runCatching { controller.rpc.request("terminal.stop", mapOf("terminalId" to initial.id, "force" to force)) } }
        }
        override suspend fun currentDirectory() = latest.cwd
        override suspend fun restart(rows: Int, columns: Int) = metadata(controller.rpc.request("terminal.restart",
            mapOf("terminalId" to initial.id, "rows" to rows, "columns" to columns), 60_000))
        override suspend fun rename(title: String?) = metadata(controller.rpc.request("terminal.rename",
            mapOf("terminalId" to initial.id, "title" to title)))
        override suspend fun clear() = metadata(controller.rpc.request("terminal.clear", mapOf("terminalId" to initial.id)))
    }

    private fun metadata(value: Any?): TerminalMetadata {
        val item = WorkspaceWire.obj(value)
        return TerminalMetadata(WorkspaceWire.string(item, "id"), (item["ordinal"] as Number).toInt(),
            WorkspaceWire.long(item, "generation"), WorkspaceWire.string(item, "cwd"), item["title"] as? String,
            item["customTitle"] as? String, WorkspaceWire.string(item, "status"), (item["exitCode"] as? Number)?.toInt(),
            item["error"] as? String, (item["rows"] as Number).toInt(), (item["columns"] as Number).toInt())
    }
}
