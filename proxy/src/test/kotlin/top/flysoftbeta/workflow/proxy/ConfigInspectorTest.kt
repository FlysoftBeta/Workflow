package top.flysoftbeta.workflow.proxy

import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.proxy.config.IssueCode
import top.flysoftbeta.workflow.proxy.config.IssueSeverity
import top.flysoftbeta.workflow.proxy.config.MihomoConfigInspector
import top.flysoftbeta.workflow.proxy.config.MihomoConfigTemplate
import top.flysoftbeta.workflow.proxy.config.ProxySafeDefaults

class ConfigInspectorTest {
    private val secret = "0123456789abcdef"
    private fun codes(text: String) = MihomoConfigInspector.inspect(text).issues.map { it.code }.toSet()

    @Test fun templateHasSafeTunValuesAndNoWarnings() {
        val template = MihomoConfigTemplate.render(MihomoConfigTemplate.newSecret())
        val inspection = MihomoConfigInspector.inspect(template)
        assertFalse(inspection.tun.enable)
        assertTrue(inspection.issues.none { it.severity != IssueSeverity.INFO })
        assertEquals(ProxySafeDefaults.TABLE_INDEX, inspection.tun.tableIndex)
        assertEquals(ProxySafeDefaults.RULE_INDEX, inspection.tun.ruleIndex)
        assertEquals(ProxySafeDefaults.AUTO_REDIRECT_INPUT_MARK, inspection.tun.autoRedirectInputMark)
        assertEquals(ProxySafeDefaults.AUTO_REDIRECT_OUTPUT_MARK, inspection.tun.autoRedirectOutputMark)
        assertEquals(ProxySafeDefaults.TUN_DEVICE, inspection.tun.device)
        assertEquals("rule", inspection.mode)
        assertEquals(mapOf("mixed-port" to ProxySafeDefaults.MIXED_PORT), inspection.ports)

        // Turning TUN on in the template must not raise a single collision warning.
        val enabled = MihomoConfigInspector.inspect(template.replace("  enable: false\n  device:", "  enable: true\n  device:"))
        assertTrue(enabled.tun.enable)
        assertEquals(emptyList<Any>(), enabled.issues.filter { it.severity != IssueSeverity.INFO })
        assertEquals(9500..9510, enabled.tun.effectiveRuleRange)
    }

    @Test fun defaultTunValuesCollideWithCmfa() {
        val inspection = MihomoConfigInspector.inspect("external-controller: 127.0.0.1:9090\nsecret: $secret\ntun:\n  enable: true\n  auto-redirect: true\n")
        val codes = inspection.issues.map { it.code }.toSet()
        assertTrue(codes.containsAll(setOf(IssueCode.TUN_DEFAULT_TABLE, IssueCode.TUN_DEFAULT_RULE_INDEX, IssueCode.TUN_MARK_COLLISION, IssueCode.TUN_AUTO_DEVICE)))
        assertEquals(2022, inspection.tun.effectiveTableIndex)
        assertEquals(9000..9010, inspection.tun.effectiveRuleRange)
        assertTrue(inspection.blocking.isEmpty())
        // TUN checks only apply when TUN is enabled.
        assertTrue(codes("external-controller: 127.0.0.1:9090\nsecret: $secret\ntun:\n  enable: false\n").none { it.name.startsWith("TUN_") })
    }

    @Test fun reservedTablesAndInterleavedRuleIndexesBlockStart() {
        for (table in listOf(97, 99, 1015, 254)) {
            val inspection = MihomoConfigInspector.inspect("tun:\n  enable: true\n  iproute2-table-index: $table\n  iproute2-rule-index: 9500\n")
            assertTrue("table $table", inspection.blocking.any { it.code == IssueCode.TUN_TABLE_RESERVED })
        }
        for (rule in listOf(9995, 10000, 32000)) {
            val inspection = MihomoConfigInspector.inspect("tun:\n  enable: true\n  iproute2-table-index: 9500\n  iproute2-rule-index: $rule\n")
            assertTrue("rule $rule", inspection.blocking.any { it.code == IssueCode.TUN_RULE_INDEX_RANGE })
        }
    }

