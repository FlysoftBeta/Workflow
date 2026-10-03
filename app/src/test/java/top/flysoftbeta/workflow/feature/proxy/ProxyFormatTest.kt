package top.flysoftbeta.workflow.feature.proxy

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import top.flysoftbeta.workflow.proxy.controller.DelayResult
import top.flysoftbeta.workflow.proxy.controller.ProxyGroup
import top.flysoftbeta.workflow.proxy.controller.ProxyMode
import top.flysoftbeta.workflow.proxy.controller.ProxyNode
import top.flysoftbeta.workflow.proxy.controller.ProxySnapshot

class ProxyFormatTest {
    @Test fun latencyTiersFollowUiSpec() {
        assertEquals(ProxyFormat.Latency("86", ProxyFormat.Tier.GOOD), ProxyFormat.latency(DelayResult.Success(86), null))
        assertEquals(ProxyFormat.Tier.FAIR, ProxyFormat.latency(DelayResult.Success(300), null)!!.tier)
        assertEquals(ProxyFormat.Tier.BAD, ProxyFormat.latency(DelayResult.Success(800), null)!!.tier)
        assertEquals(ProxyFormat.Latency("超时", ProxyFormat.Tier.BAD), ProxyFormat.latency(DelayResult.Timeout, 90))
        assertEquals("explicit test wins over history", "120", ProxyFormat.latency(DelayResult.Success(120), 90)!!.text)
        assertEquals("history when never tested", "90", ProxyFormat.latency(null, 90)!!.text)
        assertNull(ProxyFormat.latency(null, null))
    }

    @Test fun ratesAndSizes() {
        assertEquals("512 B/s", ProxyFormat.rate(512))
        assertEquals("12 KB/s", ProxyFormat.rate(12 * 1024))
        assertEquals("1.2 MB/s", ProxyFormat.rate((1.2 * 1024 * 1024).toLong()))
        assertEquals("3.0 GB", ProxyFormat.size(3L * 1024 * 1024 * 1024))
    }

    @Test fun visibleGroupsPerMode() {
        val node = ProxyNode("HK", "Vmess", true, null, true)
        val proxy = ProxyGroup("Proxy", "Selector", "HK", listOf(node), false, null, null)
        val hidden = ProxyGroup("Hidden", "Selector", null, listOf(node), true, null, null)
        val global = ProxyGroup("GLOBAL", "Selector", "DIRECT", listOf(node), false, null, null)
        val snapshot = ProxySnapshot(listOf(proxy, hidden), global)
        assertEquals(listOf("Proxy"), ProxyFormat.visibleGroups(snapshot, ProxyMode.RULE).map { it.name })
        assertEquals(listOf("GLOBAL"), ProxyFormat.visibleGroups(snapshot, ProxyMode.GLOBAL).map { it.name })
        assertEquals("direct keeps the rule groups (dimmed)", listOf("Proxy"), ProxyFormat.visibleGroups(snapshot, ProxyMode.DIRECT).map { it.name })
        assertTrue(ProxyFormat.automatic(proxy.copy(type = "URLTest")))
        assertFalse(ProxyFormat.automatic(proxy))
        assertFalse("nested groups are not tested as nodes", ProxyFormat.testable(node.copy(now = "HK")))
        assertFalse(ProxyFormat.testable(node.copy(type = "Reject")))
    }

    @Test fun gridColumns() {
        assertEquals(1, ProxyFormat.columns(100f, 168f, 8f))
        assertEquals(3, ProxyFormat.columns(3 * 168f + 2 * 8f, 168f, 8f))
        assertEquals(2, ProxyFormat.columns(3 * 168f + 2 * 8f - 1, 168f, 8f))
    }

    @Test fun logrusLinesAreParsed() {
        val line = ProxyFormat.parseLog("time=\"2026-09-29T10:44:19.332145+08:00\" level=warning msg=\"[TCP] dial \\\"a\\\" failed\"")
        assertEquals(ProxyFormat.LogLine("10:44:19", "warning", "[TCP] dial \"a\" failed"), line)
        assertEquals(ProxyFormat.LogLine(null, null, "plain output"), ProxyFormat.parseLog("plain output"))
    }
}
