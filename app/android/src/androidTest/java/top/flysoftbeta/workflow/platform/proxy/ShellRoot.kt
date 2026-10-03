package top.flysoftbeta.workflow.platform.proxy

import android.app.Instrumentation
import android.os.ParcelFileDescriptor
import android.system.Os
import java.io.File
import top.flysoftbeta.workflow.proxy.config.TunSettings
import java.io.FileInputStream
import java.io.FileOutputStream
import java.io.FilterInputStream
import java.io.IOException
import java.io.InputStream
import java.io.OutputStream
import java.util.UUID
import java.util.concurrent.CompletableFuture
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.runInterruptible
import top.flysoftbeta.workflow.proxy.guardian.GuardianCommand
import top.flysoftbeta.workflow.proxy.runtime.GuardianLauncher
import top.flysoftbeta.workflow.proxy.runtime.RootShell
import top.flysoftbeta.workflow.proxy.runtime.ShellResult

/**
 * Emulator-only root channel: with `adb root` on a userdebug image, `am instrument` runs as uid 0 and
 * UiAutomation shell commands inherit that. The app's own `su` path is not available to app uids there, so
 * this replaces only *how* the root process is spawned; protocol, guardian and kernel are the real ones.
 */
class ShellRoot(private val instrumentation: Instrumentation, private val scratch: File) {
    init { check(scratch.mkdirs() || scratch.isDirectory) }

    fun spawn(command: String): ParcelFileDescriptor = instrumentation.uiAutomation.executeShellCommand(command)

    /** Runs a script file as root (executeShellCommand has no shell quoting); returns merged output + exit code. */
    fun run(script: String, timeoutMs: Long = 20_000): ShellResult {
        val file = File(scratch, "cmd-${UUID.randomUUID()}.sh")
        file.writeText("(\n$script\n) 2>&1\necho \"__WF_EXIT__\$?\"\n")
        try {
            val output = CompletableFuture.supplyAsync {
                ParcelFileDescriptor.AutoCloseInputStream(spawn("sh ${file.path}")).use { it.readBytes().toString(Charsets.UTF_8) }
            }.get(timeoutMs, TimeUnit.MILLISECONDS)
            val marker = output.lastIndexOf("__WF_EXIT__")
            check(marker >= 0) { "root shell produced no exit marker: $output" }
            return ShellResult(output.substring(marker + 11).trim().toInt(), output.substring(0, marker))
        } finally { file.delete() }
    }

    fun uid(): String = run("id -u").output.trim()

    private var createdTunAlias = false
    /** API 28 exposes /dev/tun while its ip tuntap tool expects /dev/net/tun. Emulator fixture only. */
    fun prepareTunTool() {
        check(android.os.Build.HARDWARE in listOf("ranchu", "goldfish"))
        val result = run("if [ ! -e /dev/net/tun ] && [ -e /dev/tun ]; then mkdir -p /dev/net && ln -s /dev/tun /dev/net/tun && echo created; fi")
        check(result.code == 0) { "Unable to prepare emulator TUN fixture" }
        createdTunAlias = result.output.trim() == "created"
    }

    fun restoreTunTool() {
        if (createdTunAlias) { run("rm /dev/net/tun"); createdTunAlias = false }
    }

    val rootShell: RootShell = object : RootShell {
        override suspend fun run(script: String, timeoutMs: Long): ShellResult = runInterruptible(Dispatchers.IO) { this@ShellRoot.run(script, timeoutMs) }
    }

    /** Guardian with stdin/stderr on FIFOs in the app's own directory and stdout on the shell pipe. */
    inner class Launcher : GuardianLauncher {
        val processes = mutableListOf<ShellGuardianProcess>()
        override fun launch(guardian: File, kernel: File, directory: File, config: File, runId: String, tun: TunSettings?): Process {
            val dir = File(scratch, "guard-$runId").apply { check(mkdirs()) }
            val input = File(dir, "in"); val errors = File(dir, "err"); val status = File(dir, "status")
            Os.mkfifo(input.path, 384) // 0600
            Os.mkfifo(errors.path, 384)
            val script = File(dir, "run.sh")
            script.writeText(GuardianCommand.supervise(guardian.path, kernel.path, directory.path, config.path, runId, tun).removePrefix("exec ") +
                " < ${GuardianCommand.quote(input.path)} 2> ${GuardianCommand.quote(errors.path)}\necho \$? > ${GuardianCommand.quote(status.path)}\n")
            return ShellGuardianProcess(spawn("sh ${script.path}"), input, errors, status).also { processes += it }
        }
    }

    class ShellGuardianProcess(pfd: ParcelFileDescriptor, input: File, errors: File, private val status: File) : Process() {
        private val finished = CountDownLatch(1)
        // Opening a FIFO blocks until the other side opens it; do it off the caller's thread.
        private val writer = CompletableFuture.supplyAsync { FileOutputStream(input) }
        private val reader = CompletableFuture.supplyAsync { FileInputStream(errors) }
        private val stdout = object : FilterInputStream(ParcelFileDescriptor.AutoCloseInputStream(pfd)) {
            override fun read(): Int = super.read().also { if (it < 0) finished.countDown() }
            override fun read(b: ByteArray, off: Int, len: Int): Int = super.read(b, off, len).also { if (it < 0) finished.countDown() }
        }
        private val stdin = object : OutputStream() {
            private fun target() = try { writer.get(10, TimeUnit.SECONDS) } catch (error: Exception) { throw IOException("guardian stdin not opened", error) }
            override fun write(b: Int) = target().write(b)
            override fun write(b: ByteArray, off: Int, len: Int) = target().write(b, off, len)
            override fun flush() = target().flush()
            override fun close() { writer.getNow(null)?.close() ?: writer.thenAccept { it.close() } }
        }
        private val stderr = object : InputStream() {
            private fun source() = try { reader.get(10, TimeUnit.SECONDS) } catch (error: Exception) { throw IOException("guardian stderr not opened", error) }
            override fun read(): Int = source().read()
            override fun read(b: ByteArray, off: Int, len: Int): Int = source().read(b, off, len)
            override fun close() { reader.getNow(null)?.close() }
        }

        /** Simulates the app process dying: the control pipe closes without a stop record. */
        fun closeControlPipe() = stdin.close()

        override fun getOutputStream(): OutputStream = stdin
        override fun getInputStream(): InputStream = stdout
        override fun getErrorStream(): InputStream = stderr
        override fun waitFor(): Int { finished.await(); return exitValue() }
        override fun waitFor(timeout: Long, unit: TimeUnit): Boolean = finished.await(timeout, unit)
        override fun exitValue(): Int {
            if (finished.count > 0) throw IllegalThreadStateException()
            repeat(20) { status.takeIf { it.isFile }?.readText()?.trim()?.toIntOrNull()?.let { return it }; Thread.sleep(50) }
            return 0
        }
        override fun destroy() { runCatching { stdout.close() }; runCatching { stdin.close() }; runCatching { stderr.close() } }
    }
}
