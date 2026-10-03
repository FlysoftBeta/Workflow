package top.flysoftbeta.workflow.platform.pty

import android.os.Process
import android.system.ErrnoException
import android.system.Os
import android.system.OsConstants
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import java.util.UUID
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.core.terminal.StreamingUtf8Decoder
import top.flysoftbeta.workflow.core.terminal.TerminalProcess
import top.flysoftbeta.workflow.core.terminal.TerminalSpec
import top.flysoftbeta.workflow.platform.service.LocalRuntimeService

/**
 * The Android-shell TerminalBackend on the real JNI PTY, under the app's own uid, in a throwaway
 * workspace (never the app's `.workspace`): tty, cwd, resize, job control, split UTF-8, ordered input,
 * /proc cwd, exit and reaping, and the foreground-service lease.
 */
@RunWith(AndroidJUnit4::class)
class AndroidShellBackendTest {
    private val context = InstrumentationRegistry.getInstrumentation().targetContext
    private val root = File(context.cacheDir, "pty-test-${UUID.randomUUID()}").apply { mkdirs() }.canonicalFile
    private val backend = AndroidShellBackend(context, root)

    @After fun cleanUp() { root.deleteRecursively() }

    private class Running(val process: TerminalProcess, val transcript: StringBuffer)

    private suspend fun launch(spec: TerminalSpec, scope: kotlinx.coroutines.CoroutineScope): Running {
        val process = backend.start(spec)
        val transcript = StringBuffer()
        val decoder = StreamingUtf8Decoder()
        scope.launch(Dispatchers.IO) { process.output.collect { transcript.append(decoder.decode(it)) } }
        return Running(process, transcript)
    }

    private suspend fun awaitFile(file: File, expected: String) = withTimeout(8000) {
        while (runCatching { file.readText() }.getOrNull() != expected) delay(20)
    }

    @Test fun ptyRunsInTheWorkspaceWithJobControlResizeAndUtf8() = runBlocking<Unit> {
        File(root, "dir").mkdirs()
        File(root, "dir/editor.txt").writeText("editor-to-pty-🌟中文")
        coroutineScope {
            val t = launch(TerminalSpec(directory = "dir", rows = 24, columns = 80), this)
            try {
                assertTrue(LocalRuntimeService.state.value.foreground)
                val dir = File(root, "dir")
                t.process.write("test -t 0 && test -t 1 && printf TTY_OK > tty.txt; pwd > cwd.txt; id -u > uid.txt; cat editor.txt\n".toByteArray())
                awaitFile(File(dir, "tty.txt"), "TTY_OK")
                awaitFile(File(dir, "cwd.txt"), dir.path + "\n")
                awaitFile(File(dir, "uid.txt"), Process.myUid().toString() + "\n")
                withTimeout(6000) { while (!t.transcript.contains("editor-to-pty-🌟中文")) delay(20) }
                assertEquals(dir.path, t.process.currentDirectory())

                t.process.resize(37, 111)
                t.process.write("stty size > size.txt\n".toByteArray())
                awaitFile(File(dir, "size.txt"), "37 111\n")

                t.process.write("sleep 30\n".toByteArray())
                delay(400)
                t.process.write(byteArrayOf(3))
                t.process.write("printf INTERRUPTED > int.txt\n".toByteArray())
                awaitFile(File(dir, "int.txt"), "INTERRUPTED")

                t.process.write("printf '\\360\\237'; sleep 1; printf '\\232\\200\\346\\265\\213\\350\\257\\225\\n'\n".toByteArray())
                withTimeout(6000) { while (!t.transcript.contains("🚀测试")) delay(20) }

                t.process.write("cd /system; echo moved\n".toByteArray())
                withTimeout(6000) { while (t.process.currentDirectory() != "/system") delay(20) }

                t.process.write("exit 7\n".toByteArray())
                assertEquals(7, withTimeout(6000) { t.process.awaitExit() })
            } finally {
                t.process.terminate(force = true)
            }
        }
    }

    @Test fun rapidInputKeepsItsOrderAndTerminateReapsABusyShell() = runBlocking<Unit> {
        coroutineScope {
            val t = launch(TerminalSpec(), this)
            val expected = "bookkeeper0123456789_".repeat(8)
            t.process.write("stty -echo\n".toByteArray())
            val command = "printf '%s' '$expected' > ordered.txt\n"
            withContext(Dispatchers.Main.immediate) {
                coroutineScope {
                    command.forEach { c -> launch(start = CoroutineStart.UNDISPATCHED) { t.process.write(c.toString().toByteArray()) } }
                }
            }
            awaitFile(File(root, "ordered.txt"), expected)
            t.process.write("sleep 60\n".toByteArray())
            delay(200)
            val pid = (t.process as PtyTerminalProcess).pid
            t.process.terminate()
            val code = withTimeout(6000) { t.process.awaitExit() }
            assertTrue(code != 0)
            try { Os.kill(pid, 0); fail("Child $pid still exists") } catch (e: ErrnoException) { assertEquals(OsConstants.ESRCH, e.errno) }
            assertTrue(runCatching { t.process.write("echo no\n".toByteArray()) }.isFailure)
        }
    }
}
