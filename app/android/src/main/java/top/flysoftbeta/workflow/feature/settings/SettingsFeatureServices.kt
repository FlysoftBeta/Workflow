package top.flysoftbeta.workflow.feature.settings

import kotlinx.coroutines.flow.StateFlow
import top.flysoftbeta.workflow.platform.agent.AgentHub
import top.flysoftbeta.workflow.platform.engine.EnvironmentHealth

/** Settings consumes workspace capabilities supplied by the application composition root. */
interface SettingsFeatureServices {
    val hub: AgentHub
    val environment: StateFlow<EnvironmentHealth>
    fun retryEnvironment()
    suspend fun restartEnvironment()
}
