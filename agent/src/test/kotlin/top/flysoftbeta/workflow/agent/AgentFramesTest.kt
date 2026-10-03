package top.flysoftbeta.workflow.agent

import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonObject
import org.junit.Assert.*
import org.junit.Test

class AgentFramesTest {
    @Test fun unknownServerRequestKeepsItsRawIdAndOpaqueParameters() {
        val text = """{"id":42,"method":"future/requestApproval","params":{"opaque":[1,{"future":true}],"big":12345678901234567890}}"""
        val frame = AgentFrames.parse(text)
        assertEquals(JsonPrimitive(42), frame["id"])
        assertEquals("42", frame["id"].toString())
        assertEquals("future/requestApproval", (frame["method"] as JsonPrimitive).content)
        assertEquals(text, AgentFrames.encode(frame))
        assertEquals(frame, AgentFrames.parse(AgentFrames.encode(frame)))
        assertTrue(frame["params"]!!.jsonObject.containsKey("big"))
    }
}
