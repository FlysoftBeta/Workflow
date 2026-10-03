package top.flysoftbeta.workflow.proxy

import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.proxy.config.MihomoConfigInspector
import top.flysoftbeta.workflow.proxy.config.MihomoConfigTemplate
import top.flysoftbeta.workflow.proxy.network.NetInterface
import top.flysoftbeta.workflow.proxy.network.TunConflictDetector

class TunConflictsTest {
    /** `ip rule show` on the user's tablet while ClashMetaForAndroid's TUN runs (artifacts/physical-review). */
    private val tabletRules = """
        0:	from all lookup local
        9000:	from all iif Meta goto 9010
        9001:	not from all iif lo lookup 2022
        9001:	from 0.0.0.0 iif lo lookup 2022
        9001:	from 198.18.0.0/30 iif lo lookup 2022
        9010:	from all nop
        10000:	from all fwmark 0xc0000/0xd0000 lookup 99
        10500:	from all iif lo oif dummy0 uidrange 0-0 lookup 1002
        10500:	from all iif lo oif wlan0 uidrange 0-0 lookup 1015
        13000:	from all fwmark 0x10063/0x1ffff iif lo lookup 97
        13000:	from all fwmark 0x1006c/0x1ffff iif lo lookup 1015
        14000:	from all iif lo oif dummy0 lookup 1002
        14000:	from all iif lo oif wlan0 lookup 1015
        15000:	from all fwmark 0x0/0x10000 lookup 99
        16000:	from all fwmark 0x0/0x10000 lookup 98
        17000:	from all fwmark 0x0/0x10000 lookup 97
        19000:	from all fwmark 0x6c/0x1ffff iif lo lookup 1015
        22000:	from all fwmark 0x0/0xffff iif lo lookup 1015
        32000:	from all unreachable
    """.trimIndent()
    private val safeTun = MihomoConfigInspector.inspect(MihomoConfigTemplate.render("ab").replace("  enable: false\n  device:", "  enable: true\n  device:")).tun
    private val defaultTun = MihomoConfigInspector.inspect("tun:\n  enable: true\n").tun

    @Test fun parsesRealRuleDump() {
        val rules = TunConflictDetector.parseRules(tabletRules)
        assertEquals(19, rules.size)
        assertEquals(9010, rules[1].goto)
        assertEquals("2022", rules[2].table)
        assertNull(rules[5].table)
        assertEquals("1015", rules.last { it.priority == 22000 }.table)
    }

    @Test fun safeDefaultsDoNotCollideWithTheTabletsCmfa() {
        assertEquals(emptyList<String>(), TunConflictDetector.ruleCollisions(safeTun, TunConflictDetector.parseRules(tabletRules)))
    }

    @Test fun singTunDefaultsCollideWithTheTabletsCmfa() {
        val rules = TunConflictDetector.parseRules(tabletRules)
        // Familiar priorities do not establish ownership; existing rules always block launch.
        assertEquals(5, TunConflictDetector.ruleCollisions(defaultTun, rules, foreignInterfaces = listOf("Meta")).size)
        // An explicit device does not authorize removing the rest of another owner's rules.
        val named = defaultTun.copy(device = "workflow-tun")
        assertEquals(5, TunConflictDetector.ruleCollisions(named, rules).size)
    }

    @Test fun staleLookingRulesAreNeverAssumedToBeOurs() {
        val stale = "9500:\tfrom all iif workflow-tun goto 9510\n9501:\tnot from all iif lo lookup 9500\n9510:\tfrom all nop\n"
        assertEquals(3, TunConflictDetector.ruleCollisions(safeTun, TunConflictDetector.parseRules(tabletRules + "\n" + stale)).size)
        val foreign = "9505:\tfrom all lookup 3000\n20000:\tfrom all lookup 9500\n"
        assertEquals(listOf("9505: from all lookup 3000", "20000: from all lookup 9500"),
            TunConflictDetector.ruleCollisions(safeTun, TunConflictDetector.parseRules(foreign)))
    }

    @Test fun detectsForeignTunAndVpnButNotOrdinaryInterfaces() {
        val interfaces = listOf(NetInterface("lo", true), NetInterface("wlan0", true), NetInterface("dummy0", true),
            NetInterface("Meta", true), NetInterface("tun0", false), NetInterface("sipa_eth0", false))
        val conflict = TunConflictDetector.detect(interfaces, vpnActive = false, ownDevice = null, configuredDevice = "workflow-tun")
        assertEquals(listOf("Meta"), conflict.foreignInterfaces)
        assertFalse(conflict.hard)
        assertTrue(conflict.any)
        assertTrue(conflict.describe().contains("Meta"))
        assertFalse(TunConflictDetector.detect(listOf(NetInterface("wlan0", true)), false, null, "workflow-tun").any)
        assertTrue(TunConflictDetector.detect(emptyList(), vpnActive = true, ownDevice = null, configuredDevice = null).any)
    }

    @Test fun sysfsTunFlagOverridesTheNameHeuristic() {
        val interfaces = listOf(NetInterface("oddname", true, tun = true), NetInterface("tun0", true, tun = false))
        assertEquals(listOf("oddname"), TunConflictDetector.detect(interfaces, false, null, null).foreignInterfaces)
    }

    @Test fun carrierlessPointToPointTunStillBelongsToItsOwner() {
        val interfaces = listOf(NetInterface("Meta", up = false, pointToPoint = true),
            NetInterface("wlan0", up = false), NetInterface("ordinary-link", up = false, pointToPoint = true))
        assertEquals(listOf("Meta"), TunConflictDetector.detect(interfaces, false, null, "workflow-tun").foreignInterfaces)
        assertFalse(TunConflictDetector.detect(interfaces, false, ownDevice = "Meta", configuredDevice = "Meta").any)
    }

    @Test fun parsesKernelLinkFlagsWithoutMistakingCarrierForOwnership() {
        val links = TunConflictDetector.parseLinks("1: lo: <LOOPBACK,UP,LOWER_UP> mtu 65536 state UNKNOWN\n" +
            "12: Meta: <NO-CARRIER,POINTOPOINT,MULTICAST,NOARP,UP> mtu 1500 state DOWN\\    link/none\n" +
            "13: veth0@if14: <BROADCAST,UP,LOWER_UP> mtu 1500 state UP\n")
        assertEquals(listOf("lo", "Meta", "veth0"), links.map { it.name })
        assertEquals(listOf("Meta"), TunConflictDetector.detect(links, false, null, "workflow-tun").foreignInterfaces)
        assertThrows(IllegalArgumentException::class.java) { TunConflictDetector.parseLinks("unexpected output") }
        assertThrows(IllegalArgumentException::class.java) { TunConflictDetector.parseLinks("") }
    }

    @Test fun ownRunningDeviceIsExcludedAndATakenNameIsHard() {
        val interfaces = listOf(NetInterface("workflow-tun", true, tun = true))
        assertFalse(TunConflictDetector.detect(interfaces, false, ownDevice = "workflow-tun", configuredDevice = "workflow-tun").any)
        val stopped = TunConflictDetector.detect(interfaces, false, ownDevice = null, configuredDevice = "workflow-tun")
        assertTrue(stopped.deviceTaken && stopped.hard)
        assertEquals(emptyList<String>(), stopped.foreignInterfaces)
    }
}
