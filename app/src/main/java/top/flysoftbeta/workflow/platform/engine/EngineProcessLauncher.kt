package top.flysoftbeta.workflow.platform.engine

import java.io.InputStream
import java.io.OutputStream
import java.io.IOException
import java.util.Base64
import kotlinx.coroutines.*
import top.flysoftbeta.workflow.agent.process.AgentProcess
import top.flysoftbeta.workflow.agent.process.LaunchSpec
import top.flysoftbeta.workflow.agent.process.ProcessLauncher
import top.flysoftbeta.workflow.core.connection.WorkspaceRpc
import top.flysoftbeta.workflow.core.connection.WorkspaceWire

/** Vendor adapters use ordinary streams; the server owns the underlying guest process and its lifecycle. */
class EngineProcessLauncher(private val controller: EngineController) : ProcessLauncher {
    override suspend fun launch(spec: LaunchSpec): AgentProcess {
        controller.awaitUsable()
        val response = WorkspaceWire.obj(controller.rpc.request("process.spawn", mapOf(
            "argv" to spec.argv, "cwd" to spec.cwd, "env" to spec.env, "terminal" to false, "label" to spec.label,
        ), 60_000))
        return RpcAgentProcess(controller.rpc, WorkspaceWire.string(response, "processId"), controller.scope)
    }
}

internal class RpcAgentProcess(private val rpc: WorkspaceRpc, val processId: String, private val scope: CoroutineScope) : AgentProcess {
    @Volatile private var running = true
    override val pid: Long? = null // Server process ids are opaque and never interpreted as client OS pids.
    override val isAlive get() = running
    override val stdout: InputStream = RpcInput("stdout")
    override val stderr: InputStream = RpcInput("stderr")
    override val stdin: OutputStream = object : OutputStream() {
        override fun write(b: Int) = write(byteArrayOf(b.toByte()))
        @Synchronized override fun write(b: ByteArray, off: Int, len: Int) {
            var offset = off
            while (offset < off + len) {
                val count = minOf(65_536, off + len - offset)
                val data = Base64.getEncoder().encodeToString(b.copyOfRange(offset, offset + count))
                runBlocking(Dispatchers.IO) { rpc.request("process.write", mapOf("processId" to processId, "data" to data)) }
                offset += count
            }
        }
    }
    private inner class RpcInput(private val stream: String) : InputStream() {
        private var offset = 0L
        private var buffered = ByteArray(0)
        private var position = 0
        private var ended = false
        override fun read(): Int { val one = ByteArray(1); return if (read(one) < 0) -1 else one[0].toInt() and 255 }
        @Synchronized override fun read(b: ByteArray, off: Int, len: Int): Int {
            if (len == 0) return 0
            while (position >= buffered.size) {
                if (ended) return -1
                val result = runBlocking(Dispatchers.IO) { WorkspaceWire.obj(rpc.request("process.read", mapOf(
                    "processId" to processId, "stream" to stream, "offset" to offset, "maxBytes" to 65_536, "waitMs" to 1000,
                ))) }
                val start = (result["startOffset"] as? Number)?.toLong() ?: offset
                if (start > offset) throw IOException("代理输出超出保留窗口，连接需要恢复")
                buffered = Base64.getDecoder().decode(result["data"] as? String ?: "")
                position = 0
                offset = WorkspaceWire.long(result, "nextOffset")
                ended = result["eof"] == true
                if (result["exitCode"] != null) running = false
            }
            val count = minOf(len, buffered.size - position)
            buffered.copyInto(b, off, position, position + count); position += count
            return count
        }
        override fun close() { ended = true }
    }
    override suspend fun awaitExit(): Int {
        while (true) {
            val result = WorkspaceWire.obj(rpc.request("process.wait", mapOf("processId" to processId, "timeoutMs" to 1000)))
            if (result["running"] != true) { running = false; return (result["exitCode"] as? Number)?.toInt() ?: 1 }
        }
    }
    override fun kill(force: Boolean) { scope.launch { runCatching { rpc.request("process.stop", mapOf("processId" to processId, "force" to force)) }; running = false } }
}
