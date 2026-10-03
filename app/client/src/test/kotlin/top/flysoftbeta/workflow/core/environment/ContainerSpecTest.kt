package top.flysoftbeta.workflow.core.environment

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

class ContainerSpecTest {
    private fun parse(text: String) = ContainerSpecCodec.parse(text)
    private fun problems(text: String): List<String> =
        assertThrows(ContainerSpecException::class.java) { parse(text) }.problems.map { it.toString() }

    @Test fun `missing fields mean image defaults and zero configuration is valid`() {
        val parsed = parse("{}")
        assertEquals(ContainerSpec(), parsed.spec)
        assertEquals(ResolvedSpec("3.14", "24", emptyList(), emptyMap()), parsed.spec.resolve(ToolchainDefaults.BUNDLED))
    }

    @Test fun `full document parses and normalizes packages`() {
        val spec = parse("""{"version":1,"python":"3.13","node":"22.1","packages":["ripgrep","jq","jq"],"env":{"PIP_INDEX_URL":"https://m/simple"}}""").spec
        assertEquals(ContainerSpec("3.13", "22.1", setOf("jq", "ripgrep"), mapOf("PIP_INDEX_URL" to "https://m/simple")), spec)
        assertEquals(listOf("jq", "ripgrep"), spec.resolve(ToolchainDefaults.BUNDLED).packages)
    }

    @Test fun `fingerprint ignores ordering and explicit defaults but not content`() {
        val defaults = ToolchainDefaults.BUNDLED
        val a = parse("""{"packages":["bb","aa"],"env":{"Y":"2","X":"1"}}""").spec.resolve(defaults)
        val b = parse("""{"env":{"X":"1","Y":"2"},"version":1,"python":"3.14","node":"24","packages":["aa","bb"]}""").spec.resolve(defaults)
        assertEquals(a.fingerprint, b.fingerprint)
        assertTrue(a.fingerprint.matches(Regex("sha256:[0-9a-f]{64}")))
        assertNotEquals(a.fingerprint, a.copy(env = mapOf("X" to "1")).fingerprint)
        assertEquals(a, ResolvedSpec.fromJson(a.toJson()))
    }

    @Test fun `every problem is reported with its location`() {
        val found = problems("""{"pyhton":"3.12","version":2,"python":"2.7","node":"lts","packages":["Bad_Name",3],"env":{"PATH":"/x","WORKFLOW_A":"1","OK":1,"1BAD":"x"}}""")
        assertTrue(found.toString(), "pyhton: unknown field" in found)
        assertTrue(found.toString(), "version: unsupported version" in found)
        assertTrue(found.toString(), "packages[1]: must be a string" in found)
        assertTrue(found.toString(), "env.OK: must be a string" in found)
        val semantic = problems("""{"python":"2.7","node":"lts","packages":["Bad_Name"],"env":{"PATH":"/x","WORKFLOW_A":"1","1BAD":"x"}}""")
        assertEquals(6, semantic.size)
        assertTrue(semantic.toString(), semantic.any { it.startsWith("python:") } && semantic.any { it.startsWith("node:") })
        assertTrue(semantic.toString(), "env.PATH: reserved variable" in semantic && "env.WORKFLOW_A: reserved variable" in semantic)
        assertTrue(problems("""{"packages":["x"]}""").single().contains("invalid Debian package name 'x'"))
        assertTrue(problems("[]").single().contains("object"))
        assertTrue(problems("{unfinished").single().contains("not valid JSON"))
        assertTrue(problems("""{"packages":"jq"}""").single().contains("array"))
        assertTrue(problems("{\"env\":{\"A\":\"" + "x".repeat(70_000) + "\"}}").single().contains("64 KiB"))
        assertThrows(IllegalArgumentException::class.java) { ContainerSpec(python = "3.x") }
    }

    @Test fun `version prefixes match on dot boundaries`() {
        assertTrue(VersionSpec.satisfies("3.14.7", "3"))
        assertTrue(VersionSpec.satisfies("3.14.7", "3.14"))
        assertTrue(VersionSpec.satisfies("3.14.7", "3.14.7"))
        assertFalse(VersionSpec.satisfies("3.14.7", "3.1"))
        assertFalse(VersionSpec.satisfies("24.21.0", "2"))
        assertFalse(VersionSpec.satisfies(null, "24"))
    }

    @Test fun `old schema fields are rejected rather than migrated`() {
        val errors = problems("""{"schemaVersion":1,"pythonVersion":"3.13","engine":"native","bindMounts":[]}""")
        assertEquals(setOf("schemaVersion", "pythonVersion", "engine", "bindMounts"), errors.map { it.substringBefore(':') }.toSet())
        assertTrue(errors.all { it.endsWith("unknown field") })
    }

    @Test fun `formatted files are explicit and round trip`() {
        val spec = ContainerSpec(packages = setOf("jq"), env = mapOf("B" to "2", "A" to "\"quoted\""))
        val text = ContainerSpecCodec.format(spec)
        assertTrue(text, text.startsWith("{\n  \"version\": 1,\n  \"python\": \"3.14\",\n  \"node\": \"24\",\n  \"packages\": [\"jq\"],"))
        assertEquals(spec.copy(python = "3.14", node = "24"), parse(text).spec)
        assertEquals(ContainerSpec("3.14", "24"), parse(ContainerSpecCodec.format(ContainerSpec())).spec)
    }

    @Test fun `bundled defaults equal image versions env`() {
        var directory: File? = File(System.getProperty("user.dir")).absoluteFile
        while (directory != null && !File(directory, "image/versions.env").exists()) directory = directory.parentFile
        val versions = File(requireNotNull(directory) { "image/versions.env not found" }, "image/versions.env").readLines()
            .filter { '=' in it && !it.startsWith("#") }.associate { it.substringBefore('=') to it.substringAfter('=') }
        assertEquals(versions["DEFAULT_PYTHON"], ToolchainDefaults.BUNDLED.python)
        assertEquals(versions["DEFAULT_NODE"], ToolchainDefaults.BUNDLED.node)
        assertNull(versions["DEFAULT_MISSING"])
    }
}
