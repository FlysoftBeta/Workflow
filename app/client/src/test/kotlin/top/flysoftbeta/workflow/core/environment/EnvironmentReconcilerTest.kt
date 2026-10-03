package top.flysoftbeta.workflow.core.environment

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import top.flysoftbeta.workflow.core.json.Json
import top.flysoftbeta.workflow.core.json.jsonObject
import top.flysoftbeta.workflow.core.json.list
import java.io.File
import java.nio.file.Files
import java.nio.file.Paths

private val RESOLVE = mapOf("3.12" to "3.12.11", "3.13" to "3.13.15", "3.14" to "3.14.7", "22" to "22.20.0", "24" to "24.21.0")

/**
 * Simulates the engine plus envctl: the rootfs package set lives in `<generation>/packages.json`,
 * toolchains are real directories and profile symlinks in the store, like the guest would create.
 */
private class FakeEngine(private val image: ImageInfo, private val tools: File) : EnvironmentEngine {
    val calls = mutableListOf<String>()
    var failOperation: String? = null
    var wrongVerify = false
    @Volatile var gate: CompletableDeferred<Unit>? = null
    @Volatile var entered = CompletableDeferred<Unit>()

    private fun packages(root: File) = Json.parse(File(root, "packages.json").readText()).jsonObject().list("packages").map { it as String }.toMutableSet()
    private fun writePackages(root: File, packages: Collection<String>) = File(root, "packages.json").writeText(Json.stringify(mapOf("packages" to packages.sorted())))
    private fun pythonDir(base: File, version: String) = File(base, "uv/python/cpython-$version-linux-x86_64-gnu")
    private fun nodeDir(base: File, version: String) = File(base, "nvm/versions/node/v$version")
    private fun versions(directory: File, prefix: String, suffix: String) = directory.list().orEmpty()
        .filter { it.startsWith(prefix) }.map { it.removePrefix(prefix).removeSuffix(suffix) }
    private fun best(installed: List<String>, spec: String) = installed.filter { VersionSpec.satisfies(it, spec) }
        .maxWithOrNull(compareBy<String>({ it.split('.')[0].toInt() }, { it.split('.')[1].toInt() }, { it.split('.')[2].toInt() }))

    override suspend fun install(image: ImageInfo, target: File, log: (String) -> Unit) {
        calls += "install"
        check(!target.exists())
        File(target, "rootfs/opt/toolchains").mkdirs()
        writePackages(target, emptyList())
        val seed = File(target, "seeds/toolchains")
        File(pythonDir(seed, image.profile.python), "bin").mkdirs()
        File(nodeDir(seed, image.profile.node), "bin").mkdirs()
        val profile = File(seed, "profiles/${image.profile.name}").apply { mkdirs() }
        Files.createSymbolicLink(File(profile, "python").toPath(), Paths.get("../../uv/python/cpython-${image.profile.python}-linux-x86_64-gnu"))
        Files.createSymbolicLink(File(profile, "node").toPath(), Paths.get("../../nvm/versions/node/v${image.profile.node}"))
        Files.createSymbolicLink(File(seed, "active").toPath(), Paths.get("profiles/${image.profile.name}"))
        File(target, "seeds/home/work").mkdirs()
        File(target, "seeds/home/work/.profile").writeText("seed")
    }

    override suspend fun clone(source: File, target: File, log: (String) -> Unit) {
        calls += "clone ${source.name}"
        check(!target.exists())
        source.copyRecursively(target)
        File(target, "generation.json").delete()
    }

