package top.flysoftbeta.workflow.core.io

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class FileNamesTest {
    @Test fun `collision variants insert the counter before the extension`() {
        assertEquals("photo.jpg", FileNames.unique("photo.jpg", setOf("other.jpg")))
        assertEquals("photo (1).jpg", FileNames.unique("photo.jpg", setOf("photo.jpg")))
        assertEquals("photo (3).jpg", FileNames.unique("photo.jpg", setOf("photo.jpg", "photo (1).jpg", "photo (2).jpg")))
        assertEquals("backup (1).tar.gz", FileNames.unique("backup.tar.gz", setOf("backup.tar.gz")))
        assertEquals(".bashrc (1)", FileNames.unique(".bashrc", setOf(".bashrc")))
        assertEquals("Makefile (1)", FileNames.unique("Makefile", setOf("Makefile")))
        assertEquals("notes. (1)", FileNames.unique("notes.", setOf("notes.")))
        // Case-sensitive file system: a differently cased name is free.
        assertEquals("Photo.JPG", FileNames.unique("Photo.JPG", setOf("photo.jpg")))
    }

    @Test fun `sanitize removes separators and control characters and falls back when empty`() {
        assertEquals("a_b.txt", FileNames.sanitize("a\\b.txt", "f"))
        assertEquals("evil.sh", FileNames.sanitize("../../evil.sh", "f"))
        assertEquals("x_y", FileNames.sanitize("x\u0001y", "f"))
        assertEquals("file", FileNames.sanitize("  ", "file"))
        assertEquals("file", FileNames.sanitize("..", "file"))
        assertEquals("file", FileNames.sanitize(null, "file"))
        assertEquals("照片 1.jpg", FileNames.sanitize(" 照片 1.jpg ", "f"))
    }

    @Test fun `long names are shortened to 255 UTF-8 bytes keeping the extension and surrogate pairs`() {
        val long = "文".repeat(200) + ".md"
        val short = FileNames.truncate(long)
        assertTrue(short.toByteArray().size <= 255)
        assertTrue(short.endsWith(".md"))
        val emoji = "😀".repeat(100)
        val cut = FileNames.truncate(emoji)
        assertTrue(cut.toByteArray().size <= 255)
        assertEquals(0, cut.length % 2) // no lone surrogate
        assertTrue(FileNames.variants(long).drop(1).first().toByteArray().size <= 255)
    }

    @Test fun `problems name the rule that is violated`() {
        assertNull(FileNames.problem("notes.md"))
        assertNotNull(FileNames.problem(""))
        assertNotNull(FileNames.problem("   "))
        assertNotNull(FileNames.problem("a/b"))
        assertNotNull(FileNames.problem(".."))
        assertNotNull(FileNames.problem("x".repeat(256)))
        assertNotNull(FileNames.problem(".workspace", directory = ""))
        assertNull(FileNames.problem(".workspace", directory = "docs"))
    }
}
