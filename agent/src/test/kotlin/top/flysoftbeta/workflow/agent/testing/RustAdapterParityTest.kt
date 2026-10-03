package top.flysoftbeta.workflow.agent.testing

import java.io.File
import kotlinx.serialization.json.*
import org.junit.Assert.assertEquals
import org.junit.Test
import top.flysoftbeta.workflow.agent.codex.CodexDisposition
import top.flysoftbeta.workflow.agent.codex.CodexEvents
import top.flysoftbeta.workflow.agent.claude.ClaudeRequests
import top.flysoftbeta.workflow.agent.rpc.ChatWire

/** Frozen cards come from the retained adapters, never from the Rust implementation. */
class RustAdapterParityTest {
    @Test fun exportApprovalCardsForRustParity() {
        val directory = System.getenv("WORKFLOW_RUST_PARITY_DIR") ?: return
        fun obj(text: String) = ChatWire.json.parseToJsonElement(text).jsonObject
        val out = mutableListOf<JsonObject>()
        fun codex(name: String, method: String, params: String) {
            val id = JsonPrimitive(9007199254740993L)
            val p = obj(params)
            val raw = buildJsonObject { put("id", id); put("method", method); put("params", p); put("futureEnvelope", true) }
            val disposition = CodexEvents.request(id, method, p, raw, 17_000)
            out += buildJsonObject {
                put("case", name); put("backend", "codex"); put("id", id); put("method", method); put("params", p); put("raw", raw)
                when (disposition) {
                    is CodexDisposition.Ask -> { put("disposition", "ask"); put("expected", ChatWire.encode(disposition.request)) }
                    is CodexDisposition.AutoResult -> { put("disposition", "fact"); put("expected", ChatWire.encode(disposition.record)); put("result", disposition.result) }
                    is CodexDisposition.AutoError -> { put("disposition", "error"); put("expected", ChatWire.encode(disposition.record)); put("code", disposition.code); put("message", disposition.message) }
                }
            }
        }
        fun claude(name: String, subtype: String, params: String) {
            val p = obj(params)
            val raw = buildJsonObject { put("type", "control_request"); put("request_id", "r1"); put("request", p); put("futureEnvelope", true) }
            val request = ClaudeRequests.pending("r1", subtype, p, raw, "s", "t", 17_000)!!
            out += buildJsonObject { put("case", name); put("backend", "claude"); put("id", "r1"); put("method", subtype); put("params", p); put("raw", raw); put("disposition", "ask"); put("expected", ChatWire.encode(request)) }
        }
        codex("command-null-actions", "item/commandExecution/requestApproval", """{"threadId":"t","turnId":"u","command":"ls","cwd":"/workspace","commandActions":null,"availableDecisions":null}""")
        codex("command-offered-decisions", "item/commandExecution/requestApproval", """{"threadId":"t","command":"ls","availableDecisions":["accept","acceptForSession",{"acceptWithExecpolicyAmendment":{"execpolicy_amendment":["ls","-l"]}},{"applyNetworkPolicyAmendment":{"network_policy_amendment":{"action":"deny","host":"example.test"}}},"accept"]}""")
        codex("file-change", "item/fileChange/requestApproval", """{"threadId":"t","grantRoot":"/workspace"}""")
        codex("permissions", "item/permissions/requestApproval", """{"threadId":"t","permissions":{"network":{"enabled":true},"future":42}}""")
        codex("questions", "item/tool/requestUserInput", """{"threadId":"t","turnId":"u","questions":[{"id":"db","question":"Which?","options":[{"label":"A","description":"a"}],"isOther":true,"isSecret":true}]}""")
        codex("elicitation", "mcpServer/elicitation/request", """{"_meta":{"serverName":"mcp"},"title":"Choose","requestedSchema":{"type":"object"}}""")
        codex("legacy-command", "execCommandApproval", """{"conversationId":"t","command":["ls","-l"],"callId":"i"}""")
        codex("legacy-patch", "applyPatchApproval", """{"conversationId":"t","callId":"i"}""")
        codex("time-is-factual", "currentTime/read", """{"threadId":"t"}""")
        codex("dynamic-tool-is-negative", "item/tool/call", """{"threadId":"t","callId":"i","tool":"unregistered"}""")
        codex("auth-refresh-is-rejected", "account/chatgptAuthTokens/refresh", """{}""")
        claude("suppressed-persistence", "can_use_tool", """{"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"rm x"},"decision_reason":"\u001b[31mdangerous\u001b[0m","default_to_no":true,"suppress_always_allow_rule":true,"permission_suggestions":[{"type":"setMode","mode":"acceptEdits","destination":"localSettings"}]}""")
        claude("interactive-tool", "can_use_tool", """{"subtype":"can_use_tool","tool_name":"Bash","input":{},"requires_user_interaction":true}""")
        claude("offered-persistence", "can_use_tool", """{"subtype":"can_use_tool","tool_name":"Edit","input":{"file_path":"/workspace/a"},"permission_suggestions":[{"type":"setMode","mode":"acceptEdits","destination":"session"},{"type":"addRules","behavior":"allow","rules":[{"toolName":"Edit","ruleContent":"a"}],"destination":"localSettings"}]}""")
        claude("questions", "can_use_tool", """{"subtype":"can_use_tool","tool_name":"AskUserQuestion","input":{"questions":[{"question":"Which?","multiSelect":true,"options":[{"label":"A","description":"a","preview":"preview"}]}]}}""")
        claude("plan", "can_use_tool", """{"subtype":"can_use_tool","tool_name":"ExitPlanMode","input":{"plan":"1. do it"}}""")
        claude("elicitation", "elicitation", """{"subtype":"elicitation","mcp_server_name":"gh","message":"Choose?","requested_schema":{"type":"object"}}""")
        claude("undeclared-dialog", "request_user_dialog", """{"subtype":"request_user_dialog","dialog_kind":"future","payload":{"data":1}}""")
        assertEquals(18, out.size)
        File(directory, "approval-cards.json").apply { parentFile.mkdirs(); writeText(JsonArray(out).toString()) }
    }
}
