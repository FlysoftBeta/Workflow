package top.flysoftbeta.workflow.platform

import kotlinx.coroutines.flow.MutableStateFlow

/** A transient navigation request from a platform window; settings consumes it on arrival. */
object SettingsEntryPoint {
    enum class Destination { OVERLAY_APPS, PERMISSIONS }
    val destination = MutableStateFlow<Destination?>(null)
}
