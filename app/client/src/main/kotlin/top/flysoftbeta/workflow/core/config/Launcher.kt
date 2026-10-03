package top.flysoftbeta.workflow.core.config

enum class BuiltinApp(val id: String) {
    WORKBENCH("workbench"), PROXY("proxy"), SETTINGS("settings");

    companion object {
        fun of(id: String): BuiltinApp? = entries.firstOrNull { it.id == id }
    }
}

/**
 * A third-party app: a package, optionally with an explicit launcher activity.
 * Written in config.json as "package" or "package/activity".
 */
data class AppRef(val packageName: String, val activity: String? = null) {
    init {
        require(PACKAGE.matches(packageName)) { "Invalid package name: $packageName" }
        require(activity == null || ACTIVITY.matches(activity)) { "Invalid activity: $activity" }
    }

    val id: String get() = if (activity == null) packageName else "$packageName/$activity"

    companion object {
        private val PACKAGE = Regex("[A-Za-z][A-Za-z0-9_]*(\\.[A-Za-z0-9_]+)+")
        private val ACTIVITY = Regex("\\.?[A-Za-z_$][A-Za-z0-9_$]*(\\.[A-Za-z_$][A-Za-z0-9_$]*)*")

        fun parse(value: String): AppRef {
            val slash = value.indexOf('/')
            return if (slash < 0) AppRef(value) else AppRef(value.substring(0, slash), value.substring(slash + 1))
        }
    }
}

/** An entry of the Launcher's Apps grid. */
sealed interface LauncherEntry {
    val id: String

    data class Builtin(val app: BuiltinApp) : LauncherEntry {
        override val id: String get() = app.id
    }

    data class App(val ref: AppRef) : LauncherEntry {
        override val id: String get() = ref.id
    }

    companion object {
        fun parse(value: String): LauncherEntry = BuiltinApp.of(value)?.let(::Builtin) ?: App(AppRef.parse(value))
    }
}

/**
 * Pure launcher list operations (applied through `WorkspaceStore.updateConfig`). All built-in
 * apps remain present and can be reordered. Third-party apps appear only after being added.
 */
object Launcher {
    val DEFAULT: List<LauncherEntry> = BuiltinApp.entries.map { LauncherEntry.Builtin(it) }

    fun normalize(entries: List<LauncherEntry>): List<LauncherEntry> {
        val unique = entries.distinctBy { it.id }
        val workbench = LauncherEntry.Builtin(BuiltinApp.WORKBENCH)
        val missing = DEFAULT.filter { required -> unique.none { it.id == required.id } }
        return if (unique.any { it.id == workbench.id }) unique + missing
        else listOf(workbench) + unique + missing.filter { it.id != workbench.id }
    }

    fun add(entries: List<LauncherEntry>, entry: LauncherEntry): List<LauncherEntry> =
        if (entries.any { it.id == entry.id }) entries else normalize(entries + entry)

    fun remove(entries: List<LauncherEntry>, id: String): List<LauncherEntry> =
        if (BuiltinApp.of(id) != null) entries else entries.filter { it.id != id }

    /** Drag reorder: moves [id] to [index] of the resulting list (clamped). */
    fun move(entries: List<LauncherEntry>, id: String, index: Int): List<LauncherEntry> {
        val entry = entries.firstOrNull { it.id == id } ?: return entries
        val rest = entries.filter { it.id != id }
        return rest.toMutableList().apply { add(index.coerceIn(0, rest.size), entry) }
    }

    /** Apps offered by the overlay's quick switcher: launcher apps, then overlay-only extras. */
    fun overlayApps(config: AppConfig): List<LauncherEntry> =
        normalize(config.launcher + config.overlay.extraApps.map { LauncherEntry.App(it) })
}
