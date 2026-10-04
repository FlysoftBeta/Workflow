package top.flysoftbeta.workflow.core.store

import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.core.json.Json

/**
 * Saves [declaration] (a JSON object) as the `environment` section of config.json, keeping every
 * other key, the way a user edits the declaration in the editor.
 */
suspend fun WorkspaceStore.saveEnvironmentDeclaration(declaration: String): SaveResult {
    @Suppress("UNCHECKED_CAST")
    val config = LinkedHashMap(Json.parse(openFile(WorkspacePaths.CONFIG).text) as Map<String, Any?>)
    config["environment"] = Json.parse(declaration)
    return saveFile(WorkspacePaths.CONFIG, Json.stringify(config, pretty = true) + "\n")
}
