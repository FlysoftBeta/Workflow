package top.flysoftbeta.workflow.platform.agent

import java.io.ByteArrayOutputStream
import java.io.IOException
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*
import kotlinx.serialization.json.*
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.process.AgentProcess
import top.flysoftbeta.workflow.agent.process.LaunchSpec
import top.flysoftbeta.workflow.agent.process.ProcessLauncher
import top.flysoftbeta.workflow.core.store.WorkspaceStore
import top.flysoftbeta.workflow.platform.engine.EngineController
import top.flysoftbeta.workflow.platform.engine.EngineProcessLauncher
import top.flysoftbeta.workflow.platform.engine.Guest

/** Chat plugin execution setup. Workspace core knows only generic guest process channels. */
object GuestAgents {
    private fun setup(controller: EngineController, executable: String, home: String) = BackendSetup(
        launcher = ProcessLauncher { spec ->
            val prepared = runGuest(controller, listOf("/usr/bin/mkdir", "-p", "-m", "0700", "--", home))
            check(prepared.exitCode == 0) { prepared.error.ifBlank { "无法准备代理工作目录" } }
            EngineProcessLauncher(controller).launch(spec)
        }, executable = executable,
        workspaceRoot = Guest.WORKSPACE, homeDir = home, tmpDir = "/tmp",
        env = mapOf("HOME" to Guest.HOME, "USER" to Guest.USER, "LOGNAME" to Guest.USER, "SHELL" to Guest.SHELL, "TERM" to "dumb"),
        readFile = { path, maxBytes ->
            // Backends may read their own guest home/history and user workspace; never arbitrary host paths.
            require(path.startsWith("${Guest.HOME}/") || (path.startsWith("${Guest.WORKSPACE}/") && !path.startsWith("${Guest.INTERNAL}/")))
            val result = runGuest(controller, listOf("/usr/bin/cat", "--", path), maxBytes)
            if (result.exitCode == 0) result.output else null
        },
    )
    fun codex(controller: EngineController) = setup(controller, "/opt/workflow/bundled/libcodex.so", "${Guest.HOME}/.codex")
    fun claude(controller: EngineController) = setup(controller, ClaudeCodeInstaller.GUEST_BINARY, "${Guest.HOME}/.claude")
}

internal data class GuestResult(val exitCode: Int, val output: ByteArray, val error: String)
internal suspend fun runGuest(controller: EngineController, argv: List<String>, limit: Int = 64 * 1024, timeout: Long = 60_000): GuestResult = withTimeout(timeout) {
    val process = EngineProcessLauncher(controller).launch(LaunchSpec(argv, emptyMap(), Guest.HOME, "chat-tool"))
    try {
        coroutineScope {
            val output = async(Dispatchers.IO) { process.stdout.use { input ->
                val bytes = ByteArrayOutputStream(); val chunk = ByteArray(16 * 1024)
                while (true) { val count = input.read(chunk); if (count < 0) break; require(bytes.size() + count <= limit) { "后端文件超过读取限制" }; bytes.write(chunk, 0, count) }
                bytes.toByteArray()
            } }
            val error = async(Dispatchers.IO) { process.stderr.use { input ->
                val bytes = ByteArray(4096); val text = StringBuilder()
                while (true) { val count = input.read(bytes); if (count < 0) break; text.append(String(bytes, 0, count, Charsets.UTF_8)); if (text.length > 8192) text.delete(0, text.length - 8192) }
                text.toString()
            } }
            GuestResult(process.awaitExit(), output.await(), error.await())
        }
    } finally { if (process.isAlive) process.kill(true) }
}

/** Installs this optional backend using an Engine-managed guest process, never client filesystem writes. */
class ClaudeCodeInstaller(private val controller: EngineController, private val scope: CoroutineScope) {
    sealed interface State {
        data object NotInstalled : State
        data class Installing(val progress: Float?) : State
        data object Installed : State
        data class Failed(val message: String) : State
    }
    private val mutable = MutableStateFlow<State>(State.NotInstalled)
    val state = mutable.asStateFlow()
    private var job: Job? = null
    @Volatile var onInstalled: () -> Unit = {}

    suspend fun refresh(): State {
        if (state.value is State.Installing || !controller.health.value.usable) return state.value
        val probe = runCatching { runGuest(controller, listOf(GUEST_BINARY, "--version"), timeout = 20_000) }.getOrNull()
        mutable.value = if (probe?.exitCode == 0 && probe.output.toString(Charsets.UTF_8).contains(VERSION)) State.Installed
            else state.value.takeIf { it is State.Failed } ?: State.NotInstalled
        return state.value
    }

