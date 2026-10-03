package top.flysoftbeta.workflow.core.store

import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.core.io.readText
import top.flysoftbeta.workflow.core.io.writeAtomic
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.resource.ComposerAttachment
import top.flysoftbeta.workflow.core.resource.ComposerDraft
import top.flysoftbeta.workflow.core.resource.ComposerRevisionConflictException
import top.flysoftbeta.workflow.core.resource.ResourceRef
import top.flysoftbeta.workflow.core.session.ArchiveDecision
import top.flysoftbeta.workflow.core.session.SessionPolicy

/** Archive protection and independent composer persistence. */
class ArchiveAndComposerTest {
    private suspend fun WorkspaceStore.fileSession(h: StoreHarness, name: String? = null, path: String = "notes.txt"): String {
        if (h.fs.stat(path) == null) h.fs.writeAtomic(path, "Original")
        val id = createSession(name)
        applyLayout(id, LayoutOp.Open(PanelTarget.File(path)))
        return id
    }

    private suspend fun WorkspaceStore.conversationSession(name: String? = null, conversation: String): String {
        val id = createSession(name)
        applyLayout(id, LayoutOp.showConversation(conversation))
        return id
    }

    private suspend fun WorkspaceStore.unsent(conversation: String, text: String = "unsent", attachments: List<String> = emptyList()): ComposerDraft {
        val current = state.value.composer(conversation)
        return editComposer(current.edit(text, attachments.map { ComposerAttachment(it) }), current.revision)
    }

