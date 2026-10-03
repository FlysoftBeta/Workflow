package top.flysoftbeta.workflow.core.io

import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File
import java.io.IOException
import java.nio.file.Files

class FileSystemTest {
    @get:Rule val temporary = TemporaryFolder()

    private fun jvm(): Pair<File, JvmFileSystem> {
        val root = File(temporary.root, ".workspace")
        val fs = JvmFileSystem(root)
        assertFalse("construction does no IO", root.exists())
        fs.root // first use creates the workspace directory
        return root to fs
    }

    @Test fun `paths normalize and reject escapes`() {
        assertEquals("a/b", WorkspacePaths.normalize("a//./b/"))
        assertEquals("", WorkspacePaths.normalize("."))
        assertNull(WorkspacePaths.normalizeOrNull("../a"))
        assertNull(WorkspacePaths.normalizeOrNull("a/../../b"))
        assertNull(WorkspacePaths.normalizeOrNull("/abs"))
        assertTrue(WorkspacePaths.isValidEntry("a/b.txt"))
        assertFalse(WorkspacePaths.isValidEntry("a//b"))
        assertFalse(WorkspacePaths.isValidEntry(""))
        assertEquals("docs/x/y", WorkspacePaths.rebase("a/x/y", "a", "docs"))
        assertNull(WorkspacePaths.rebase("ab/x", "a", "docs"))
        assertEquals("b", WorkspacePaths.rebase("a", "a", "b"))
        assertTrue(WorkspacePaths.isReserved(".workspace/state/sessions.json"))
        assertFalse(WorkspacePaths.isReserved(".state/state.json"))
        assertFalse(WorkspacePaths.isReserved(".workspace/proxy/config.yaml"))
        assertFalse(WorkspacePaths.isReserved(".workspacex/a"))
        assertTrue(WorkspacePaths.isHiddenInExplorer(".workspace/proxy"))
        assertEquals("a/b", WorkspacePaths.child("a", "b"))
        assertThrows(IllegalArgumentException::class.java) { WorkspacePaths.child("a", "b/c") }
    }

    @Test fun `atomic writes replace content, create parents and leave no temp files`() {
        val (root, fs) = jvm()
        fs.writeAtomic("dir/sub/a.txt", "one")
        fs.writeAtomic("dir/sub/a.txt", "two")
        assertEquals("two", fs.readText("dir/sub/a.txt"))
        assertEquals(listOf("a.txt"), File(root, "dir/sub").list()!!.toList())
        assertTrue(File(root, WorkspacePaths.TMP).list()!!.isEmpty())
        assertEquals(FileStat(false, 3, File(root, "dir/sub/a.txt").lastModified()), fs.stat("dir/sub/a.txt"))
        assertTrue(fs.stat("dir")!!.isDirectory)
        assertNull(fs.stat("missing"))
        assertThrows(IOException::class.java) { fs.writeAtomic("dir", "x") }
    }

    @Test fun `a crash before the rename keeps the previous content`() {
        val root = File(temporary.root, ".workspace")
        val fs = object : JvmFileSystem(root) {
            var crash = false
            override fun commit(temp: File, target: File) {
                if (crash) throw IOException("power loss")
                super.commit(temp, target)
            }
        }
        fs.writeAtomic("a.txt", "safe")
        fs.crash = true
        assertThrows(IOException::class.java) { fs.writeAtomic("a.txt", "lost") }
        assertEquals("safe", fs.readText("a.txt"))
        assertTrue(File(root, WorkspacePaths.TMP).list()!!.isEmpty())
        // A failing writer callback also leaves the old content.
        fs.crash = false
        assertThrows(IllegalStateException::class.java) { fs.writeAtomic("a.txt") { error("boom") } }
        assertEquals("safe", fs.readText("a.txt"))
    }

    @Test fun `symlinks cannot escape the workspace`() {
        val (root, fs) = jvm()
        val outside = temporary.newFolder(".workspace-sibling")
        File(outside, "secret.txt").writeText("Outside")
        Files.createSymbolicLink(File(root, "link").toPath(), outside.toPath())
        Files.createSymbolicLink(File(root, "inside").toPath(), File(root, WorkspacePaths.INTERNAL).also { it.mkdirs() }.toPath())
        assertThrows(IllegalArgumentException::class.java) { fs.readBytes("link/secret.txt") }
        assertThrows(IllegalArgumentException::class.java) { fs.writeAtomic("link/new.txt", "bad") }
        assertThrows(IllegalArgumentException::class.java) { fs.readBytes("../.workspace-sibling/secret.txt") }
        assertFalse(fs.list("").any { it.name == "link" })
        assertTrue(fs.list("").any { it.name == "inside" })
    }

    @Test fun `move refuses to overwrite and delete removes trees without following links`() {
        val (root, fs) = jvm()
        fs.writeAtomic("a/one.txt", "1")
        fs.writeAtomic("a/two/three.txt", "3")
        fs.writeAtomic("b.txt", "b")
        assertThrows(IOException::class.java) { fs.move("a/one.txt", "b.txt") }
        assertThrows(IOException::class.java) { fs.move("a", "a/two/inner") }
        fs.move("a", "c/a")
        assertEquals("3", fs.readText("c/a/two/three.txt"))
        val target = temporary.newFolder("keep")
        File(target, "precious.txt").writeText("keep")
        Files.createSymbolicLink(File(root, "c/a/two/link").toPath(), File(root, "b.txt").toPath())
        assertTrue(fs.delete("c"))
        assertFalse(fs.delete("c"))
        assertTrue(File(root, "b.txt").exists())
        assertTrue(File(target, "precious.txt").exists())
        assertThrows(IOException::class.java) { fs.delete("") }
        fs.createDirectories("x/y")
        assertTrue(fs.stat("x/y")!!.isDirectory)
    }

    @Test fun `memory file system mirrors the contract`() {
        var now = 5L
        val fs = MemoryFileSystem { now }
        fs.writeAtomic("a/b.txt", "x")
        assertEquals(listOf("a/b.txt"), fs.list("a").map { it.path })
        assertEquals(5L, fs.stat("a/b.txt")!!.modifiedAt)
        fs.failWrite = { it == "a/b.txt" }
        assertThrows(IOException::class.java) { fs.writeAtomic("a/b.txt", "y") }
        assertEquals("x", fs.readText("a/b.txt"))
        fs.failWrite = { false }
        now = 9
        fs.move("a", "c")
        assertEquals("x", fs.readText("c/b.txt"))
        assertThrows(IOException::class.java) { fs.writeAtomic("c/b.txt/d", "z") }
        assertTrue(fs.delete("c"))
        assertEquals(emptyList<String>(), fs.paths())
        assertThrows(IOException::class.java) { fs.readBytes("c/b.txt") }
    }

    @Test fun `manual watcher notifies the parent directory`() {
        val watcher = ManualFileWatcher()
        val seen = mutableListOf<String?>()
        val handle = watcher.watch("docs") { seen += it }
        watcher.changed("docs/a.md")
        watcher.changed("other/b.md")
        handle.close()
        watcher.changed("docs/c.md")
        assertEquals(listOf<String?>("a.md"), seen)
        assertTrue(watcher.watchedDirectories.isEmpty())
        FileWatcher.NONE.watch("") { }.close()
        assertEquals(64, Hashing.sha256("abc").length)
        assertEquals("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", Hashing.sha256("abc"))
    }
}
