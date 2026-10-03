package top.flysoftbeta.workflow.agent.json

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonObjectBuilder
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.buildJsonArray
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.doubleOrNull
import kotlinx.serialization.json.longOrNull

/**
 * Protocol JSON codec. Frames are kept as [JsonElement] so unknown fields survive; nothing is
 * decoded into lossy data classes on the transport path.
 */
val ProtocolJson: Json = Json {
    ignoreUnknownKeys = true
    isLenient = false
    explicitNulls = true
    encodeDefaults = true
}

fun parseJson(text: String): JsonElement = ProtocolJson.parseToJsonElement(text)

fun JsonElement.encode(): String = ProtocolJson.encodeToString(JsonElement.serializer(), this)

val JsonElement?.obj: JsonObject? get() = this as? JsonObject
val JsonElement?.arr: JsonArray? get() = this as? JsonArray

/** String value of a primitive string (not of numbers/booleans). */
val JsonElement?.str: String?
    get() = (this as? JsonPrimitive)?.takeIf { it.isString }?.content

/** Any primitive rendered as text (numbers, booleans, strings); null for JSON null / containers. */
val JsonElement?.text: String?
    get() = (this as? JsonPrimitive)?.takeIf { it !is JsonNull }?.content

val JsonElement?.long: Long? get() = (this as? JsonPrimitive)?.takeIf { !it.isString }?.longOrNull
val JsonElement?.int: Int? get() = long?.toInt()
val JsonElement?.double: Double? get() = (this as? JsonPrimitive)?.takeIf { !it.isString }?.doubleOrNull
val JsonElement?.bool: Boolean? get() = (this as? JsonPrimitive)?.takeIf { !it.isString }?.booleanOrNull
val JsonElement?.isNullish: Boolean get() = this == null || this is JsonNull

operator fun JsonElement?.get(key: String): JsonElement? = (this as? JsonObject)?.get(key)

fun JsonElement?.path(vararg keys: String): JsonElement? {
    var cur: JsonElement? = this
    for (key in keys) cur = cur[key] ?: return null
    return cur
}

fun JsonElement?.strings(): List<String> = arr?.mapNotNull { it.str } ?: emptyList()

fun jsonOf(vararg pairs: Pair<String, Any?>): JsonObject = buildJsonObject { putAll(*pairs) }

fun JsonObjectBuilder.putAll(vararg pairs: Pair<String, Any?>) {
    for ((key, value) in pairs) put(key, toJson(value))
}

/** Converts simple Kotlin values into JSON. Maps must have string keys. */
fun toJson(value: Any?): JsonElement = when (value) {
    null -> JsonNull
    is JsonElement -> value
    is String -> JsonPrimitive(value)
    is Number -> JsonPrimitive(value)
    is Boolean -> JsonPrimitive(value)
    is Map<*, *> -> buildJsonObject { value.forEach { (k, v) -> put(k as String, toJson(v)) } }
    is Iterable<*> -> buildJsonArray { value.forEach { add(toJson(it)) } }
    is Array<*> -> buildJsonArray { value.forEach { add(toJson(it)) } }
    is Enum<*> -> JsonPrimitive(value.name)
    else -> throw IllegalArgumentException("Unsupported JSON value ${value::class}")
}

/** Returns a copy of [this] with [key] set, preserving the order of all other members. */
fun JsonObject.plusMember(key: String, value: JsonElement): JsonObject = JsonObject(LinkedHashMap(this).apply { put(key, value) })

fun JsonObject.minusMember(key: String): JsonObject = JsonObject(LinkedHashMap(this).apply { remove(key) })

private val ANSI = Regex("\u001B\\[[0-?]*[ -/]*[@-~]|\u001B\\][^\u0007\u001B]*(\u0007|\u001B\\\\)|\u001B[@-Z\\\\-_]")

/** Removes terminal escape sequences and other control characters (except newline/tab) from producer-authored text. */
fun sanitizeDisplayText(value: String?): String? = value
    ?.replace(ANSI, "")
    ?.filter { it == '\n' || it == '\t' || !it.isISOControl() }
