package top.flysoftbeta.workflow.core.resource

import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.core.io.Hashing

class DraftsTest {
    private val base = DiskVersion.of("base".toByteArray(), 10)
    private val other = DiskVersion.of("other".toByteArray(), 20)

    @Test fun `disk versions compare by hash, else by size and mtime`() {
        assertTrue(DiskVersion.MISSING.sameContent(DiskVersion(false, 5, 5)))
        assertFalse(DiskVersion.MISSING.sameContent(base))
        assertTrue(base.sameContent(base.copy(modifiedAt = 99)))
        assertFalse(base.sameContent(other))
        assertTrue(DiskVersion(true, 4, 10).sameContent(DiskVersion(true, 4, 10)))
        assertFalse(DiskVersion(true, 4, 10).sameContent(DiskVersion(true, 4, 11)))
        assertFalse("unknown file sizes never match by size", DiskVersion(true, -1, -1).sameContent(DiskVersion(true, -1, -1)))
        assertTrue(DiskVersion(true, -1, -1, base.sha256).sameContent(base))
    }

    @Test fun `status is derived from the draft base and the disk`() {
        val draft = FileDraft("a", "mine", base, 1)
        assertEquals(FileStatus.CLEAN, Drafts.status(null, base))
        assertEquals(FileStatus.DIRTY, Drafts.status(draft, null))
        assertEquals(FileStatus.DIRTY, Drafts.status(draft, base))
        assertEquals(FileStatus.CONFLICT, Drafts.status(draft, other))
        assertEquals(FileStatus.DELETED, Drafts.status(draft, DiskVersion.MISSING))
        assertEquals(FileStatus.DIRTY, Drafts.status(draft.copy(base = DiskVersion.MISSING), DiskVersion.MISSING))
    }

    @Test fun `edits keep the first base and disappear when the text equals the base`() {
        val first = Drafts.edit(null, "a", "x", base, 5)!!
        assertEquals(base, first.base)
        assertEquals(1, first.revision)
        val second = Drafts.edit(first, "a", "xy", other, 6)!!
        assertEquals("the first shown version stays the base", base, second.base)
        assertEquals(2, second.revision)
        assertSame(second, Drafts.edit(second, "a", "xy", other, 7))
        assertNull(Drafts.edit(second, "a", "base", base, 8))
        assertNotNull("a missing base keeps even an empty draft", Drafts.edit(null, "a", "", DiskVersion.MISSING, 1))
        assertNotNull("unknown hashes keep the draft", Drafts.edit(null, "a", "base", DiskVersion(true, 4, 10), 1))
    }

    @Test fun `disk changes drop drafts that are now saved and rebasing resolves conflicts`() {
        val draft = FileDraft("a", "other", base, 1)
        assertNull(Drafts.afterDiskChange(draft, other))
        assertSame(draft, Drafts.afterDiskChange(draft, DiskVersion.of("third".toByteArray(), 30)))
        val rebased = Drafts.rebase(draft, other, 9)
        assertEquals(other, rebased.base)
        assertEquals(2, rebased.revision)
        assertEquals(Hashing.sha256("other"), rebased.textSha256)
    }

    @Test fun `composer drafts keep the 0_2_1 revision semantics`() {
        val empty = ComposerDraft("c")
        assertSame(empty, empty.edit())
        val a = empty.edit("hi", listOf(ComposerAttachment("x"), ComposerAttachment("x", "image/png")))
        assertEquals(1, a.revision)
        assertEquals(1, a.attachments.size)
        assertNull(ComposerDraft("other", a.revision, a.text, a.attachments).acknowledge(a))
        assertNull(a.edit("hi!").acknowledge(a))
        assertEquals(ComposerDraft("c", 2), a.acknowledge(a))
        assertThrows(IllegalStateException::class.java) { a.copy(revision = Long.MAX_VALUE).edit("z") }
        assertThrows(IllegalArgumentException::class.java) { ComposerDraft("c", -1) }
        assertEquals("file:a", ResourceRef.File("a").key)
        assertEquals("conversation:c", ResourceRef.Conversation("c").key)
    }
}
