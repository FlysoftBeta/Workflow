package top.flysoftbeta.workflow.feature.chat

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.StateFlow
import top.flysoftbeta.workflow.platform.agent.AgentHub
import top.flysoftbeta.workflow.platform.importer.ImportService

/** Injected by the shell; the feature neither locates services nor reads client workspace files. */
interface ChatFeatureServices {
    val hub: AgentHub
    val importer: ImportService
    val processScope: CoroutineScope
    val environment: StateFlow<ChatEnvironmentState>
    fun retryEnvironment()
    fun installClaude()
    /** Engine-owned workspace resources only. Null means absent/unsupported; bound before transfer. */
    suspend fun readResource(path: String, maxBytes: Int): ByteArray?
}

data class ChatEnvironmentState(
    val description: String,
    val usable: Boolean,
    val canRetry: Boolean = false,
    val claudeInstalling: Boolean = false,
    val claudeProgress: Float? = null,
    val claudeFailure: String? = null,
)
