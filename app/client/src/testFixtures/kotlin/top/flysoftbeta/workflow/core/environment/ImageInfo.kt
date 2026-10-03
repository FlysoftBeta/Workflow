package top.flysoftbeta.workflow.core.environment

import top.flysoftbeta.workflow.core.json.Json
import top.flysoftbeta.workflow.core.json.jsonObject

enum class SeedPolicy(val wire: String) {
    /** Copy the seed only when the store does not exist yet (home). */
    IF_ABSENT("if-absent"),
    /** Add seed entries the store lacks; existing entries win (toolchains). */
    MERGE("merge");

    companion object {
        fun of(wire: String): SeedPolicy = entries.firstOrNull { it.wire == wire } ?: throw IllegalArgumentException("seed policy $wire")
    }
}

/** A guest directory backed by the persistent store `engine/<store>`; the image only seeds it. */
data class StoreSeed(val guestPath: String, val store: String, val policy: SeedPolicy) {
    init {
        require(STORE.matches(store)) { "Invalid store name $store" }
        require(guestPath.startsWith("/") && guestPath != "/" && ".." !in guestPath.split('/')) { "Invalid store path $guestPath" }
    }

    private companion object {
        val STORE = Regex("^[a-z0-9][a-z0-9._-]*(/[a-z0-9][a-z0-9._-]*)*$")
    }
}

/** The bundled image as described by its `image.json` index (docs/engine/environment.md §2.1, §2.3). */
data class ImageInfo(
    val sha256: String,
    val size: Long,
    val type: String,
    val typeVersion: Int,
    val architecture: String,
    val defaults: ToolchainDefaults,
    /** Toolchain profile seeded by the image; the baseline of a fresh install. */
    val profile: ToolchainProfile,
    val environment: Map<String, String>,
    val user: String,
    val stores: List<StoreSeed>,
) {
    val ref: ImageRef get() = ImageRef(sha256, type, typeVersion, architecture)

    companion object {
        const val FORMAT = "workflow-image"
        const val FORMAT_VERSION = 2L

        /** Parses and validates `image.json`; only workspace images of a known variant are accepted. */
        fun parse(text: String): ImageInfo {
            val index = Json.parse(text).jsonObject()
            val metadata = (index["metadata"] ?: error("image.json has no metadata")).jsonObject()
            fun string(map: Map<String, Any?>, key: String): String = map[key] as? String ?: throw IllegalArgumentException("image metadata: $key")
            require(metadata["format"] == FORMAT) { "Not a workflow image" }
            require(metadata["formatVersion"] == FORMAT_VERSION) { "Unsupported image formatVersion ${metadata["formatVersion"]}" }
            require(metadata["profile"] == "workspace") { "Not a workspace image" }
            val sha256 = string(index, "sha256")
            require(Regex("^[0-9a-f]{64}$").matches(sha256)) { "Invalid image sha256" }
            val type = string(metadata, "type")
            val typeVersion = (metadata["typeVersion"] as? Long)?.toInt() ?: throw IllegalArgumentException("image metadata: typeVersion")
            requireNotNull(ImageVariants.find(type, typeVersion)) { "Unsupported image variant $type/$typeVersion" }
            val defaults = (metadata["defaults"] ?: error("image metadata: defaults")).jsonObject()
            val toolchains = (metadata["toolchains"] ?: error("image metadata: toolchains")).jsonObject()
            val environment = (metadata["environment"] ?: error("image metadata: environment")).jsonObject()
            val user = (metadata["user"] ?: error("image metadata: user")).jsonObject()
            val profile = ToolchainProfile(string(toolchains, "python"), string(toolchains, "node"))
            toolchains["profile"]?.let { require(it == profile.name) { "image metadata: toolchains.profile" } }
            val stores = (metadata["stores"] as? Map<*, *>).orEmpty().map { (path, value) ->
                val entry = (value as? Map<*, *>) ?: throw IllegalArgumentException("image metadata: stores")
                StoreSeed(path as String, entry["store"] as? String ?: "", SeedPolicy.of(entry["seed"] as? String ?: ""))
            }
            return ImageInfo(
                sha256 = sha256,
                size = index["size"] as? Long ?: throw IllegalArgumentException("image.json: size"),
                type = type,
                typeVersion = typeVersion,
                architecture = string(metadata, "architecture"),
                defaults = ToolchainDefaults(string(defaults, "python"), string(defaults, "node")),
                profile = profile,
                environment = environment.mapValues { (key, value) -> value as? String ?: throw IllegalArgumentException("environment.$key") },
                user = string(user, "name"),
                stores = stores,
            )
        }
    }
}

data class ImageRef(val sha256: String, val type: String, val typeVersion: Int, val architecture: String)

/** Who runs a guest step. */
enum class GuestUser { ROOT, DEFAULT_USER }

data class GuestCommand(val user: GuestUser, val argv: List<String>)

/** Maps planner steps to guest commands for one image variant (`metadata.type` / `typeVersion`). */
interface ImageVariant {
    val type: String
    val typeVersions: Set<Int>
    fun command(step: Step): GuestCommand
}

/** debian-trixie typeVersion 1: steps run through /usr/local/libexec/workflow/envctl (docs/engine/environment.md §5.3). */
object DebianTrixie : ImageVariant {
    const val ENVCTL = "/usr/local/libexec/workflow/envctl"
    override val type = "debian-trixie"
    override val typeVersions = setOf(1)

    override fun command(step: Step): GuestCommand = when (step) {
        is Step.RemovePackages -> GuestCommand(GuestUser.ROOT, listOf(ENVCTL, "apt-remove") + step.names)
        is Step.InstallPackages -> GuestCommand(GuestUser.ROOT, listOf(ENVCTL, "apt-install") + step.names)
        is Step.InstallPython -> GuestCommand(GuestUser.DEFAULT_USER, listOf(ENVCTL, "python", step.spec))
        is Step.InstallNode -> GuestCommand(GuestUser.DEFAULT_USER, listOf(ENVCTL, "node", step.spec))
        is Step.Verify -> GuestCommand(GuestUser.DEFAULT_USER, listOf(ENVCTL, "verify", step.python, step.node) + step.packages)
    }
}

object ImageVariants {
    private val all = listOf<ImageVariant>(DebianTrixie)
    fun find(type: String, typeVersion: Int): ImageVariant? = all.firstOrNull { it.type == type && typeVersion in it.typeVersions }
}
