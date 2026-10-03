package top.flysoftbeta.workflow.core.environment

import top.flysoftbeta.workflow.core.json.Json
import java.security.MessageDigest

/**
 * `.workspace/container.json` version 1 (docs/engine/environment.md §3): the desired environment.
 * `null` Python/Node means "the image default".
 */
data class ContainerSpec(
    val python: String? = null,
    val node: String? = null,
    val packages: Set<String> = emptySet(),
    val env: Map<String, String> = emptyMap(),
) {
    init {
        val problems = ContainerSpecCodec.validate(this)
        if (problems.isNotEmpty()) throw ContainerSpecException(problems)
    }

    fun resolve(defaults: ToolchainDefaults): ResolvedSpec =
        ResolvedSpec(python ?: defaults.python, node ?: defaults.node, packages.sorted(), env.toSortedMap())
}

/** Defaults baked into the image (`metadata.defaults`); must equal DEFAULT_* in image/versions.env. */
data class ToolchainDefaults(val python: String, val node: String) {
    companion object {
        val BUNDLED = ToolchainDefaults(python = "3.14", node = "24")
    }
}

/** container.json with defaults filled in: the unit that is fingerprinted and activated. */
data class ResolvedSpec(val python: String, val node: String, val packages: List<String>, val env: Map<String, String>) {
    fun toJson(): Map<String, Any?> = linkedMapOf(
        "env" to env.toSortedMap(), "node" to node, "packages" to packages.sorted(), "python" to python, "version" to 1L,
    )

    /** sha256 of the canonical JSON (sorted keys, compact, UTF-8). */
    val fingerprint: String by lazy {
        "sha256:" + MessageDigest.getInstance("SHA-256").digest(Json.stringify(toJson()).toByteArray(Charsets.UTF_8))
            .joinToString("") { "%02x".format(it.toInt() and 255) }
    }

    fun toSpec(): ContainerSpec = ContainerSpec(python, node, packages.toSet(), env)

    companion object {
        fun fromJson(value: Map<String, Any?>): ResolvedSpec {
            val spec = ContainerSpecCodec.fromJson(value).spec
            return ResolvedSpec(requireNotNull(spec.python), requireNotNull(spec.node), spec.packages.sorted(), spec.env.toSortedMap())
        }
    }
}

data class ConfigProblem(val path: String, val message: String) {
    override fun toString(): String = if (path.isEmpty()) message else "$path: $message"
}

class ContainerSpecException(val problems: List<ConfigProblem>) :
    IllegalArgumentException("Invalid container.json: " + problems.joinToString("; "))

/** Dotted-prefix version matching: `3.14.7` satisfies `3`, `3.14` and `3.14.7`, but not `3.1`. */
object VersionSpec {
    fun satisfies(installed: String?, spec: String): Boolean =
        installed != null && (installed == spec || installed.startsWith("$spec."))
}

object ContainerSpecCodec {
    const val VERSION = 1L
    const val MAX_BYTES = 64 * 1024
    const val MAX_PACKAGES = 256
    private val PYTHON = Regex("^3(\\.[0-9]{1,4}){0,2}$")
    private val NODE = Regex("^[0-9]{1,4}(\\.[0-9]{1,4}){0,2}$")
    private val PACKAGE = Regex("^[a-z0-9][a-z0-9+.-]{1,127}$")
    private val ENV_KEY = Regex("^[A-Za-z_][A-Za-z0-9_]{0,127}$")
    private val RESERVED_ENV = setOf("HOME", "USER", "LOGNAME", "SHELL", "PATH", "PWD", "OLDPWD", "TERM")
    private const val MAX_ENV_VALUE = 32 * 1024
    private val FIELDS = setOf("version", "python", "node", "packages", "env")

    data class Parsed(val spec: ContainerSpec)

    fun parse(text: String): Parsed {
        if (text.toByteArray(Charsets.UTF_8).size > MAX_BYTES) throw ContainerSpecException(listOf(ConfigProblem("", "file exceeds 64 KiB")))
        val value = try {
            Json.parse(text)
        } catch (error: RuntimeException) {
            throw ContainerSpecException(listOf(ConfigProblem("", "not valid JSON: ${error.message}")))
        }
        @Suppress("UNCHECKED_CAST")
        val document = value as? Map<String, Any?> ?: throw ContainerSpecException(listOf(ConfigProblem("", "must be a JSON object")))
        return fromJson(document)
    }

