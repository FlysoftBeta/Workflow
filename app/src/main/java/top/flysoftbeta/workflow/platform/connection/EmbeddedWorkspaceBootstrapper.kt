package top.flysoftbeta.workflow.platform.connection

import android.content.Context
import java.io.File
import java.io.InputStream
import java.io.OutputStream
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.*
import java.util.concurrent.atomic.AtomicBoolean
import top.flysoftbeta.workflow.core.connection.*

/** Android process bootstrap; business clients see only the transport interface. */
internal class EmbeddedWorkspaceBootstrapper(context: Context) : WorkspaceBootstrapper {
    private val app = context.applicationContext
    override suspend fun connect(configuration: WorkspaceConnectionConfig): WorkspaceTransport {
        var pending: EmbeddedWorkspaceTransport? = null
        try { return withContext(Dispatchers.IO) {
        val endpoint = configuration.endpoint as? WorkspaceEndpoint.Embedded
            ?: throw UnsupportedWorkspaceTransport("远程连接只提供扩展接口，此版本尚未实现")
        check(Regex("[A-Za-z0-9_-]{1,64}").matches(endpoint.workspaceId)) { "工作区标识无效" }
        val root = File(app.filesDir, "workspaces/${endpoint.workspaceId}")
        val native = File(app.applicationInfo.nativeLibraryDir)
        val server = File(native, "libworkflow-engine.so")
        check(server.isFile && server.canExecute()) { "安装包缺少工作区 Engine" }
        val cache = File(app.cacheDir, "engine").apply { check(isDirectory || mkdirs()) { "无法创建 Engine 临时目录" } }
        val process = ProcessBuilder(
            server.path, "serve", "--root", root.path,
            "--runtime", File(native, "libworkflow-runtime.so").path,
            "--loader", File(native, "libworkflow-loader.so").path,
            "--apk", app.applicationInfo.sourceDir, "--native-dir", native.path,
        ).directory(app.filesDir).apply {
            environment().keys.toList().forEach { key ->
                if (!key.startsWith("ANDROID_") && key !in setOf("BOOTCLASSPATH", "DEX2OATBOOTCLASSPATH", "SYSTEMSERVERCLASSPATH")) environment().remove(key)
            }
            environment()["PATH"] = "/system/bin:/system/xbin:/vendor/bin"
            environment()["TMPDIR"] = cache.path
        }.start()
        EmbeddedWorkspaceTransport(process).also { pending = it }
        } } catch (error: Exception) {
            // withContext has prompt cancellation: a newly created transport may never reach its caller.
            withContext(NonCancellable + Dispatchers.IO) { pending?.close() }
            throw error
        }
    }
}

internal class EmbeddedWorkspaceTransport(private val process: Process) : WorkspaceTransport {
    override val input: InputStream get() = process.inputStream
    override val output: OutputStream get() = process.outputStream
    private val tail = StringBuilder()
    private val closed = AtomicBoolean()
    init { Thread({
        val bytes = ByteArray(4096)
        runCatching { process.errorStream.use { stream -> while (true) {
            val count = stream.read(bytes); if (count < 0) break
            synchronized(tail) { tail.append(String(bytes, 0, count, Charsets.UTF_8)); if (tail.length > 8192) tail.delete(0, tail.length - 8192) }
        } } }
    }, "workspace-diagnostics").apply { isDaemon = true }.start() }
    fun diagnostic() = synchronized(tail) { tail.toString() }
    override fun close() {
        if (!closed.compareAndSet(false, true)) return
        process.destroy()
        if (!process.waitFor(2, TimeUnit.SECONDS)) process.destroyForcibly()
        runCatching { output.close() }
        runCatching { input.close() }
    }
}
