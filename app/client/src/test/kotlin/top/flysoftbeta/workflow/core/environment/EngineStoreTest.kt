package top.flysoftbeta.workflow.core.environment

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File
import java.nio.file.Files
import java.nio.file.Paths
import java.time.Instant

class EngineStoreTest {
    @get:Rule val temporary = TemporaryFolder()
    private val image = ImageInfo.parse(sampleIndex())
    private val store by lazy { EngineStore(File(temporary.root, "engine")) { Instant.parse("2026-09-27T00:00:00Z") } }
    private val config = ContainerSpec().resolve(ToolchainDefaults.BUNDLED)
    private val p314 = ToolchainProfile("3.14.7", "24.21.0")
    private val p313 = ToolchainProfile("3.13.15", "24.21.0")
    private val p312 = ToolchainProfile("3.12.11", "22.20.0")

    private fun committed(parent: Long? = null, packages: Set<String> = emptySet()): Long {
        val id = store.allocateGeneration()
        File(store.partialDirectory(id), "rootfs").mkdirs()
        store.writeGeneration(GenerationRecord(id, parent, store.now(), image.ref, packages))
        store.commitGeneration(id)
        return id
    }

    /** Installs the profile's versions into the toolchain store and creates the profile, like envctl. */
    private fun profile(profile: ToolchainProfile, managed: Boolean = true): ToolchainProfile {
        val tools = store.toolchainsDirectory
        val python = "cpython-${profile.python}-linux-x86_64-gnu"
        File(tools, "uv/python/$python/bin").mkdirs()
        File(tools, "nvm/versions/node/v${profile.node}/bin").mkdirs()
        val minor = File(tools, "uv/python/cpython-${profile.python.substringBeforeLast('.')}-linux-x86_64-gnu").toPath()
        Files.deleteIfExists(minor)
        Files.createSymbolicLink(minor, Paths.get(python))
        val directory = File(store.profilesDirectory, profile.name).apply { mkdirs() }
        Files.createSymbolicLink(File(directory, "python").toPath(), Paths.get("../../uv/python/$python"))
        Files.createSymbolicLink(File(directory, "node").toPath(), Paths.get("../../nvm/versions/node/v${profile.node}"))
        if (managed) store.recordManaged(profile.python, profile.node)
        return profile
    }

    @Test fun `generations commit atomically and activations chain`() {
        val first = committed()
        profile(p314)
        assertThrows(IllegalArgumentException::class.java) { store.activate(99, p314, config, ActivationReason.INSTALL) }
        assertThrows(IllegalStateException::class.java) { store.activate(first, p313, config, ActivationReason.INSTALL) }
        val a = store.activate(first, p314, config, ActivationReason.INSTALL)
        val second = committed(first, setOf("jq"))
        val b = store.activate(second, p314, config.copy(packages = listOf("jq")), ActivationReason.CONFIG, running = a)
        assertEquals(2L, b.id)
        assertEquals(b, store.current()!!.activation)
        assertEquals(a, store.previous()!!.activation)
        assertEquals(setOf("jq"), store.current()!!.generation.packages)
        assertEquals("2026-09-27T00:00:00Z", b.activatedAt)
        assertEquals(3L, store.allocateGeneration())
    }

    @Test fun `an intermediate activation that never ran is not kept as previous`() {
        val first = committed()
        profile(p314); profile(p313); profile(p312)
        val running = store.activate(first, p314, config, ActivationReason.INSTALL)
        store.activate(first, p313, config.copy(python = "3.13"), ActivationReason.CONFIG, running)
        val third = store.activate(first, p312, config.copy(python = "3.12", node = "22"), ActivationReason.CONFIG, running)
        assertEquals(running, store.previous()!!.activation)
        assertEquals(4L, third.id.plus(1))
        val collected = store.collectGarbage(running)
        assertEquals(listOf(p313.name), collected.profiles)
        assertEquals(listOf("3.13.15"), collected.pythons)
        assertEquals(emptyList<String>(), collected.nodes)
        assertFalse(File(store.toolchainsDirectory, "uv/python/cpython-3.13.15-linux-x86_64-gnu").exists())
        assertFalse(Files.exists(File(store.toolchainsDirectory, "uv/python/cpython-3.13-linux-x86_64-gnu").toPath(),
            java.nio.file.LinkOption.NOFOLLOW_LINKS))
        assertTrue(File(store.toolchainsDirectory, "uv/python/cpython-3.14.7-linux-x86_64-gnu").isDirectory)
        assertEquals(setOf("3.14.7", "3.12.11") to setOf("24.21.0", "22.20.0"), store.managedToolchains())
    }

    @Test fun `unmanaged toolchains are never removed`() {
        val first = committed()
        profile(p314)
        profile(p313, managed = false)
        val a = store.activate(first, p314, config, ActivationReason.INSTALL)
        File(store.profilesDirectory, p313.name).deleteRecursively()
        assertEquals(emptyList<String>(), store.collectGarbage(a).pythons)
        assertTrue(File(store.toolchainsDirectory, "uv/python/cpython-3.13.15-linux-x86_64-gnu").isDirectory)
    }