    @Test fun unsafeTunDeviceNamesAreRejectedBeforeRoot() {
        for (device in listOf("..", "long-interface-name", "with space", "bad/name")) {
            val inspection = MihomoConfigInspector.inspect("tun:\n  enable: true\n  device: '$device'\n")
            assertTrue(inspection.blocking.any { it.code == IssueCode.TUN_DEVICE_INVALID })
            assertTrue(inspection.issues.none { it.message.contains(device) && device != ".." })
        }
    }

    @Test fun marksInNetdBitsAreFlaggedAndHexYamlIsRead() {
        val inspection = MihomoConfigInspector.inspect("tun:\n  enable: true\n  auto-redirect: true\n  auto-redirect-input-mark: 0x10063\n  auto-redirect-output-mark: 0x40000000\n")
        assertEquals(0x10063L, inspection.tun.autoRedirectInputMark)
        assertTrue(inspection.issues.any { it.code == IssueCode.TUN_MARK_COLLISION })
        val clean = MihomoConfigInspector.inspect("tun:\n  enable: true\n  auto-redirect: true\n  auto-redirect-input-mark: 0x40000000\n  auto-redirect-output-mark: 0x80000000\n")
        assertTrue(clean.issues.none { it.code == IssueCode.TUN_MARK_COLLISION })
    }

    @Test fun controllerWithoutSecretBlocksBecauseTheKernelRunsAsRoot() {
        val inspection = MihomoConfigInspector.inspect("external-controller: 127.0.0.1:9090\n")
        assertEquals(listOf(IssueCode.CONTROLLER_WITHOUT_SECRET), inspection.blocking.map { it.code })
        assertTrue(codes("mode: rule").contains(IssueCode.CONTROLLER_MISSING))
        val remote = MihomoConfigInspector.inspect("external-controller: 192.168.1.2:9090\nsecret: $secret\n")
        assertNull(remote.controller)
        assertTrue(remote.issues.any { it.code == IssueCode.CONTROLLER_REJECTED })
    }

    @Test fun exposedControllerAndLanAreWarnings() {
        for (address in listOf("0.0.0.0:9090", ":9090", "'[::]:9090'")) {
            val inspection = MihomoConfigInspector.inspect("external-controller: $address\nsecret: $secret\nallow-lan: true\n")
            assertNotNull(inspection.controller)
            val codes = inspection.issues.map { it.code }
            assertTrue(address, codes.contains(IssueCode.CONTROLLER_EXPOSED) && codes.contains(IssueCode.ALLOW_LAN))
        }
    }

    @Test fun issuesAndToStringNeverContainTheSecret() {
        val inspection = MihomoConfigInspector.inspect("external-controller: 127.0.0.1:9090\nsecret: \"$secret\"\nmode: script\nmixed-port: 7890\ntun:\n  enable: true\n")
        assertFalse(inspection.toString().contains(secret))
        assertTrue(inspection.issues.none { it.message.contains(secret) })
        assertTrue(inspection.issues.any { it.code == IssueCode.UNKNOWN_MODE })
        assertTrue(inspection.issues.any { it.code == IssueCode.DEFAULT_PORT })
    }

    @Test fun providersAndPackagesAreListed() {
        val inspection = MihomoConfigInspector.inspect("""
            proxy-providers:
              sub: {type: http, url: "https://example.invalid/sub"}
            rule-providers:
              ads: {type: http, behavior: domain, url: "https://example.invalid/ads"}
            tun:
              enable: true
              include-package: [top.flysoftbeta.workflow]
        """.trimIndent())
        assertEquals(listOf("sub"), inspection.proxyProviders)
        assertEquals(listOf("ads"), inspection.ruleProviders)
        assertEquals(listOf("top.flysoftbeta.workflow"), inspection.tun.includePackages)
    }

    @Test fun emptyAndInvalidDocuments() {
        assertEquals(IssueCode.CONTROLLER_MISSING, MihomoConfigInspector.inspect("").issues.single().code)
        val error = assertThrows(IllegalArgumentException::class.java) { MihomoConfigInspector.inspect("secret: $secret\nsecret: again") }
        assertFalse(error.message.orEmpty().contains(secret))
        assertThrows(IllegalArgumentException::class.java) { MihomoConfigInspector.inspect("- a\n- b") }
    }
}
