package top.flysoftbeta.workflow.core.store

import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.core.io.readText
import top.flysoftbeta.workflow.core.io.writeAtomic
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.session.ArchiveDecision

class ConversationPersistenceTest {
    @Test fun `deleting a conversation removes all references without touching other drafts`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        val first = store.createSession("first")
        store.applyLayout(first, LayoutOp.Open(PanelTarget.Conversation("deleted")))
        val other = store.createSession("other")
        store.applyLayout(other, LayoutOp.Open(PanelTarget.Conversation("deleted")))
        store.applyLayout(other, LayoutOp.Open(PanelTarget.Conversation("kept")))
        store.editComposer(store.state.value.composer("deleted").edit("unsent", emptyList()), 0)
        store.editComposer(store.state.value.composer("kept").edit("preserved", emptyList()), 0)
        store.archiveSession(first, ArchiveDecision.KEEP_DRAFTS)
        store.removeConversation("deleted")
        assertFalse(store.state.value.composer("deleted").hasContent)
        assertEquals("preserved", store.state.value.composer("kept").text)
        assertTrue(store.state.value.sessions.all { it.workbench.panelFor(PanelTarget.Conversation("deleted")) == null })
        assertTrue(store.flush())
        assertTrue(h.ready().state.value.sessions.all { it.workbench.panelFor(PanelTarget.Conversation("deleted")) == null })
    }

    @Test fun `conversation writes share the store flush and retry path`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        store.writeConversationIndex("first")
        store.writeConversationIndex("latest")
        assertEquals("latest", store.readConversationIndex())
        h.mem.failWrite = { it == StatePaths.CONVERSATIONS }
        assertFalse(store.flush())
        assertNotNull(store.state.value.writeError)
        assertEquals("latest", store.readConversationIndex())
        h.mem.failWrite = { false }
        h.idle()
        assertNull(store.state.value.writeError)
        assertEquals("latest", h.fs.readText(StatePaths.CONVERSATIONS))
        assertEquals("latest", h.ready().readConversationIndex())
    }

    @Test fun `corrupt index preservation is serialized with workspace IO`() = runTest {
        val h = StoreHarness(this)
        h.fs.writeAtomic(StatePaths.CONVERSATIONS, "broken")
        val store = h.ready()
        assertEquals("broken", store.readConversationIndex())
        store.quarantineConversationIndex()
        assertNull(store.readConversationIndex())
        val copy = h.fs.list(StatePaths.CORRUPT).single()
        assertEquals("broken", h.fs.readText(copy.path))
        assertTrue(store.state.value.notices.any { it.kind == NoticeKind.RECOVERED })
    }
}
