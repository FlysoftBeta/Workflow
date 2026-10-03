package top.flysoftbeta.workflow.proxy.guardian

import top.flysoftbeta.workflow.proxy.config.TunSettings

/** Shell text handed to `su -c`. Every argument is single-quoted; never build it from unquoted paths. */
object GuardianCommand {
    fun supervise(guardian: String, kernel: String, directory: String, config: String, runId: String, tun: TunSettings? = null): String =
        "exec ${quote(guardian)} supervise ${quote(kernel)} ${quote(directory)} ${quote(config)} ${quote(runId)}" +
            if (tun?.enable == true && tun.autoRoute && tun.explicitDevice != null) {
                " --tun-cleanup ${quote(tun.effectiveTableIndex.toString())} ${quote(tun.effectiveRuleIndex.toString())} ${quote(tun.explicitDevice!!)}"
            } else ""

    /** POSIX single quoting: Android `su` may run `sh`, not bash. */
    fun quote(value: String): String = "'" + value.replace("'", "'\"'\"'") + "'"
}
