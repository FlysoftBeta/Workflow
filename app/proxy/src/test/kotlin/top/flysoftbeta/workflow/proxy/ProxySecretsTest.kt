package top.flysoftbeta.workflow.proxy

import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.proxy.config.MihomoConfigInspector
import top.flysoftbeta.workflow.proxy.redact.ProxySecrets

class ProxySecretsTest {
    @Test fun controllerNodeAndSubscriptionCredentialsAreRedactedAcrossEveryChunkBoundary() {
        val yaml = """
            external-controller: 127.0.0.1:19090
            secret: controller-credential
            proxies:
              - name: test-node
                type: http
                server: localhost
                port: 1234
                password: node-credential
            proxy-providers:
              test:
                type: http
                url: https://example.invalid/subscription?token=provider-credential
        """.trimIndent()
        val secrets = MihomoConfigInspector.inspect(yaml).secrets
        val line = "controller-credential node-credential https://example.invalid/subscription?token=provider-credential provider-credential"
        for (split in 0..line.length) {
            val stream = secrets.stream()
            val result = stream.append(line.take(split)) + stream.append(line.drop(split), finished = true)
            assertFalse("split=$split: $result", result.contains("credential"))
            assertFalse(result.contains("example.invalid"))
        }
        assertFalse(secrets.toString().contains("credential"))
    }

    @Test fun encodedSecretsAndRecursiveYamlCollectionsAreBounded() {
        val root = linkedMapOf<String, Any>()
        root["password"] = "space and/slash"
        root["cycle"] = root
        val secrets = ProxySecrets.fromConfig(root)
        assertEquals("[已隐藏] [已隐藏]", secrets.redact("space and/slash space+and%2Fslash"))
    }
}
