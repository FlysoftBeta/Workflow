package top.flysoftbeta.workflow.proxy

import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.proxy.config.MihomoConfigAssets

class MihomoConfigAssetsTest {
    @Test fun onlyFileProvidersRequireCanonicalAssetsAndDotPrefixIsNormalized() {
        assertEquals(listOf("nodes.yaml", "providers/rules.mrs"), MihomoConfigAssets.localProviders("""
            proxy-providers:
              nodes: {type: file, path: ./nodes.yaml}
              remote: {type: http, path: cached.yaml, url: https://example.invalid}
            rule-providers:
              rules: {type: file, path: providers/rules.mrs}
        """.trimIndent()))
    }
    @Test fun assetsCannotEscapeOrOverwriteExecutorFiles() {
        for (path in listOf("../a", "/absolute", "config.yaml", ".process.json", "runtime.log", "bad//path", "providers/../x")) {
            assertTrue(path, runCatching { MihomoConfigAssets.localProviders("proxy-providers: {fixture: {type: file, path: '$path'}}") }.isFailure)
        }
    }
}
