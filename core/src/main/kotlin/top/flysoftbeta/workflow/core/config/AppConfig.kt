package top.flysoftbeta.workflow.core.config

enum class ThemeMode { SYSTEM, LIGHT, DARK }
enum class Density { COMPACT, STANDARD }

data class Appearance(
    val theme: ThemeMode = ThemeMode.SYSTEM,
    val density: Density = Density.COMPACT,
    /** UI text scale on top of the system font scale (ui.md §7: up to 1.3). */
    val fontScale: Double = 1.0,
    /** Editor and terminal font size in sp; pinch-zoom writes it back (10–20). */
    val monoFontSize: Double = 13.0,
) {
    init {
        require(fontScale.isFinite() && fontScale in FONT_SCALE) { "fontScale must be within $FONT_SCALE" }
        require(monoFontSize.isFinite() && monoFontSize in MONO_SIZE) { "monoFontSize must be within $MONO_SIZE" }
    }

    companion object {
        val FONT_SCALE = 0.8..1.3
        val MONO_SIZE = 10.0..20.0
    }
}

/** Last chosen model and effort for a backend (the model + effort slider). */
data class BackendDefaults(val model: String? = null, val effort: String? = null)

/**
 * Agent defaults. Backend ids are the agent layer's ids ("codex", "claude"); :core does not
 * interpret them. [backend] is the backend for new conversations (remembered from the last choice).
 */
data class AgentDefaults(
    val backend: String = "codex",
    val permissions: String? = null,
    val backends: Map<String, BackendDefaults> = emptyMap(),
) {
    init { require(backend.isNotBlank()) { "agent.backend must not be empty" } }
}

/** Floating overlay. [extraApps] are apps shown only in the overlay's quick switcher. */
data class OverlayConfig(val enabled: Boolean = false, val extraApps: List<AppRef> = emptyList())

data class TerminalConfig(val extraKeysPinned: Boolean = false)

/**
 * Typed `.workspace/config.json` (version 2). Editing the file and using Settings are equivalent;
 * keys the app does not know are preserved when it writes.
 */
data class AppConfig(
    val appearance: Appearance = Appearance(),
    val agent: AgentDefaults = AgentDefaults(),
    val overlay: OverlayConfig = OverlayConfig(),
    val launcher: List<LauncherEntry> = Launcher.DEFAULT,
    val terminal: TerminalConfig = TerminalConfig(),
)
