package top.flysoftbeta.workflow.core.config

import top.flysoftbeta.workflow.core.json.Json
import java.util.Locale
import kotlin.math.floor

sealed interface ConfigParse {
    data class Ok(val config: AppConfig, val document: Map<String, Any?>) : ConfigParse
    data class Invalid(val problems: List<String>) : ConfigParse {
        val message: String get() = problems.joinToString("; ")
    }
}

/**
 * config.json codec. Parsing is strict (duplicate keys, wrong types and unknown enum values are
 * errors with their key path) but unknown keys are allowed and preserved on write. Output is
 * 2-space indented.
 */
object ConfigCodec {
    const val VERSION = 2

    fun decode(text: String): ConfigParse {
        val root = try {
            Json.parse(text)
        } catch (error: IllegalArgumentException) {
            return ConfigParse.Invalid(listOf("syntax: ${error.message}"))
        } catch (error: IllegalStateException) {
            return ConfigParse.Invalid(listOf("syntax: ${error.message}"))
        }
        @Suppress("UNCHECKED_CAST")
        val document = root as? Map<String, Any?> ?: return ConfigParse.Invalid(listOf("expected a JSON object"))
        val reader = Reader()
        val version = reader.long(document, "version") ?: return ConfigParse.Invalid(listOf("version: expected $VERSION"))
        if (version != VERSION.toLong()) return ConfigParse.Invalid(listOf("version: unsupported version $version"))
        val config = reader.config(document)
        return if (reader.problems.isEmpty() && config != null) ConfigParse.Ok(config, document) else ConfigParse.Invalid(reader.problems)
    }

    /** Writes [config] merged into [original] (unknown keys and their order are preserved). */
    fun encode(config: AppConfig, original: Map<String, Any?> = emptyMap()): String {
        val base = LinkedHashMap<String, Any?>()
        base["version"] = VERSION
        original.forEach { (key, value) -> if (key != "version") base[key] = value }
        return Json.stringify(merge(base, generate(config), ""), pretty = true) + "\n"
    }

    private fun generate(config: AppConfig): Map<String, Any?> = linkedMapOf(
        "version" to VERSION,
        "appearance" to linkedMapOf(
            "theme" to config.appearance.theme.name.lowercase(Locale.ROOT),
            "density" to config.appearance.density.name.lowercase(Locale.ROOT),
            "fontScale" to number(config.appearance.fontScale),
            "monoFontSize" to number(config.appearance.monoFontSize),
        ),
        "agent" to linkedMapOf(
            "backend" to config.agent.backend,
            "permissions" to (config.agent.permissions ?: Json.Omit),
            "backends" to config.agent.backends.mapValues { (_, defaults) ->
                linkedMapOf("model" to (defaults.model ?: Json.Omit), "effort" to (defaults.effort ?: Json.Omit))
            },
        ),
        "overlay" to linkedMapOf("enabled" to config.overlay.enabled, "extraApps" to config.overlay.extraApps.map { it.id }),
        "launcher" to config.launcher.map { it.id },
        "terminal" to linkedMapOf("extraKeysPinned" to config.terminal.extraKeysPinned),
    )

    /** Known objects merge key by key; `agent.backends` drops backends the config no longer has. */
    private fun merge(original: Map<String, Any?>, generated: Map<String, Any?>, path: String): Map<String, Any?> {
        val result = LinkedHashMap<String, Any?>()
        original.forEach { (key, value) ->
            if (path == "agent.backends" && key !in generated) return@forEach
            result[key] = value
        }
        generated.forEach { (key, value) ->
            val previous = result[key]
            @Suppress("UNCHECKED_CAST")
            result[key] = if (value is Map<*, *> && previous is Map<*, *>)
                merge(previous as Map<String, Any?>, value as Map<String, Any?>, if (path.isEmpty()) key else "$path.$key")
            else value
        }
        return result
    }

    private fun number(value: Double): Any = if (value == floor(value) && kotlin.math.abs(value) < 1e15) value.toLong() else value

    private class Reader {
        val problems = mutableListOf<String>()

