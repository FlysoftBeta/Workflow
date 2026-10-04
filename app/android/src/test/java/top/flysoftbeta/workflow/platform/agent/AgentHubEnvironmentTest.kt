package top.flysoftbeta.workflow.platform.agent

import java.io.IOException
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import top.flysoftbeta.workflow.core.connection.WorkspaceRpcException

/** A chat command waits while the Engine prepares the environment instead of failing the user's action. */
class AgentHubEnvironmentTest {
    private fun refused(kind: String) = WorkspaceRpcException(-32000, "chat requires a ready environment", mapOf("kind" to kind))

    @Test fun recognizesOnlyTheEnginesPreparationRefusal() {
        assertTrue(AgentHub.preparingEnvironment(refused("environment_preparing")))
        assertFalse(AgentHub.preparingEnvironment(refused("closed")))
        assertFalse(AgentHub.preparingEnvironment(WorkspaceRpcException(-32000, "no data", null)))
        assertFalse(AgentHub.preparingEnvironment(IOException("工作区连接已断开")))
    }

    @Test fun repeatsTheCommandUntilTheEnvironmentIsReady() = runBlocking {
        var calls = 0
        val result = AgentHub.awaitingEnvironment(retryDelayMs = 1) {
            if (++calls < 3) throw refused("environment_preparing")
            "accepted"
        }
        assertEquals("accepted", result)
        assertEquals(3, calls)
    }

    @Test fun otherFailuresEndTheWait() = runBlocking {
        val closed = refused("closed")
        var calls = 0
        try {
            AgentHub.awaitingEnvironment(retryDelayMs = 1) { calls++; throw closed }
            fail("A non-preparation failure must propagate")
        } catch (error: WorkspaceRpcException) {
            assertSame(closed, error)
        }
        assertEquals(1, calls)
    }
}
