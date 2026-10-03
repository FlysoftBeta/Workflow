package top.flysoftbeta.workflow.agent

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonObject

/**
 * W1a skeleton of the agent module (backend-neutral API, Codex and Claude Code adapters, transport).
 * Protocol frames stay as [JsonObject]: unknown methods, fields and server request IDs survive untouched.
 */
object AgentFrames {
    private val format = Json

    fun parse(text: String): JsonObject = format.parseToJsonElement(text).jsonObject

    fun encode(frame: JsonObject): String = format.encodeToString(JsonObject.serializer(), frame)
}
