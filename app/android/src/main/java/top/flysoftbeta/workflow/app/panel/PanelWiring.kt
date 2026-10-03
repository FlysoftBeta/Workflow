package top.flysoftbeta.workflow.app.panel

import android.content.Context
import top.flysoftbeta.workflow.core.layout.PanelKind
import top.flysoftbeta.workflow.feature.chat.ChatPanelProvider
import top.flysoftbeta.workflow.feature.editor.EditorPanelProvider
import top.flysoftbeta.workflow.feature.files.FilesExplorerProvider
import top.flysoftbeta.workflow.feature.proxy.ProxyPanelProvider
import top.flysoftbeta.workflow.feature.terminal.TerminalPanelProvider
import top.flysoftbeta.workflow.feature.settings.SettingsPanelProvider

/**
 * The composition point for independent feature panels. A caller can supply a scoped proxy provider
 * without replacing the process-wide service, for example when hosting an isolated acceptance UI.
 */
object PanelWiring {
    fun create(
        context: Context,
        proxy: PanelProvider = ProxyPanelProvider(context.applicationContext),
    ): PanelRegistry {
        val editor = EditorPanelProvider()
        val chat = ChatPanelProvider(top.flysoftbeta.workflow.app.AndroidChatFeatureServices(context.applicationContext))
        return PanelRegistry(
            providers = mapOf(
                PanelKind.FILE to editor,
                PanelKind.IMAGE to editor,
                PanelKind.DIFF to editor,
                PanelKind.TERMINAL to TerminalPanelProvider(context),
                PanelKind.CONVERSATION to chat,         // feature.chat
                PanelKind.PROXY to proxy,
                PanelKind.SETTINGS to SettingsPanelProvider(top.flysoftbeta.workflow.app.AndroidSettingsFeatureServices(context.applicationContext)),
            ),
            explorer = FilesExplorerProvider(context),
            rail = chat,                                // feature.chat
            fallback = UnavailablePanelProvider(),
        )
    }
}
