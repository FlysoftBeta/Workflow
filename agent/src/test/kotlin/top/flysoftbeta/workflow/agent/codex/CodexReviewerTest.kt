package top.flysoftbeta.workflow.agent.codex

import kotlinx.serialization.json.*
import org.junit.Assert.*
import org.junit.Test

class CodexReviewerTest {
    @Test fun rawConsoleForcesUserReviewerForAllSixSettingsMethods() {
        val methods = setOf("thread/start", "thread/resume", "thread/fork", "turn/start", "thread/settings/update", "turn/settings/update")
        assertEquals(methods, CodexParams.REVIEWER_METHODS)
        methods.forEach { method ->
            val params = buildJsonObject { put("approvalsReviewer", "auto_review"); put("futureField", "preserved") }
            val guarded = CodexParams.enforceReviewer(method, params)!!.jsonObject
            assertEquals(JsonPrimitive("user"), guarded["approvalsReviewer"])
            assertEquals(JsonPrimitive("preserved"), guarded["futureField"])
            assertEquals(JsonPrimitive("user"), CodexParams.enforceReviewer(method, null)!!.jsonObject["approvalsReviewer"])
        }
    }
}