    override suspend fun run(root: File, command: GuestCommand, environment: Map<String, String>, log: (String) -> Unit,
                             stdout: StringBuilder): Int {
        entered.complete(Unit)
        gate?.await()
        check(root.isDirectory) { "no guest root" }
        val operation = command.argv[1]
        val arguments = command.argv.drop(2)
        calls += (listOf(command.user.name.lowercase(), operation) + arguments).joinToString(" ")
        check(environment["PATH"] == image.environment["PATH"])
        if (operation == failOperation) {
            log("E: simulated failure")
            return 100
        }
        val pythons = versions(File(tools, "uv/python"), "cpython-", "-linux-x86_64-gnu")
        val nodes = versions(File(tools, "nvm/versions/node"), "v", "")
        when (operation) {
            "apt-install" -> writePackages(root, packages(root) + arguments)
            "apt-remove" -> writePackages(root, packages(root) - arguments.toSet())
            "python" -> {
                val found = best(pythons, arguments[0])
                val version = found ?: RESOLVE.getValue(arguments[0]).also { File(pythonDir(tools, it), "bin").mkdirs() }
                stdout.append(Json.stringify(mapOf("python" to version, "installed" to (found == null))))
            }
            "node" -> {
                val found = best(nodes, arguments[0])
                val version = found ?: RESOLVE.getValue(arguments[0]).also { File(nodeDir(tools, it), "bin").mkdirs() }
                stdout.append(Json.stringify(mapOf("node" to version, "installed" to (found == null))))
            }
            "verify" -> {
                val python = best(pythons, arguments[0]) ?: return 2
                val node = best(nodes, arguments[1]) ?: return 2
                val profile = ToolchainProfile(python, node)
                val directory = File(tools, "profiles/${profile.name}")
                if (!directory.exists()) {
                    directory.mkdirs()
                    Files.createSymbolicLink(File(directory, "python").toPath(), Paths.get("../../uv/python/cpython-$python-linux-x86_64-gnu"))
                    Files.createSymbolicLink(File(directory, "node").toPath(), Paths.get("../../nvm/versions/node/v$node"))
                }
                val installed = packages(root)
                stdout.append(Json.stringify(mapOf("profile" to profile.name, "python" to if (wrongVerify) "3.11.0" else python,
                    "node" to node, "packages" to arguments.drop(2).associateWith { if (it in installed) "1.0" else null })))
            }
        }
        return 0
    }
}

class EnvironmentReconcilerTest {
    @get:Rule val temporary = TemporaryFolder()
    private val image = ImageInfo.parse(sampleIndex())
    private val store by lazy { EngineStore(File(temporary.root, "engine")) }
    private val engine by lazy { FakeEngine(image, store.toolchainsDirectory) }

    private fun reconciler(info: ImageInfo = image, fake: FakeEngine = engine) = EnvironmentReconciler(store, fake, info)
    private fun valid(spec: ContainerSpec = ContainerSpec()) = DesiredConfig.Valid(spec)
    private fun generations() = store.generationsDirectory.list().orEmpty().sorted()

    private suspend fun started(environment: EnvironmentReconciler): Activation =
        environment.prepareStart()!!.also { environment.instanceStarted(it) }

    @Test fun `zero configuration installs the image, seeds the stores and is up to date`() = runBlocking {
        val environment = reconciler()
        environment.start()
        assertEquals(EnvironmentStatus.Applying(Stage.INSTALL, 0, 0), environment.status.value)
        environment.reconcile(valid())
        assertEquals(listOf("install", "default_user verify 3.14 24"), engine.calls)
        assertEquals(EnvironmentStatus.UpToDate, environment.status.value)
        val current = store.current()!!
        assertEquals(ActivationReason.INSTALL, current.activation.reason)
        assertEquals(image.profile, current.activation.profile)
        assertEquals(listOf("1"), generations())
        assertEquals("seed", File(store.homeDirectory, ".profile").readText())
        assertFalse(File(store.generationDirectory(1), "seeds").exists())
        assertEquals(setOf("3.14.7") to setOf("24.21.0"), store.managedToolchains())
        val running = started(environment)
        assertEquals(image.profile.name, store.activeProfile())
        assertEquals(current.activation, running)
        environment.reconcile(valid())
        assertEquals(2, engine.calls.size)
    }

    @Test fun `python and node changes install side by side and switch profiles at restart`() = runBlocking {
        val environment = reconciler()
        environment.start()
        environment.reconcile(valid())
        val first = started(environment)
        engine.calls.clear()
        environment.reconcile(valid(ContainerSpec(python = "3.13", node = "22")))
        assertEquals(listOf("default_user python 3.13", "default_user node 22", "default_user verify 3.13 22"), engine.calls)
        assertEquals(listOf("1"), generations())          // no rootfs copy
        val status = environment.status.value as EnvironmentStatus.PendingRestart
        assertEquals(ActivationReason.CONFIG, status.reason)
        assertTrue(status.prompt)
        assertEquals(ToolchainProfile("3.13.15", "22.20.0"), environment.activation!!.profile)
        assertEquals(image.profile.name, store.activeProfile())   // the running instance keeps its profile
        started(environment)
        assertEquals("py3.13.15-node22.20.0", store.activeProfile())
        assertEquals(EnvironmentStatus.UpToDate, environment.status.value)
        assertEquals(first, store.previous()!!.activation)       // same rootfs: rollback stays available
        assertTrue(File(store.toolchainsDirectory, "uv/python/cpython-3.14.7-linux-x86_64-gnu").isDirectory)
        val spec = environment.rollback()!!
        assertEquals(ContainerSpec("3.14", "24"), spec)
        started(environment)
        assertEquals(image.profile.name, store.activeProfile())
        engine.calls.clear()
        environment.reconcile(valid(spec))
        assertTrue(engine.calls.isEmpty())
    }

