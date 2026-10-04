package top.flysoftbeta.workflow.core.connection

import java.io.IOException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class FailureTest {
    private fun rpc(kind: String?) = WorkspaceRpcException(-32000, "engine says $kind", kind?.let { mapOf("kind" to it) })

    @Test fun connectionLossIsNotReportedAsAnActionError() {
        assertTrue(Failure.of(WorkspaceClosedException("工作区连接已断开")) is Failure.Lost)
        assertTrue(Failure.of(rpc("closed")) is Failure.Lost)
    }

    @Test fun preparationAndTimeoutsAreTransient() {
        assertTrue(Failure.of(rpc("environment_preparing")) is Failure.Transient)
        assertTrue(Failure.of(WorkspaceTimeoutException("terminal.create", 60_000)) is Failure.Transient)
    }

    @Test fun refusalsKeepEnginesKindAndMessageForDetails() {
        val failure = Failure.of(rpc("exists"))
        assertEquals(Failure.Rejected("exists", "同名项目已存在", "engine says exists"), failure)
        assertEquals("操作没有完成", Failure.of(rpc("something_new")).summary)
        assertEquals("操作没有完成", Failure.of(rpc(null)).summary)
    }

    @Test fun blockedCapabilitiesUseTheirOwnReason() {
        assertEquals(Failure.Blocked("环境安装失败", null), Failure.of(CapabilityBlockedException("环境安装失败")))
    }

    @Test fun anythingElseIsUnexpected() {
        assertTrue(Failure.of(IOException("disk")) is Failure.Unexpected)
        assertTrue(Failure.of(IllegalStateException()) is Failure.Unexpected)
    }
}
