package top.flysoftbeta.workflow.proxy.guardian

data class ProxyProcessIdentity(
    val pid: Int,
    val startTime: Long,
    val guardPid: Int,
    val guardStartTime: Long,
    val runId: String,
    val executable: String,
    val directory: String,
    val config: String,
) {
    init {
        require(pid > 1 && guardPid > 1 && pid != guardPid && startTime > 0 && guardStartTime > 0)
        require(runId.matches(Regex("[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}")))
        require(executable.startsWith('/') && directory.startsWith('/') && config.startsWith('/'))
    }
    fun stopCommand(): String = "stop $runId $pid $startTime\n"

    /** Diagnostics only: matching a recorded process never authorizes sending it a signal. */
    fun matchesObservedProcess(stat: String, executable: String): Boolean =
        startTimeFromStat(stat) == startTime && executable.removeSuffix(" (deleted)") == this.executable

    companion object {
        fun fromGuardian(fields: Map<String, Any?>, expectedRunId: String, executable: String, directory: String, config: String): ProxyProcessIdentity {
            require(guardianInteger(fields["uid"]) == 0L && fields["runId"] == expectedRunId) { "守护进程未确认本次 Root 身份" }
            val pid = guardianInteger(fields["pid"])
            val guardPid = guardianInteger(fields["guardPid"])
            require(pid in 2..Int.MAX_VALUE.toLong() && guardPid in 2..Int.MAX_VALUE.toLong()) { "守护进程 PID 无效" }
            return ProxyProcessIdentity(pid.toInt(), guardianInteger(fields["startTime"]), guardPid.toInt(),
                guardianInteger(fields["guardStartTime"]), expectedRunId, executable, directory, config)
        }

        fun startTimeFromStat(stat: String): Long? {
            val boundary = stat.lastIndexOf(')')
            if (boundary < 0 || stat.getOrNull(boundary + 1) != ' ') return null
            val fields = stat.substring(boundary + 2).trim().split(Regex("\\s+"))
            val numeric = fields.getOrNull(19) ?: return null
            if (numeric.isEmpty() || numeric.any { it !in '0'..'9' }) return null
            return numeric.toLongOrNull()?.takeIf { it > 0 }
        }
    }
}

fun guardianInteger(value: Any?): Long = when (value) {
    is Int -> value.toLong()
    is Long -> value
    else -> throw IllegalArgumentException("守护进程整数记录无效")
}

fun verifyGuardianExit(fields: Map<String, Any?>, identity: ProxyProcessIdentity): Pair<Int, Boolean> {
    require(guardianInteger(fields["pid"]) == identity.pid.toLong()) { "守护进程退出 PID 不匹配" }
    val code = guardianInteger(fields["exitCode"])
    require(code in 0..255 && fields["forced"] is Boolean) { "守护进程退出记录无效" }
    return code.toInt() to (fields["forced"] as Boolean)
}
