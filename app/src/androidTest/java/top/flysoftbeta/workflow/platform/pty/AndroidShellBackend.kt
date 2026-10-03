package top.flysoftbeta.workflow.platform.pty

import android.content.Context
import java.io.File
import java.io.IOException
import java.util.UUID
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.core.terminal.ShellPaths
import top.flysoftbeta.workflow.core.terminal.TerminalBackend
import top.flysoftbeta.workflow.core.terminal.TerminalProcess
import top.flysoftbeta.workflow.core.terminal.TerminalSpec
import top.flysoftbeta.workflow.platform.service.LocalRuntimeService

/**
 * [TerminalBackend] running Android's `/system/bin/sh` on the JNI PTY under the app's uid, in the
 * app-owned workspace (`filesDir/.workspace`), which is also HOME. It does not claim to be the Debian
 * environment; this adapter exists only in the instrumentation APK for isolated PTY tests. Each running process
 * holds a [LocalRuntimeService] lease, so terminals outlive the activity.
 */
class AndroidShellBackend(
    context: Context,
    root: File = top.flysoftbeta.workflow.platform.workspace.WorkspaceLocation.root(context),
) : TerminalBackend {
    private val appContext = context.applicationContext
    private val workspace: File by lazy { root.apply { mkdirs() }.canonicalFile }

    override val paths: ShellPaths by lazy { ShellPaths(workspace.path, workspace.path) }

    override suspend fun start(spec: TerminalSpec): TerminalProcess = withContext(Dispatchers.IO) {
        require(WorkspacePaths.normalizeOrNull(spec.directory) == spec.directory) { "终端目录必须相对于工作区" }
        val requested = if (spec.directory.isEmpty()) workspace else File(workspace, spec.directory).canonicalFile
        val directory = requested.takeIf { it.isDirectory && it.toPath().startsWith(workspace.toPath()) } ?: workspace
        val internal = File(workspace, "${WorkspacePaths.INTERNAL}/terminal").apply { mkdirs() }
        val temporary = File(appContext.cacheDir, "terminal-tmp").apply { mkdirs() }
        val environment = linkedMapOf(
            "HOME" to workspace.path,
            "PWD" to directory.path,
            "TMPDIR" to temporary.path,
            "SHELL" to "/system/bin/sh",
            "PATH" to "/system/bin:/system/xbin:/vendor/bin",
            "TERM" to "xterm-256color",
            "COLORTERM" to "truecolor",
            "LANG" to "C.UTF-8",
            "HISTFILE" to File(internal, "sh_history").path,
            "WORKFLOW_WORKSPACE" to workspace.path,
        )
        // Keep the Android runtime variables the shell's tools need (ANDROID_DATA, BOOTCLASSPATH, …).
        System.getenv().forEach { (key, value) ->
            if (key.startsWith("ANDROID_") || key == "BOOTCLASSPATH" || key == "DEX2OATBOOTCLASSPATH" || key == "SYSTEMSERVERCLASSPATH" ||
                key == "EXTERNAL_STORAGE") environment.putIfAbsent(key, value)
        }
        // Android's /system/etc/mkshrc overwrites even an inherited PS1; an explicit ~/.mkshrc wins.
        val profile = File(workspace, ".mkshrc")
        environment["ENV"] = if (profile.isFile) profile.path else "/dev/null"
        environment["PS1"] = "\${PWD##*/}\${PWD##/?*} $ "
        environment.putAll(spec.env.filterKeys { it.matches(NAME) })
        val argv = spec.argv ?: listOf("/system/bin/sh", "-i")
        val lease = "sh-" + UUID.randomUUID().toString().take(8)
        LocalRuntimeService.retainTerminal(appContext, lease).getOrThrow()
        try {
            val started = NativePty.spawn(
                directory.path.toByteArray(Charsets.UTF_8),
                argv.map { it.toByteArray(Charsets.UTF_8) }.toTypedArray(),
                environment.filterValues { '\u0000' !in it }.map { (key, value) -> "$key=$value".toByteArray(Charsets.UTF_8) }.toTypedArray(),
                spec.rows, spec.columns,
            )
            PtyTerminalProcess(started[0], started[1].toInt()) { LocalRuntimeService.releaseTerminal(appContext, lease) }
        } catch (error: CancellationException) {
            LocalRuntimeService.releaseTerminal(appContext, lease)
            throw error
        } catch (error: LinkageError) {
            LocalRuntimeService.releaseTerminal(appContext, lease)
            throw IOException("终端组件未能加载", error)
        } catch (error: Exception) {
            LocalRuntimeService.releaseTerminal(appContext, lease)
            throw error
        }
    }

    private companion object {
        val NAME = Regex("[A-Za-z_][A-Za-z0-9_]*")
    }
}
