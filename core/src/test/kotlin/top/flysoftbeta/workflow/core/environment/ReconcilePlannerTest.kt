package top.flysoftbeta.workflow.core.environment

import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Assume.assumeTrue
import org.junit.Test
import java.io.File

internal fun sampleIndex(sha256: String = "a".repeat(64), formatVersion: Int = 2, profile: String = "workspace",
                         type: String = "debian-trixie", python: String = "3.14.7") = """
    {"sha256":"$sha256","size":189340282,"metadata":{"format":"workflow-image","formatVersion":$formatVersion,
     "type":"$type","typeVersion":1,"profile":"$profile","architecture":"amd64",
     "user":{"name":"work","uid":1000,"gid":1000,"home":"/home/work","shell":"/bin/bash"},
     "environment":{"PATH":"/opt/toolchains/active/python/bin:/opt/toolchains/active/node/bin:/usr/bin:/bin","LANG":"en_US.UTF-8"},
     "toolchains":{"uv":"0.12.19","nvm":"0.40.8","python":"$python","node":"24.21.0","profile":"py$python-node24.21.0"},
     "stores":{"/home/work":{"store":"home/work","seed":"if-absent"},"/opt/toolchains":{"store":"toolchains","seed":"merge"}},
     "defaults":{"python":"3.14","node":"24"}}}
""".trimIndent()

class ReconcilePlannerTest {
    private val image = ImageInfo.parse(sampleIndex())
    private val defaults = image.defaults
    private val imageProfile = ToolchainProfile("3.14.7", "24.21.0")

    private fun resolved(spec: ContainerSpec = ContainerSpec()) = spec.resolve(defaults)

    private fun environment(packages: Set<String> = emptySet(), profile: ToolchainProfile = imageProfile,
                            config: ResolvedSpec = resolved(), imageSha: String = image.sha256) =
        ActiveEnvironment(
            Activation(3, 5, profile, config, ActivationReason.CONFIG, "t"),
            GenerationRecord(5, 4, "t", image.ref.copy(sha256 = imageSha), packages),
        )

    @Test fun `image index is parsed and validated`() {
        assertEquals(ToolchainDefaults("3.14", "24"), image.defaults)
        assertEquals(imageProfile, image.profile)
        assertEquals("work", image.user)
        assertEquals(listOf(StoreSeed("/home/work", "home/work", SeedPolicy.IF_ABSENT), StoreSeed("/opt/toolchains", "toolchains", SeedPolicy.MERGE)),
            image.stores)
        assertThrows(IllegalArgumentException::class.java) { ImageInfo.parse(sampleIndex(formatVersion = 1)) }
        assertThrows(IllegalArgumentException::class.java) { ImageInfo.parse(sampleIndex(profile = "base")) }
        assertThrows(IllegalArgumentException::class.java) { ImageInfo.parse(sampleIndex(type = "alpine-edge")) }
        assertThrows(IllegalArgumentException::class.java) { ImageInfo.parse(sampleIndex(sha256 = "xyz")) }
        assertThrows(IllegalArgumentException::class.java) { ImageInfo.parse(sampleIndex().replace("\"toolchains\",", "\"../escape\",")) }
        assertThrows(IllegalArgumentException::class.java) { StoreSeed("/", "x", SeedPolicy.MERGE) }
    }

    @Test fun `profile names round trip and reject anything else`() {
        assertEquals("py3.13.15-node22.20.0", ToolchainProfile("3.13.15", "22.20.0").name)
        assertEquals(ToolchainProfile("3.13.15", "22.20.0"), ToolchainProfile.parse("py3.13.15-node22.20.0"))
        assertThrows(IllegalArgumentException::class.java) { ToolchainProfile.parse("py3.13-node22") }
        assertThrows(IllegalArgumentException::class.java) { ToolchainProfile("3.14.7+freethreaded", "24.21.0") }
    }

    @Test fun `the built amd64 image index is accepted when present`() {
        var directory: File? = File(System.getProperty("user.dir")).absoluteFile
        while (directory != null && !File(directory, "image/versions.env").exists()) directory = directory.parentFile
        val index = File(directory ?: File("."), "artifacts/image/amd64/image.json")
        assumeTrue(index.exists())
        val built = ImageInfo.parse(index.readText())
        assertEquals(ToolchainDefaults.BUNDLED, built.defaults)
        assertEquals("amd64", built.architecture)
        assertEquals(setOf("home/work", "toolchains"), built.stores.map { it.store }.toSet())
    }

    @Test fun `first install only verifies when the defaults are wanted`() {
        assertEquals(PlanOutcome.Build(BuildBase.Image, listOf(Step.Verify("3.14", "24", emptyList())), resolved(), ActivationReason.INSTALL),
            ReconcilePlanner.plan(resolved(), image, null))
    }

