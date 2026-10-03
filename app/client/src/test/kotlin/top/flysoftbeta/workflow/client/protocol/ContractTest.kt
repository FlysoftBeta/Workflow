package top.flysoftbeta.workflow.client.protocol

import kotlinx.serialization.json.*
import org.junit.Assert.*
import org.junit.Test
import java.security.MessageDigest

class ContractTest {
    private fun resource(name: String) = checkNotNull(javaClass.getResourceAsStream("/$name")).readBytes()
    @Test fun catalogAndGeneratedBindingsMatchExactRustContract() {
        val contractBytes = resource("contract.json")
        val schemaBytes = resource("schema.json")
        val digest = MessageDigest.getInstance("SHA-256").digest(schemaBytes + contractBytes).joinToString("") { "%02x".format(it) }
        assertEquals("Run tools/generate-client-protocol.py after Rust schema changes", ENGINE_CONTRACT_SHA256, digest)
        val catalog = Json.parseToJsonElement(contractBytes.toString(Charsets.UTF_8)).jsonObject
        assertEquals(catalog.getValue("methods").jsonArray.map { it.jsonPrimitive.content }.toSet(), EngineMethods.all.map { it.name }.toSet())
    }
    @Test fun everyRustGoldenDecodesAndReencodesWithoutLosingFields() {
        val results = mapOf("hello-response" to EngineMethods.hello, "empty-workspace" to EngineMethods.workspace_snapshot,
            "session-created" to EngineMethods.workspace_command, "file-chunk" to EngineMethods.files_read,
            "terminal-output" to EngineMethods.terminal_read, "null-id-response" to EngineMethods.documents_write,
            "precise-id-response" to EngineMethods.documents_write)
        val golden = Json.parseToJsonElement(resource("golden.json").toString(Charsets.UTF_8)).jsonArray
        for (example in golden) {
            val name = example.jsonObject.getValue("name").jsonPrimitive.content
            val message = example.jsonObject.getValue("message")
            val envelope = RpcEnvelope.decode(message)
            assertEquals(name, message, parseWire(envelope.json.toString()))
            envelope.method?.let { method ->
                EngineMethods.all.firstOrNull { it.name == method }?.let { binding ->
                    val params = envelope.params?.json ?: JsonObject(emptyMap())
                    assertEquals(name, params, binding.decodeParams(params).json)
                }
            }
            results[name]?.let { method ->
                val value = checkNotNull(envelope.result).json
                assertEquals(name, value, method.decodeResult(value).json)
            }
        }
    }
    @Test fun exactNumericIdsMissingAndNullRemainDistinctAndUnknownRequestsSurvive() {
        val number = RpcEnvelope.parse("""{"jsonrpc":"2.0","id":9007199254740993,"method":"future/approve","params":{"future":[1,2]}}""")
        val text = RpcEnvelope.parse("""{"jsonrpc":"2.0","id":"9007199254740993","method":"future/approve"}""")
        val missing = RpcEnvelope.parse("""{"jsonrpc":"2.0","method":"future/event"}""")
        val explicitNull = RpcEnvelope.parse("""{"jsonrpc":"2.0","id":null,"method":"future/event"}""")
        assertEquals(EnvelopeId.Number("9007199254740993"), number.id)
        assertNotEquals(number.id, text.id)
        assertNotEquals(missing.id, explicitNull.id)
        assertEquals("future/approve", number.method)
        assertEquals(number.json, RpcEnvelope.parse(number.json.toString()).json)
        assertTrue(runCatching { RpcEnvelope.parse("""{"jsonrpc":"2.0","id":1,"id":2,"result":{}}""") }.isFailure)
    }
    @Test fun optionalNullAndUnknownFieldsRoundTripAndTypedFieldsRejectWrongTypes() {
        for (text in listOf("""{"namespace":"proxy","key":"config","future":{"flag":true}}""",
            """{"namespace":"proxy","key":"config","document":null,"expectedRevision":null}""")) {
            val value = parseWire(text)
            val document = DocumentParams.decode(value)
            assertEquals("proxy", document.namespace)
            assertEquals(value, document.json)
        }
        assertTrue(runCatching { DocumentResult.decode(parseWire("""{"revision":"7"}""")) }.isFailure)
        assertTrue(runCatching { HelloResult.decode(parseWire("{}")) }.isFailure)
    }
    @Test fun serviceDiscriminatorsAndTupleLengthsFollowTheExportedSchema() {
        assertTrue(runCatching { ServiceReportParams.decode(parseWire("""{"serviceId":"proxy","state":{}}""")) }.isFailure)
        assertTrue(runCatching { ServiceReportParams.decode(parseWire("""{"serviceId":"network","state":{}}""")) }.isFailure)
        assertEquals("custom-service", ServiceReportParams.decode(parseWire("""{"serviceId":"custom-service","state":{}}""")).asExtensionReportParams2?.serviceId?.value)
        assertTrue(runCatching { PanelView.decode(parseWire("""{"cursor":[1,2,3]}""")) }.isFailure)
        assertTrue(runCatching { PanelView.decode(parseWire("""{"cursor":[1,2,3,4,5]}""")) }.isFailure)
        assertEquals(4, PanelView.decode(parseWire("""{"cursor":[1,2,3,4]}""")).cursor!!.size)
    }

}
