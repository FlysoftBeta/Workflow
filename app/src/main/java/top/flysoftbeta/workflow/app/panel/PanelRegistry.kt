package top.flysoftbeta.workflow.app.panel

import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.flowOf
import top.flysoftbeta.workflow.core.layout.PanelKind
import top.flysoftbeta.workflow.core.resource.ResourceRef

/**
 * The process-wide set of panel providers and region slots. Built once in
 * [top.flysoftbeta.workflow.app.AppGraph.panelRegistry] from [PanelWiring]; feature workstreams replace
 * their placeholder there (one line each).
 */
class PanelRegistry(
    private val providers: Map<PanelKind, PanelProvider>,
    val explorer: ExplorerProvider,
    val rail: RailProvider,
    private val fallback: PanelProvider,
) {
    fun provider(kind: PanelKind): PanelProvider = providers[kind] ?: fallback

    /** Display title of a resource: the owning provider's title, else the file name. */
    fun titleOf(ref: ResourceRef): String? = when (ref) {
        is ResourceRef.File -> provider(PanelKind.FILE).resourceTitle(ref) ?: ref.path.substringAfterLast('/')
        is ResourceRef.Conversation -> provider(PanelKind.CONVERSATION).resourceTitle(ref)
    }

    /** Any provider reports a request waiting for the user. */
    val attention: Flow<Boolean> = providers.values.distinct().mapNotNull { it.attention }.let { flows ->
        if (flows.isEmpty()) flowOf(false) else combine(flows) { values -> values.any { it } }
    }
}
