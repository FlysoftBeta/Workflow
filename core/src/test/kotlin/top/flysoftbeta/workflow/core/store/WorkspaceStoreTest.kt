package top.flysoftbeta.workflow.core.store

import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.core.config.BuiltinApp
import top.flysoftbeta.workflow.core.config.Density
import top.flysoftbeta.workflow.core.config.LauncherEntry
import top.flysoftbeta.workflow.core.config.AppRef
import top.flysoftbeta.workflow.core.config.Launcher
import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.core.io.readText
import top.flysoftbeta.workflow.core.io.writeAtomic
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.Paradigm
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.Region
import top.flysoftbeta.workflow.core.resource.ConflictResolution
import top.flysoftbeta.workflow.core.resource.FileStatus
import top.flysoftbeta.workflow.core.session.SessionKind
import top.flysoftbeta.workflow.core.session.UsageStats

class WorkspaceStoreTest {
    @Test fun `fresh workspace writes default config and creates a temporary session on entry`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        assertEquals(StoreStatus.READY, store.state.value.status)
        assertTrue(h.fs.readText(".workspace/config.json").startsWith("{\n  \"version\": 2,"))
        assertTrue(store.state.value.sessions.isEmpty())
        val id = store.enterWorkbench()
        assertEquals(SessionKind.TEMPORARY, store.state.value.activeSession!!.kind)
        assertEquals(id, store.enterWorkbench())
        h.idle()
        val reopened = h.ready()
        assertEquals(store.state.value.sessions, reopened.state.value.sessions)
        assertEquals(id, reopened.state.value.activeSessionId)
    }

    @Test fun `layout changes persist debounced as one write and survive reload`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        val id = store.createSession("Research")
        h.idle()
        h.mem.clearWriteLog()
        repeat(20) { store.layout(LayoutOp.Open(PanelTarget.File("f$it.txt"))) }
        store.layout(LayoutOp.ResizeRegion(Region.EXPLORER, 300.0))
        store.layout(LayoutOp.SwitchParadigm(Paradigm.CHAT))
        h.idle()
        assertEquals(1, h.writes(StatePaths.SESSIONS))
        val restored = h.ready().state.value.session(id)!!.workbench
        assertEquals(store.state.value.session(id)!!.workbench, restored)
        assertEquals(Paradigm.CHAT, restored.paradigm)
        assertEquals(300.0, restored.files.explorer.size, 0.0)
    }

    @Test fun `typing writes only the draft file, debounced, never the session manifest`() = runTest {
        val h = StoreHarness(this)
        h.fs.writeAtomic("notes.txt", "Original")
        val store = h.ready()
        store.createSession()
        store.layout(LayoutOp.Open(PanelTarget.File("notes.txt")))
        val shown = store.openFile("notes.txt")
        h.idle()
        h.mem.clearWriteLog()
        repeat(50) { store.editFile("notes.txt", "Original + ${"x".repeat(it)}", shown.shownVersion) }
        h.idle()
        assertEquals(0, h.writes(StatePaths.SESSIONS))
        assertEquals(1, h.writes(StatePaths.draft("notes.txt")))
        assertEquals(FileStatus.DIRTY, store.state.value.fileStatus("notes.txt"))
        val reopened = h.ready()
        assertEquals("Original + ${"x".repeat(49)}", reopened.state.value.drafts.getValue("notes.txt").text)
        assertEquals("Original", h.fs.readText("notes.txt"))
    }

    @Test fun `continuous typing is still persisted within the maximum delay`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready(StoreOptions(draftDebounceMs = 400, maxWriteDelayMs = 2_000))
        h.idle()
        h.mem.clearWriteLog()
        repeat(20) {
            store.editFile("a.txt", "text $it", top.flysoftbeta.workflow.core.resource.DiskVersion.MISSING)
            testScheduler.advanceTimeBy(300)
        }
        assertTrue(h.writes(StatePaths.draft("a.txt")) >= 2)
    }

    @Test fun `editing back to the disk content removes the draft and its file`() = runTest {
        val h = StoreHarness(this)
        h.fs.writeAtomic("a.txt", "same")
        val store = h.ready()
        val shown = store.openFile("a.txt").shownVersion
        store.editFile("a.txt", "changed", shown)
        h.idle()
        assertNotNull(h.fs.stat(StatePaths.draft("a.txt")))
        store.editFile("a.txt", "same", shown)
        h.idle()
        assertEquals(FileStatus.CLEAN, store.state.value.fileStatus("a.txt"))
        assertNull(h.fs.stat(StatePaths.draft("a.txt")))
    }

    @Test fun `save writes the draft and returns the version to continue from`() = runTest {
        val h = StoreHarness(this)
        h.fs.writeAtomic("a.txt", "v1")
        val store = h.ready()
        val shown = store.openFile("a.txt").shownVersion
        store.editFile("a.txt", "v2", shown)
        val saved = store.saveFile("a.txt") as SaveResult.Saved
        assertEquals("v2", h.fs.readText("a.txt"))
        assertEquals(FileStatus.CLEAN, store.state.value.fileStatus("a.txt"))
        assertEquals(saved.version, store.state.value.disk["a.txt"])
        // Typing after the save is based on the saved version: an external write is still detected.
        store.editFile("a.txt", "v3", saved.version)
        h.external("a.txt", "terminal")
        h.idle()
        assertEquals(FileStatus.CONFLICT, store.state.value.fileStatus("a.txt"))
        assertTrue(store.saveFile("a.txt") is SaveResult.Conflict)
        assertEquals("terminal", h.fs.readText("a.txt"))
        assertTrue(store.saveFile("missing.txt") is SaveResult.Unchanged)
        assertEquals(SaveResult.Saved::class, store.saveFile("new.txt", "created")::class)
        assertEquals("created", h.fs.readText("new.txt"))
    }

    @Test fun `the first keystroke keeps the version shown before an external write`() = runTest {
        val h = StoreHarness(this)
        h.fs.writeAtomic("a.txt", "Original")
        val store = h.ready()
        val shown = store.openFile("a.txt")
        h.fs.writeAtomic("a.txt", "Terminal changed before first key")
        store.editFile("a.txt", "Edited old buffer", shown.shownVersion)
        assertTrue(store.saveFile("a.txt") is SaveResult.Conflict)
        assertEquals("Terminal changed before first key", h.fs.readText("a.txt"))
        assertEquals("Edited old buffer", store.state.value.drafts.getValue("a.txt").text)
    }

    @Test fun `external changes refresh clean files, conflict dirty ones, and resolve both ways`() = runTest {
        val h = StoreHarness(this)
        h.fs.writeAtomic("clean.txt", "1")
        h.fs.writeAtomic("dirty.txt", "1")
        val store = h.ready()
        store.createSession()
        store.layout(LayoutOp.Open(PanelTarget.File("clean.txt")))
        store.layout(LayoutOp.Open(PanelTarget.File("dirty.txt")))
        val clean = store.openFile("clean.txt")
        store.editFile("dirty.txt", "mine", store.openFile("dirty.txt").shownVersion)
        h.idle()
        h.external("clean.txt", "2")
        h.external("dirty.txt", "theirs")
        h.idle()
        assertNotEquals(clean.disk, store.state.value.disk["clean.txt"])
        assertEquals(FileStatus.CLEAN, store.state.value.fileStatus("clean.txt"))
        assertEquals(FileStatus.CONFLICT, store.state.value.fileStatus("dirty.txt"))
        store.resolveConflict("dirty.txt", ConflictResolution.KEEP_MINE)
        assertEquals(FileStatus.DIRTY, store.state.value.fileStatus("dirty.txt"))
        assertTrue(store.saveFile("dirty.txt") is SaveResult.Saved)
        assertEquals("mine", h.fs.readText("dirty.txt"))
        store.editFile("dirty.txt", "again", store.state.value.disk.getValue("dirty.txt"))
        h.external("dirty.txt", "other")
        h.idle()
        store.resolveConflict("dirty.txt", ConflictResolution.TAKE_DISK)
        assertEquals(FileStatus.CLEAN, store.state.value.fileStatus("dirty.txt"))
        assertEquals("other", store.openFile("dirty.txt").text)
    }

    @Test fun `an external write equal to the draft makes the file clean`() = runTest {
        val h = StoreHarness(this)
        h.fs.writeAtomic("a.txt", "1")
        val store = h.ready()
        store.createSession()
        store.layout(LayoutOp.Open(PanelTarget.File("a.txt")))
        store.editFile("a.txt", "2", store.openFile("a.txt").shownVersion)
        h.idle()
        h.external("a.txt", "2")
        h.idle()
        assertEquals(FileStatus.CLEAN, store.state.value.fileStatus("a.txt"))
    }

    @Test fun `deleted sources keep drafts, even empty ones, until the user decides`() = runTest {
        val h = StoreHarness(this)
        h.fs.writeAtomic("a.txt", "content")
        val store = h.ready()
        store.createSession()
        store.layout(LayoutOp.Open(PanelTarget.File("a.txt")))
        val shown = store.openFile("a.txt").shownVersion
        h.external("a.txt", null)
        h.idle()
        store.editFile("a.txt", "", shown)
        h.idle()
        assertEquals(FileStatus.DELETED, store.state.value.fileStatus("a.txt"))
        assertTrue(store.saveFile("a.txt") is SaveResult.Conflict)
        assertNull(h.fs.stat("a.txt"))
        store.resolveConflict("a.txt", ConflictResolution.KEEP_MINE)
        assertTrue(store.saveFile("a.txt", "restored") is SaveResult.Saved)
        assertEquals("restored", h.fs.readText("a.txt"))
    }

    @Test fun `binary and oversized files are not opened as text`() = runTest {
        val h = StoreHarness(this)
        h.fs.writeAtomic("bin.dat", byteArrayOf(1, 0, 2))
        h.fs.writeAtomic("bad.txt", byteArrayOf(0xC3.toByte(), 0x28))
        h.fs.writeAtomic("big.txt", "x".repeat(100))
        val store = h.ready(StoreOptions(maxEditableBytes = 50))
        assertTrue(store.openFile("bin.dat").binary)
        assertTrue(store.openFile("bad.txt").binary)
        assertTrue(store.openFile("big.txt").tooLarge)
        h.fs.writeAtomic("zh.txt", "中文")
        assertEquals("中文", store.openFile("zh.txt").text)
        assertEquals(FileStatus.CLEAN, store.openFile("nothing.txt").status)
    }

    @Test fun `app state and escaping paths are not editable`() = runTest {
        val store = StoreHarness(this).ready()
        for (path in listOf(".workspace/state/sessions.json", ".workspace/x", "../x", "/etc/passwd", "")) {
            assertTrue(path, runCatching { store.openFile(path) }.exceptionOrNull() is IllegalArgumentException)
            assertTrue(path, runCatching { store.saveFile(path, "x") }.exceptionOrNull() is IllegalArgumentException)
        }
        assertEquals(FileStatus.CLEAN, store.openFile(".workspace/services/proxy/config.yaml").status)
        assertTrue(runCatching { store.editComposer(store.state.value.composer("c").edit("x", listOf(top.flysoftbeta.workflow.core.resource.ComposerAttachment(".workspace/state/meta.json"))), 0) }.exceptionOrNull() is IllegalArgumentException)
        assertFalse(store.state.value.composer("c").hasContent)
    }

    @Test fun `listing hides app directories and dot files unless asked`() = runTest {
        val h = StoreHarness(this)
        h.fs.writeAtomic(".hidden", "")
        h.fs.writeAtomic("src/a.kt", "")
        h.fs.writeAtomic("b.txt", "")
        h.fs.writeAtomic(".state/state.json", "{}")
        val store = h.ready()
        assertEquals(listOf("src", "b.txt"), store.listDirectory("").map { it.name })
        assertEquals(listOf(".state", "src", ".hidden", "b.txt"), store.listDirectory("", showHidden = true).map { it.name })
    }

    @Test fun `move and delete keep drafts, panels and attachments consistent`() = runTest {
        val h = StoreHarness(this)
        h.fs.writeAtomic("docs/a.md", "A")
        h.fs.writeAtomic("docs/b.md", "B")
        val store = h.ready()
        val session = store.createSession()
        store.layout(LayoutOp.Open(PanelTarget.File("docs/a.md")))
        store.layout(LayoutOp.Open(PanelTarget.File("docs/b.md")))
        store.editFile("docs/a.md", "A draft", store.openFile("docs/a.md").shownVersion)
        store.editComposer(store.state.value.composer("c1").edit("see", listOf(top.flysoftbeta.workflow.core.resource.ComposerAttachment("docs/b.md"))), 0)
        assertEquals(FileOpResult.Done, store.movePath("docs", "notes"))
        val state = store.state.value
        assertEquals(setOf("notes/a.md"), state.drafts.keys)
        assertEquals(setOf(PanelTarget.File("notes/a.md"), PanelTarget.File("notes/b.md")), state.session(session)!!.workbench.panels.values.map { it.target }.toSet())
        assertEquals("notes/b.md", state.composer("c1").attachments.single().path)
        h.idle()
        assertNull(h.fs.stat(StatePaths.draft("docs/a.md")))
        assertEquals("A draft", h.ready().state.value.drafts.getValue("notes/a.md").text)
        assertEquals(FileOpResult.Done, store.deletePath("notes"))
        val after = store.state.value
        assertEquals(listOf(PanelTarget.File("notes/a.md")), after.session(session)!!.workbench.panels.values.map { it.target })
        assertEquals(FileStatus.DELETED, after.fileStatus("notes/a.md"))
        assertTrue(store.movePath("nothing", "x") is FileOpResult.Failed)
        assertTrue(store.createFile("notes2/new.txt", "n".toByteArray()) is FileOpResult.Done)
        assertTrue(store.createFile("notes2/new.txt") is FileOpResult.Failed)
        assertTrue(store.createDirectory(".workspace/state/x") is FileOpResult.Failed)
    }

    @Test fun `open in separate session creates an active solo session`() = runTest {
        val store = StoreHarness(this).ready()
        store.createSession("Main")
        val solo = store.openInSeparateSession(PanelTarget.Proxy())
        val session = store.state.value.activeSession!!
        assertEquals(solo, session.id)
        assertEquals(Paradigm.SOLO, session.workbench.paradigm)
        assertEquals(SessionKind.TEMPORARY, session.kind)
    }

    @Test fun `renaming makes a session persistent and pins order the persistent list`() = runTest {
        val store = StoreHarness(this).ready()
        val a = store.createSession()
        val b = store.createSession("B")
        assertTrue(store.renameSession(a, "  A  "))
        assertEquals("A", store.state.value.session(a)!!.name)
        assertThrows(IllegalArgumentException::class.java) { store.state.value.session(a)!!.renamed(" ") }
        assertFalse(store.renameSession("missing", "x"))
        assertTrue(store.pinSession(b, 0))
        assertTrue(store.pinSession(a, 0))
        assertEquals(listOf(a, b), store.state.value.pinned)
        assertTrue(store.unpinSession(a))
        assertEquals(listOf(b), store.state.value.pinned)
        val temp = store.createSession()
        assertFalse(store.pinSession(temp, 0))
    }

    @Test fun `activation counts a use only after a pause`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        val a = store.createSession("A")
        val b = store.createSession("B")
        h.now += 60_000
        store.activateSession(a)
        assertEquals(1, store.state.value.session(a)!!.usage.uses)
        h.now += UsageStats.EPISODE_GAP_MS
        store.activateSession(b)
        store.activateSession(a)
        assertEquals(2, store.state.value.session(a)!!.usage.uses)
        assertEquals(a, store.state.value.activeSessionId)
    }

    @Test fun `config updates keep unknown keys and settings are blocked while the file is invalid`() = runTest {
        val h = StoreHarness(this)
        h.fs.writeAtomic(".workspace/config.json", "{\"version\":2,\"future\":{\"x\":1},\"appearance\":{\"theme\":\"dark\",\"extra\":true}}")
        val store = h.ready()
        assertEquals(top.flysoftbeta.workflow.core.config.ThemeMode.DARK, store.state.value.config.appearance.theme)
        val updated = store.updateConfig { it.copy(appearance = it.appearance.copy(density = Density.STANDARD), launcher = Launcher.add(it.launcher, LauncherEntry.App(AppRef("com.brave.browser")))) }
        assertTrue(updated is ConfigUpdate.Updated)
        val text = h.fs.readText(".workspace/config.json")
        assertTrue(text.contains("\"future\": {"))
        assertTrue(text.contains("\"extra\": true"))
        assertTrue(text.contains("\"density\": \"standard\""))
        assertEquals(listOf("workbench", "proxy", "settings", "com.brave.browser"), store.state.value.config.launcher.map { it.id })

        h.external(".workspace/config.json", "{\"version\":2,\"appearance\":{\"theme\":\"purple\"}}")
        h.idle()
        assertEquals(Density.STANDARD, store.state.value.config.appearance.density)
        assertTrue(store.state.value.configProblem!!.contains("appearance.theme"))
        assertTrue(store.updateConfig { it } is ConfigUpdate.Blocked)
        // Restart with the broken file: the last good configuration is used.
        val restarted = h.ready()
        assertEquals(Density.STANDARD, restarted.state.value.config.appearance.density)
        assertNotNull(restarted.state.value.configProblem)
        h.external(".workspace/config.json", "{\"version\":2}")
        h.idle()
        assertNull(store.state.value.configProblem)
        assertEquals(Density.COMPACT, store.state.value.config.appearance.density)
    }

    @Test fun `editor saves of config json are validated and keep the draft when invalid`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        val original = h.fs.readText(".workspace/config.json")
        val shown = store.openFile(".workspace/config.json").shownVersion
        store.editFile(".workspace/config.json", "{unfinished", shown)
        assertTrue(store.saveFile(".workspace/config.json") is SaveResult.Invalid)
        assertEquals(original, h.fs.readText(".workspace/config.json"))
        assertTrue(store.saveFile(".workspace/config.json", "{\"version\":2,\"appearance\":{\"density\":\"standard\"}}") is SaveResult.Saved)
        assertEquals(Density.STANDARD, store.state.value.config.appearance.density)
        val withValidator = h.ready(StoreOptions(validators = mapOf(".workspace/env.json" to { text -> if ("bad" in text) "env.json: bad" else null })))
        assertEquals(SaveResult.Invalid("env.json: bad"), withValidator.saveFile(".workspace/env.json", "bad"))
    }

    @Test fun `unsupported config remains untouched and is never copied or upgraded`() = runTest {
        val h = StoreHarness(this)
        val original = """{"schemaVersion":1,"theme":"light","model":"old"}"""
        h.fs.writeAtomic(".workspace/config.json", original)
        val store = h.ready()
        assertNotNull(store.state.value.configProblem)
        assertEquals(top.flysoftbeta.workflow.core.config.AppConfig(), store.state.value.config)
        assertTrue(store.updateConfig { it } is ConfigUpdate.Blocked)
        assertEquals(original, h.fs.readText(".workspace/config.json"))
        assertNull(h.fs.stat(StatePaths.CONFIG_LAST_GOOD))
        assertNull(h.fs.stat(".workspace/state/legacy"))
    }

    @Test fun `old state directories are ordinary files and never imported or moved`() = runTest {
        val h = StoreHarness(this)
        val original = """{"sessions":[{"id":"old-session"}]}"""
        h.fs.writeAtomic(".state/state.json", original)
        h.fs.writeAtomic("config.json.bak", "unrelated backup")
        val store = h.ready()
        assertTrue(store.state.value.sessions.isEmpty())
        assertTrue(store.state.value.drafts.isEmpty())
        assertEquals(original, store.openFile(".state/state.json").text)
        assertEquals("unrelated backup", h.fs.readText("config.json.bak"))
        assertNull(h.fs.stat(".workspace/state/legacy-0.2.1.json"))
    }

    @Test fun `write failures are reported and retried`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        var failing = true
        h.mem.failWrite = { failing && it.startsWith(StatePaths.DRAFTS) }
        store.editFile("a.txt", "draft", top.flysoftbeta.workflow.core.resource.DiskVersion.MISSING)
        testScheduler.advanceTimeBy(500)
        testScheduler.runCurrent()
        assertNotNull(store.state.value.writeError)
        failing = false
        h.idle()
        assertNull(store.state.value.writeError)
        assertNotNull(h.fs.stat(StatePaths.draft("a.txt")))
    }

    @Test fun `flush writes immediately`() = runTest {
        val h = StoreHarness(this)
        val store = h.ready()
        store.editFile("a.txt", "x", top.flysoftbeta.workflow.core.resource.DiskVersion.MISSING)
        assertTrue(store.flush())
        assertNotNull(h.fs.stat(StatePaths.draft("a.txt")))
        store.close()
    }

    @Test fun `referenced terminals are reported for runtime cleanup`() = runTest {
        val store = StoreHarness(this).ready()
        store.createSession()
        store.layout(LayoutOp.Open(PanelTarget.Terminal("t1")))
        store.layout(LayoutOp.Open(PanelTarget.Terminal("t2")))
        assertEquals(setOf("t1", "t2"), store.awaitIdle().referencedTerminals)
    }

    @Test fun `builtin workbench cannot be removed from the launcher`() {
        val list = Launcher.remove(Launcher.DEFAULT, BuiltinApp.WORKBENCH.id)
        assertEquals(Launcher.DEFAULT, list)
    }
}

/** Waits until previously posted commands have run. */
internal suspend fun WorkspaceStore.awaitIdle(): WorkspaceState { flush(); return state.value }
