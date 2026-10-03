package top.flysoftbeta.workflow.core.environment

/** Engine status projection used by the Android shell. */
sealed interface EnvironmentStatus {
    /** [step] of [steps]; `0 of 0` before the first install has started. */
    data class Applying(val stage: Stage, val step: Int, val steps: Int) : EnvironmentStatus
    data object UpToDate : EnvironmentStatus
    data class PendingRestart(val reason: ActivationReason) : EnvironmentStatus {
        /** Environment configuration changes and rollbacks ask for a restart; image updates wait silently. */
        val prompt: Boolean get() = reason != ActivationReason.IMAGE
    }
    data class Failed(val stage: Stage, val message: String, val log: String?, val environmentAvailable: Boolean) : EnvironmentStatus
}

/** Stages exposed by an environment status projection. */
enum class Stage(val wire: String) {
    CONFIG("config"), INSTALL("install"), CLONE("clone"), PACKAGES("packages"), PYTHON("python"), NODE("node"),
    VERIFY("verify"), ACTIVATE("activate");

    companion object {
        fun of(wire: String): Stage = entries.firstOrNull { it.wire == wire } ?: error("Unknown stage $wire")
    }
}

enum class ActivationReason(val wire: String) {
    INSTALL("install"), CONFIG("config"), IMAGE("image"), ROLLBACK("rollback");

    companion object {
        fun of(wire: String): ActivationReason = entries.firstOrNull { it.wire == wire } ?: error("Unknown activation reason $wire")
    }
}
