package top.flysoftbeta.workflow.agent.testing

import java.io.File
import java.util.concurrent.atomic.AtomicInteger
import kotlinx.serialization.json.*
import top.flysoftbeta.workflow.agent.model.*
import top.flysoftbeta.workflow.agent.rpc.ChatWire

/** Opt-in frozen oracle export; normal tests never rewrite checked-in data. */
object RustParity {
    private val serial = AtomicInteger()
    fun record(before: AgentState, events: List<AgentEvent>, after: AgentState) {
        val root = System.getenv("WORKFLOW_RUST_PARITY_DIR") ?: return
        val caller = Thread.currentThread().stackTrace.firstOrNull { it.className.endsWith("AgentReducerTest") && it.methodName != "fold" && !it.methodName.startsWith("fold$") }
        val name = caller?.methodName ?: "reducer"
        val file = File(root, "$name-${serial.incrementAndGet()}.json")
        file.parentFile.mkdirs()
        file.writeText(buildJsonObject {
            put("source", "Kotlin AgentReducerTest / ChatWire")
            put("case", name)
            put("before", ChatWire.encode(before))
            put("events", ChatWire.encode(events))
            put("after", ChatWire.encode(after))
        }.toString())
    }
}
