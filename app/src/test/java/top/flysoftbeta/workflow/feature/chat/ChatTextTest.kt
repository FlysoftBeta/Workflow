package top.flysoftbeta.workflow.feature.chat

import java.time.LocalDate
import java.time.LocalDateTime
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import org.junit.Assert.assertEquals
import org.junit.Test
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.Decision
import top.flysoftbeta.workflow.agent.model.DecisionKind
import top.flysoftbeta.workflow.agent.model.PendingRequest
import top.flysoftbeta.workflow.agent.model.RequestKey
import top.flysoftbeta.workflow.agent.model.RequestKind
import top.flysoftbeta.workflow.agent.model.RequestStatus

class ChatTextTest {
    @Test fun unwrapsShellWrappers() {
        assertEquals("git status --short", ChatText.unwrapShell("/bin/zsh -lc \"git status --short\""))
        assertEquals("printf '%s\\n' hi", ChatText.unwrapShell("bash -lc 'printf '\\''%s\\n'\\'' hi'"))
        assertEquals("echo \"x\"", ChatText.unwrapShell("/system/bin/sh -c \"echo \\\"x\\\"\""))
        assertEquals("ls -la", ChatText.unwrapShell("ls -la"))
    }

    @Test fun settledRequestsBecomeOneLine() {
        val decisions = listOf(Decision("accept", DecisionKind.ALLOW_ONCE), Decision("decline", DecisionKind.DENY))
        fun request(status: RequestStatus, answer: String?) = PendingRequest(
            RequestKey(BackendKind.CODEX, JsonPrimitive(0)), "item/commandExecution/requestApproval",
            RequestKind.CommandApproval("/bin/zsh -lc \"git status\"", "/workspace", null), decisions, status = status, answer = answer,
        )
        assertEquals("已批准 · 运行 git status", ChatText.requestRecord(request(RequestStatus.ANSWERED, "accept")))
        assertEquals("已拒绝 · 运行 git status", ChatText.requestRecord(request(RequestStatus.ANSWERED, "decline")))
        assertEquals("已失效 · 运行 git status", ChatText.requestRecord(request(RequestStatus.EXPIRED, null)))
    }

    @Test fun timeSeparatorsAndRelativeAges() {
        val today = LocalDate.of(2026, 9, 29)
        assertEquals("14:32", ChatText.timeSeparator(LocalDateTime.of(2026, 9, 29, 14, 32), today))
        assertEquals("昨天 6:05", ChatText.timeSeparator(LocalDateTime.of(2026, 9, 28, 6, 5), today))
        assertEquals("星期日 6:48", ChatText.timeSeparator(LocalDateTime.of(2026, 9, 27, 6, 48), today))
        assertEquals("9月2日 9:00", ChatText.timeSeparator(LocalDateTime.of(2026, 9, 2, 9, 0), today))
        val now = 1_000_000_000L
        assertEquals("刚刚", ChatText.relative(now - 10_000, now, today, today))
        assertEquals("5 分钟", ChatText.relative(now - 5 * 60_000, now, today, today))
        assertEquals("昨天", ChatText.relative(now - 30 * 3600_000L, now, today, today.minusDays(1)))
        assertEquals("近 7 天", ChatText.railGroup(today.minusDays(3), today))
        assertEquals("更早", ChatText.railGroup(today.minusDays(9), today))
    }

    @Test fun elicitationValuesAreTyped() {
        val schema = buildJsonObject {
            put("name", buildJsonObject { put("type", "string") })
            put("count", buildJsonObject { put("type", "integer") })
            put("ok", buildJsonObject { put("type", "boolean") })
        }
        val content = elicitationContent(schema, mapOf("name" to "x", "count" to "3", "ok" to "true"))
        assertEquals("""{"name":"x","count":3,"ok":true}""", content.toString())
    }

    @Test fun effortLabels() {
        assertEquals("高", ChatText.effortLabel("high"))
        assertEquals("极高", ChatText.effortLabel("xhigh"))
        assertEquals("custom", ChatText.effortLabel("custom"))
    }
}
