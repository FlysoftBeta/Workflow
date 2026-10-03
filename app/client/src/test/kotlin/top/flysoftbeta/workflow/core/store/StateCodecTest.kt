package top.flysoftbeta.workflow.core.store

import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.core.layout.LayoutPropertyTest
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.PanelView
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.TextCursor
import top.flysoftbeta.workflow.core.layout.Workbench
import top.flysoftbeta.workflow.core.resource.ComposerAttachment
import top.flysoftbeta.workflow.core.resource.ComposerDraft
import top.flysoftbeta.workflow.core.resource.DiskVersion
import top.flysoftbeta.workflow.core.resource.FileDraft
import top.flysoftbeta.workflow.core.session.Session
import top.flysoftbeta.workflow.core.session.UsageStats
import kotlin.random.Random

class StateCodecTest {
    @Test fun `random workbenches round trip through sessions json exactly`() {
        repeat(300) { seed ->
            val random = Random(seed)
            var w = LayoutPropertyTest.randomWorkbench(random, random.nextInt(80))
            w = w.copy(explorer = w.explorer.copy(showHidden = seed % 2 == 0))
            w.panels.keys.firstOrNull()?.let { w = w.apply(LayoutOp.UpdateView(it, PanelView("7", 3, TextCursor(1, 2, 3, 4), mapOf("zoom" to "1.5")))) }
            val session = Session("s$seed", if (seed % 2 == 0) "Named" else null, 5, 9, UsageStats(3, 2.25, 7), if (seed % 3 == 0) 11 else null, w)
            val text = StateCodec.encodeSessions(listOf(session), session.id, listOf(session.id))
            val decoded = StateCodec.decodeSessions(text)
            assertEquals("seed $seed", listOf(session), decoded.sessions)
            assertEquals(session.id, decoded.activeSessionId)
            assertEquals(listOf(session.id), decoded.pinned)
        }
    }

    @Test fun `damaged sessions are skipped and damaged layouts are reset`() {
        val good = Session("good", createdAt = 1, workbench = Workbench.empty().apply(LayoutOp.Open(PanelTarget.Settings)))
        val text = StateCodec.encodeSessions(listOf(good), "gone", listOf("gone", "good"))
            .replace("\"sessions\":[", "\"sessions\":[{\"id\":7},{\"id\":\"bad-layout\",\"createdAt\":3,\"workbench\":{\"panels\":5}},")
        val decoded = StateCodec.decodeSessions(text)
        assertEquals(1, decoded.dropped)
        assertEquals(listOf("bad-layout", "good"), decoded.sessions.map { it.id })
        assertEquals(Workbench.empty(), decoded.sessions.first().workbench)
        assertNull(decoded.activeSessionId)
        assertEquals(listOf("good"), decoded.pinned)
    }

    @Test fun `unknown panel kinds from newer versions are dropped, newer formats are refused`() {
        val w = Workbench.empty().apply(LayoutOp.Open(PanelTarget.File("a"))).apply(LayoutOp.Open(PanelTarget.Settings))
        val text = StateCodec.encodeSessions(listOf(Session("s", createdAt = 1, workbench = w)), null, emptyList())
            .replace("{\"kind\":\"settings\"}", "{\"kind\":\"hologram\"}")
        assertEquals(listOf(PanelTarget.File("a")), StateCodec.decodeSessions(text).sessions.single().workbench.panels.values.map { it.target })
        assertThrows(UnsupportedFormatException::class.java) { StateCodec.decodeSessions("{\"format\":2,\"sessions\":[]}") }
        assertThrows(IllegalArgumentException::class.java) { StateCodec.decodeSessions("{\"format\":0}") }
    }

    @Test fun `drafts and composers round trip and keep file names path-independent`() {
        val draft = FileDraft("dir/中文 name.txt", "text\n\"quoted\"", DiskVersion(true, 3, 4, "abc"), 5, 6)
        assertEquals(draft, StateCodec.decodeDraft(StateCodec.encodeDraft(draft)))
        val missing = draft.copy(base = DiskVersion.MISSING)
        assertEquals(missing, StateCodec.decodeDraft(StateCodec.encodeDraft(missing)))
        val composer = ComposerDraft("conv/../odd id", 4, "hi", listOf(ComposerAttachment("a.png", "image/png"), ComposerAttachment("b.txt")))
        assertEquals(composer, StateCodec.decodeComposer(StateCodec.encodeComposer(composer)))
        assertTrue(StatePaths.composer(composer.conversationId).matches(Regex("\\.workspace/state/composers/[0-9a-f]{40}\\.json")))
        assertTrue(StatePaths.draft(draft.path).matches(Regex("\\.workspace/state/drafts/[0-9a-f]{40}\\.json")))
        assertThrows(UnsupportedFormatException::class.java) { StateCodec.decodeDraft("{\"format\":9}") }
        assertThrows(UnsupportedFormatException::class.java) { StateCodec.decodeComposer("{\"format\":9}") }
    }
}
