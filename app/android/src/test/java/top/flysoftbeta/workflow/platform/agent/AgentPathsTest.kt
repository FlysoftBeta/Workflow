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

    @Test fun agentHomesMapToTheirVisibleWorkspaceConfiguration() {
        assertEquals(".workspace/agents/codex/config.toml", paths.toWorkspace("/home/work/.codex/config.toml"))
        assertEquals(".workspace/agents/claude", paths.toWorkspace("/home/work/.claude"))
        assertEquals(".workspace/agents/claude/commands/fix.md", paths.toWorkspace("file:///home/work/.claude/commands/fix.md"))
        assertEquals("/home/work/.codex/prompts/review.md", paths.toAgent(".workspace/agents/codex/prompts/review.md"))
        assertEquals("/home/work/.claude", paths.toAgent(".workspace/agents/claude"))
        // Other configuration stays at its workspace path, which the guest can read.
        assertEquals("/workspace/.workspace/config.json", paths.toAgent(".workspace/config.json"))
        assertNull(paths.toWorkspace("/home/work/.codex/../.bashrc"))
        assertNull(paths.toWorkspace("/home/work/.codexx/config.toml"))
        assertNull(paths.toWorkspace("/home/work/.bashrc"))
        assertEquals(AgentPaths.Link(".workspace/agents/codex/config.toml", 3, null), paths.parseLink("/home/work/.codex/config.toml:3"))
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
