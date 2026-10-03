package top.flysoftbeta.workflow.core.store

import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.core.io.readText
import top.flysoftbeta.workflow.core.io.writeAtomic
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.resource.DiskVersion

class RecoveryTest {
    private suspend fun seeded(h: StoreHarness): Pair<String, String> {
        val store = h.ready()
        val first = store.createSession("First")
        h.idle()
        val second = store.createSession("Second")
        store.editFile("a.txt", "draft a", DiskVersion.MISSING)
        store.editFile("b.txt", "draft b", DiskVersion.MISSING)
        h.idle()
        return first to second
    }

    @Test fun `a damaged sessions json falls back to the backup and keeps the damaged bytes`() = runTest {
        val h = StoreHarness(this)
        val (first, _) = seeded(h)
        h.fs.writeAtomic(StatePaths.SESSIONS, "{broken")
        val store = h.ready()
        assertEquals(listOf(first), store.state.value.sessions.map { it.id })
        assertEquals(NoticeKind.RECOVERED, store.state.value.notices.single().kind)
        assertEquals("{broken", h.fs.readText(h.fs.list(StatePaths.CORRUPT).single().path))
        assertEquals(setOf("a.txt", "b.txt"), store.state.value.drafts.keys)
        store.dismissNotice(store.state.value.notices.single().id)
        assertTrue(store.state.value.notices.isEmpty())
    }

    @Test fun `unreadable sessions start fresh without losing drafts`() = runTest {
        val h = StoreHarness(this)
        seeded(h)
        h.fs.writeAtomic(StatePaths.SESSIONS, "garbage")
        h.fs.writeAtomic(StatePaths.SESSIONS_BACKUP, "garbage too")
        val store = h.ready()
        assertEquals(StoreStatus.READY, store.state.value.status)
        assertTrue(store.state.value.sessions.isEmpty())
        assertEquals(setOf("a.txt", "b.txt"), store.state.value.drafts.keys)
        assertTrue(store.state.value.notices.any { it.kind == NoticeKind.RECOVERED })
        store.createSession()
        h.idle()
        assertEquals(1, StateCodec.decodeSessions(h.fs.readText(StatePaths.SESSIONS)).sessions.size)
    }

    @Test fun `a damaged draft file is set aside while the others load`() = runTest {
        val h = StoreHarness(this)
        seeded(h)
        h.fs.writeAtomic(StatePaths.draft("a.txt"), "not json")
        val store = h.ready()
        assertEquals(setOf("b.txt"), store.state.value.drafts.keys)
        assertNull(h.fs.stat(StatePaths.draft("a.txt")))
        assertEquals("not json", h.fs.readText(h.fs.list(StatePaths.CORRUPT).single().path))
    }

    @Test fun `state from a newer version is never overwritten`() = runTest {
        val h = StoreHarness(this)
        val newer = "{\"format\":99,\"sessions\":[]}"
        h.fs.writeAtomic(StatePaths.SESSIONS, newer)
        h.fs.writeAtomic(StatePaths.draft("x"), "{\"format\":99}")
        val store = h.ready()
        assertEquals(NoticeKind.NEWER_FORMAT, store.state.value.notices.single().kind)
        store.createSession("Work")
        h.idle()
        assertEquals(newer, h.fs.readText(StatePaths.SESSIONS))
        assertEquals("{\"format\":99}", h.fs.readText(StatePaths.draft("x")))
    }

    @Test fun `only stale temporary files are cleaned at start`() = runTest {
        val h = StoreHarness(this)
        h.mem.nextModifiedAt = h.now - 3_600_000
        h.fs.writeAtomic("${WorkspacePaths.TMP}/old.tmp", "x")
        h.mem.nextModifiedAt = h.now
        h.fs.writeAtomic("${WorkspacePaths.TMP}/in-flight.tmp", "y")
        h.mem.nextModifiedAt = null
        h.ready()
        assertEquals(listOf("in-flight.tmp"), h.fs.list(WorkspacePaths.TMP).map { it.name })
    }

    @Test fun `a failed load is reported`() = runTest {
        val h = StoreHarness(this)
        h.fs.writeAtomic(".workspace/state", "a file where a directory must be")
        val store = h.ready()
        assertEquals(StoreStatus.FAILED, store.state.value.status)
        assertNotNull(store.state.value.failure)
    }

    @Test fun `directory changes are observable for the explorer and tracked directories are watched`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        store.createSession()
        store.layout(LayoutOp.Open(PanelTarget.File("docs/a.md")))
        store.awaitIdle()
        assertTrue("docs" in h.watcher.watchedDirectories)
        assertTrue("" in h.watcher.watchedDirectories)
        val seen = launch { store.directoryChanges("src").first() }
        testScheduler.runCurrent()
        h.watcher.changed("src/new.kt")
        testScheduler.runCurrent()
        assertTrue(seen.isCompleted)
        store.layout(LayoutOp.Close(store.state.value.activeSession!!.workbench.panels.keys.toList()))
        store.awaitIdle()
        assertFalse("docs" in h.watcher.watchedDirectories)
    }
}
