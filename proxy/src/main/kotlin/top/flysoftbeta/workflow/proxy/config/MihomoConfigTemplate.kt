package top.flysoftbeta.workflow.proxy.config

import java.security.SecureRandom

/**
 * Starter configuration, written only when no config.yaml exists. Creating it never starts the kernel.
 * TUN stays disabled until the user edits the file; its routing values are pre-set to [ProxySafeDefaults]
 * so that turning it on cannot collide with a Mihomo/CMFA left at sing-tun defaults.
 */
object MihomoConfigTemplate {
    private val random = SecureRandom()

    /** 32 random bytes as hex: printable ASCII, no YAML quoting issues. */
    fun newSecret(): String = ByteArray(32).also(random::nextBytes).joinToString("") { "%02x".format(it.toInt() and 255) }

    fun render(secret: String): String {
        require(secret.isNotEmpty() && secret.all { it in '0'..'9' || it in 'a'..'f' }) { "模板密钥格式无效" }
        return """
            # Workflow 代理配置（Mihomo）。应用不会改写此文件；直接编辑即可，重新启动后生效。
            # 创建此文件不会启动内核。启动需要 Root。
            mixed-port: ${ProxySafeDefaults.MIXED_PORT}
            allow-lan: false
            bind-address: "127.0.0.1"
            mode: rule
            log-level: info
            # 应用通过此本机接口切换模式与节点；secret 保护以 Root 运行的内核，请勿删除。
            external-controller: 127.0.0.1:${ProxySafeDefaults.CONTROLLER_PORT}
            secret: "$secret"
            tun:
              # 改为 true 后以 TUN 接管本机流量。
              enable: false
              device: ${ProxySafeDefaults.TUN_DEVICE}
              stack: mixed
              auto-route: true
              auto-detect-interface: true
              dns-hijack:
                - any:53
              # 与其他 Mihomo/CMFA 的默认值（表 2022、规则 9000、标记 0x2023/0x2024）错开，请勿改回默认值。
              iproute2-table-index: ${ProxySafeDefaults.TABLE_INDEX}
              iproute2-rule-index: ${ProxySafeDefaults.RULE_INDEX}
              auto-redirect: false
              auto-redirect-input-mark: 0x%x
              auto-redirect-output-mark: 0x%x
              # 仅代理部分应用：
              # include-package:
              #   - top.flysoftbeta.workflow
            dns:
              enable: true
              ipv6: true
              enhanced-mode: redir-host
              nameserver:
                - 223.5.5.5
                - 119.29.29.29
            proxies: []
            proxy-groups: []
            rules:
              - MATCH,DIRECT
        """.trimIndent().format(ProxySafeDefaults.AUTO_REDIRECT_INPUT_MARK, ProxySafeDefaults.AUTO_REDIRECT_OUTPUT_MARK) + "\n"
    }
}
