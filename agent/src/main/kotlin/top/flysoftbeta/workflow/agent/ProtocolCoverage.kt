package top.flysoftbeta.workflow.agent

import top.flysoftbeta.workflow.agent.codex.CodexEvents
import top.flysoftbeta.workflow.agent.codex.CodexProtocol
import top.flysoftbeta.workflow.agent.codex.CodexProtocol.ClientRequest as C
import top.flysoftbeta.workflow.agent.codex.CodexProtocol.ServerRequest as R

/** How the agent layer handles one protocol method / message kind. */
enum class Coverage(val label: String) {
    /** Mapped to neutral model events (rendered natively). */
    MODELED("model"),
    /** Sent by the adapter as part of a neutral operation. */
    USED("adapter"),
    /** Consumed by the transport/adapter itself (correlation, refresh). */
    HANDLED("transport"),
    /** Server request shown to the user as a card; answered only by the user. */
    CARD("card"),
    /** Server request answered automatically with a factual/negative result (never consent). */
    AUTO("auto"),
    /** Server request answered with a protocol error (unsupported). */
    REJECTED("reject"),
    /** Must not be answered (Claude undeclared dialog kinds). */
    NEVER_ANSWERED("never-answer"),
    /** Preserved verbatim (unknown record / generic RPC console); no native UI. */
    GENERIC("generic"),
}

data class CoverageEntry(val backend: String, val direction: String, val method: String, val coverage: Coverage, val note: String = "")

object CodexCoverage {
    /** Client requests the Codex adapter sends. */
    val USED_REQUESTS = setOf(
        C.INITIALIZE, C.ACCOUNT_READ, C.ACCOUNT_RATE_LIMITS_READ, C.MODEL_LIST, C.ACCOUNT_LOGIN_START, C.ACCOUNT_LOGIN_CANCEL,
        C.ACCOUNT_LOGOUT, C.THREAD_START, C.THREAD_RESUME, C.THREAD_FORK, C.THREAD_TURNS_LIST, C.THREAD_LIST, C.THREAD_NAME_SET,
        C.THREAD_ARCHIVE, C.THREAD_UNARCHIVE, C.THREAD_DELETE, C.THREAD_COMPACT_START, C.TURN_START, C.TURN_STEER,
        C.THREAD_QUEUE_ADD, C.THREAD_QUEUE_DELETE, C.THREAD_QUEUE_LIST, C.TURN_INTERRUPT, C.THREAD_SETTINGS_UPDATE,
    )

    /** Notes for generic client requests that map to checklist items (docs/engine/chat.md). */
    val NOTES = mapOf(
        C.REVIEW_START to "P1 review mode: console only",
        C.THREAD_REVERT to "P1 rollback: console only",
        C.THREAD_READ to "deprecated full hydration; thread/turns/list is used",
        C.THREAD_ITEMS_LIST to "P2 item paging",
        C.THREAD_SEARCH to "P2 server-side search (app index searches locally)",
        C.FUZZY_FILE_SEARCH to "P2 @-file suggestions",
        C.MCP_SERVER_STATUS_LIST to "P2 MCP panel",
        C.SKILLS_LIST to "P2 skills list",
        C.COMMAND_EXEC to "P2 standalone exec",
        C.THREAD_QUEUE_LIST to "P1 queue refresh",
        C.COLLABORATION_MODE_LIST to "P1 plan mode (collaboration modes)",
        R.ITEM_TOOL_CALL to "Workflow registers no dynamic tools: success=false",
        R.CURRENT_TIME_READ to "host clock",
        R.ACCOUNT_CHATGPT_AUTH_TOKENS_REFRESH to "external token auth not used",
        R.ATTESTATION_GENERATE to "attestation not requested",
        R.EXEC_COMMAND_APPROVAL to "legacy v1 approval",
        R.APPLY_PATCH_APPROVAL to "legacy v1 approval",
    )

    fun entries(): List<CoverageEntry> = buildList {
        for (m in CodexProtocol.ClientRequest.ALL) add(CoverageEntry("codex", "client→server request", m, if (m in USED_REQUESTS) Coverage.USED else Coverage.GENERIC, NOTES[m] ?: ""))
        for (m in CodexProtocol.ClientNotification.ALL) add(CoverageEntry("codex", "client→server notification", m, Coverage.USED))
        for (m in CodexProtocol.ServerRequest.ALL) {
            val coverage = when (m) {
                R.CURRENT_TIME_READ, R.ITEM_TOOL_CALL -> Coverage.AUTO
                R.ACCOUNT_CHATGPT_AUTH_TOKENS_REFRESH, R.ATTESTATION_GENERATE -> Coverage.REJECTED
                else -> Coverage.CARD
            }
            add(CoverageEntry("codex", "server→client request", m, coverage, NOTES[m] ?: ""))
        }
        for (m in CodexProtocol.ServerNotification.ALL) {
            val coverage = when (m) {
                in CodexEvents.MODELED_NOTIFICATIONS -> Coverage.MODELED
                in CodexEvents.REFRESH_NOTIFICATIONS -> Coverage.HANDLED
                else -> Coverage.GENERIC
            }
            add(CoverageEntry("codex", "server→client notification", m, coverage, NOTES[m] ?: ""))
        }
    }
}

