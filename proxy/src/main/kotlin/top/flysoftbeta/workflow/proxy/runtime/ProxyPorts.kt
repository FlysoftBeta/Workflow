package top.flysoftbeta.workflow.proxy.runtime

import java.io.File
import top.flysoftbeta.workflow.proxy.config.TunSettings
import java.io.IOException
import java.net.NetworkInterface
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.runInterruptible
import top.flysoftbeta.workflow.proxy.guardian.GuardianCommand
import top.flysoftbeta.workflow.proxy.network.NetInterface

data class ShellResult(val code: Int, val output: String)

/** Root command channel. Production: `su -c`. Every call may show the device's root-authorization prompt. */
interface RootShell {
    /** Runs [script] as root; [ShellResult.output] is stdout+stderr, bounded. */
    suspend fun run(script: String, timeoutMs: Long): ShellResult
}

/**
 * Starts `guardian supervise kernel directory config runId` as root with three separate pipes:
 * stdin (control, kept open by the app), stdout (guardian JSONL only), stderr (kernel output).
 */
interface GuardianLauncher {
    fun launch(guardian: File, kernel: File, directory: File, config: File, runId: String, tun: TunSettings?): Process
}

/** Unprivileged kernel invocations (`-v`, `-t`). */
interface KernelTool {
    suspend fun version(kernel: File): ShellResult
    suspend fun validate(kernel: File, directory: File, config: File): ShellResult
}

/** Root-free view of other VPNs/TUNs. */
interface NetworkProbe {
    fun interfaces(): List<NetInterface>
    /** Android reports a network with TRANSPORT_VPN. */
    fun vpnActive(): Boolean
}

/** Keeps the process alive (foreground service) while a kernel runs. */
interface ForegroundLease {
    suspend fun retain(id: String): Result<Unit>
    fun release(id: String)
}

class ProxyPorts(
    val rootShell: RootShell = SuRootShell(),
    val guardianLauncher: GuardianLauncher = SuGuardianLauncher(),
    val kernelTool: KernelTool = ProcessKernelTool(),
    val networkProbe: NetworkProbe = JavaNetworkProbe(),
    val lease: ForegroundLease,
)

/** Runs a process with merged output, bounded capture and a hard timeout. Pure JVM; used by the defaults below. */
internal suspend fun runBounded(arguments: List<String>, directory: File?, timeoutMs: Long, timeoutMessage: String): ShellResult =
    runInterruptible(Dispatchers.IO) {
        val process = ProcessBuilder(arguments).directory(directory?.takeIf { it.isDirectory }).redirectErrorStream(true).start()
        val captured = StringBuffer()
        val reader = Thread({
            try { process.inputStream.bufferedReader().use { input ->
                val buffer = CharArray(4096)
                while (true) {
                    val count = input.read(buffer); if (count < 0) break
                    if (captured.length < MAX_OUTPUT) captured.append(buffer, 0, count.coerceAtMost(MAX_OUTPUT - captured.length))
                }
            } } catch (_: IOException) { }
        }, "workflow-proxy-command").apply { isDaemon = true; start() }
        try {
            if (!process.waitFor(timeoutMs, TimeUnit.MILLISECONDS)) {
                process.destroy(); if (!process.waitFor(300, TimeUnit.MILLISECONDS)) process.destroyForcibly()
                throw IOException(timeoutMessage)
            }
            reader.join(500)
            ShellResult(process.exitValue(), captured.toString())
        } finally {
            runCatching { process.outputStream.close() }
            if (process.isAlive) process.destroyForcibly()
        }
    }

private const val MAX_OUTPUT = 65536

class SuRootShell(private val su: String = "su") : RootShell {
    override suspend fun run(script: String, timeoutMs: Long): ShellResult =
        runBounded(listOf(su, "-c", script), null, timeoutMs, "Root 命令超时；如有授权提示，请在权限管理器中确认后重试")
}

class SuGuardianLauncher(private val su: String = "su") : GuardianLauncher {
    override fun launch(guardian: File, kernel: File, directory: File, config: File, runId: String, tun: TunSettings?): Process =
        ProcessBuilder(su, "-c", GuardianCommand.supervise(guardian.path, kernel.path, directory.path, config.path, runId, tun))
            .directory(directory).start()
}

class ProcessKernelTool : KernelTool {
    override suspend fun version(kernel: File) = runBounded(listOf(kernel.path, "-v"), kernel.parentFile, 5000, "读取内核版本超时")
    override suspend fun validate(kernel: File, directory: File, config: File) =
        runBounded(listOf(kernel.path, "-t", "-d", directory.path, "-f", config.path), directory, 20_000, "配置校验超时")
}

/** Interface enumeration via java.net only; cannot see Android VPN state. */
open class JavaNetworkProbe : NetworkProbe {
    override fun interfaces(): List<NetInterface> = try {
        NetworkInterface.getNetworkInterfaces()?.toList().orEmpty().map {
            NetInterface(it.name, runCatching { it.isUp }.getOrDefault(false), tunFlag(it.name),
                pointToPoint = runCatching { it.isPointToPoint }.getOrDefault(false))
        }
    } catch (_: Exception) { emptyList() }
    override fun vpnActive(): Boolean = false

    /** Linux exposes /sys/class/net/<if>/tun_flags only for tun/tap devices; null when sysfs is unreadable. */
    protected open fun tunFlag(name: String): Boolean? {
        if (name.isEmpty() || '/' in name || name == "." || name == "..") return null
        val base = File("/sys/class/net", name)
        return try { if (!base.isDirectory) null else File(base, "tun_flags").exists() } catch (_: SecurityException) { null }
    }
}