        fun config(document: Map<String, Any?>): AppConfig? {
            val defaults = AppConfig()
            val appearanceMap = obj(document, "appearance", "appearance")
            val agentMap = obj(document, "agent", "agent")
            val overlayMap = obj(document, "overlay", "overlay")
            val terminalMap = obj(document, "terminal", "terminal")
            val appearance = appearanceMap?.let { map ->
                val theme = enum(map, "theme", "appearance.theme", ThemeMode.entries) ?: defaults.appearance.theme
                val density = enum(map, "density", "appearance.density", Density.entries) ?: defaults.appearance.density
                val scale = double(map, "fontScale", "appearance.fontScale", Appearance.FONT_SCALE) ?: defaults.appearance.fontScale
                val mono = double(map, "monoFontSize", "appearance.monoFontSize", Appearance.MONO_SIZE) ?: defaults.appearance.monoFontSize
                Appearance(theme, density, scale, mono)
            } ?: defaults.appearance
            val agent = agentMap?.let { map ->
                val backend = string(map, "backend", "agent.backend")?.also { if (it.isBlank()) problems += "agent.backend: must not be empty" }
                val permissions = string(map, "permissions", "agent.permissions")
                val backends = obj(map, "backends", "agent.backends")?.mapNotNull { (id, value) ->
                    @Suppress("UNCHECKED_CAST")
                    val entry = value as? Map<String, Any?>
                    if (entry == null) { problems += "agent.backends.$id: expected an object"; null }
                    else id to BackendDefaults(string(entry, "model", "agent.backends.$id.model"), string(entry, "effort", "agent.backends.$id.effort"))
                }?.toMap() ?: emptyMap()
                if (backend != null && backend.isBlank()) null else AgentDefaults(backend ?: defaults.agent.backend, permissions, backends)
            } ?: defaults.agent
            val overlay = overlayMap?.let { map ->
                OverlayConfig(
                    enabled = bool(map, "enabled", "overlay.enabled") ?: false,
                    extraApps = strings(map, "extraApps", "overlay.extraApps")?.mapNotNull { appRef(it, "overlay.extraApps") } ?: emptyList(),
                )
            } ?: defaults.overlay
            val launcher = strings(document, "launcher", "launcher")?.mapNotNull { value ->
                try { LauncherEntry.parse(value) } catch (error: IllegalArgumentException) { problems += "launcher: ${error.message}"; null }
            }?.let(Launcher::normalize) ?: defaults.launcher
            val terminal = terminalMap?.let { TerminalConfig(bool(it, "extraKeysPinned", "terminal.extraKeysPinned") ?: false) } ?: defaults.terminal
            return if (problems.isEmpty()) AppConfig(appearance, agent, overlay, launcher, terminal) else null
        }

        private fun appRef(value: String, path: String): AppRef? =
            try { AppRef.parse(value) } catch (error: IllegalArgumentException) { problems += "$path: ${error.message}"; null }

        @Suppress("UNCHECKED_CAST")
        fun obj(map: Map<String, Any?>, key: String, path: String): Map<String, Any?>? {
            val value = map[key] ?: return null
            return value as? Map<String, Any?> ?: run { problems += "$path: expected an object"; null }
        }

        fun string(map: Map<String, Any?>, key: String, path: String): String? {
            val value = map[key] ?: return null
            return value as? String ?: run { problems += "$path: expected a string"; null }
        }

        fun long(map: Map<String, Any?>, key: String): Long? {
            val value = map[key] ?: return null
            return (value as? Long) ?: run { problems += "$key: expected an integer"; null }
        }

        fun bool(map: Map<String, Any?>, key: String, path: String): Boolean? {
            val value = map[key] ?: return null
            return value as? Boolean ?: run { problems += "$path: expected true or false"; null }
        }

        fun double(map: Map<String, Any?>, key: String, path: String, range: ClosedFloatingPointRange<Double>): Double? {
            val value = map[key] ?: return null
            val number = (value as? Number)?.toDouble() ?: run { problems += "$path: expected a number"; return null }
            if (number !in range) { problems += "$path: must be between ${range.start} and ${range.endInclusive}"; return null }
            return number
        }

        fun strings(map: Map<String, Any?>, key: String, path: String): List<String>? {
            val value = map[key] ?: return null
            val list = value as? List<*> ?: run { problems += "$path: expected an array"; return null }
            return list.mapNotNull { item -> item as? String ?: run { problems += "$path: expected strings"; null } }
        }

        fun <E : Enum<E>> enum(map: Map<String, Any?>, key: String, path: String, values: List<E>): E? {
            val value = string(map, key, path) ?: return null
            return values.firstOrNull { it.name.lowercase(Locale.ROOT) == value } ?: run {
                problems += "$path: expected one of ${values.joinToString(", ") { it.name.lowercase(Locale.ROOT) }}"
                null
            }
        }
    }
}