    @Test fun `env changes are metadata only`() = runBlocking {
        val environment = reconciler()
        environment.start()
        environment.reconcile(valid())
        started(environment)
        engine.calls.clear()
        environment.reconcile(valid(ContainerSpec(env = mapOf("A" to "1"))))
        assertTrue(engine.calls.isEmpty())
        assertEquals(listOf("1"), generations())
        assertEquals(mapOf("A" to "1"), environment.activation!!.config.env)
        assertTrue(environment.status.value is EnvironmentStatus.PendingRestart)
    }

    @Test fun `apt changes copy the rootfs once and the old copy is pruned after a successful restart`() = runBlocking {
        val environment = reconciler()
        environment.start()
        environment.reconcile(valid())
        started(environment)
        engine.calls.clear()
        environment.reconcile(valid(ContainerSpec(packages = setOf("jq"))))
        assertEquals(listOf("clone 1", "root apt-install jq", "default_user verify 3.14 24 jq"), engine.calls)
        assertEquals(listOf("1", "2"), generations())
        assertEquals(setOf("jq"), store.current()!!.generation.packages)
        assertEquals(1L, store.previous()!!.activation.generation)
        started(environment)
        assertEquals(listOf("2"), generations())
        assertNull(store.previous())
        assertEquals(null, environment.rollback())
    }

    @Test fun `repeated changes without a restart keep only the running and the newest rootfs`() = runBlocking {
        val environment = reconciler()
        environment.start()
        environment.reconcile(valid())
        started(environment)
        environment.reconcile(valid(ContainerSpec(packages = setOf("jq"))))
        environment.reconcile(valid(ContainerSpec(packages = setOf("jq", "tree"))))
        assertEquals(listOf("1", "3"), generations())
        environment.reconcile(valid(ContainerSpec(python = "3.13", packages = setOf("jq", "tree"))))
        environment.reconcile(valid(ContainerSpec(python = "3.12", packages = setOf("jq", "tree"))))
        assertEquals(listOf("1", "3"), generations())
        assertFalse(File(store.profilesDirectory, "py3.13.15-node24.21.0").exists())
        assertFalse(File(store.toolchainsDirectory, "uv/python/cpython-3.13.15-linux-x86_64-gnu").exists())
        assertTrue(File(store.toolchainsDirectory, "uv/python/cpython-3.14.7-linux-x86_64-gnu").exists())
    }

    @Test fun `a failed build keeps the old environment and is not retried automatically`() = runBlocking {
        val environment = reconciler()
        environment.start()
        environment.reconcile(valid())
        val before = environment.activation
        engine.failOperation = "apt-install"
        environment.reconcile(valid(ContainerSpec(packages = setOf("jq"))))
        val failed = environment.status.value as EnvironmentStatus.Failed
        assertEquals(Stage.PACKAGES, failed.stage)
        assertTrue(failed.environmentAvailable)
        assertTrue(failed.message, failed.message.contains("status 100"))
        assertTrue(File(failed.log!!).readText().contains("E: simulated failure"))
        assertEquals(before, environment.activation)
        assertEquals(listOf("1"), generations())
        val calls = engine.calls.size
        environment.reconcile(valid(ContainerSpec(packages = setOf("jq"))))
        assertEquals(calls, engine.calls.size)
        val restarted = reconciler()
        restarted.start()
        assertTrue(restarted.status.value is EnvironmentStatus.Failed)
        engine.failOperation = null
        restarted.reconcile(valid(ContainerSpec(packages = setOf("jq"))), retry = true)
        assertEquals(EnvironmentStatus.UpToDate, restarted.status.value)
        assertEquals(setOf("jq"), store.current()!!.generation.packages)
    }

    @Test fun `a failed toolchain install leaves the environment untouched`() = runBlocking {
        val environment = reconciler()
        environment.start()
        environment.reconcile(valid())
        val before = environment.activation
        engine.failOperation = "node"
        environment.reconcile(valid(ContainerSpec(node = "22")))
        assertEquals(Stage.NODE, (environment.status.value as EnvironmentStatus.Failed).stage)
        assertEquals(before, environment.activation)
        assertEquals(listOf("1"), generations())
        environment.reconcile(valid())
        assertEquals(EnvironmentStatus.UpToDate, environment.status.value)
    }

