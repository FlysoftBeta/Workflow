package top.flysoftbeta.workflow.platform.agent

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

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

}
