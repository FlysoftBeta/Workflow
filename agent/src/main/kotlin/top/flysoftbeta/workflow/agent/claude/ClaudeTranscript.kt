package top.flysoftbeta.workflow.agent.claude

import kotlinx.serialization.json.JsonObject
import top.flysoftbeta.workflow.agent.json.arr
import top.flysoftbeta.workflow.agent.json.bool
import top.flysoftbeta.workflow.agent.json.get
import top.flysoftbeta.workflow.agent.json.parseJson
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.model.AgentEvent
import top.flysoftbeta.workflow.agent.model.AgentReducer
import top.flysoftbeta.workflow.agent.model.AgentState
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.MarkerItem
import top.flysoftbeta.workflow.agent.model.MarkerKind
import top.flysoftbeta.workflow.agent.model.ThreadKey
import top.flysoftbeta.workflow.agent.model.Turn
import top.flysoftbeta.workflow.agent.model.TurnStatus

/** Reads a session transcript (`$CLAUDE_CONFIG_DIR/projects/<cwd>/<session>.jsonl`) by environment path. */
fun interface ClaudeTranscriptSource {
    suspend fun read(path: String): List<String>?
}

/**
 * History hydration from a Claude transcript. Tolerant: unknown entry types (`attachment`,
 * `queue-operation`, `atis-latch`, `cost-state`, …) and malformed lines are skipped; subagent
 * side chains are not part of the main history. Produces the same items as live streaming by
 * reusing [ClaudeMapper] and [AgentReducer].
 */
object ClaudeTranscript {
    data class History(val turns: List<Turn>, val title: String?)

    fun events(sessionId: String, lines: List<String>): List<AgentEvent> {
        val mapper = ClaudeMapper(sessionId)
        val out = ArrayList<AgentEvent>()
        var turn: String? = null
        var interrupted = false
        fun close() {
            turn?.let { out += mapper.endTurn(it, if (interrupted) TurnStatus.INTERRUPTED else TurnStatus.COMPLETED) }
            turn = null
            interrupted = false
        }
        for (line in lines) {
            val entry = runCatching { parseJson(line) as? JsonObject }.getOrNull() ?: continue
            if (entry["isSidechain"].bool == true) continue
            when (entry["type"].str) {
                "user" -> {
                    val text = entry["message"]["content"].str
                    if (entry["isMeta"].bool != true && text != null && (text.contains("<command-name>") || text.contains("<local-command-"))) close()
                    if (isPrompt(entry)) {
                        close()
                        val id = entry["uuid"].str ?: continue
                        turn = id
                        out += mapper.beginTurn(id)
                    }
                    val mapped = mapper.user(sessionId, entry, live = false)
                    if (mapped.any { it is AgentEvent.ItemCompleted && (it.item as? MarkerItem)?.kind == MarkerKind.INTERRUPTED }) interrupted = true
                    out += mapped
                }
                "assistant" -> {
                    if (entry["isAbortedMidStream"].bool == true) interrupted = true
                    out += mapper.map(entry)
                }
                "custom-title" -> entry["customTitle"].str?.let { out += AgentEvent.ThreadRenamed(BackendKind.CLAUDE, sessionId, it) }
                "system" -> out += mapper.map(entry).filterNot { it is AgentEvent.Unknown }
                else -> Unit
            }
        }
        close()
        return out
    }

    fun history(sessionId: String, lines: List<String>): History {
        val state = AgentReducer.reduceAll(AgentState(), events(sessionId, lines))
        val thread = state.threads[ThreadKey(BackendKind.CLAUDE, sessionId)]
        return History(thread?.turns ?: emptyList(), thread?.title)
    }

    /** Last message uuid of [turnId] (fork anchor for `--resume-session-at`). */
    fun lastUuid(lines: List<String>, turnId: String): String? {
        var inTurn = false
        var last: String? = null
        for (line in lines) {
            val entry = runCatching { parseJson(line) as? JsonObject }.getOrNull() ?: continue
            if (entry["isSidechain"].bool == true) continue
            val type = entry["type"].str
            if (type == "user" && isPrompt(entry)) {
                if (inTurn) return last
                inTurn = entry["uuid"].str == turnId
            }
            if (inTurn && (type == "user" || type == "assistant")) last = entry["uuid"].str ?: last
        }
        return if (inTurn) last else null
    }

    /** A real prompt: a user entry that is not meta, not a tool result, not a local command, not an interrupt marker. */
    fun isPrompt(entry: JsonObject): Boolean {
        if (entry["isMeta"].bool == true || entry["parent_tool_use_id"].str != null) return false
        val content = entry["message"]["content"]
        content.str?.let { text ->
            return !text.contains("<local-command-") && !text.contains("<command-name>")
        }
        val blocks = content.arr ?: return false
        if (blocks.any { it["type"].str == "tool_result" }) return false
        val text = blocks.mapNotNull { it["text"].str }.joinToString("\n")
        return !text.startsWith("[Request interrupted by user")
    }
}
