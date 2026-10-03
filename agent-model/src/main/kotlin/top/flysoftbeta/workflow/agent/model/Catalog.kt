package top.flysoftbeta.workflow.agent.model

import kotlinx.serialization.Serializable
import kotlinx.serialization.SerialName

import kotlinx.serialization.json.JsonElement

@Serializable
@SerialName("EffortOption")
data class EffortOption(val id: String, val description: String? = null)

/**
 * A selectable model. [efforts] are the reasoning-effort detents this model supports (empty when
 * the model has no effort control). [defaultEffort] is the backend's default for this model.
 */
@Serializable
@SerialName("ModelOption")
data class ModelOption(
    val id: String,
    val displayName: String,
    val description: String? = null,
    val efforts: List<EffortOption> = emptyList(),
    val defaultEffort: String? = null,
    val isDefault: Boolean = false,
    val hidden: Boolean = false,
    /** `text`, `image`, `pdf`...; drives attachment gating. Empty = unknown, treated as text only. */
    val inputModalities: Set<String> = emptySet(),
    /** Backend resolves this alias to [resolvedModel] (Claude `default` → `claude-opus-…`). */
    val resolvedModel: String? = null,
    /** Deprecated model: suggested replacement and backend-authored migration copy. */
    val upgradeTo: String? = null,
    val upgradeMessage: String? = null,
    val raw: JsonElement? = null,
)

/** One segment of the model/effort slider: a model and its effort detents. */
@Serializable
@SerialName("SliderSegment")
data class SliderSegment(val model: ModelOption, val detents: List<String>, val defaultDetent: Int)

/** A position on the slider. [effort] is null for models without effort control. */
@Serializable
@SerialName("SliderPosition")
data class SliderPosition(val model: String, val effort: String?)

@Serializable
@SerialName("ModelCatalog")
data class ModelCatalog(
    val backend: BackendKind,
    val models: List<ModelOption>,
) {
    val defaultModel: ModelOption? get() = models.firstOrNull { it.isDefault } ?: models.firstOrNull()

    fun model(id: String?): ModelOption? = models.firstOrNull { it.id == id }

    /**
     * Data for the single model+effort slider: visible models in backend order, each with its own
     * effort detents. A hidden model is included only when it is [current] (e.g. a reserve model
     * the backend switched to), so the slider can still show where the thread is.
     */
    fun slider(current: String? = null): List<SliderSegment> = models
        .filter { !it.hidden || it.id == current }
        .map { model ->
            val detents = model.efforts.map { it.id }
            val index = detents.indexOf(model.defaultEffort).takeIf { it >= 0 } ?: 0
            SliderSegment(model, detents, if (detents.isEmpty()) -1 else index)
        }

    /** Clamps a remembered selection to what the catalog offers now. */
    fun resolve(position: SliderPosition?): SliderPosition? {
        val remembered = model(position?.model)
        val model = remembered ?: defaultModel ?: return null
        val effort = when {
            model.efforts.isEmpty() -> null
            // A remembered effort only applies to the model it was chosen for.
            remembered != null && model.efforts.any { it.id == position?.effort } -> position?.effort
            else -> model.defaultEffort ?: model.efforts.first().id
        }
        return SliderPosition(model.id, effort)
    }

    fun supports(modelId: String?, modality: String): Boolean {
        val model = model(modelId) ?: defaultModel ?: return false
        return modality in model.inputModalities
    }
}