    @Test fun `temporary and persistent retention boundaries are one and seven days`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        val temporary = store.createSession()
        val persistent = store.createSession("Writing")
        h.now += SessionPolicy.DAY_MS - 1
        store.runMaintenance()
        assertTrue(store.state.value.sessions.none { it.isArchived })
        h.now++
        store.runMaintenance()
        assertTrue(store.state.value.session(temporary)!!.isArchived)
        assertFalse(store.state.value.session(persistent)!!.isArchived)
        h.now += 6 * SessionPolicy.DAY_MS
        store.runMaintenance()
        assertTrue(store.state.value.session(persistent)!!.isArchived)
        assertNull(store.state.value.activeSessionId)
    }

    @Test fun `the latest live reference to a dirty file is protected while older sessions archive`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        val old = store.fileSession(h)
        h.now += 10
        val newer = store.fileSession(h, "Drafting")
        store.editFile("notes.txt", "Shared draft", store.openFile("notes.txt").shownVersion)
        store.awaitIdle()
        h.now += 8 * SessionPolicy.DAY_MS
        store.runMaintenance()
        val state = store.state.value
        assertTrue(state.session(old)!!.isArchived)
        assertFalse(state.session(newer)!!.isArchived)
        assertEquals(setOf(newer), SessionPolicy.protectedSessions(state.sessions, state.dirtyResources))
        h.idle()
        assertEquals("Shared draft", h.ready().state.value.drafts.getValue("notes.txt").text)
        assertEquals("Original", h.fs.readText("notes.txt"))
    }

    @Test fun `only the latest live reference to unsent conversation input blocks automatic archive`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        val old = store.conversationSession(conversation = "c")
        val unsent = store.unsent("c", "", listOf("image.png"))
        h.now++
        val newer = store.conversationSession("Latest", "c")
        h.now += 8 * SessionPolicy.DAY_MS
        store.runMaintenance()
        assertTrue(store.state.value.session(old)!!.isArchived)
        assertFalse(store.state.value.session(newer)!!.isArchived)
        store.acknowledgeComposer(unsent)
        // Sending counts as using the session; once it is idle again nothing protects it.
        assertFalse(store.state.value.session(newer)!!.isArchived)
        h.now += SessionPolicy.PERSISTENT_RETENTION_MS
        store.runMaintenance()
        assertTrue(store.state.value.session(newer)!!.isArchived)
    }

    @Test fun `file and composer protections are combined`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        val conversationSession = store.conversationSession(conversation = "c")
        store.unsent("c", "text")
        val fileSession = store.fileSession(h)
        store.editFile("notes.txt", "draft", store.openFile("notes.txt").shownVersion)
        val state = store.awaitIdle()
        assertEquals(setOf(conversationSession, fileSession), SessionPolicy.protectedSessions(state.sessions, state.dirtyResources))
        h.now += 8 * SessionPolicy.DAY_MS
        store.runMaintenance()
        assertTrue(store.state.value.sessions.none { it.isArchived })
    }

    @Test fun `equal timestamps protect exactly one deterministic session`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        val a = store.fileSession(h)
        val b = store.fileSession(h)
        store.editFile("notes.txt", "draft", store.openFile("notes.txt").shownVersion)
        val sessions = store.state.value.sessions.map { it.copy(lastUsedAt = 20, createdAt = 10) }
        assertEquals(setOf(maxOf(a, b)), SessionPolicy.protectedSessions(sessions, setOf(ResourceRef.File("notes.txt"))))
    }

    private suspend fun mixed(h: StoreHarness, store: WorkspaceStore): String {
        val session = store.fileSession(h, "Mixed")
        store.applyLayout(session, LayoutOp.showConversation("c"))
        store.editFile("notes.txt", "file draft", store.openFile("notes.txt").shownVersion)
        store.unsent("c", "unsent draft", listOf("notes.txt"))
        return session
    }

    @Test fun `manual archive needs a decision only when unsaved content is involved`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        val clean = store.createSession()
        assertEquals(ArchiveOutcome.Archived(emptyList()), store.archiveSession(clean))
        val session = mixed(h, store)
        assertEquals(ArchiveOutcome.NeedsDecision(setOf(ResourceRef.File("notes.txt"), ResourceRef.Conversation("c"))), store.archiveSession(session))
        assertFalse(store.state.value.session(session)!!.isArchived)
        assertEquals(ArchiveOutcome.NotFound, store.archiveSession("missing"))
    }

    @Test fun `keep drafts archives and restores with everything intact`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        val session = mixed(h, store)
        assertEquals(ArchiveOutcome.Archived(emptyList()), store.archiveSession(session, ArchiveDecision.KEEP_DRAFTS))
        assertTrue(store.state.value.session(session)!!.isArchived)
        assertNull(store.state.value.activeSessionId)
        assertEquals("file draft", store.state.value.drafts.getValue("notes.txt").text)
        h.idle()
        val reopened = h.ready()
        assertTrue(reopened.restoreSession(session))
        assertEquals(session, reopened.state.value.activeSessionId)
        assertEquals("unsent draft", reopened.state.value.composer("c").text)
        assertEquals("file draft", reopened.state.value.drafts.getValue("notes.txt").text)
    }

    @Test fun `save all writes files and keeps unsent conversation input`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        val session = mixed(h, store)
        assertEquals(ArchiveOutcome.Archived(listOf("notes.txt")), store.archiveSession(session, ArchiveDecision.SAVE_ALL))
        assertEquals("file draft", h.fs.readText("notes.txt"))
        assertFalse(store.state.value.isDirty(ResourceRef.File("notes.txt")))
        assertTrue(store.state.value.isDirty(ResourceRef.Conversation("c")))
    }

    @Test fun `a save conflict leaves files, composers and the session untouched`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        val session = mixed(h, store)
        h.fs.writeAtomic("notes.txt", "external")
        assertEquals(ArchiveOutcome.SaveConflict(listOf("notes.txt")), store.archiveSession(session, ArchiveDecision.SAVE_ALL))
        assertEquals("external", h.fs.readText("notes.txt"))
        assertEquals("unsent draft", store.state.value.composer("c").text)
        assertFalse(store.state.value.session(session)!!.isArchived)
    }

    @Test fun `discard drops drafts and advances composer revisions so stale snapshots cannot resurrect them`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        val session = mixed(h, store)
        val unsent = store.state.value.composer("c")
        assertEquals(ArchiveOutcome.Archived(emptyList()), store.archiveSession(session, ArchiveDecision.DISCARD))
        assertEquals("Original", h.fs.readText("notes.txt"))
        assertTrue(store.state.value.drafts.isEmpty())
        val tombstone = store.state.value.composer("c")
        assertEquals(unsent.revision + 1, tombstone.revision)
        assertFalse(tombstone.hasContent)
        assertTrue(runCatching { store.editComposer(unsent.edit("queued earlier"), unsent.revision) }.exceptionOrNull() is ComposerRevisionConflictException)
        assertEquals(tombstone, store.acknowledgeComposer(unsent))
        h.idle()
        assertEquals(tombstone, h.ready().state.value.composer("c"))
    }

    @Test fun `discard validates revision overflow before changing anything`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        val session = mixed(h, store)
        val current = store.state.value.composer("c")
        store.editComposer(current.copy(revision = Long.MAX_VALUE), current.revision)
        assertTrue(runCatching { store.archiveSession(session, ArchiveDecision.DISCARD) }.exceptionOrNull() is IllegalStateException)
        assertEquals("file draft", store.state.value.drafts.getValue("notes.txt").text)
        assertFalse(store.state.value.session(session)!!.isArchived)
    }

    @Test fun `composer drafts are independent per conversation and survive reconstruction`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        val a = store.unsent("a", "A text", listOf("imports/a.png"))
        val b = store.unsent("b", "B text", listOf("notes.md"))
        assertEquals(ComposerDraft("new"), store.state.value.composer("new"))
        h.idle()
        val reopened = h.ready()
        assertEquals(a, reopened.state.value.composer("a"))
        assertEquals(b, reopened.state.value.composer("b"))
        assertTrue(reopened.state.value.drafts.isEmpty())
    }

    @Test fun `acknowledging identical text in one conversation never clears another`() = runTest {
        val store = StoreHarness(this).ready()
        val submitted = store.unsent("a", "same words", listOf("same.txt"))
        val other = store.unsent("b", "same words", listOf("same.txt"))
        assertEquals("", store.acknowledgeComposer(submitted).text)
        assertEquals(other, store.state.value.composer("b"))
    }

    @Test fun `typing after send preserves the newer revision even with identical final text`() = runTest {
        val store = StoreHarness(this).ready()
        val submitted = store.unsent("a", "send this", listOf("a.txt"))
        val typed = submitted.edit("new text").edit("send this")
        store.editComposer(typed, submitted.revision)
        assertEquals(typed, store.acknowledgeComposer(submitted))
        val withAttachment = typed.edit(attachments = listOf(ComposerAttachment("a.txt"), ComposerAttachment("b.txt")))
        store.editComposer(withAttachment, typed.revision)
        assertEquals(withAttachment, store.acknowledgeComposer(typed))
    }

    @Test fun `an old queued snapshot cannot overwrite a newer composer`() = runTest {
        val store = StoreHarness(this).ready()
        val first = ComposerDraft("a").edit("first")
        val newer = first.edit("newer")
        store.editComposer(newer, 0)
        assertTrue(runCatching { store.editComposer(first, 0) }.exceptionOrNull() is ComposerRevisionConflictException)
        assertEquals(newer, store.state.value.composer("a"))
        assertEquals(newer, store.editComposer(newer, 0))
        assertEquals(ComposerDraft("a", newer.revision + 1), store.discardComposer("a"))
    }
}