object ClaudeCoverage {
    /** Control subtypes the CLI sends to us; everything else in the SDK union is ours to send. */
    val INBOUND_CONTROL = mapOf(
        "can_use_tool" to Coverage.CARD,
        "elicitation" to Coverage.CARD,
        "request_user_dialog" to Coverage.NEVER_ANSWERED,
        "hook_callback" to Coverage.AUTO,
        "mcp_message" to Coverage.REJECTED,
    )

    val USED_CONTROL = setOf(
        "initialize", "interrupt", "set_model", "apply_flag_settings", "set_permission_mode", "rename_session",
        "list_models", "get_usage", "cancel_async_message",
    )

    /** stdout message kinds (`type` or `type/subtype`) mapped to model events. */
    val MODELED_MESSAGES = setOf(
        "assistant", "user", "stream_event", "rate_limit_event", "auth_status", "tool_progress",
        "result/success", "result/error_during_execution", "result/error_max_turns", "result/error_max_budget_usd",
        "result/error_max_structured_output_retries",
        "system/init", "system/status", "system/session_state_changed", "system/thinking_tokens", "system/api_retry",
        "system/compact_boundary", "system/hook_started", "system/hook_progress", "system/hook_response",
        "system/local_command_output", "system/permission_denied", "system/notification", "system/informational",
        "system/model_refusal_fallback", "system/model_refusal_no_fallback", "system/task_started", "system/task_progress",
        "system/task_updated", "system/task_notification",
        // observed on the wire (2.1.283), not in the SDK union
        "command_lifecycle",
    )

    /** Consumed by [top.flysoftbeta.workflow.agent.transport.ControlConnection]. */
    val TRANSPORT_MESSAGES = setOf("control_request", "control_response", "control_cancel_request", "keep_alive")

    /** Wire-observed kinds missing from the SDK typings, kept so the table documents them. */
    val OBSERVED_EXTRA = setOf("command_lifecycle", "system/dev_intent")

    val NOTES = mapOf(
        "control_response" to "echoes of our own responses are ignored",
        "hook_callback" to "app-registered hooks only; none by default → error",
        "request_user_dialog" to "no supportedDialogKinds declared → CLI fails closed",
        "mcp_message" to "no SDK MCP servers",
        "get_context_usage" to "P1 context meter: console only",
        "mcp_status" to "P2 MCP panel",
        "file_suggestions" to "P2 @-file suggestions",
        "rewind_files" to "P2",
        "stop_task" to "P1 background tasks: console only",
        "command_lifecycle" to "turn queued/started/completed/cancelled",
        "system/dev_intent" to "project-scan hint",
    )

    fun entries(controlSubtypes: List<String>, messageKinds: List<String>): List<CoverageEntry> = buildList {
        for (s in controlSubtypes.sorted()) {
            val inbound = INBOUND_CONTROL[s]
            if (inbound != null) add(CoverageEntry("claude", "cli→client control_request", s, inbound, NOTES[s] ?: ""))
            else add(CoverageEntry("claude", "client→cli control_request", s, if (s in USED_CONTROL) Coverage.USED else Coverage.GENERIC, NOTES[s] ?: ""))
        }
        for (k in (messageKinds + OBSERVED_EXTRA).distinct().sorted()) {
            val coverage = when (k) {
                in MODELED_MESSAGES -> Coverage.MODELED
                in TRANSPORT_MESSAGES -> Coverage.HANDLED
                else -> Coverage.GENERIC
            }
            add(CoverageEntry("claude", "cli→client message", k, coverage, NOTES[k] ?: ""))
        }
    }
}

object ProtocolCoverageReport {
    fun markdown(codex: List<CoverageEntry>, codexVersion: String, claude: List<CoverageEntry>, claudeVersion: String): String = buildString {
        appendLine("# Agent protocol coverage (generated)")
        appendLine()
        appendLine("Generated by `ProtocolCoverageTest` from `protocol/codex/inventory.json` ($codexVersion) and")
        appendLine("`protocol/claude/inventory.json` (claude-agent-sdk $claudeVersion). Do not edit; regenerate with")
        appendLine("`flock artifacts/.gradle.lock ./gradlew :agent:test -Pagent.updateGolden=true`. Legend: see docs/engine/chat.md.")
        appendLine()
        for ((title, list) in listOf("Codex App-Server" to codex, "Claude Code" to claude)) {
            appendLine("## $title")
            appendLine()
            val counts = list.groupingBy { it.coverage }.eachCount()
            appendLine(Coverage.entries.filter { counts[it] != null }.joinToString(" · ") { "${it.label}: ${counts[it]}" })
            appendLine()
            appendLine("| Direction | Method / kind | Coverage | Note |")
            appendLine("|---|---|---|---|")
            for (e in list) appendLine("| ${e.direction} | `${e.method}` | ${e.coverage.label} | ${e.note} |")
            appendLine()
        }
    }
}
