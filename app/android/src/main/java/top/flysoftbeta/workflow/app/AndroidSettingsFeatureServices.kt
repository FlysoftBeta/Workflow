package top.flysoftbeta.workflow.app

import android.content.Context
import top.flysoftbeta.workflow.feature.settings.SettingsFeatureServices

class AndroidSettingsFeatureServices(private val context: Context) : SettingsFeatureServices {
    override val hub get() = AppGraph.agentHub(context)
    override val environment get() = AppGraph.engine(context).health
    override fun retryEnvironment() = AppGraph.engine(context).retry()
    override suspend fun restartEnvironment() = AppGraph.engine(context).restartEnvironment()
}
