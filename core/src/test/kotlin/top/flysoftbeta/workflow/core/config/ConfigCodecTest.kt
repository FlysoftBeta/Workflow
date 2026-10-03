package top.flysoftbeta.workflow.core.config

import org.junit.Assert.*
import org.junit.Test

class ConfigCodecTest {
    private fun ok(text: String) = ConfigCodec.decode(text) as ConfigParse.Ok
    private fun invalid(text: String) = ConfigCodec.decode(text) as ConfigParse.Invalid

    @Test fun `defaults round trip with pretty output`() {
        val text = ConfigCodec.encode(AppConfig())
        assertEquals(AppConfig(), ok(text).config)
        assertTrue(text.startsWith("{\n  \"version\": 2,\n  \"appearance\": {\n    \"theme\": \"system\""))
        assertTrue(text.contains("\"launcher\": [\"workbench\", \"proxy\", \"settings\"]"))
        assertTrue(text.contains("\"fontScale\": 1,"))
        assertTrue(invalid("{}").message.contains("version"))
        assertEquals(AppConfig(), ok("{\"version\":2}").config)
    }

    @Test fun `every field round trips`() {
        val config = AppConfig(
            appearance = Appearance(ThemeMode.DARK, Density.STANDARD, 1.15, 14.5),
            agent = AgentDefaults("claude", "ask", mapOf("codex" to BackendDefaults("gpt-5", "high"), "claude" to BackendDefaults(effort = "low"))),
            overlay = OverlayConfig(true, listOf(AppRef("com.example.notes"), AppRef("com.example.cam", ".Main"))),
            launcher = listOf(LauncherEntry.Builtin(BuiltinApp.SETTINGS), LauncherEntry.Builtin(BuiltinApp.WORKBENCH), LauncherEntry.Builtin(BuiltinApp.PROXY), LauncherEntry.App(AppRef("org.mozilla.firefox", "org.mozilla.fenix.HomeActivity"))),
            terminal = TerminalConfig(extraKeysPinned = true),
        )
        assertEquals(config, ok(ConfigCodec.encode(config)).config)
    }

    @Test fun `unknown keys and their order are preserved, removed optional keys disappear`() {
        val original = ok("{\"z\":1,\"version\":2,\"appearance\":{\"theme\":\"dark\",\"custom\":[1,2]},\"agent\":{\"permissions\":\"ask\",\"backends\":{\"old\":{\"model\":\"m\",\"note\":\"x\"}}}}")
        val text = ConfigCodec.encode(original.config.copy(agent = original.config.agent.copy(permissions = null, backends = emptyMap())), original.document)
        val reparsed = ok(text)
        assertEquals(listOf("version", "z", "appearance", "agent", "overlay", "launcher", "terminal"), reparsed.document.keys.toList())
        @Suppress("UNCHECKED_CAST")
        val appearance = reparsed.document["appearance"] as Map<String, Any?>
        assertEquals(listOf(1L, 2L), appearance["custom"])
        assertFalse(text.contains("permissions"))
        assertFalse(text.contains("\"old\""))
    }

    @Test fun `invalid values report key paths`() {
        assertTrue(invalid("{").message.startsWith("syntax"))
        assertTrue(invalid("[]").message.contains("object"))
        assertTrue(invalid("{\"a\":1,\"a\":2}").message.contains("Duplicate"))
        assertTrue(invalid("{\"version\":3}").message.contains("version"))
        val problems = invalid("{\"version\":2,\"appearance\":{\"theme\":\"purple\",\"fontScale\":9,\"monoFontSize\":\"big\"},\"launcher\":[\"not a package\"],\"overlay\":{\"enabled\":\"yes\"},\"agent\":{\"backend\":\"\"}}").problems
        assertTrue(problems.any { it.startsWith("appearance.theme") })
        assertTrue(problems.any { it.startsWith("appearance.fontScale") })
        assertTrue(problems.any { it.startsWith("appearance.monoFontSize") })
        assertTrue(problems.any { it.startsWith("launcher") })
        assertTrue(problems.any { it.startsWith("overlay.enabled") })
        assertTrue(problems.any { it.startsWith("agent.backend") })
        assertTrue(invalid("{\"version\":2,\"agent\":{\"backends\":{\"codex\":3}}}").message.contains("agent.backends.codex"))
        assertTrue(invalid("{\"version\":2,\"terminal\":[]}").message.contains("terminal"))
    }

    @Test fun `unversioned and old schema documents are refused`() {
        assertTrue(invalid("""{"schemaVersion":1,"theme":"dark","model":"old"}""").message.contains("version"))
        assertTrue(invalid("""{"version":1}""").message.contains("version"))
    }

    @Test fun `launcher keeps all builtins, de-duplicates and reorders`() {
        val brave = LauncherEntry.App(AppRef("com.brave.browser"))
        assertEquals(listOf("workbench", "proxy", "settings"), Launcher.normalize(listOf(LauncherEntry.Builtin(BuiltinApp.PROXY), LauncherEntry.Builtin(BuiltinApp.PROXY))).map { it.id })
        val added = Launcher.add(Launcher.DEFAULT, brave)
        assertSame(added, Launcher.add(added, brave))
        assertEquals(listOf("com.brave.browser", "workbench", "proxy", "settings"), Launcher.move(added, brave.id, -5).map { it.id })
        assertSame(added, Launcher.remove(added, "proxy"))
        assertSame(added, Launcher.remove(added, "settings"))
        assertSame(added, Launcher.move(added, "missing", 0))
        val config = AppConfig(launcher = added, overlay = OverlayConfig(extraApps = listOf(AppRef("com.example.cam"), AppRef("com.brave.browser"))))
        assertEquals(listOf("workbench", "proxy", "settings", "com.brave.browser", "com.example.cam"), Launcher.overlayApps(config).map { it.id })
        assertEquals(AppRef("a.b", "c.D"), AppRef.parse("a.b/c.D"))
        assertThrows(IllegalArgumentException::class.java) { AppRef("nodots") }
        assertEquals(LauncherEntry.Builtin(BuiltinApp.PROXY), LauncherEntry.parse("proxy"))
    }
}
