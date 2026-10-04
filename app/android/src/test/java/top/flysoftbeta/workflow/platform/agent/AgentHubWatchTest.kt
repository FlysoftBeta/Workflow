package top.flysoftbeta.workflow.platform.agent

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import top.flysoftbeta.workflow.agent.rpc.ChatUpdate

/** The watch loop never re-applies changes that a command's snapshot already contains. */
class AgentHubWatchTest {
    private fun update(epoch: String, revision: Long) = ChatUpdate(epoch = epoch, revision = revision)

    @Test fun appliesAnUpdateWhileTheProjectionIsStillAtTheRequestedRevision() {
        assertTrue(AgentHub.appliesTo(update("e1", 7), requested = "e1" to 4L, current = "e1" to 4L))
    }

    @Test fun dropsAnUpdateThatOverlapsASnapshotTakenDuringTheWatch() {
        // Watch from 4; a command's snapshot moved the projection to 6; the update carries 5..7.
        // Changes 5 and 6 are already in the snapshot, so the update is dropped and the next watch
        // asks for changes after 6, which the Engine's journal still holds.
        assertFalse(AgentHub.appliesTo(update("e1", 7), requested = "e1" to 4L, current = "e1" to 6L))
        // The snapshot already covers the whole update.
        assertFalse(AgentHub.appliesTo(update("e1", 7), requested = "e1" to 4L, current = "e1" to 7L))
        assertFalse(AgentHub.appliesTo(update("e1", 7), requested = "e1" to 4L, current = "e1" to 9L))
    }

    @Test fun dropsAnUpdateAfterTheSnapshotChangedTheServiceEpoch() {
        assertFalse(AgentHub.appliesTo(update("e1", 7), requested = "e1" to 4L, current = "e2" to 1L))
        assertFalse(AgentHub.appliesTo(update("e2", 7), requested = "e1" to 4L, current = "e1" to 4L))
    }

    @Test fun ignoresAnEmptyTimeoutAtTheSameRevision() {
        assertFalse(AgentHub.appliesTo(update("e1", 4), requested = "e1" to 4L, current = "e1" to 4L))
    }
}
