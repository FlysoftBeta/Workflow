package top.flysoftbeta.workflow.proxy.config

import java.net.URI
import org.yaml.snakeyaml.LoaderOptions
import org.yaml.snakeyaml.Yaml
import org.yaml.snakeyaml.constructor.SafeConstructor

/** [secret] is never logged or put into an exception message; see [toString]. */
data class ProxyControllerSettings(val endpoint: String, val secret: String) {
    override fun toString(): String = "ProxyControllerSettings(endpoint=$endpoint, secret=${if (secret.isEmpty()) "<none>" else "<redacted>"})"
}

/** Safe YAML loading shared by every read of the user's Mihomo configuration. Never constructs classes. */
internal object MihomoYaml {
    fun load(text: String): Map<*, *> {
        require(text.toByteArray(Charsets.UTF_8).size <= LocalProxyConfig.MAX_CONFIG_BYTES) { "代理配置超过 16 MiB" }
        val options = LoaderOptions().apply {
            codePointLimit = LocalProxyConfig.MAX_CONFIG_BYTES
            maxAliasesForCollections = 128
            nestingDepthLimit = 48
            isAllowDuplicateKeys = false
        }
        // SnakeYAML exception messages quote the offending document text, possibly a secret.
        val parsed = try { Yaml(SafeConstructor(options)).load<Any?>(text) } catch (_: Exception) {
            throw IllegalArgumentException("无法安全解析代理 YAML 配置")
        }
        if (parsed == null) return emptyMap<String, Any?>()
        return parsed as? Map<*, *> ?: throw IllegalArgumentException("代理配置必须是 YAML 映射")
    }
}

/** Read user YAML without constructing arbitrary classes or rewriting any setting. */
object LocalProxyConfig {
    const val MAX_CONFIG_BYTES = 16 * 1024 * 1024

    fun controller(text: String): ProxyControllerSettings? = controller(MihomoYaml.load(text))

    internal fun controller(root: Map<*, *>): ProxyControllerSettings? {
        val configured = root["external-controller"] ?: return null
        require(configured is String) { "external-controller 必须是主机与端口" }
        if (configured.isBlank()) return null
        val (host, port) = controllerAddress(configured)
        val loopback = when (host) {
            "localhost", "127.0.0.1", "0.0.0.0" -> "127.0.0.1"
            "::1", "::" -> "[::1]"
            else -> throw IllegalArgumentException("仅连接本机控制接口；不会向其他主机发送配置密钥")
        }
        val secret = root["secret"]
        require(secret == null || secret is String) { "secret 必须是字符串" }
        require(secret == null || secret.all { it.code in 0x20..0x7e }) { "secret 必须是不含换行的 ASCII 字符串" }
        return ProxyControllerSettings("http://$loopback:$port", secret ?: "")
    }

    /** Host (lower case, no brackets) and port of an `external-controller` value. */
    internal fun controllerAddress(configured: String): Pair<String, Int> {
        require('/' !in configured && '@' !in configured && '?' !in configured && '#' !in configured) { "控制接口必须是本机地址与端口" }
        val address = if (configured.startsWith(":")) "127.0.0.1$configured" else configured
        val uri = try { URI("http://$address") } catch (_: Exception) {
            // URI's default exception embeds the untrusted address, possibly credentials.
            throw IllegalArgumentException("无法解析控制接口地址")
        }
        val host = uri.host?.removePrefix("[")?.removeSuffix("]")?.lowercase()
            ?: throw IllegalArgumentException("无法解析控制接口地址")
        require(uri.port in 1..65535) { "控制接口端口无效" }
        return host to uri.port
    }
}