    @Synchronized fun install() {
        if (job?.isActive == true || state.value is State.Installed) return
        mutable.value = State.Installing(null)
        job = scope.launch {
            try {
                controller.awaitUsable()
                val release = withContext(Dispatchers.IO) { releaseFor(controller.architecture) }
                val script = """
                    set -euo pipefail
                    dir="$GUEST_DIRECTORY"
                    mkdir -p "${'$'}dir" "${Guest.HOME}/.local/bin"
                    curl -fsSL --connect-timeout 20 --max-time 240 --retry 3 --retry-max-time 240 -C - -o "${'$'}dir/claude.download" "${release.url}"
                    if ! echo "${release.sha256}  ${'$'}dir/claude.download" | sha256sum -c --quiet -; then
                        rm -f "${'$'}dir/claude.download"; exit 1
                    fi
                    chmod 0755 "${'$'}dir/claude.download"
                    "${'$'}dir/claude.download" --version
                    mv -f "${'$'}dir/claude.download" "${'$'}dir/claude"
                    ln -sfn "${'$'}dir/claude" "${Guest.HOME}/.local/bin/claude"
                """.trimIndent()
                val result = runGuest(controller, listOf(Guest.SHELL, "-c", script), timeout = 600_000)
                check(result.exitCode == 0) { result.error.lineSequence().filter { it.isNotBlank() }.lastOrNull() ?: "安装未完成" }
                check(result.output.toString(Charsets.UTF_8).contains(VERSION)) { "安装后的版本不匹配" }
                mutable.value = State.Installed
                onInstalled()
            } catch (cancelled: CancellationException) { mutable.value = State.NotInstalled; throw cancelled }
            catch (error: Exception) { mutable.value = State.Failed(error.message ?: "Claude Code 安装失败") }
        }
    }

    private fun releaseFor(architecture: String?): Release {
        val abi = when (architecture) { "amd64" -> "x86_64"; "arm64" -> "arm64-v8a"; else -> throw IOException("未知运行环境架构") }
        val manifest = controller.appContext.assets.open("notices/claude-code-manifest.json").bufferedReader().use { Json.parseToJsonElement(it.readText()).jsonObject }
        check(manifest.getValue("version").jsonPrimitive.content == VERSION)
        val record = manifest.getValue("abis").jsonObject.getValue(abi).jsonObject
        return Release(record.getValue("url").jsonPrimitive.content, record.getValue("sha256").jsonPrimitive.content).also {
            require(it.url.matches(Regex("https://downloads\\.claude\\.ai/[A-Za-z0-9/._-]+")))
            require(it.sha256.matches(Regex("[0-9a-f]{64}")))
        }
    }
    private data class Release(val url: String, val sha256: String)
    companion object {
        const val VERSION = "2.1.283"
        const val GUEST_DIRECTORY = "${Guest.HOME}/.local/share/workflow/claude/$VERSION"
        const val GUEST_BINARY = "$GUEST_DIRECTORY/claude"
    }
}

object AgentEnvironmentWiring {
    suspend fun run(hub: AgentHub, engine: EngineController, claude: ClaudeCodeInstaller, store: WorkspaceStore) {
        var codex = false
        var installed = false
        claude.onInstalled = { hub.install(BackendKind.CLAUDE, GuestAgents.claude(engine)); installed = true }
        engine.addRestartListener(object : top.flysoftbeta.workflow.platform.engine.EnvironmentRestartListener {
            override suspend fun beforeRestart() { hub.replaceBackend(BackendKind.CODEX, null); hub.replaceBackend(BackendKind.CLAUDE, null) }
            override suspend fun afterRestart() {
                hub.replaceBackend(BackendKind.CODEX, GuestAgents.codex(engine))
                if (claude.refresh() is ClaudeCodeInstaller.State.Installed) hub.replaceBackend(BackendKind.CLAUDE, GuestAgents.claude(engine))
            }
        })
        combine(engine.health.map { it.usable }.distinctUntilChanged(), store.state.map { it.config.agent.backend == "claude" }.distinctUntilChanged()) { ready, wants -> ready to wants }
            .distinctUntilChanged().collect { (ready, wants) ->
                if (ready != codex) { codex = ready; hub.install(BackendKind.CODEX, if (ready) GuestAgents.codex(engine) else null) }
                if (ready) {
                    if (claude.refresh() is ClaudeCodeInstaller.State.Installed) {
                        if (!installed) { installed = true; hub.install(BackendKind.CLAUDE, GuestAgents.claude(engine)) }
                    } else if (wants && claude.state.value !is ClaudeCodeInstaller.State.Failed) claude.install()
                } else if (installed) { installed = false; hub.install(BackendKind.CLAUDE, null) }
            }
    }
}