    fun fromJson(value: Map<String, Any?>): Parsed =
        Parsed(current(value))

    private fun current(value: Map<String, Any?>): ContainerSpec {
        val problems = mutableListOf<ConfigProblem>()
        value.keys.filter { it !in FIELDS }.forEach { problems += ConfigProblem(it, "unknown field") }
        if ("version" in value && (value["version"] as? Long) != VERSION) problems += ConfigProblem("version", "unsupported version")
        val python = optionalString(value, "python", problems)
        val node = optionalString(value, "node", problems)
        val packages = stringList(value, "packages", problems)
        val env = stringMap(value, "env", problems)
        if (problems.isNotEmpty()) throw ContainerSpecException(problems)
        return ContainerSpec(python, node, packages.toSet(), env)
    }

    private fun optionalString(value: Map<String, Any?>, key: String, problems: MutableList<ConfigProblem>): String? {
        if (key !in value) return null
        return value[key] as? String ?: null.also { problems += ConfigProblem(key, "must be a string") }
    }

    private fun stringList(value: Map<String, Any?>, key: String, problems: MutableList<ConfigProblem>): List<String> {
        if (key !in value) return emptyList()
        val list = value[key] as? List<*> ?: return emptyList<String>().also { problems += ConfigProblem(key, "must be an array") }
        return list.mapIndexedNotNull { index, item ->
            item as? String ?: null.also { problems += ConfigProblem("$key[$index]", "must be a string") }
        }
    }

    private fun stringMap(value: Map<String, Any?>, key: String, problems: MutableList<ConfigProblem>): Map<String, String> {
        if (key !in value) return emptyMap()
        val map = value[key] as? Map<*, *> ?: return emptyMap<String, String>().also { problems += ConfigProblem(key, "must be an object") }
        return map.entries.mapNotNull { (name, item) ->
            val text = item as? String
            if (text == null) problems += ConfigProblem("$key.$name", "must be a string")
            text?.let { name as String to it }
        }.toMap()
    }

    fun validate(spec: ContainerSpec): List<ConfigProblem> {
        val problems = mutableListOf<ConfigProblem>()
        spec.python?.let { if (!PYTHON.matches(it)) problems += ConfigProblem("python", "expected 3, 3.X or 3.X.Y") }
        spec.node?.let { if (!NODE.matches(it)) problems += ConfigProblem("node", "expected N, N.X or N.X.Y") }
        if (spec.packages.size > MAX_PACKAGES) problems += ConfigProblem("packages", "more than $MAX_PACKAGES packages")
        spec.packages.filterNot { PACKAGE.matches(it) }.forEach { problems += ConfigProblem("packages", "invalid Debian package name '$it'") }
        spec.env.forEach { (key, text) ->
            when {
                !ENV_KEY.matches(key) -> problems += ConfigProblem("env.$key", "invalid variable name")
                key in RESERVED_ENV || key.startsWith("WORKFLOW_") -> problems += ConfigProblem("env.$key", "reserved variable")
                '\u0000' in text -> problems += ConfigProblem("env.$key", "contains NUL")
                text.length > MAX_ENV_VALUE -> problems += ConfigProblem("env.$key", "value exceeds 32 KiB")
            }
        }
        return problems
    }

    /** Human-friendly file content with every field explicit (used for new files). */
    fun format(spec: ContainerSpec, defaults: ToolchainDefaults = ToolchainDefaults.BUNDLED): String {
        val resolved = spec.resolve(defaults)
        val packages = resolved.packages.joinToString(", ", "[", "]") { Json.stringify(it) }
        val env = if (resolved.env.isEmpty()) "{}" else resolved.env.entries.joinToString(",\n", "{\n", "\n  }") { (key, text) ->
            "    ${Json.stringify(key)}: ${Json.stringify(text)}"
        }
        return "{\n  \"version\": 1,\n  \"python\": ${Json.stringify(resolved.python)},\n  \"node\": ${Json.stringify(resolved.node)},\n" +
            "  \"packages\": $packages,\n  \"env\": $env\n}\n"
    }
}
