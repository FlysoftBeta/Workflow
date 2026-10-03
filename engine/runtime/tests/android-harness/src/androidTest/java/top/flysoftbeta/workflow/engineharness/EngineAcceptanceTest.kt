package top.flysoftbeta.workflow.engineharness

import android.app.Instrumentation
import android.content.pm.ApplicationInfo
import android.os.Build
import android.os.Bundle
import android.os.SystemClock
import android.system.Os
import android.system.OsConstants
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.io.RandomAccessFile
import java.security.MessageDigest
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.regex.Pattern

/**
 * Runs inside the harness app's own zygote-forked process (the instrumentation target), so every engine
 * run inherits the app seccomp filter and the untrusted_app SELinux domain.
 *
 * Instrumentation arguments (`am instrument -e KEY VALUE`):
 *   cases      case file name inside filesDir (default cases.json; format: engine/runtime/tests/device/README.md)
 *   only       comma-separated case ids to run (default: all)
 *   results    results directory name inside filesDir (default results)
 * Output: filesDir/<results>/{g0.json,results.json,out/<id>.stdout,out/<id>.stderr}.
 * The test fails only on harness errors; case verdicts live in results.json.
 */
@RunWith(AndroidJUnit4::class)
class EngineAcceptanceTest {
    private val instrumentation: Instrumentation = InstrumentationRegistry.getInstrumentation()
    private val context = instrumentation.targetContext
    private val args: Bundle = InstrumentationRegistry.getArguments()
    private val libDir = File(context.applicationInfo.nativeLibraryDir)

    @Test
    fun runCases() {
        val files = context.filesDir
        val resultsDir = File(files, args.getString("results") ?: "results")
        resultsDir.deleteRecursively()
        val outDir = File(resultsDir, "out").apply { mkdirs() }
        val vars = mapOf(
            "LIB" to libDir.path,
            "ROOT" to File(files, "rootfs").path,
            "FILES" to files.path,
            "CACHE" to context.cacheDir.path,
            "RESULTS" to resultsDir.path,
            "ABI" to Build.SUPPORTED_ABIS[0],
        )
        val g0 = collectG0(vars)
        writeJson(File(resultsDir, "g0.json"), g0)
        report("G0 seccomp=${g0.optString("seccomp")} selinux=${g0.optString("selinux")} page=${g0.optLong("pageSize")}")

        val caseFile = File(files, args.getString("cases") ?: "cases.json")
        assertTrue("case file $caseFile missing (push it with run-as)", caseFile.isFile)
        val doc = JSONObject(caseFile.readText())
        val defaults = doc.optJSONObject("defaults") ?: JSONObject()
        val only = args.getString("only")?.split(',')?.map { it.trim() }?.filter { it.isNotEmpty() }?.toSet()

        val setupLog = JSONArray()
        doc.optJSONArray("setup")?.let { steps -> for (i in 0 until steps.length()) setupLog.put(setup(steps.getJSONObject(i), vars)) }

        val results = JSONArray()
        val cases = doc.getJSONArray("cases")
        val startedAt = System.currentTimeMillis()
        for (i in 0 until cases.length()) {
            val c = cases.getJSONObject(i)
            val id = c.getString("id")
            if (only != null && id !in only) continue
            val skip = skipReason(c)
            val r = if (skip != null) JSONObject().put("id", id).put("verdict", "SKIP").put("reasons", JSONArray().put(skip))
            else runCase(c, defaults, vars, outDir)
            results.put(r)
            report("${r.getString("verdict")} $id exit=${r.opt("exit")} ${r.optLong("durationMs")}ms ${r.optJSONArray("reasons") ?: ""}")
            writeResults(resultsDir, g0, setupLog, results, caseFile, startedAt, done = false)
        }
        writeResults(resultsDir, g0, setupLog, results, caseFile, startedAt, done = true)
    }

    // ---------------------------------------------------------------- G0 facts

