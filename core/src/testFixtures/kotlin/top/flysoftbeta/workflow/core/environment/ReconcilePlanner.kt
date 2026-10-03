package top.flysoftbeta.workflow.core.environment

/** A guest step of the plan; mapped to a command by the image variant. */
sealed interface Step {
    val stage: Stage

    data class RemovePackages(val names: List<String>) : Step { override val stage get() = Stage.PACKAGES }
    data class InstallPackages(val names: List<String>) : Step { override val stage get() = Stage.PACKAGES }
    /** Installs a matching Python side by side in the toolchain store (no-op when one is present). */
    data class InstallPython(val spec: String) : Step { override val stage get() = Stage.PYTHON }
    data class InstallNode(val spec: String) : Step { override val stage get() = Stage.NODE }
    /** Always last: creates the toolchain profile and reports what is installed. */
    data class Verify(val python: String, val node: String, val packages: List<String>) : Step { override val stage get() = Stage.VERIFY }
}

/** Where a build runs. Only [Image] and [Clone] produce a new rootfs generation. */
sealed interface BuildBase {
    /** First install or a new bundled image: install the image into a new generation. */
    data object Image : BuildBase
    /** The apt package set changed: copy the rootfs of [generation]. */
    data class Clone(val generation: Long) : BuildBase
    /** Only Python/Node changed: install into the toolchain store using [generation] as it is. */
    data class InPlace(val generation: Long) : BuildBase
}

sealed interface PlanOutcome {
    data object UpToDate : PlanOutcome
    /** Only `env` (or an already satisfied version spec) changed: a new activation, no guest work. */
    data class ActivateOnly(val config: ResolvedSpec) : PlanOutcome
    data class Build(val base: BuildBase, val steps: List<Step>, val config: ResolvedSpec, val reason: ActivationReason) : PlanOutcome
}

/** Result of the Verify step. */
data class Verified(val profile: ToolchainProfile, val packages: Set<String>)

/** Pure diff of desired against applied state (docs/environment.md §5.2). */
object ReconcilePlanner {
    fun plan(desired: ResolvedSpec, image: ImageInfo, current: ActiveEnvironment?): PlanOutcome {
        val fresh = current == null || current.generation.image.sha256 != image.sha256
        val packages = if (fresh) emptySet() else current!!.generation.packages
        val toolchain = if (fresh) image.profile else current!!.activation.profile
        val packageSteps = buildList {
            val remove = (packages - desired.packages.toSet()).sorted()
            val install = (desired.packages.toSet() - packages).sorted()
            if (remove.isNotEmpty()) add(Step.RemovePackages(remove))
            if (install.isNotEmpty()) add(Step.InstallPackages(install))
        }
        val toolchainSteps = buildList {
            if (!VersionSpec.satisfies(toolchain.python, desired.python)) add(Step.InstallPython(desired.python))
            if (!VersionSpec.satisfies(toolchain.node, desired.node)) add(Step.InstallNode(desired.node))
        }
        val verify = Step.Verify(desired.python, desired.node, desired.packages)
        if (fresh) {
            val reason = when {
                current == null -> ActivationReason.INSTALL
                current.activation.configFingerprint != desired.fingerprint -> ActivationReason.CONFIG
                else -> ActivationReason.IMAGE
            }
            return PlanOutcome.Build(BuildBase.Image, packageSteps + toolchainSteps + verify, desired, reason)
        }
        val generation = current!!.generation.id
        return when {
            packageSteps.isNotEmpty() ->
                PlanOutcome.Build(BuildBase.Clone(generation), packageSteps + toolchainSteps + verify, desired, ActivationReason.CONFIG)
            toolchainSteps.isNotEmpty() ->
                PlanOutcome.Build(BuildBase.InPlace(generation), toolchainSteps + verify, desired, ActivationReason.CONFIG)
            current.activation.configFingerprint == desired.fingerprint -> PlanOutcome.UpToDate
            else -> PlanOutcome.ActivateOnly(desired)
        }
    }

    /** Validates the Verify step output (`envctl verify`) against the desired state. */
    fun verified(output: Map<String, Any?>, desired: ResolvedSpec): Verified {
        val python = output["python"] as String?
        val node = output["node"] as String?
        @Suppress("UNCHECKED_CAST")
        val packages = output["packages"] as? Map<String, Any?> ?: emptyMap()
        val missing = desired.packages.filter { packages[it] !is String }
        require(VersionSpec.satisfies(python, desired.python)) { "Python is ${python ?: "missing"}, expected ${desired.python}" }
        require(VersionSpec.satisfies(node, desired.node)) { "Node.js is ${node ?: "missing"}, expected ${desired.node}" }
        require(missing.isEmpty()) { "Packages not installed: ${missing.joinToString(" ")}" }
        val profile = ToolchainProfile(python!!, node!!)
        require(output["profile"] == profile.name) { "Unexpected profile ${output["profile"]}" }
        return Verified(profile, desired.packages.toSet())
    }
}
