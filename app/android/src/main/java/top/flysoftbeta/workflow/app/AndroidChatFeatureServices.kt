package top.flysoftbeta.workflow.app

import android.content.Context
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import top.flysoftbeta.workflow.feature.chat.ChatEnvironmentState
import top.flysoftbeta.workflow.feature.chat.ChatFeatureServices
import top.flysoftbeta.workflow.platform.agent.ClaudeCodeInstaller
import top.flysoftbeta.workflow.platform.engine.EnvironmentHealth
import top.flysoftbeta.workflow.platform.importer.ImportService

/** Composition root: the chat feature consumes ports and never locates the workspace itself. */
class AndroidChatFeatureServices(private val context: Context) : ChatFeatureServices {
    override val hub get() = AppGraph.agentHub(context)
    override val importer get() = ImportService.get(context)
    override val processScope get() = AppGraph.session(context).scope
    override val environment by lazy {
        val engine = AppGraph.engine(context)
        combine(engine.health, AppGraph.claudeInstaller(context).state) { health, claude ->
            ChatEnvironmentState(engine.describe(health), health.usable,
                canRetry = health is EnvironmentHealth.Failed,
                claudeInstalling = claude is ClaudeCodeInstaller.State.Installing,
                claudeProgress = (claude as? ClaudeCodeInstaller.State.Installing)?.progress,
                claudeFailure = (claude as? ClaudeCodeInstaller.State.Failed)?.message,
            )
        }.stateIn(processScope, SharingStarted.Eagerly, ChatEnvironmentState("正在准备环境", false))
    }
    override fun retryEnvironment() = AppGraph.engine(context).retry()
    override fun installClaude() = AppGraph.claudeInstaller(context).install()
    override suspend fun readResource(path: String, maxBytes: Int): ByteArray? =
        runCatching { AppGraph.session(context).store.readBytes(path, maxBytes) }.getOrNull()
}
