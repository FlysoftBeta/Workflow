package top.flysoftbeta.workflow.agent.codex

import kotlinx.coroutines.*
import kotlinx.serialization.json.*
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.agent.AgentStateStore
import top.flysoftbeta.workflow.agent.json.get
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.model.*
import top.flysoftbeta.workflow.agent.testing.*

class CodexQueueTest {
    @Test fun queueNotificationsFetchPagesAndAllowCancellingExternalSubmissions() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        val state = AgentStateStore()
        val launcher = FakeLauncher { _, spec -> FakeProcess(spec) { process, frame ->
            if (frame["id"] == null) return@FakeProcess
            val result = when (frame["method"].str) {
                "thread/queue/list" -> if (frame["params"]["cursor"].str == null) {
                    Json.parseToJsonElement("""{"data":[{"id":"q1","clientUserMessageId":"c1","input":[{"type":"text","text":"first"}]}],"nextCursor":"page2"}""")
                } else {
                    Json.parseToJsonElement("""{"data":[{"id":"q2","clientUserMessageId":"c2","input":[{"type":"text","text":"second"}]}]}""")
                }
                else -> JsonObject(emptyMap())
            }
            process.emit(buildJsonObject { put("id", frame["id"]!!); put("result", result) })
        } }
        val backend = CodexBackend(launcher, CodexConfig("/guest/codex", "/home/work/.codex"), state, scope)
        try {
            backend.start()
            val child = launcher.launched.single()
            child.emit(buildJsonObject {
                put("method", "thread/queue/changed")
                put("params", buildJsonObject { put("threadId", "th") })
            })
            val key = ThreadKey(BackendKind.CODEX, "th")
            state.await(what = "queue pages") { it.thread(key)?.queuedTurns?.size == 2 }
            assertEquals(listOf("c1", "c2"), state.state.value.thread(key)!!.queuedTurns.map { it.clientMessageId })
            backend.cancelQueued("th", "c2")
            val deletion = child.written.single { it["method"].str == "thread/queue/delete" }
            assertEquals("q2", deletion["params"]["queuedSubmissionId"].str)
            assertEquals(TurnStatus.CANCELLED, state.state.value.thread(key)!!.turn("c2")!!.status)
        } finally { backend.stop(); scope.cancel() }
    }
}