    private fun collectG0(vars: Map<String, String>): JSONObject {
        val g = JSONObject()
        g.put("harnessVersion", Harness.VERSION)
        g.put("sdkInt", Build.VERSION.SDK_INT).put("release", Build.VERSION.RELEASE)
            .put("securityPatch", Build.VERSION.SECURITY_PATCH).put("fingerprint", Build.FINGERPRINT)
            .put("model", Build.MODEL).put("supportedAbis", JSONArray(Build.SUPPORTED_ABIS.toList()))
        val u = Os.uname()
        g.put("uname", JSONObject().put("sysname", u.sysname).put("release", u.release).put("version", u.version)
            .put("machine", u.machine))
        g.put("pageSize", Os.sysconf(OsConstants._SC_PAGESIZE))
        g.put("pid", Os.getpid()).put("uid", Os.getuid()).put("ppid", Os.getppid())
        g.put("cmdline", readSmall("/proc/self/cmdline").replace('\u0000', ' ').trim())
        g.put("parentCmdline", readSmall("/proc/${Os.getppid()}/cmdline").replace('\u0000', ' ').trim())
        g.put("selinux", readSmall("/proc/self/attr/current").trim('\u0000', '\n', ' '))
        val status = runCatching { File("/proc/self/status").readLines() }.getOrDefault(emptyList())
        val statusKeys = listOf("Seccomp", "Seccomp_filters", "NoNewPrivs", "CapEff", "CapBnd", "TracerPid", "Uid", "Gid")
        val statusObj = JSONObject()
        for (line in status) {
            val k = line.substringBefore(':')
            if (k in statusKeys) statusObj.put(k, line.substringAfter(':').trim())
        }
        g.put("status", statusObj)
        g.put("seccomp", statusObj.optString("Seccomp", "absent"))
        g.put("noNewPrivs", statusObj.optString("NoNewPrivs", "absent"))
        g.put("yamaPtraceScope", readSmall("/proc/sys/kernel/yama/ptrace_scope").trim())
        val ai = context.applicationInfo
        g.put("targetSdk", ai.targetSdkVersion)
            .put("debuggable", ai.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0)
            .put("extractNativeLibs", ai.flags and ApplicationInfo.FLAG_EXTRACT_NATIVE_LIBS != 0)
            .put("processName", ai.processName)
        g.put("nativeLibraryDir", libDir.path)
        g.put("nativeLibraryDirLabel", label(libDir.path))
        val libs = JSONArray()
        (libDir.listFiles() ?: emptyArray()).sortedBy { it.name }.forEach { f ->
            libs.put(JSONObject().put("name", f.name).put("size", f.length()).put("canExecute", f.canExecute())
                .put("sha256", sha256(f)).put("label", label(f.path)))
        }
        g.put("nativeLibraryDirListing", libs)
        g.put("vars", JSONObject(vars as Map<*, *>))
        g.put("filesDirLabel", label(vars.getValue("FILES")))
        val root = File(vars.getValue("ROOT"))
        g.put("rootfs", JSONObject().put("exists", root.isDirectory).put("label", label(root.path))
            .put("stamp", readSmall(File(context.filesDir, "rootfs.stamp").path).trim())
            .put("top", JSONArray((root.list() ?: emptyArray()).sorted())))
        val mounts = runCatching { File("/proc/self/mounts").readLines() }.getOrDefault(emptyList())
        g.put("dataMount", mounts.firstOrNull { it.split(' ').getOrNull(1) == "/data" } ?: "")
        return g
    }

    // ---------------------------------------------------------------- cases

    private fun skipReason(c: JSONObject): String? {
        if (!c.optBoolean("enabled", true)) return "disabled in case file"
        c.optJSONArray("abis")?.let { a ->
            if ((0 until a.length()).none { a.getString(it) == Build.SUPPORTED_ABIS[0] }) return "abi ${Build.SUPPORTED_ABIS[0]} not in ${a}"
        }
        if (c.has("minApi") && Build.VERSION.SDK_INT < c.getInt("minApi")) return "api < ${c.getInt("minApi")}"
        if (c.has("maxApi") && Build.VERSION.SDK_INT > c.getInt("maxApi")) return "api > ${c.getInt("maxApi")}"
        return null
    }

