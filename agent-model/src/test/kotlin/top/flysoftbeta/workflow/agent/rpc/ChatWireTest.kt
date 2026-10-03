package top.flysoftbeta.workflow.agent.rpc

import kotlinx.serialization.*
import kotlinx.serialization.descriptors.*
import kotlinx.serialization.json.*
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.agent.model.*

@OptIn(ExperimentalSerializationApi::class)
class ChatWireTest {
    @Test fun everySealedVariantRoundTrips() {
        listOf(Item.serializer(), ItemDelta.serializer(), AgentEvent.serializer(), RequestKind.serializer(), RequestResponse.serializer(), LoginFlow.serializer(), ProcessState.serializer(), UserPart.serializer()).forEach { serializer ->
            roundTripVariants(serializer)
        }
    }
    private fun <T> roundTripVariants(serializer: KSerializer<T>) {
        val variants = serializer.descriptor.getElementDescriptor(1)
        for (i in 0 until variants.elementsCount) {
            val descriptor = variants.getElementDescriptor(i)
            val value = buildJsonObject {
                put("_type", descriptor.serialName)
                sample(descriptor).jsonObject.forEach { (name, field) -> put(name, field) }
            }
            val decoded = ChatWire.json.decodeFromJsonElement(serializer, value)
            val wire = ChatWire.json.encodeToJsonElement(serializer, decoded)
            assertEquals(descriptor.serialName, decoded, ChatWire.json.decodeFromJsonElement(serializer, wire))
        }
    }
    private fun sample(d: SerialDescriptor): JsonElement {
        if (d.isNullable) return JsonNull
        if (d.serialName.startsWith("kotlinx.serialization.json.")) return buildJsonObject { put("futureField", JsonArray(listOf(JsonPrimitive(9007199254740993L), JsonPrimitive("007")))) }
        return when (d.kind) {
            PrimitiveKind.STRING, PrimitiveKind.CHAR -> JsonPrimitive("sample")
            PrimitiveKind.BOOLEAN -> JsonPrimitive(false)
            is PrimitiveKind -> JsonPrimitive(1)
            SerialKind.ENUM -> JsonPrimitive(d.getElementName(0))
            StructureKind.LIST -> JsonArray(emptyList())
            StructureKind.MAP -> if (d.getElementDescriptor(0).kind is PrimitiveKind || d.getElementDescriptor(0).kind == SerialKind.ENUM) JsonObject(emptyMap()) else JsonArray(emptyList())
            PolymorphicKind.SEALED -> {
                val concrete = d.getElementDescriptor(1).getElementDescriptor(0)
                JsonObject(mapOf("_type" to JsonPrimitive(concrete.serialName)) + sample(concrete).jsonObject)
            }
            else -> buildJsonObject {
                for (i in 0 until d.elementsCount) if (!d.isElementOptional(i)) put(d.getElementName(i), sample(d.getElementDescriptor(i)))
            }
        }
    }
    @Test fun snapshotPreservesRawRequestIdTypesAndStreamContent() {
        val numeric = RequestKey(BackendKind.CODEX, JsonPrimitive(9007199254740993L))
        val string = RequestKey(BackendKind.CODEX, JsonPrimitive("9007199254740993"))
        val thread = ThreadKey(BackendKind.CODEX, "thread")
        val state = AgentState(threads = mapOf(thread to ThreadState(thread, turns = listOf(Turn("turn", items = listOf(AgentMessageItem("message", StreamText.of("Hello").append(" 世界")), UnknownItem("unknown", "future", raw = buildJsonObject { put("nested", JsonArray(listOf(JsonPrimitive(true)))) })))))))
        assertEquals(state, ChatWire.decode<AgentState>(ChatWire.encode(state)))
        val ids = mapOf(numeric to "number", string to "string")
        assertEquals(ids, ChatWire.decode<Map<RequestKey, String>>(ChatWire.encode(ids)))
        assertNotEquals(numeric, string)
    }
}
