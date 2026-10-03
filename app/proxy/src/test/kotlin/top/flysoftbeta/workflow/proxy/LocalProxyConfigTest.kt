package top.flysoftbeta.workflow.proxy

import top.flysoftbeta.workflow.proxy.config.LocalProxyConfig
import top.flysoftbeta.workflow.proxy.guardian.ProxyProcessIdentity
import top.flysoftbeta.workflow.proxy.guardian.verifyGuardianExit
import top.flysoftbeta.workflow.proxy.io.readProxyBytes
import top.flysoftbeta.workflow.proxy.redact.ProxyLogRedactor
import java.io.ByteArrayInputStream
import java.io.InputStream
import org.junit.Assert.*
import org.junit.Test

class LocalProxyConfigTest {
    private val id = "12345678-1234-1234-1234-123456789abc"
    private fun identity() = ProxyProcessIdentity(123, 456, 122, 450, id, "/data/app/lib/libmihomo.so", "/workspace/proxy", "/workspace/proxy/config.yaml")
    private fun stat(start: String = "456", name: String = "mihomo") = "123 ($name) S " + (1..18).joinToString(" ") + " $start 0"

    @Test fun controllerRestrictsSecretToNormalizedLoopback() {
        for ((address, endpoint) in mapOf(
            ":9090" to "http://127.0.0.1:9090", "localhost:9090" to "http://127.0.0.1:9090",
            "0.0.0.0:9090" to "http://127.0.0.1:9090", "127.0.0.1:9090" to "http://127.0.0.1:9090",
            "[::]:9090" to "http://[::1]:9090", "[::1]:9090" to "http://[::1]:9090",
        )) {
            val parsed = LocalProxyConfig.controller("external-controller: '$address'\nsecret: 'private-key'")!!
            assertEquals(endpoint, parsed.endpoint)
            assertEquals("private-key", parsed.secret)
        }
    }

    @Test fun remoteAndAmbiguousControllerAddressesAreRejected() {
        for (address in listOf("example.com:9090", "127.0.0.1.example.com:9090", "user@127.0.0.1:9090", "127.0.0.1:9090/path", "127.0.0.1:9090?x", "127.0.0.1:0", "127.0.0.1:65536", "127.0.0.1:9090#x")) {
            assertThrows(IllegalArgumentException::class.java) { LocalProxyConfig.controller("external-controller: '$address'") }
        }
    }

    @Test fun missingControllerIsNotFabricated() {
        assertNull(LocalProxyConfig.controller("mode: rule"))
        assertNull(LocalProxyConfig.controller("external-controller: ''"))
    }

    @Test fun duplicateKeysAndArbitraryYamlClassesAreRejectedWithoutEchoingConfig() {
        for (text in listOf("secret: private-key\nsecret: second\nexternal-controller: ':9090'", "!!javax.script.ScriptEngineManager [private-key]", "secret: [private-key")) {
            val error = assertThrows(IllegalArgumentException::class.java) { LocalProxyConfig.controller(text) }
            assertFalse(error.message.orEmpty().contains("private-key"))
        }
    }

    @Test fun headerInjectionAndNonStringSecretsAreRejected() {
        for (secret in listOf("123", "true", "\"key\\nInjected: yes\"", "\"中文\"")) {
            assertThrows(IllegalArgumentException::class.java) { LocalProxyConfig.controller("external-controller: ':9090'\nsecret: $secret") }
        }
    }

    @Test fun statParserHandlesRealCommGrammarAndRejectsMalformedNumbers() {
        assertEquals(456L, ProxyProcessIdentity.startTimeFromStat(stat(name = "odd ) (x\nname")))
        for (value in listOf("", "-2", "+2", "0", "18446744073709551616", "4x")) {
            assertNull(ProxyProcessIdentity.startTimeFromStat(stat(start = value)))
        }
        assertNull(ProxyProcessIdentity.startTimeFromStat("123 (short) S 1 2"))
        assertNull(ProxyProcessIdentity.startTimeFromStat("bad stat"))
    }

