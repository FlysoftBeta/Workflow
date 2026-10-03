package top.flysoftbeta.workflow.platform.proxy

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import java.util.UUID
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.proxy.config.MihomoConfigTemplate
import top.flysoftbeta.workflow.proxy.runtime.ProxyPhase
import top.flysoftbeta.workflow.proxy.runtime.RootStatus

/**
 * Safe on any device, including the user's tablet: never invokes su, never starts a proxy listener or TUN,
 * never touches the production proxy directory.
 */
@RunWith(AndroidJUnit4::class)
class ProxyExecutableTest {
    private val context = InstrumentationRegistry.getInstrumentation().targetContext

    private fun command(directory: File, vararg args: String): Pair<Int, String> {
        val child = ProcessBuilder(args.toList()).directory(directory).redirectErrorStream(true).start()
        try {
            assertTrue("Native command timed out", child.waitFor(20, TimeUnit.SECONDS))
            return child.exitValue() to child.inputStream.bufferedReader().use { it.readText() }
        } finally { if (child.isAlive) child.destroyForcibly() }
    }

    @Test fun packagedKernelValidatesTheSafeTemplateAndGuardianRequiresRoot() {
        val kernel = ProxyService.kernel(context)
        val guardian = ProxyService.guardian(context)
        assertTrue("libmihomo.so must ship for this ABI", kernel.isFile && kernel.canExecute())
        assertTrue(guardian.isFile && guardian.canExecute())
        val fixture = File(context.noBackupFilesDir, "proxy-smoke-${UUID.randomUUID()}").apply { mkdirs() }
        try {
            val version = command(fixture, kernel.path, "-v")
            assertEquals(version.second, 0, version.first)
            assertTrue(version.second, version.second.contains("1.19.31"))
            // The starter template, with TUN switched on, must be accepted by the real kernel unchanged.
            val template = MihomoConfigTemplate.render(MihomoConfigTemplate.newSecret())
            for ((name, text) in listOf("off" to template, "on" to template.replace("  enable: false\n  device:", "  enable: true\n  device:"))) {
                val config = File(fixture, "config-$name.yaml").apply { writeText(text) }
                val validation = command(fixture, kernel.path, "-t", "-d", fixture.path, "-f", config.path)
                assertEquals("template tun=$name: ${validation.second}", 0, validation.first)
            }
            val refused = command(fixture, guardian.path)
            assertEquals(126, refused.first)
            assertTrue(refused.second, refused.second.contains("Root uid is required"))
        } finally { fixture.deleteRecursively() }
    }

    @Test fun engineConfigurationAndServiceReportsNeedNoRoot() = runBlocking<Unit> {
        val engine = EngineProxyFixture.connect(context)
        try {
            val service = engine.bind(ProxyService.ports(context))
            val state = service.refresh().getOrThrow()
            assertTrue(state.capabilities.kernel && state.capabilities.guardian)
            assertEquals(RootStatus.UNKNOWN, state.capabilities.root)
            assertNotEquals(ProxyPhase.RUNNING, state.phase)
            assertEquals(File(engine.staging, "config.yaml").canonicalPath, service.configFile.canonicalPath)
            assertEquals(false, (engine.serviceState()["config"] as Map<*, *>)["exists"])

            service.ensureConfig().getOrThrow()
            val original = engine.documents.read("services.proxy", "config.yaml")
            assertTrue("Canonical document/file bytes differ", original.text == engine.read(ProxyService.CONFIG_PATH))
            assertTrue("Initial staging differs from canonical document", original.text == service.configFile.readText())
            assertTrue(service.checkConfig().getOrThrow().valid)

            val edited = original.text!! + "# Canonical editor change\n"
            assertTrue(engine.session.store.saveFile(ProxyService.CONFIG_PATH, edited) is top.flysoftbeta.workflow.core.store.SaveResult.Saved)
            val conflict = runCatching { engine.documents.write("services.proxy", "config.yaml", original.text, original.revision) }
            assertTrue("An editor save must invalidate document CAS", conflict.isFailure)
            service.refresh().getOrThrow()
            assertTrue("Editor bytes did not replace staging", edited == service.configFile.readText())
            assertTrue("Document did not follow editor save", edited == engine.documents.read("services.proxy", "config.yaml").text)
            assertEquals(true, (engine.serviceState()["config"] as Map<*, *>)["exists"])
            assertEquals(RootStatus.UNKNOWN, service.state.value.capabilities.root)

            engine.disconnect()
            assertTrue("Disconnected Engine cannot start from cached YAML", service.start().isFailure)
            assertEquals(RootStatus.UNKNOWN, service.state.value.capabilities.root)
        } finally { engine.close() }
    }
}
