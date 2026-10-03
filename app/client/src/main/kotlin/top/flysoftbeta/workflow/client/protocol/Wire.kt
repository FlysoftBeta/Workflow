package top.flysoftbeta.workflow.client.protocol

import java.math.BigDecimal
import java.math.BigInteger
import kotlinx.serialization.json.*
import top.flysoftbeta.workflow.core.json.Json as StrictJson

/** A typed, lossless view. Encoding retains unknown fields and absent versus explicit null values. */
interface WireValue { val json: JsonElement }

class RpcMethod<P : WireValue, R : WireValue>(
    val name: String,
    val decodeParams: (JsonElement) -> P,
    val decodeResult: (JsonElement) -> R,
)

/** JSON access is confined to this codec and generated bindings, never spread through new consumers. */
internal object Wire {
    fun obj(value: JsonElement) = value as? JsonObject ?: error("Expected object")
    fun array(value: JsonElement) = value as? JsonArray ?: error("Expected array")
    fun string(value: JsonElement): String = (value as? JsonPrimitive)?.takeIf { it.isString }?.content ?: error("Expected string")
    fun integer(value: JsonElement): BigInteger = (value as? JsonPrimitive)?.takeIf { !it.isString && INTEGER.matches(it.content) }
        ?.content?.toBigInteger() ?: error("Expected integer")
    fun number(value: JsonElement): BigDecimal = (value as? JsonPrimitive)?.takeIf { !it.isString }?.content?.toBigDecimalOrNull() ?: error("Expected number")
    fun boolean(value: JsonElement): Boolean = (value as? JsonPrimitive)?.takeIf { !it.isString }?.booleanOrNull ?: error("Expected boolean")
    fun nil(value: JsonElement): JsonNull { require(value === JsonNull); return JsonNull }
    fun member(value: JsonObject, name: String, default: String? = null): JsonElement = value[name]
        ?: default?.let(Json::parseToJsonElement) ?: error("Missing $name")
    fun optional(value: JsonObject, name: String): JsonElement? = value[name]?.takeUnless { it === JsonNull }
    inline fun <T> nullable(value: JsonElement, decode: (JsonElement) -> T): T? = if (value === JsonNull) null else decode(value)
    inline fun <T> attempt(decode: () -> T): T? = try { decode() } catch (_: IllegalArgumentException) { null } catch (_: IllegalStateException) { null }
    inline fun matches(decode: () -> Any?): Boolean = try { decode(); true } catch (_: IllegalArgumentException) { false } catch (_: IllegalStateException) { false }
    private val INTEGER = Regex("-?(0|[1-9][0-9]*)")
}

/** Strict duplicate-key validation plus kotlinx JSON's exact numeric representation. */
fun parseWire(text: String): JsonElement {
    StrictJson.parse(text)
    return Json.parseToJsonElement(text)
}

sealed interface EnvelopeId {
    data object Missing : EnvelopeId
    data object Null : EnvelopeId
    data class Text(val value: String) : EnvelopeId
    /** Lexical JSON numeric token: never passes through Double, including large server request IDs. */
    data class Number(val token: String) : EnvelopeId
}

data class RpcFailure(val code: Long, val message: String, val data: OpaqueJson?)

class RpcEnvelope private constructor(override val json: JsonElement) : WireValue {
    private val fields = Wire.obj(json)
    val id: EnvelopeId = when (val value = fields["id"]) {
        null -> EnvelopeId.Missing
        JsonNull -> EnvelopeId.Null
        else -> if ((value as? JsonPrimitive)?.isString == true) EnvelopeId.Text(Wire.string(value))
            else { Wire.number(value); EnvelopeId.Number(value.toString()) }
    }
    val method: String? = fields["method"]?.let(Wire::string)
    val params: OpaqueJson? = fields["params"]?.let(::OpaqueJson)
    val result: OpaqueJson? = fields["result"]?.let(::OpaqueJson)
    val error: RpcFailure? = fields["error"]?.let { value ->
        val error = Wire.obj(value)
        RpcFailure(Wire.integer(Wire.member(error, "code")).longValueExact(), Wire.string(Wire.member(error, "message")), error["data"]?.let(::OpaqueJson))
    }
    init {
        require(Wire.string(Wire.member(fields, "jsonrpc")) == "2.0")
        if (method == null) require(id != EnvelopeId.Missing && (result != null) != (error != null)) { "Invalid RPC response" }
        else require(result == null && error == null) { "Invalid RPC request" }
    }
    companion object {
        fun decode(value: JsonElement) = RpcEnvelope(value)
        fun parse(text: String) = decode(parseWire(text))
        fun request(id: String, method: String, params: JsonElement) = buildJsonObject {
            put("jsonrpc", "2.0"); put("id", id); put("method", method); put("params", params)
        }
    }
}