    @Test fun recordedIdentityIsOnlyAnExactReadOnlyMatch() {
        val identity = identity()
        assertTrue(identity.matchesObservedProcess(stat(), identity.executable))
        assertTrue(identity.matchesObservedProcess(stat(), identity.executable + " (deleted)"))
        assertFalse(identity.matchesObservedProcess(stat("457"), identity.executable))
        assertFalse(identity.matchesObservedProcess(stat(), "/data/app/different/libmihomo.so"))
        assertEquals("stop $id 123 456\n", identity.stopCommand())
    }

    @Test fun identityRequiresCanonicalUuidAndDistinctPositivePids() {
        val identity = identity()
        for (invalid in listOf("abcdefghijklmnop", id + "\n", "12345678-1234-1234-1234-123456789abz")) {
            assertThrows(IllegalArgumentException::class.java) { identity.copy(runId = invalid) }
        }
        assertThrows(IllegalArgumentException::class.java) { identity.copy(pid = 1) }
        assertThrows(IllegalArgumentException::class.java) { identity.copy(guardPid = identity.pid) }
        assertThrows(IllegalArgumentException::class.java) { identity.copy(startTime = -1) }
        assertThrows(IllegalArgumentException::class.java) { identity.copy(executable = "relative/kernel") }
    }

    private fun started(): Map<String, Any?> = mapOf("uid" to 0, "pid" to 123, "startTime" to 456L, "guardPid" to 122,
        "guardStartTime" to 450L, "runId" to id)

    private fun parse(fields: Map<String, Any?>) = ProxyProcessIdentity.fromGuardian(fields, id,
        identity().executable, identity().directory, identity().config)

    @Test fun startedEventRequiresExactRootRunAndIntegralIdentity() {
        assertEquals(identity(), parse(started()))
        for ((key, value) in listOf("uid" to 1000, "uid" to "0", "pid" to 123.0, "pid" to 4294967419L,
            "startTime" to null, "guardStartTime" to -1L, "runId" to "12345678-1234-1234-1234-123456789abd")) {
            assertThrows(IllegalArgumentException::class.java) { parse(started() + (key to value)) }
        }
    }

    @Test fun exitEventCannotConfirmAnotherPidOrMalformedStatus() {
        val good = mapOf("pid" to 123, "exitCode" to 143, "forced" to false)
        assertEquals(143 to false, verifyGuardianExit(good, identity()))
        for ((key, value) in listOf("pid" to 124, "pid" to "123", "exitCode" to 256, "forced" to "false")) {
            assertThrows(IllegalArgumentException::class.java) { verifyGuardianExit(good + (key to value), identity()) }
        }
    }

    @Test fun portableReaderAcceptsExactlyBoundedInputAndRejectsAnExtraByte() {
        assertArrayEquals(byteArrayOf(), readProxyBytes(ByteArrayInputStream(byteArrayOf()), 0))
        assertArrayEquals("abc".toByteArray(), readProxyBytes(ByteArrayInputStream("abc".toByteArray()), 3))
        assertThrows(java.io.IOException::class.java) { readProxyBytes(ByteArrayInputStream("abcd".toByteArray()), 3) }
        assertThrows(java.io.IOException::class.java) { readProxyBytes(ByteArrayInputStream(byteArrayOf(1)), 0) }
    }

    @Test fun portableReaderHandlesShortAndZeroLengthReads() {
        val source = object : InputStream() {
            val bytes = ByteArrayInputStream("abcdef".toByteArray())
            var zero = true
            override fun read(): Int = bytes.read()
            override fun read(buffer: ByteArray, offset: Int, length: Int): Int {
                if (zero) { zero = false; return 0 }
                return bytes.read(buffer, offset, minOf(2, length))
            }
        }
        assertEquals("abcdef", readProxyBytes(source, 6).toString(Charsets.UTF_8))
    }

    @Test fun secretRedactionSurvivesEveryPossibleChunkBoundary() {
        val secret = "private-key"
        val source = "prefix $secret and $secret suffix"
        for (size in 1..source.length) {
            val redactor = ProxyLogRedactor(secret)
            val result = source.chunked(size).joinToString("") { redactor.append(it) } + redactor.append("", finished = true)
            assertEquals("prefix [已隐藏] and [已隐藏] suffix", result)
        }
        assertEquals("unchanged", ProxyLogRedactor("").append("unchanged"))
    }
}
