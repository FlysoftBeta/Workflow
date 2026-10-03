package top.flysoftbeta.workflow.core.environment

import top.flysoftbeta.workflow.core.json.jsonObject
import top.flysoftbeta.workflow.core.json.list

/**
 * A toolchain profile: one Python and one Node.js version from the side-by-side toolchain store,
 * exposed to an environment instance as `/opt/toolchains/active` (docs/environment.md §2.6, §4).
 */
data class ToolchainProfile(val python: String, val node: String) {
    init {
        require(VERSION.matches(python) && VERSION.matches(node)) { "Invalid toolchain versions $python/$node" }
    }

    val name: String get() = "py$python-node$node"

    companion object {
        private val VERSION = Regex("^[0-9]+\\.[0-9]+\\.[0-9]+$")
        private val NAME = Regex("^py([0-9]+\\.[0-9]+\\.[0-9]+)-node([0-9]+\\.[0-9]+\\.[0-9]+)$")

        fun parse(name: String): ToolchainProfile {
            val match = requireNotNull(NAME.matchEntire(name)) { "Invalid profile name $name" }
            return ToolchainProfile(match.groupValues[1], match.groupValues[2])
        }

        fun isName(name: String): Boolean = NAME.matches(name)
    }
}

/** `generations/<n>/generation.json`: a rootfs. Only the apt package set distinguishes generations. */
data class GenerationRecord(val id: Long, val parent: Long?, val createdAt: String, val image: ImageRef, val packages: Set<String>) {
    fun toJson(): Map<String, Any?> = linkedMapOf(
        "id" to id, "parent" to parent, "createdAt" to createdAt,
        "image" to linkedMapOf("sha256" to image.sha256, "type" to image.type, "typeVersion" to image.typeVersion.toLong(),
            "architecture" to image.architecture),
        "packages" to packages.sorted(),
    )

    companion object {
        fun fromJson(value: Map<String, Any?>): GenerationRecord {
            val image = (value["image"] ?: error("generation: image")).jsonObject()
            return GenerationRecord(
                id = value["id"] as? Long ?: error("generation: id"),
                parent = value["parent"] as Long?,
                createdAt = value["createdAt"] as? String ?: "",
                image = ImageRef(image["sha256"] as String, image["type"] as String, (image["typeVersion"] as Long).toInt(),
                    image["architecture"] as String),
                packages = value.list("packages").map { it as String }.toSet(),
            )
        }
    }
}

/** `current.json` / `previous.json`: rootfs generation + toolchain profile + configuration. */
data class Activation(
    val id: Long,
    val generation: Long,
    val profile: ToolchainProfile,
    val config: ResolvedSpec,
    val reason: ActivationReason,
    val activatedAt: String,
) {
    val configFingerprint: String get() = config.fingerprint

    fun toJson(): Map<String, Any?> = linkedMapOf(
        "activation" to id, "generation" to generation, "profile" to profile.name, "configFingerprint" to configFingerprint,
        "config" to config.toJson(), "reason" to reason.wire, "activatedAt" to activatedAt,
    )

    companion object {
        fun fromJson(value: Map<String, Any?>): Activation {
            val config = ResolvedSpec.fromJson((value["config"] ?: error("activation: config")).jsonObject())
            require(value["configFingerprint"] == config.fingerprint) { "activation fingerprint does not match its config" }
            return Activation(
                id = value["activation"] as? Long ?: error("activation: id"),
                generation = value["generation"] as? Long ?: error("activation: generation"),
                profile = ToolchainProfile.parse(value["profile"] as? String ?: error("activation: profile")),
                config = config,
                reason = ActivationReason.of(value["reason"] as? String ?: error("activation: reason")),
                activatedAt = value["activatedAt"] as? String ?: "",
            )
        }
    }
}

/** An activation together with the generation it points at. */
data class ActiveEnvironment(val activation: Activation, val generation: GenerationRecord)

/** A failed build, remembered so the same inputs are not retried automatically (§5.2). */
data class FailureRecord(val fingerprint: String, val imageSha256: String, val stage: Stage, val message: String, val log: String?) {
    fun toJson(): Map<String, Any?> = linkedMapOf(
        "fingerprint" to fingerprint, "image" to imageSha256, "stage" to stage.wire, "message" to message, "log" to log,
    )

    companion object {
        fun fromJson(value: Map<String, Any?>) = FailureRecord(
            value["fingerprint"] as String, value["image"] as String, Stage.of(value["stage"] as String),
            value["message"] as? String ?: "", value["log"] as String?,
        )
    }
}
