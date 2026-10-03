package top.flysoftbeta.workflow.proxy

import java.io.File
import top.flysoftbeta.workflow.proxy.config.TunSettings
import java.io.InputStream
import java.io.OutputStream
import java.nio.channels.Channels
import java.nio.channels.Pipe
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import top.flysoftbeta.workflow.proxy.network.NetInterface
import top.flysoftbeta.workflow.proxy.runtime.ForegroundLease
import top.flysoftbeta.workflow.proxy.runtime.GuardianLauncher
import top.flysoftbeta.workflow.proxy.runtime.KernelTool
import top.flysoftbeta.workflow.proxy.runtime.NetworkProbe
import top.flysoftbeta.workflow.proxy.runtime.RootShell
import top.flysoftbeta.workflow.proxy.runtime.ShellResult

/** What the fake guardian/kernel pair does after launch. */
enum class GuardianBehavior { NORMAL, FAIL_START, CRASH_AFTER_START, IGNORE_STOP, LOSE_CHANNEL, NEVER_START }

/**
 * Speaks the real guardian protocol (app/native/proxy-guard/README.md) over OS-independent NIO pipes and plays the
 * kernel: it serves a [FakeMihomo] controller on the configured port and "creates" a TUN interface.
 */
class FakeGuardianLauncher(
    private val controllerPort: () -> Int?,
    private val secret: String,
    private val probe: FakeNetworkProbe,
    private val tunDevice: () -> String?,
) : GuardianLauncher {
    @Volatile var behavior = GuardianBehavior.NORMAL
    val processes = CopyOnWriteArrayList<FakeGuardianProcess>()
    val servers = CopyOnWriteArrayList<FakeMihomo>()
    private val pids = AtomicInteger(4000)

    override fun launch(guardian: File, kernel: File, directory: File, config: File, runId: String, tun: TunSettings?): Process =
        FakeGuardianProcess(runId, pids.addAndGet(2), behavior).also { processes += it; it.start() }

    inner class FakeGuardianProcess(private val runId: String, private val pid: Int, private val behavior: GuardianBehavior) : Process() {
        private val stdin = Pipe.open(); private val stdout = Pipe.open(); private val stderr = Pipe.open()
        private val out = Channels.newOutputStream(stdout.sink())
        private val err = Channels.newOutputStream(stderr.sink())
        private val done = CountDownLatch(1)
        @Volatile private var code: Int? = null
        @Volatile var stopLine: String? = null
        @Volatile var eof = false
        private var server: FakeMihomo? = null

        fun start() = Thread({ run() }, "fake-guardian").apply { isDaemon = true }.start()

        private fun emit(line: String) = synchronized(out) { out.write((line + "\n").toByteArray()); out.flush() }

        private fun run() {
            try {
                when (behavior) {
                    GuardianBehavior.FAIL_START -> { emit("""{"event":"error","message":"Kernel exec or initial process identity verification failed","fatal":true}"""); finish(126); return }
                    GuardianBehavior.NEVER_START -> { Thread.sleep(60_000); return }
                    else -> {}
                }
                err.write("INFO starting with secret $secret\n".toByteArray())
                controllerPort()?.let { port -> server = FakeMihomo(secret, port).also { servers += it } }
                tunDevice()?.let { probe.interfaces += NetInterface(it, true, tun = true) }
                emit("""{"event":"started","uid":0,"pid":$pid,"startTime":777,"guardPid":${pid - 1},"guardStartTime":770,"runId":"$runId"}""")
                if (behavior == GuardianBehavior.CRASH_AFTER_START) {
                    Thread.sleep(1500)
                    err.write("FATAL listener failed\n".toByteArray())
                    teardown(); emit("""{"event":"exit","pid":$pid,"exitCode":2,"forced":false}"""); finish(2); return
                }
                val reader = Channels.newInputStream(stdin.source()).bufferedReader()
                while (true) {
                    val line = reader.readLine()
                    if (line == null) { eof = true; break }
                    if (line == "stop $runId $pid 777") { stopLine = line; break }
                    emit("""{"event":"error","message":"Stop rejected: control or owned child identity mismatch","fatal":false}""")
                }
                when (behavior) {
                    GuardianBehavior.IGNORE_STOP -> { Thread.sleep(60_000) }
                    GuardianBehavior.LOSE_CHANNEL -> { teardown(); finish(1) }
                    else -> { teardown(); emit("""{"event":"exit","pid":$pid,"exitCode":143,"forced":false}"""); finish(143) }
                }
            } catch (_: InterruptedException) { }
        }

        private fun teardown() {
            server?.close(); server = null
            tunDevice()?.let { name -> probe.interfaces.removeIf { it.name == name } }
        }

        private fun finish(exit: Int) {
            runCatching { out.close() }; runCatching { err.close() }
            code = exit; done.countDown()
        }

        override fun getOutputStream(): OutputStream = Channels.newOutputStream(stdin.sink())
        override fun getInputStream(): InputStream = Channels.newInputStream(stdout.source())
        override fun getErrorStream(): InputStream = Channels.newInputStream(stderr.source())
        override fun waitFor(): Int { done.await(); return code!! }
        override fun waitFor(timeout: Long, unit: TimeUnit): Boolean = done.await(timeout, unit)
        override fun exitValue(): Int = code ?: throw IllegalThreadStateException()
        override fun destroy() { teardown(); finish(137) }
    }
}

