package top.flysoftbeta.workflow.core.json

import org.junit.Assert.assertEquals
import org.junit.Test

class JsonPrecisionTest {
    @Test fun preservesOpaqueVendorNumbersThroughWorkspaceTransport() {
        val text = "{\"rawId\":123456789012345678901234567890,\"precise\":0.12345678901234567890123456789,\"stringId\":\"123456789012345678901234567890\"}"
        assertEquals(text, Json.stringify(Json.parse(text)))
    }
}