    private fun runCase(c: JSONObject, defaults: JSONObject, vars: Map<String, String>, outDir: File): JSONObject {
        val id = c.getString("id")
        require(id.matches(Regex("[A-Za-z0-9._-]+"))) { "bad case id $id" }
        val r = JSONObject().put("id", id)
        c.optString("note").takeIf { it.isNotEmpty() }?.let { r.put("note", it) }
        c.optJSONArray("tags")?.let { r.put("tags", it) }
        val exe = expand(c.optString("exe", defaults.optString("exe", "\${LIB}/${Harness.ENGINE}")), vars)
        val argv = listOf(exe) + strings(c.optJSONArray("argv")).map { expand(it, vars) }
        val timeoutMs = (c.optDouble("timeout", defaults.optDouble("timeout", 60.0)) * 1000).toLong()
        val cwd = expand(c.optString("cwd", defaults.optString("cwd", "\${FILES}")), vars)
        val pb = ProcessBuilder(argv).directory(File(cwd))
        if (c.optBoolean("clearEnv", defaults.optBoolean("clearEnv", false))) pb.environment().clear()
        val env = JSONObject()
        for (src in listOf(defaults.optJSONObject("env"), c.optJSONObject("env"))) {
            src ?: continue
            for (k in src.keys()) {
                if (src.isNull(k)) { pb.environment().remove(k); env.put(k, JSONObject.NULL) }
                else { val v = expand(src.getString(k), vars); pb.environment()[k] = v; env.put(k, v) }
            }
        }
        val stdoutFile = File(outDir, "$id.stdout")
        val stderrFile = File(outDir, "$id.stderr")
        pb.redirectInput(ProcessBuilder.Redirect.from(File("/dev/null")))
        pb.redirectOutput(ProcessBuilder.Redirect.to(stdoutFile))
        pb.redirectError(ProcessBuilder.Redirect.to(stderrFile))
        r.put("argv", JSONArray(argv)).put("env", env).put("cwd", cwd).put("timeoutMs", timeoutMs)

        val reasons = JSONArray()
        val t0 = SystemClock.elapsedRealtime()
        var exit: Int? = null
        var timedOut = false
        var t1 = t0
        val watchdog = Executors.newSingleThreadScheduledExecutor()
        try {
            val p = pb.start()
            // Process.waitFor(timeout) polls every 100 ms on Android; a blocking waitFor() plus a watchdog
            // keeps durations precise. SIGTERM first (the engine forwards it and SIGKILLs the tree after its
            // grace period), SIGKILL to the engine 6 s later.
            val stop = AtomicBoolean(false)
            watchdog.schedule({ stop.set(true); p.destroy() }, timeoutMs, TimeUnit.MILLISECONDS)
            watchdog.schedule({ p.destroyForcibly() }, timeoutMs + 6000, TimeUnit.MILLISECONDS)
            exit = p.waitFor()
            t1 = SystemClock.elapsedRealtime()
            timedOut = stop.get()
        } catch (e: Exception) {
            reasons.put("spawn failed: $e")
            t1 = SystemClock.elapsedRealtime()
        } finally {
            watchdog.shutdownNow()
        }
        r.put("durationMs", t1 - t0)
        r.put("exit", exit ?: JSONObject.NULL).put("timedOut", timedOut)
        SystemClock.sleep(200)
        val leftovers = leftoverProcesses()
        if (leftovers.length() > 0) r.put("leftoverProcesses", leftovers)

        val stdout = readCapped(stdoutFile, 8 shl 20)
        val stderr = readCapped(stderrFile, 8 shl 20)
        r.put("stdoutBytes", stdoutFile.length()).put("stderrBytes", stderrFile.length())
        r.put("stdoutHead", stdout.take(4096))
        r.put("stderrHead", stderr.take(4096))
        if (stderr.length > 4096) r.put("stderrTail", stderr.takeLast(4096))

        if (timedOut) reasons.put("timed out after ${timeoutMs}ms")
        if (c.has("exit") && !timedOut) {
            val want = c.get("exit")
            val ok = when (want) {
                is JSONArray -> (0 until want.length()).any { want.getInt(it) == exit }
                else -> c.getInt("exit") == exit
            }
            if (!ok) reasons.put("exit $exit, expected $want")
        }
        checkRegex(c, "stdout", stdout, true, reasons)
        checkRegex(c, "stderr", stderr, true, reasons)
        checkRegex(c, "stdoutNot", stdout, false, reasons)
        checkRegex(c, "stderrNot", stderr, false, reasons)
        if (c.optBoolean("noLeftovers", defaults.optBoolean("noLeftovers", true)) && leftovers.length() > 0)
            reasons.put("left ${leftovers.length()} process(es) behind")
        r.put("reasons", reasons)
        r.put("verdict", if (reasons.length() == 0) "PASS" else "FAIL")
        return r
    }