class FakeNetworkProbe : NetworkProbe {
    val interfaces = CopyOnWriteArrayList(listOf(NetInterface("lo", true, false), NetInterface("wlan0", true, false)))
    @Volatile var vpn = false
    override fun interfaces(): List<NetInterface> = interfaces.toList()
    override fun vpnActive(): Boolean = vpn
}

class FakeRootShell : RootShell {
    val scripts = CopyOnWriteArrayList<String>()
    @Volatile var granted = true
    @Volatile var links = "1: lo: <LOOPBACK,UP,LOWER_UP> mtu 65536 state UNKNOWN\n2: wlan0: <BROADCAST,UP,LOWER_UP> mtu 1500 state UP\n"
    @Volatile var linksCode = 0
    @Volatile var rules = "0:\tfrom all lookup local\n10000:\tfrom all fwmark 0xc0000/0xd0000 lookup 99\n32000:\tfrom all unreachable\n"
    /** Rules that appear while our TUN is up. */
    @Volatile var tunRules: String? = null
    var tunRunning: () -> Boolean = { false }
    /** Exit code of the read-only /proc identity check: 3 = process gone, 4 = unreadable. */
    @Volatile var procCheck = 3
    override suspend fun run(script: String, timeoutMs: Long): ShellResult {
        scripts += script
        if (!granted) return ShellResult(1, "Permission denied")
        return when {
            script == "id -u" -> ShellResult(0, "0\n")
            script == "ip -o link show" -> ShellResult(linksCode, links)
            script.startsWith("ip rule show") -> ShellResult(0, rules + (if (tunRunning()) tunRules.orEmpty() else ""))
            script.startsWith("if [ ! -e /proc/") -> ShellResult(procCheck, "")
            else -> ShellResult(127, "unexpected")
        }
    }
}

class FakeKernelTool : KernelTool {
    @Volatile var valid = true
    override suspend fun version(kernel: File) = ShellResult(0, "Mihomo Meta v1.19.31 android arm64\n")
    override suspend fun validate(kernel: File, directory: File, config: File) =
        if (valid) ShellResult(0, "configuration file ${config.path} test is successful\n") else ShellResult(1, "yaml: line 3: bad indentation\n")
}

class FakeLease : ForegroundLease {
    val held: MutableSet<String> = java.util.concurrent.ConcurrentHashMap.newKeySet()
    val retained = AtomicInteger()
    override suspend fun retain(id: String): Result<Unit> { held += id; retained.incrementAndGet(); return Result.success(Unit) }
    override fun release(id: String) { held -= id }
}