    @Test fun `first install applies declared packages and versions`() {
        val desired = resolved(ContainerSpec(python = "3.13", packages = setOf("jq")))
        val plan = ReconcilePlanner.plan(desired, image, null) as PlanOutcome.Build
        assertEquals(listOf(Step.InstallPackages(listOf("jq")), Step.InstallPython("3.13"), Step.Verify("3.13", "24", listOf("jq"))), plan.steps)
    }

    @Test fun `unchanged configuration is up to date and env only changes are metadata`() {
        assertEquals(PlanOutcome.UpToDate, ReconcilePlanner.plan(resolved(), image, environment()))
        val withEnv = resolved(ContainerSpec(env = mapOf("A" to "1")))
        assertEquals(PlanOutcome.ActivateOnly(withEnv), ReconcilePlanner.plan(withEnv, image, environment()))
        val broader = resolved(ContainerSpec(python = "3"))
        assertEquals(PlanOutcome.ActivateOnly(broader), ReconcilePlanner.plan(broader, image, environment()))
    }

    @Test fun `toolchain changes install side by side without copying the rootfs`() {
        val desired = resolved(ContainerSpec(python = "3.13", node = "22", packages = setOf("jq")))
        assertEquals(PlanOutcome.Build(BuildBase.InPlace(5),
            listOf(Step.InstallPython("3.13"), Step.InstallNode("22"), Step.Verify("3.13", "22", listOf("jq"))), desired, ActivationReason.CONFIG),
            ReconcilePlanner.plan(desired, image, environment(packages = setOf("jq"))))
    }

    @Test fun `only apt changes clone the rootfs`() {
        val desired = resolved(ContainerSpec(node = "22", packages = setOf("jq", "ripgrep")))
        assertEquals(PlanOutcome.Build(BuildBase.Clone(5), listOf(
            Step.RemovePackages(listOf("htop")), Step.InstallPackages(listOf("ripgrep")), Step.InstallNode("22"),
            Step.Verify("3.14", "22", listOf("jq", "ripgrep"))), desired, ActivationReason.CONFIG),
            ReconcilePlanner.plan(desired, image, environment(packages = setOf("jq", "htop"))))
    }

    @Test fun `a new bundled image rebuilds from the image and replays declared state`() {
        val config = resolved(ContainerSpec(packages = setOf("jq")))
        val current = environment(setOf("jq"), config = config, imageSha = "b".repeat(64))
        val plan = ReconcilePlanner.plan(config, image, current) as PlanOutcome.Build
        assertEquals(BuildBase.Image, plan.base)
        assertEquals(ActivationReason.IMAGE, plan.reason)
        assertEquals(listOf(Step.InstallPackages(listOf("jq")), Step.Verify("3.14", "24", listOf("jq"))), plan.steps)
        val changed = resolved(ContainerSpec(packages = setOf("jq", "tree")))
        assertEquals(ActivationReason.CONFIG, (ReconcilePlanner.plan(changed, image, current) as PlanOutcome.Build).reason)
    }

    @Test fun `verification checks what the guest reports`() {
        val desired = resolved(ContainerSpec(packages = setOf("jq")))
        val good = mapOf("profile" to "py3.14.7-node24.21.0", "python" to "3.14.7", "node" to "24.21.0", "packages" to mapOf("jq" to "1.7.1-6"))
        assertEquals(Verified(imageProfile, setOf("jq")), ReconcilePlanner.verified(good, desired))
        assertThrows(IllegalArgumentException::class.java) { ReconcilePlanner.verified(good + ("packages" to mapOf("jq" to null)), desired) }
        assertThrows(IllegalArgumentException::class.java) { ReconcilePlanner.verified(good + ("python" to "3.13.9"), desired) }
        assertThrows(IllegalArgumentException::class.java) { ReconcilePlanner.verified(good + ("node" to null), desired) }
        assertThrows(IllegalArgumentException::class.java) { ReconcilePlanner.verified(good + ("profile" to "other"), desired) }
    }

    @Test fun `debian trixie maps steps to envctl with the right user`() {
        val envctl = DebianTrixie.ENVCTL
        assertEquals(GuestCommand(GuestUser.ROOT, listOf(envctl, "apt-install", "jq", "tree")),
            DebianTrixie.command(Step.InstallPackages(listOf("jq", "tree"))))
        assertEquals(GuestCommand(GuestUser.ROOT, listOf(envctl, "apt-remove", "htop")), DebianTrixie.command(Step.RemovePackages(listOf("htop"))))
        assertEquals(GuestCommand(GuestUser.DEFAULT_USER, listOf(envctl, "python", "3.13")), DebianTrixie.command(Step.InstallPython("3.13")))
        assertEquals(GuestCommand(GuestUser.DEFAULT_USER, listOf(envctl, "node", "22")), DebianTrixie.command(Step.InstallNode("22")))
        assertEquals(GuestCommand(GuestUser.DEFAULT_USER, listOf(envctl, "verify", "3.13", "22", "jq")),
            DebianTrixie.command(Step.Verify("3.13", "22", listOf("jq"))))
    }
}