    @Test fun `verification failures are reported as such`() = runBlocking {
        val environment = reconciler()
        environment.start()
        environment.reconcile(valid())
        engine.wrongVerify = true
        environment.reconcile(valid(ContainerSpec(python = "3.13")))
        val failed = environment.status.value as EnvironmentStatus.Failed
        assertEquals(Stage.VERIFY, failed.stage)
        assertTrue(failed.message, failed.message.contains("Python is 3.11.0"))
    }

    @Test fun `a newer container json supersedes an in-flight build`() = runBlocking {
        val environment = reconciler()
        environment.start()
        environment.reconcile(valid())
        val desired = MutableStateFlow(valid(ContainerSpec(packages = setOf("jq"))))
        engine.entered = CompletableDeferred()
        engine.gate = CompletableDeferred()
        val loop = launch(Dispatchers.Default) { desired.collectLatest { environment.reconcile(it) } }
        withTimeout(5_000) { engine.entered.await() }
        assertTrue(store.partialDirectory(2).exists())
        assertTrue(environment.status.value is EnvironmentStatus.Applying)
        engine.gate = null
        desired.value = valid(ContainerSpec(packages = setOf("tree")))
        withTimeout(5_000) { environment.status.first { it == EnvironmentStatus.UpToDate } }
        loop.cancelAndJoin()
        assertEquals(setOf("tree"), store.current()!!.generation.packages)
        assertFalse(store.partialDirectory(2).exists())
        assertEquals(listOf("tree"), engine.calls.filter { it.startsWith("root apt-install") }.map { it.removePrefix("root apt-install ") })
    }

    @Test fun `a new bundled image rebuilds silently, merges its toolchains and waits for the next restart`() = runBlocking {
        val environment = reconciler()
        environment.start()
        environment.reconcile(valid(ContainerSpec(packages = setOf("jq"))))
        val old = started(environment)
        val newImage = ImageInfo.parse(sampleIndex(sha256 = "c".repeat(64), python = "3.14.9"))
        val fake = FakeEngine(newImage, store.toolchainsDirectory)
        val upgraded = reconciler(newImage, fake)
        upgraded.start()
        upgraded.instanceStarted(old)
        upgraded.reconcile(valid(ContainerSpec(packages = setOf("jq"))))
        assertEquals(listOf("install", "root apt-install jq", "default_user verify 3.14 24 jq"), fake.calls)
        val status = upgraded.status.value as EnvironmentStatus.PendingRestart
        assertEquals(ActivationReason.IMAGE, status.reason)
        assertFalse(status.prompt)
        assertEquals("c".repeat(64), store.current()!!.generation.image.sha256)
        assertEquals(ToolchainProfile("3.14.9", "24.21.0"), upgraded.activation!!.profile)
        assertTrue(File(store.toolchainsDirectory, "uv/python/cpython-3.14.7-linux-x86_64-gnu").exists())
        started(upgraded)
        assertFalse(File(store.toolchainsDirectory, "uv/python/cpython-3.14.7-linux-x86_64-gnu").exists())
        assertEquals(listOf("2"), generations())
    }

    @Test fun `invalid container json keeps the last valid configuration`() = runBlocking {
        val environment = reconciler()
        environment.start()
        val spec = ContainerSpec(packages = setOf("jq"))
        environment.reconcile(valid(spec))
        val calls = engine.calls.size
        val error = ContainerSpecException(listOf(ConfigProblem("python", "expected 3, 3.X or 3.X.Y")))
        environment.reconcile(DesiredConfig.Invalid(error, spec))
        assertEquals(calls, engine.calls.size)
        val failed = environment.status.value as EnvironmentStatus.Failed
        assertEquals(Stage.CONFIG, failed.stage)
        assertTrue(failed.environmentAvailable)
        environment.reconcile(valid(spec))
        assertEquals(EnvironmentStatus.UpToDate, environment.status.value)
    }

    @Test fun `startup removes a build interrupted by process death`() = runBlocking {
        val environment = reconciler()
        environment.start()
        environment.reconcile(valid())
        File(store.partialDirectory(7), "rootfs/usr/bin").mkdirs()
        File(store.toolchainsDirectory, ".tmp/envctl.abc").mkdirs()
        val restarted = reconciler()
        val recovery = restarted.start()
        assertEquals(listOf("7.partial"), recovery.removedPartials)
        assertFalse(File(store.toolchainsDirectory, ".tmp").exists())
        assertEquals(EnvironmentStatus.UpToDate, restarted.status.value)
    }
}
