package top.flysoftbeta.workflow.feature.chat

import kotlinx.coroutines.flow.StateFlow
import top.flysoftbeta.workflow.app.panel.NewPanelRequest
import top.flysoftbeta.workflow.app.panel.PanelContext
import top.flysoftbeta.workflow.app.panel.PanelController
import top.flysoftbeta.workflow.app.panel.PanelProvider
import top.flysoftbeta.workflow.app.panel.RailContext
import top.flysoftbeta.workflow.app.panel.RailController
import top.flysoftbeta.workflow.app.panel.RailProvider
import top.flysoftbeta.workflow.core.layout.Panel
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.resource.ResourceRef
import top.flysoftbeta.workflow.platform.agent.AgentHub

/**
 * The chat feature's side of the panel contract (docs/app/connection.md): conversation
 * panels and Chat's conversation rail, both backed by the process-scoped [AgentHub].
 */
class ChatPanelProvider(private val services: ChatFeatureServices) : PanelProvider, RailProvider {
    private val hub: AgentHub get() = services.hub

    override fun create(panel: Panel, context: PanelContext): PanelController {
        val id = (panel.target as PanelTarget.Conversation).conversationId
        return ConversationController(id, context, services)
    }

    override suspend fun newTarget(request: NewPanelRequest): PanelTarget = PanelTarget.Conversation(hub.newConversation())

    override fun resourceTitle(ref: ResourceRef): String? {
        val id = (ref as? ResourceRef.Conversation)?.id ?: return null
        val entry = hub.entry(id) ?: return null
        return entry.title ?: ChatText.untitled(entry.preview)
    }

    override val attention: StateFlow<Boolean> get() = hub.attention

    override fun create(context: RailContext): RailController = ChatRailController(context, hub)
}