    private fun checkRegex(c: JSONObject, key: String, text: String, mustMatch: Boolean, reasons: JSONArray) {
        val re = c.optString(key).takeIf { it.isNotEmpty() } ?: return
        val found = Pattern.compile(re, Pattern.MULTILINE).matcher(text).find()
        if (found != mustMatch) reasons.put(if (mustMatch) "$key !~ /$re/" else "$key ~ /$re/ (forbidden)")
    }

    private fun setup(step: JSONObject, vars: Map<String, String>): JSONObject {
        val log = JSONObject(step.toString())
        try {
            val from = File(expand(step.getString("copy"), vars))
            val to = File(expand(step.getString("to"), vars))
            when {
                !from.isFile -> log.put("result", "skipped: $from missing")
                !to.parentFile!!.isDirectory -> log.put("result", "skipped: ${to.parentFile} missing")
                else -> {
                    from.copyTo(to, overwrite = true)
                    Os.chmod(to.path, step.optString("mode", "755").toInt(8))
                    log.put("result", "ok").put("sha256", sha256(to))
                }
            }
        } catch (e: Exception) {
            log.put("result", "error: $e")
        }
        return log
    }

    // ---------------------------------------------------------------- helpers

    private fun writeResults(dir: File, g0: JSONObject, setup: JSONArray, results: JSONArray, caseFile: File,
                             startedAt: Long, done: Boolean) {
        val counts = JSONObject()
        for (i in 0 until results.length()) {
            val v = results.getJSONObject(i).getString("verdict")
            counts.put(v, counts.optInt(v) + 1)
        }
        writeJson(File(dir, "results.json"), JSONObject().put("complete", done).put("caseFile", caseFile.path)
            .put("caseFileSha256", sha256(caseFile)).put("startedAt", startedAt)
            .put("finishedAt", System.currentTimeMillis()).put("summary", counts)
            .put("g0", g0).put("setup", setup).put("cases", results))
    }

    /** Processes of our UID that are not this instrumentation process (engine/guest stragglers). */
    private fun leftoverProcesses(): JSONArray {
        val me = Os.getpid()
        val uid = Os.getuid()
        val out = JSONArray()
        for (d in File("/proc").listFiles() ?: emptyArray()) {
            val pid = d.name.toIntOrNull() ?: continue
            if (pid == me) continue
            val st = runCatching { Os.stat(d.path) }.getOrNull() ?: continue
            if (st.st_uid != uid) continue
            val cmd = readSmall("${d.path}/cmdline").replace('\u0000', ' ').trim()
            out.put(JSONObject().put("pid", pid).put("cmdline", cmd))
        }
        return out
    }

    private fun report(line: String) {
        val b = Bundle()
        b.putString(Instrumentation.REPORT_KEY_STREAMRESULT, "engine-harness: $line\n")
        instrumentation.sendStatus(0, b)
    }

    private fun expand(s: String, vars: Map<String, String>): String =
        Regex("""\$\{([A-Z_]+)\}""").replace(s) { m -> vars[m.groupValues[1]] ?: m.value }

    private fun strings(a: JSONArray?): List<String> = if (a == null) emptyList() else (0 until a.length()).map { a.getString(it) }

    private fun readSmall(path: String): String = runCatching { File(path).readBytes().decodeToString() }.getOrDefault("")

    private fun readCapped(f: File, cap: Int): String {
        if (!f.isFile) return ""
        RandomAccessFile(f, "r").use { raf ->
            val n = minOf(raf.length(), cap.toLong()).toInt()
            val buf = ByteArray(n)
            raf.readFully(buf)
            return buf.decodeToString()
        }
    }

    private fun label(path: String): String = runCatching {
        Os.getxattr(path, "security.selinux").decodeToString().trim('\u0000')
    }.getOrElse { "?" }

    private fun sha256(f: File): String = runCatching {
        val md = MessageDigest.getInstance("SHA-256")
        f.inputStream().use { input ->
            val buf = ByteArray(1 shl 16)
            while (true) { val n = input.read(buf); if (n < 0) break; md.update(buf, 0, n) }
        }
        md.digest().joinToString("") { "%02x".format(it.toInt() and 0xff) }
    }.getOrDefault("?")

    private fun writeJson(f: File, o: JSONObject) {
        f.parentFile?.mkdirs()
        val tmp = File(f.path + ".tmp")
        tmp.writeText(o.toString(1))
        tmp.renameTo(f)
    }
}
