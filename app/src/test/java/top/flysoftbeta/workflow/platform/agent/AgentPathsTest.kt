package top.flysoftbeta.workflow.platform.agent

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.ConversationEntry

class AgentPathsTest {
    private val paths = AgentPaths("/workspace")

    @Test fun hostRootsAreRejected() {
        org.junit.Assert.assertThrows(IllegalArgumentException::class.java) { AgentPaths("/data/host") }
    }

    @Test fun mapsBothDirections() {
        assertEquals("/workspace/docs/a.md", paths.toAgent("docs/a.md"))
        assertEquals("/workspace", paths.toAgent(""))
        assertEquals("docs/a.md", paths.toWorkspace("/workspace/docs/a.md"))
        assertEquals("docs/a.md", paths.toWorkspace("/workspace/docs/a.md"))
        assertEquals("docs/a.md", paths.toWorkspace("./docs/a.md"))
        assertEquals("docs/a b.md", paths.toWorkspace("file:///workspace/docs/a%20b.md"))
    }

    @Test fun refusesOutsideAndInternalPaths() {
        assertNull(paths.toWorkspace("/etc/passwd"))
        assertNull(paths.toWorkspace("/workspace/../etc/passwd"))
        assertNull(paths.toWorkspace("/workspace/.workspace/state/sessions.json"))
        assertEquals(".state/ordinary.txt", paths.toWorkspace(".state/ordinary.txt"))
        assertNull(paths.toWorkspace("/data/user/0/app/files/.workspace/a.txt"))
        assertNull(paths.parseLink("/workspace/.workspace/agents/codex/auth.json"))
    }

    @Test fun parsesLineAndColumnSuffixes() {
        assertEquals(AgentPaths.Link("app/Main.kt", 12, 3), paths.parseLink("/workspace/app/Main.kt:12:3"))
        assertEquals(AgentPaths.Link("app/Main.kt", 40, null), paths.parseLink("app/Main.kt#L40"))
        assertEquals(AgentPaths.Link("app/Main.kt", 40, 2), paths.parseLink("app/Main.kt#L40C2"))
        assertEquals(AgentPaths.Link("README.md", 7, null), paths.parseLink("README.md", line = 7))
        assertEquals(AgentPaths.Link("README.md", null, null), paths.parseLink("README.md#intro"))
        assertNull(paths.parseLink(""))
    }

    @Test fun indexCodecRoundTripsAndRefusesNewerFormats() {
        val entries = listOf(
            ConversationEntry("a", BackendKind.CODEX, "t1", "标题", "/workspace", 1, 2, archived = true, model = "gpt-5.5", effort = "high", preview = "你好"),
            ConversationEntry("b", BackendKind.CLAUDE, null, null, "/workspace", 3, 4, forkedFrom = "a", forkedAt = "turn-1"),
        )
        assertEquals(entries, ConversationIndexCodec.decode(ConversationIndexCodec.encode(entries)))
        try {
            ConversationIndexCodec.decode("""{"format":2,"conversations":[]}""")
            throw AssertionError("newer format accepted")
        } catch (_: ConversationIndexCodec.NewerFormatException) {
        }
        // Entries with an unknown backend are dropped, the rest survives.
        assertEquals(listOf("x"), ConversationIndexCodec.decode(
            """{"format":1,"conversations":[{"id":"x","backend":"codex","cwd":""},{"id":"y","backend":"other"}]}""",
        ).map { it.id })
    }
}
