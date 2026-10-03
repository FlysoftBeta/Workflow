package top.flysoftbeta.workflow.core.store

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import top.flysoftbeta.workflow.core.io.readText
import top.flysoftbeta.workflow.core.io.writeAtomic
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.resource.FileStatus

class TrashTest {
    @Test fun `trash moves the item under the internal trash and undo restores it`() = runTest {
        val h = StoreHarness(this)
        h.fs.writeAtomic("docs/a.md", "A")
        h.fs.writeAtomic("docs/sub/b.md", "B")
        val store = h.ready()
        val result = store.trashPath("docs") as TrashResult.Trashed
        assertNull(h.fs.stat("docs"))
        assertTrue(store.listDirectory("").none { it.name == ".workspace" })
        assertEquals("docs", result.entry.path)
        assertTrue(result.entry.isDirectory)
        val restored = store.restoreFromTrash(result.entry.id) as RestoreResult.Restored
        assertEquals("docs", restored.path)
        assertEquals("B", h.fs.readText("docs/sub/b.md"))
        assertTrue(h.fs.list(WorkspaceTrash.ROOT).isEmpty())
        assertTrue(store.restoreFromTrash(result.entry.id) is RestoreResult.Failed)
    }

    @Test fun `restore picks a free name when the original path was taken meanwhile`() = runTest {
        val h = StoreHarness(this)
        h.fs.writeAtomic("notes.txt", "old")
        val store = h.ready()
        val entry = (store.trashPath("notes.txt") as TrashResult.Trashed).entry
        store.createFile("notes.txt", "new".toByteArray())
        val restored = store.restoreFromTrash(entry.id) as RestoreResult.Restored
        assertEquals("notes (1).txt", restored.path)
        assertEquals("new", h.fs.readText("notes.txt"))
        assertEquals("old", h.fs.readText("notes (1).txt"))
    }

    @Test fun `trash closes clean panels but keeps drafts as deleted working resources`() = runTest {
        val h = StoreHarness(this)
        h.fs.writeAtomic("clean.txt", "c")
        h.fs.writeAtomic("dirty.txt", "d")
        val store = h.ready()
        val session = store.createSession()
        store.applyLayout(session, LayoutOp.Open(PanelTarget.File("clean.txt")))
        store.applyLayout(session, LayoutOp.Open(PanelTarget.File("dirty.txt")))
        val snapshot = store.openFile("dirty.txt")
        store.editFile("dirty.txt", "d edited", snapshot.shownVersion)
        store.trashPath("clean.txt")
        store.trashPath("dirty.txt")
        val panels = store.state.value.session(session)!!.workbench.panels.values.map { it.target }
        assertEquals(listOf<PanelTarget>(PanelTarget.File("dirty.txt")), panels)
        assertEquals(FileStatus.DELETED, store.state.value.fileStatus("dirty.txt"))
    }

    @Test fun `maintenance purges entries after seven days only`() = runTest {
        val h = StoreHarness(this)
        h.fs.writeAtomic("a.txt", "a")
        h.fs.writeAtomic("b.txt", "b")
        val store = h.ready()
        store.trashPath("a.txt")
        h.now += TrashPolicy.RETENTION_MS - 60_000
        store.trashPath("b.txt")
        h.now += 60_000
        store.runMaintenance()
        val left = h.fs.list(WorkspaceTrash.ROOT)
        assertEquals(1, left.size)
        assertTrue(h.fs.readText("${left.single().path}/meta.json").contains("b.txt"))
        h.now += TrashPolicy.RETENTION_MS
        assertEquals(1, store.purgeTrash())
        assertTrue(h.fs.list(WorkspaceTrash.ROOT).isEmpty())
    }

    @Test fun `purge policy keeps future dated entries and expires at exactly the retention`() {
        assertFalse(TrashPolicy.isExpired(trashedAt = 1_000, now = 1_000 + TrashPolicy.RETENTION_MS - 1))
        assertTrue(TrashPolicy.isExpired(trashedAt = 1_000, now = 1_000 + TrashPolicy.RETENTION_MS))
        assertFalse(TrashPolicy.isExpired(trashedAt = 10_000, now = 1_000))
    }

    @Test fun `app internal paths cannot be trashed and copies refuse to overwrite`() = runTest {
        val h = StoreHarness(this)
        h.fs.writeAtomic("dir/x.txt", "x")
        val store = h.ready()
        assertTrue(store.trashPath(".workspace/state") is TrashResult.Failed)
        assertTrue(store.trashPath("missing") is TrashResult.Failed)
        assertEquals(FileOpResult.Done, store.copyPath("dir", "dir (1)"))
        assertEquals("x", h.fs.readText("dir (1)/x.txt"))
        assertTrue(store.copyPath("dir", "dir (1)") is FileOpResult.Failed)
        assertTrue(store.copyPath("dir", "dir/inner") is FileOpResult.Failed)
    }
}
