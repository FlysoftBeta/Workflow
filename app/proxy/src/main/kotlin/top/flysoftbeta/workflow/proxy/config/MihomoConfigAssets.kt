package top.flysoftbeta.workflow.proxy.config

/** File-provider content belongs beside the canonical Engine configuration, not to the Android cache. */
object MihomoConfigAssets {
    fun localProviders(text: String): List<String> {
        val config = MihomoYaml.load(text)
        return listOf("proxy-providers", "rule-providers").flatMap { section ->
            (config[section] as? Map<*, *>)?.values.orEmpty().mapNotNull { value ->
                val provider = value as? Map<*, *> ?: return@mapNotNull null
                if ((provider["type"] as? String)?.equals("file", ignoreCase = true) != true) return@mapNotNull null
                val path = provider["path"] as? String ?: error("File provider 缺少 path")
                require(!path.startsWith('/') && '\\' !in path && '\u0000' !in path) { "File provider 须使用工作区代理目录内的相对路径" }
                val normalized = path.split('/').filter { it != "." }
                require(normalized.isNotEmpty() && normalized.all { it.matches(Regex("[A-Za-z0-9_.-]{1,160}")) && it != ".." }) {
                    "File provider 路径须使用安全的文件名，且不能离开工作区代理目录"
                }
                require(normalized.first() !in setOf("config.yaml", ".process.json", "runtime.log", "runtime.previous.log")) {
                    "File provider 不能覆盖配置或执行器记录"
                }
                normalized.joinToString("/")
            }
        }.distinct().also { require(it.size <= 256) { "File provider 数量超过 256" } }
    }
}
