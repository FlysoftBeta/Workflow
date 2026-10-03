package top.flysoftbeta.workflow.agent

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import top.flysoftbeta.workflow.agent.codex.CodexEvents
import top.flysoftbeta.workflow.agent.codex.CodexProtocol
import top.flysoftbeta.workflow.agent.json.arr
import top.flysoftbeta.workflow.agent.json.get
import top.flysoftbeta.workflow.agent.json.parseJson
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.json.strings
import top.flysoftbeta.workflow.agent.testing.Fixtures

/**
 * Keeps the generated constants, the coverage classification and the golden table
 * (agent/protocol-coverage.md) in sync with the protocol inventories.
 */
class ProtocolCoverageTest {
    private val codexInventory = parseJson(Fixtures.resource("/protocol/codex/inventory.json"))
    private val claudeInventory = parseJson(Fixtures.resource("/protocol/claude/inventory.json"))

    private fun methods(key: String) = codexInventory[key].arr!!.map { it["method"].str!! }

    @Test fun generatedCodexConstantsMatchInventory() {
        assertEquals("rerun tools/update-protocol.py --kotlin-only", methods("clientRequests"), CodexProtocol.ClientRequest.ALL)
        assertEquals(methods("serverRequests"), CodexProtocol.ServerRequest.ALL)
        assertEquals(methods("serverNotifications"), CodexProtocol.ServerNotification.ALL)
        assertEquals(methods("clientNotifications"), CodexProtocol.ClientNotification.ALL)
        assertEquals(codexInventory["codexVersion"].str, CodexProtocol.VERSION)
    }

    @Test fun classificationsOnlyNameRealMethods() {
        val all = (CodexProtocol.ClientRequest.ALL + CodexProtocol.ServerRequest.ALL + CodexProtocol.ServerNotification.ALL).toSet()
        for (m in CodexCoverage.USED_REQUESTS + CodexEvents.MODELED_NOTIFICATIONS + CodexEvents.REFRESH_NOTIFICATIONS + CodexCoverage.NOTES.keys) {
            assertTrue("stale classification $m", m in all)
        }
        val subtypes = claudeInventory["controlSubtypes"].strings().toSet()
        for (s in ClaudeCoverage.USED_CONTROL + ClaudeCoverage.INBOUND_CONTROL.keys) assertTrue("stale subtype $s", s in subtypes)
        val kinds = claudeKinds().toSet() + ClaudeCoverage.OBSERVED_EXTRA
        for (k in ClaudeCoverage.MODELED_MESSAGES + ClaudeCoverage.TRANSPORT_MESSAGES) assertTrue("stale message kind $k", k in kinds)
    }

    @Test fun everyServerRequestHasAnExplicitPolicy() {
        val entries = CodexCoverage.entries().filter { it.direction == "server→client request" }
        assertEquals(CodexProtocol.ServerRequest.ALL.size, entries.size)
        assertTrue(entries.none { it.coverage == Coverage.GENERIC })
        val claude = ClaudeCoverage.entries(claudeInventory["controlSubtypes"].strings(), claudeKinds()).filter { it.direction.startsWith("cli→client control") }
        assertEquals(ClaudeCoverage.INBOUND_CONTROL.keys, claude.map { it.method }.toSet())
    }

    private fun claudeKinds() = claudeInventory["messages"].arr!!.map { m -> listOfNotNull(m["type"].str, m["subtype"].str).joinToString("/") }

    @Test fun goldenCoverageTable() {
        val markdown = ProtocolCoverageReport.markdown(
            CodexCoverage.entries(), CodexProtocol.VERSION,
            ClaudeCoverage.entries(claudeInventory["controlSubtypes"].strings(), claudeKinds()), claudeInventory["sdkVersion"].str!!,
        )
        val file = File(System.getProperty("agent.projectDir") ?: ".", "protocol-coverage.md")
        if (System.getProperty("agent.updateGolden") == "true" || !file.exists()) file.writeText(markdown)
        assertEquals("golden out of date: run :agent:test -Pagent.updateGolden=true", file.readText(), markdown)
    }
}