    @Test fun `rollback switches back and a successful restart releases only another rootfs`() {
        val first = committed()
        profile(p314); profile(p313)
        val a = store.activate(first, p314, config, ActivationReason.INSTALL)
        val b = store.activate(first, p313, config.copy(python = "3.13"), ActivationReason.CONFIG, a)
        store.startedCurrent(b)
        assertEquals(a, store.previous()!!.activation)   // same rootfs: rollback stays available
        val rolled = store.rollback(b)!!
        assertEquals(p314, rolled.profile)
        assertEquals(config, rolled.config)
        assertEquals(ActivationReason.ROLLBACK, rolled.reason)
        val second = committed(first, setOf("jq"))
        val c = store.activate(second, p314, config.copy(packages = listOf("jq")), ActivationReason.CONFIG, rolled)
        store.startedCurrent(c)
        assertNull(store.previous())
        assertEquals(listOf(first), store.collectGarbage(c).generations)
        assertNull(store.rollback(c))
    }

    @Test fun `active is switched atomically`() {
        profile(p314); profile(p313)
        store.switchActive(p314)
        assertEquals(p314.name, store.activeProfile())
        store.switchActive(p313)
        assertEquals(p313.name, store.activeProfile())
        assertEquals("profiles/${p313.name}", Files.readSymbolicLink(File(store.toolchainsDirectory, "active").toPath()).toString())
        assertThrows(IllegalStateException::class.java) { store.switchActive(p312) }
        assertEquals(listOf("active", "nvm", "profiles", "uv"), store.toolchainsDirectory.list()!!.sorted())
    }

    @Test fun `seeds seed home once and merge toolchains`() {
        val partial = File(temporary.root, "g.partial")
        fun seed(path: String, text: String) = File(partial, "seeds/$path").apply { parentFile.mkdirs(); writeText(text) }
        seed("home/work/.profile", "image")
        seed("toolchains/uv/python/cpython-3.14.7-x/bin/python3", "new")
        seed("toolchains/nvm/alias/default", "24")
        val stores = image.stores
        store.mergeSeeds(partial, stores)
        assertEquals("image", File(store.homeDirectory, ".profile").readText())
        assertFalse(File(partial, "seeds").exists())
        File(store.homeDirectory, ".profile").writeText("mine")
        seed("home/work/.profile", "image v2")
        seed("home/work/.new", "image v2")
        seed("toolchains/uv/python/cpython-3.14.7-x/bin/python3", "replacement")
        seed("toolchains/uv/python/cpython-3.14.9-x/bin/python3", "added")
        seed("toolchains/nvm/alias/default", "26")
        store.mergeSeeds(partial, stores)
        assertEquals("mine", File(store.homeDirectory, ".profile").readText())
        assertFalse(File(store.homeDirectory, ".new").exists())
        assertEquals("new", File(store.toolchainsDirectory, "uv/python/cpython-3.14.7-x/bin/python3").readText())
        assertEquals("added", File(store.toolchainsDirectory, "uv/python/cpython-3.14.9-x/bin/python3").readText())
        assertEquals("24", File(store.toolchainsDirectory, "nvm/alias/default").readText())
    }

    @Test fun `recovery removes partial builds, repairs pointers and collects garbage`() {
        val first = committed()
        profile(p314); profile(p313)
        val a = store.activate(first, p314, config, ActivationReason.INSTALL)
        val second = committed(first)
        store.activate(second, p313, config.copy(env = mapOf("A" to "1")), ActivationReason.CONFIG, a)
        val orphan = committed(second)
        File(store.partialDirectory(9), "rootfs/usr").mkdirs()
        File(store.toolchainsDirectory, ".tmp/envctl.x").mkdirs()
        File(store.profilesDirectory, ".new.abc").mkdirs()
        store.generationDirectory(second).deleteRecursively()
        val recovery = store.recover()
        assertEquals(listOf("9.partial"), recovery.removedPartials)
        assertTrue(recovery.restoredFromPrevious)
        assertEquals(listOf(orphan), recovery.collected.generations)
        assertEquals(listOf(p313.name), recovery.collected.profiles)
        assertEquals(first, store.current()!!.activation.generation)
        assertNull(store.previous())
        assertFalse(File(store.toolchainsDirectory, ".tmp").exists())
        assertFalse(File(store.profilesDirectory, ".new.abc").exists())
        store.generationDirectory(first).deleteRecursively()
        store.recover()
        assertNull(store.current())
    }

    @Test fun `tampered activation files are not trusted`() {
        val first = committed()
        profile(p314)
        store.activate(first, p314, config, ActivationReason.INSTALL)
        val file = File(store.root, "current.json")
        val original = file.readText()
        file.writeText(original.replace("\"python\":\"3.14\"", "\"python\":\"3.13\""))
        assertNull(store.current())
        file.writeText(original.replace(p314.name, "py3.14.7-node../../x"))
        assertNull(store.current())
        file.writeText("{garbage")
        assertNull(store.current())
    }

    @Test fun `failures are remembered until cleared and logs are pruned`() {
        assertNull(store.failure())
        val failure = FailureRecord(config.fingerprint, image.sha256, Stage.PACKAGES, "apt-install exited with status 100", "/log")
        store.recordFailure(failure)
        assertEquals(failure, EngineStore(store.root).failure())
        store.clearFailure()
        assertNull(store.failure())
        store.logsDirectory.mkdirs()
        (1..14).forEach { store.logFile(it.toString()).apply { writeText("line $it\n"); setLastModified(it * 1000L) } }
        store.collectGarbage(null)
        assertEquals((5..14).map { "build-$it.log" }.toSet(), store.logsDirectory.list()!!.toSet())
        assertEquals("line 14", store.logTail(store.logFile("14")))
    }
}
