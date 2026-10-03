package top.flysoftbeta.workflow.proxy.runtime

import java.io.BufferedWriter
import java.io.File
import java.io.IOException
import java.util.UUID
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.delay
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.runInterruptible
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import top.flysoftbeta.workflow.proxy.config.ConfigInspection
import top.flysoftbeta.workflow.proxy.config.MihomoConfigInspector
import top.flysoftbeta.workflow.proxy.config.TunSettings
import top.flysoftbeta.workflow.proxy.controller.ConnectionsSnapshot
import top.flysoftbeta.workflow.proxy.controller.DelayResult
import top.flysoftbeta.workflow.proxy.controller.MihomoController
import top.flysoftbeta.workflow.proxy.controller.MihomoControllerClient
import top.flysoftbeta.workflow.proxy.controller.ProviderRefresh
import top.flysoftbeta.workflow.proxy.controller.ProxyLogEntry
import top.flysoftbeta.workflow.proxy.controller.ProxyLogLevel
import top.flysoftbeta.workflow.proxy.controller.ProxyMode
import top.flysoftbeta.workflow.proxy.controller.ProxyProvider
import top.flysoftbeta.workflow.proxy.controller.ProxySnapshot
import top.flysoftbeta.workflow.proxy.guardian.GuardianEvent
import top.flysoftbeta.workflow.proxy.guardian.ProxyProcessIdentity
import top.flysoftbeta.workflow.proxy.guardian.verifyGuardianExit
import top.flysoftbeta.workflow.proxy.network.ProxyConflict
import top.flysoftbeta.workflow.proxy.network.TunConflictDetector
import top.flysoftbeta.workflow.proxy.redact.ProxySecrets
import top.flysoftbeta.workflow.proxy.redact.redactSecret

/** A start refused before root was requested (or before launch) because another VPN/TUN owns routing. */
class ProxyConflictException(val conflict: ProxyConflict) : IllegalStateException(conflict.describe())

/** Timeouts, adjustable for tests. */
data class ProxyTiming(
    val guardianStartMs: Long = 10_000,
    /** A kernel that dies this soon after exec (bad port, TUN failure) is reported as a failed start. */
    val earlyExitMs: Long = 1000,
    val controllerReadyMs: Long = 15_000,
    val tunReadyMs: Long = 10_000,
    /** Must exceed the guardian's SIGTERM→SIGKILL grace (native/proxy-guard STOP_GRACE_MS). */
    val stopMs: Long = 12_000,
    val pollMs: Long = 100,
    val rootPromptMs: Long = 60_000,
)

/**
 * The proxy lifecycle, independent of Android. One instance owns one `.workflow/proxy` directory and at most
 * one kernel, always through the root guardian: it never signals a PID by itself and never trusts a persisted
 * PID. Root is requested only by [start] and [stop] (explicit user actions). Nothing here rewrites config.yaml.
 */
class ProxyRuntime(
    val files: ProxyFiles,
    private val kernel: File,
    private val guardian: File,
    private val ports: ProxyPorts,
    private val timing: ProxyTiming = ProxyTiming(),
    private val scope: CoroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.IO),
    private val controllerClient: MihomoControllerClient = MihomoControllerClient(),
) : ProxyApi {
    private val mutableState = MutableStateFlow(ProxyState())
    override val state: StateFlow<ProxyState> = mutableState.asStateFlow()
    override val configFile: File get() = files.config
    private val lifecycle = Mutex()
    @Volatile private var active: Run? = null
    @Volatile private var connectionClosed = false
    @Volatile private var kernelVersion: String? = null
    /** Parsed config.yaml keyed by modification time and size, so polling does not re-parse large files. */
    @Volatile private var inspected: Pair<String, Result<ConfigInspection>>? = null
    /** Last groups read from a running kernel, with the config stamp they belong to (kept after a stop). */
    @Volatile private var lastLive: Pair<String, ProxySnapshot>? = null

    private class Run(
        val id: String,
        val leaseId: String,
        val process: Process,
        val secret: String?,
        val secrets: ProxySecrets,
        val controller: MihomoController?,
        val tun: TunSettings,
        val interfacesBefore: Set<String>,
        /** [configStamp] of the file this run started with. */
        val configStamp: String,
    ) {
        val input: BufferedWriter = process.outputStream.bufferedWriter()
        val started = CompletableDeferred<ProxyProcessIdentity>()
        val exited = CompletableDeferred<Int>()
        @Volatile var identity: ProxyProcessIdentity? = null
        @Volatile var childExited = false
        @Volatile var forced = false
        @Volatile var cleanupFailed = false
        @Volatile var startupRejected = false
        @Volatile var stopRequested = false
        @Volatile var diagnostic = ""
        @Volatile var device: String? = null
        val jobs = mutableListOf<Job>()
        val alive: Boolean get() = !childExited && process.isAlive
    }

    // ---------------------------------------------------------------- read-only

    /** Re-reads config, capabilities and conflicts without root; queries the controller when running. */
    override suspend fun refresh(): Result<ProxyState> = outcome {
        val inspection = readInspection()
        val run = active
        val runningNow = run != null && run.identity != null && run.alive && state.value.phase == ProxyPhase.RUNNING
        val conflict = detectConflict(inspection?.tun, if (runningNow) run.device else null)
        // A record from an earlier app process normally means its guardian already stopped the kernel (EOF).
        // Only a live sign (our controller port answers, or our TUN device is up) makes it unconfirmed.
        val orphan = run == null && files.hasRecord() && orphanLikely(inspection)
        val stamp = configStamp()
        mutableState.update { current ->
            val live = runningNow && current.controllerReachable
            current.copy(capabilities = current.capabilities.copy(kernel = kernel.isFile && kernel.canExecute(), guardian = guardian.isFile && guardian.canExecute()),
                conflict = conflict.copy(ruleCollisions = if (current.phase == ProxyPhase.CONFLICT) current.conflict.ruleCollisions else emptyList()),
                stopUnconfirmed = current.stopUnconfirmed || orphan,
                // Mihomo's default mode is rule when the key is absent.
                mode = if (live) current.mode else ProxyMode.parse(inspection?.let { it.mode ?: "rule" }),
                groups = if (live || current.busy) current.groups
                    else lastLive?.takeIf { it.first == stamp }?.second ?: inspection?.groups ?: ProxySnapshot.EMPTY)
        }
        if (kernelVersion == null && kernel.isFile && kernel.canExecute()) {
            kernelVersion = runCatching { ports.kernelTool.version(kernel) }.getOrNull()?.takeIf { it.code == 0 }?.output?.trim()?.lineSequence()?.firstOrNull()?.take(200)
            mutableState.update { it.copy(version = kernelVersion) }
        }
        if (runningNow && run.controller != null) runCatching { syncController(run) }
        mutableState.value
    }

    /** Creates the starter template on first use; an existing config.yaml is never touched. */
    override suspend fun ensureConfig(): Result<File> = outcome {
        if (files.createTemplate()) readInspection()
        files.config
    }

    suspend fun readConfig(): Result<String> = outcome { files.readConfig().toString(Charsets.UTF_8) }

    /** Explicit user import (replaces the file). A running kernel keeps its loaded config until restarted. */
    override suspend fun importConfig(bytes: ByteArray): Result<Unit> = outcome<Unit> { files.replaceConfig(bytes); readInspection() }

    /** `mihomo -t`, unprivileged; never changes the network. */
    override suspend fun checkConfig(): Result<ProxyConfigCheck> = outcome {
        val bytes = files.readConfig()
        val inspection = runCatching { MihomoConfigInspector.inspect(bytes.toString(Charsets.UTF_8)) }
        validate(inspection)
    }

    override suspend fun logTail(maxBytes: Int): String = withContext(Dispatchers.IO) { files.logTail(maxBytes) }

    // ---------------------------------------------------------------- lifecycle

    /**
     * Explicit user action. May show the root prompt. A root-free VPN/TUN check runs first; the
     * root kernel-link check then catches interfaces Android omitted. Conflicts stop launch with
     * [ProxyPhase.CONFLICT]. No caller can claim another owner's routing.
     */
    override suspend fun start(): Result<Unit> = outcome {
        val conflict = lifecycle.withLock { startLocked() }
        if (conflict != null) throw ProxyConflictException(conflict)
    }

    /** Returns the conflict that refused the start, or null when started (or already running). */
    private suspend fun startLocked(): ProxyConflict? {
            check(!connectionClosed) { "工作区连接已关闭" }
            val existing = active
            if (existing != null && existing.alive) {
                check(existing.identity != null && !state.value.stopUnconfirmed) { "上次启动或停止尚未确认完成，请先停止并检查状态" }
                return null
            }
            check(existing == null || existing.childExited || existing.startupRejected) { "守护通道异常退出，尚未确认内核状态；请先停止并检查设备网络" }
            mutableState.update { it.copy(phase = ProxyPhase.STARTING, progress = "正在校验配置", error = null, traffic = null,
                delays = emptyMap(), testing = emptySet(), tun = null, controllerReachable = false, pid = null) }
            var leaseId: String? = null
            var launched: Run? = null
            try {
                check(kernel.isFile && kernel.canExecute()) { "此设备尚未安装可执行的 Mihomo 内核" }
                check(guardian.isFile && guardian.canExecute()) { "代理进程管理组件不可用，请安装完整的 Workflow 版本" }
                val stamp = configStamp()
                val bytes = files.readConfig()
                val digest = ProxyFiles.digest(bytes)
                val inspection = MihomoConfigInspector.inspect(bytes.toString(Charsets.UTF_8))
                mutableState.update { it.copy(config = ProxyConfigState(true, inspection, null)) }
                inspection.blocking.firstOrNull()?.let { throw IllegalStateException(it.message) }
                val checked = validate(Result.success(inspection))
                check(checked.valid) { checked.output.ifBlank { "代理配置校验失败" } }
                inspection.controller?.let { check(!MihomoControllerClient.acceptsConnections(it.endpoint)) { "控制接口端口已被其他进程占用；不会启动重复内核" } }
                inspection.ports.filterValues { MihomoControllerClient.acceptsConnections("http://127.0.0.1:$it") }.keys.firstOrNull()?.let {
                    throw IllegalStateException("$it ${inspection.ports[it]} 已被其他进程占用")
                }
                val tun = inspection.tun
                val interfacesBefore = runCatching { ports.networkProbe.interfaces().map { it.name }.toSet() }.getOrDefault(emptySet())
                if (tun.enable) {
                    val conflict = detectConflict(tun, null)
                    if (conflict.any) { enterConflict(conflict); return conflict }
                }
                leaseId = UUID.randomUUID().toString()
                ports.lease.retain(leaseId).getOrThrow()
                progress("正在请求 Root 授权")
                requireRoot()
                if (tun.enable) {
                    progress("正在检查路由规则")
                    val links = ports.rootShell.run("ip -o link show", 10_000)
                    check(links.code == 0) { "无法检查系统网卡；为避免接管其他代理，未启动 TUN" }
                    val kernelConflict = TunConflictDetector.detect(TunConflictDetector.parseLinks(links.output),
                        vpnActive = false, ownDevice = null, configuredDevice = tun.explicitDevice)
                    if (kernelConflict.any) {
                        ports.lease.release(leaseId); leaseId = null
                        enterConflict(kernelConflict); return kernelConflict
                    }
                    val rules = ports.rootShell.run("ip rule show && ip -6 rule show", 10_000)
                    check(rules.code == 0) { "无法检查路由规则；为避免接管其他代理，未启动 TUN" }
                    if (rules.code == 0) {
                        val soft = detectConflict(tun, null)
                        if (soft.any) {
                            ports.lease.release(leaseId); leaseId = null
                            enterConflict(soft); return soft
                        }
                        val collisions = TunConflictDetector.ruleCollisions(tun, TunConflictDetector.parseRules(rules.output), soft.foreignInterfaces)
                        if (collisions.isNotEmpty()) {
                            ports.lease.release(leaseId); leaseId = null
                            val conflict = soft.copy(ruleCollisions = collisions)
                            enterConflict(conflict); return conflict
                        }
                    }
                }
                rejectLivePreviousRecord()
                check(ProxyFiles.digest(files.readConfig()) == digest) { "配置在校验期间已改变，请重新启动" }
                currentCoroutineContext().ensureActive()
                check(!connectionClosed) { "工作区连接已关闭" }
                progress("正在启动内核")
                val id = UUID.randomUUID().toString()
                val process = ports.guardianLauncher.launch(guardian, kernel, files.directory, files.config, id, tun)
                val controller = inspection.controller?.let { MihomoController(it, controllerClient) }
                val run = Run(id, leaseId, process, inspection.controller?.secret ?: "", inspection.secrets, controller, tun, interfacesBefore, stamp)
                leaseId = null // owned by the run from here on
                active = run; launched = run
                supervise(run)
                if (connectionClosed) closeOwnedChannel()
                val identity = withTimeoutOrNull(timing.guardianStartMs) { run.started.await() }
                    ?: throw IOException("守护进程未在 ${timing.guardianStartMs / 1000} 秒内确认启动")
                check(identity.runId == id)
                mutableState.update { it.copy(pid = identity.pid, stopUnconfirmed = false) }
                val earlyDeadline = System.nanoTime() + timing.earlyExitMs * 1_000_000
                while (System.nanoTime() < earlyDeadline) { failIfExited(run); delay(timing.pollMs) }
                if (controller != null) {
                    progress("正在等待控制接口")
                    awaitController(run)
                }
                if (tun.enable) {
                    progress("正在建立 TUN")
                    verifyTun(run)
                }
                failIfExited(run)
                mutableState.update { it.copy(phase = ProxyPhase.RUNNING, progress = null) }
                startBackgroundJobs(run)
                return null
            } catch (error: Throwable) {
                val cleanupError = withContext(NonCancellable) {
                    runCatching {
                        if (launched != null) stopRun(launched)
                        else leaseId?.let { ports.lease.release(it) }
                    }.exceptionOrNull()
                }
                if (cleanupError != null) error.addSuppressed(cleanupError)
                val cancelled = error is CancellationException && cleanupError == null
                val message = if (cancelled) null else safeMessage(error, launched?.secret)
                mutableState.update { it.copy(phase = if (cancelled) ProxyPhase.STOPPED else ProxyPhase.ERROR, progress = null, error = message, pid = null,
                    controllerReachable = false, tun = null, traffic = null,
                    stopUnconfirmed = cleanupError != null || it.stopUnconfirmed) }
                throw error
            }
    }

    /** Leaves [ProxyPhase.CONFLICT] (or a stale error) without starting anything. */
    override fun dismiss() {
        mutableState.update {
            val stale = it.phase == ProxyPhase.CONFLICT || (it.phase == ProxyPhase.ERROR && active == null && !it.stopUnconfirmed)
            if (stale) it.copy(phase = ProxyPhase.STOPPED, error = null) else it
        }
    }

    override suspend fun stop(): Result<Unit> = outcome {
        lifecycle.withLock {
            val run = active
            if (run == null && !files.hasRecord() && !state.value.stopUnconfirmed) {
                mutableState.update { it.copy(phase = ProxyPhase.STOPPED, progress = null, error = null) }
                return@withLock
            }
            mutableState.update { it.copy(phase = ProxyPhase.STOPPING, progress = "正在停止代理", error = null) }
            try {
                if (run != null) stopRun(run)
                else {
                    // No live channel: never turn a recorded PID into kill(2). Only confirm it is gone.
                    requireRoot()
                    rejectLivePreviousRecord()
                }
                mutableState.update { it.copy(phase = ProxyPhase.STOPPED, progress = null, stopUnconfirmed = false, pid = null,
                    controllerReachable = false, tun = null, traffic = null, testing = emptySet(),
                    error = when {
                        run?.cleanupFailed == true -> "内核已结束，但未能确认 TUN 路由清理；请检查网络状态"
                        run?.forced == true -> "已强制结束内核，请检查网络状态"
                        else -> null
                    }) }
            } catch (error: Exception) {
                mutableState.update { it.copy(phase = ProxyPhase.ERROR, progress = null, stopUnconfirmed = true, error = safeMessage(error, run?.secret)) }
                throw error
            }
        }
    }

    /** A lost Workspace connection closes only our guardian pipe; never probes root or signals a PID. */
    fun closeOwnedChannel() {
        connectionClosed = true
        active?.let { run ->
            run.stopRequested = true
            mutableState.update { it.copy(phase = ProxyPhase.STOPPING, progress = "工作区已断开，正在停止代理") }
            run.jobs.forEach { it.cancel() }
            scope.launch { runCatching { synchronized(run.input) { run.input.close() } } }
        }
    }

    // ---------------------------------------------------------------- controller actions

    override suspend fun setMode(mode: ProxyMode): Result<Unit> = controllerAction { run, controller ->
        controller.setMode(mode)
        mutableState.update { it.copy(mode = mode) }
        syncGroups(run, controller)
    }

    override suspend fun select(group: String, node: String): Result<Unit> = controllerAction { run, controller ->
        controller.select(group, node)
        syncGroups(run, controller)
    }

    override suspend fun refreshGroups(): Result<ProxySnapshot> = controllerAction { run, controller -> syncGroups(run, controller) }

    override suspend fun testNode(name: String, url: String?, timeoutMs: Int): Result<DelayResult> =
        controllerAction { run, controller ->
            testing(name) {
                val result = controller.nodeDelay(name, url ?: MihomoController.DEFAULT_TEST_URL, timeoutMs)
                mutableState.update { it.copy(delays = it.delays + (name to result)) }
                runCatching { syncGroups(run, controller) }
                result
            }
        }

    /** Tests every member of [group] (the group's own test URL unless [url] is given). */
    override suspend fun testGroup(group: String, url: String?, timeoutMs: Int): Result<Map<String, DelayResult>> =
        controllerAction { run, controller ->
            testing(group) {
                val testUrl = url ?: state.value.groups.group(group)?.testUrl ?: MihomoController.DEFAULT_TEST_URL
                val results = controller.groupDelay(group, testUrl, timeoutMs)
                mutableState.update { it.copy(delays = it.delays + results) }
                runCatching { syncGroups(run, controller) }
                results
            }
        }

    override suspend fun loadProviders(): Result<List<ProxyProvider>> = controllerAction { _, controller ->
        controller.providers().also { list -> mutableState.update { it.copy(providers = list) } }
    }

    /** Refreshes every HTTP/File proxy and rule provider; per-provider errors are in the result. */
    override suspend fun refreshProviders(): Result<List<ProviderRefresh>> = controllerAction { run, controller ->
        controller.refreshProviders().map { it.copy(error = it.error?.let(run.secrets::redact)) }.also {
            runCatching { mutableState.update { state -> state.copy(providers = controller.providers()) } }
            runCatching { syncGroups(run, controller) }
        }
    }

    /** Read-only snapshot of tracked connections. */
    override suspend fun connections(): Result<ConnectionsSnapshot> = controllerAction { _, controller -> controller.connections() }

    /** Live kernel log (secret-redacted) while running; completes when the kernel stops. */
    override fun logs(level: ProxyLogLevel): Flow<ProxyLogEntry> = flow {
        val (run, controller) = activeController()
        controller.logs(level).collect { emit(it.copy(payload = run.secrets.redact(it.payload))) }
    }

    // ---------------------------------------------------------------- internals

    private suspend fun <T> controllerAction(action: suspend (Run, MihomoController) -> T): Result<T> = withContext(Dispatchers.IO) {
        try {
            val (run, controller) = activeController()
            Result.success(action(run, controller))
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (error: Exception) {
            val safe = safeMessage(error, active?.secret)
            Result.failure(if (safe == error.message && (error is IOException || error is IllegalStateException || error is IllegalArgumentException)) error else IOException(safe))
        }
    }

    private fun activeController(): Pair<Run, MihomoController> {
        val run = active
        check(run != null && run.identity != null && run.alive && !run.stopRequested) { "代理内核未运行" }
        return run to (run.controller ?: throw IllegalStateException("配置中没有可用的本机 external-controller"))
    }

    private suspend fun <T> testing(name: String, block: suspend () -> T): T {
        mutableState.update { it.copy(testing = it.testing + name) }
        try { return block() } finally { mutableState.update { it.copy(testing = it.testing - name) } }
    }

    private suspend fun syncGroups(run: Run, controller: MihomoController): ProxySnapshot {
        val snapshot = controller.proxies()
        if (active === run) {
            lastLive = run.configStamp to snapshot
            mutableState.update { it.copy(groups = snapshot, controllerReachable = true) }
        }
        return snapshot
    }

    private suspend fun syncController(run: Run) {
        val controller = run.controller ?: return
        try {
            val mode = controller.mode()
            val snapshot = controller.proxies()
            if (active === run) {
                lastLive = run.configStamp to snapshot
                mutableState.update { it.copy(controllerReachable = true, mode = ProxyMode.parse(mode), groups = snapshot) }
            }
        } catch (error: Exception) {
            if (error is CancellationException) throw error
            if (active === run) mutableState.update { it.copy(controllerReachable = false) }
            throw error
        }
    }

    private suspend fun awaitController(run: Run) {
        val reached = withTimeoutOrNull(timing.controllerReadyMs) {
            while (true) {
                failIfExited(run)
                if (runCatching { syncController(run) }.isSuccess) break
                delay(timing.pollMs.coerceAtLeast(100))
            }
            true
        } == true
        if (!reached) mutableState.update { it.copy(controllerReachable = false, error = "内核已运行，但本机控制接口未响应；模式与节点暂不可用") }
    }

    private suspend fun verifyTun(run: Run) {
        val explicit = run.tun.explicitDevice
        val device = withTimeoutOrNull(timing.tunReadyMs) {
            var found: String? = null
            while (found == null) {
                failIfExited(run)
                val interfaces = runCatching { ports.networkProbe.interfaces() }.getOrDefault(emptyList())
                found = (if (explicit != null) interfaces.firstOrNull { it.name == explicit && it.up }
                    else interfaces.firstOrNull { it.up && it.name !in run.interfacesBefore && (it.tun == true || it.name.startsWith("Meta") || it.name.startsWith("tun")) })?.name
                if (found == null) delay(timing.pollMs)
            }
            found
        }
        run.device = device
        if (device == null) {
            mutableState.update { it.copy(tun = TunStatus(explicit, false, null), error = "TUN 网卡没有出现：内核仍在运行，但流量未被接管。详情见运行日志") }
            return
        }
        val table = run.tun.effectiveTableIndex.toString()
        val rules = runCatching { ports.rootShell.run("ip rule show && ip -6 rule show", 10_000) }.getOrNull()
        val routed = if (rules == null || rules.code != 0) null
            else !run.tun.autoRoute || TunConflictDetector.parseRules(rules.output).any { it.table == table || it.priority in run.tun.effectiveRuleRange }
        mutableState.update { it.copy(tun = TunStatus(device, true, routed),
            error = if (routed == false) "TUN 网卡已建立，但未发现路由规则；流量可能未被接管" else it.error) }
    }

    private fun startBackgroundJobs(run: Run) {
        val controller = run.controller ?: return
        run.jobs += scope.launch {
            while (isActive && active === run && run.alive && !run.stopRequested) {
                try {
                    controller.traffic().collect { sample -> if (active === run) mutableState.update { it.copy(traffic = sample, controllerReachable = true) } }
                } catch (cancelled: CancellationException) { throw cancelled }
                catch (_: Exception) { if (active === run && !run.stopRequested) mutableState.update { it.copy(traffic = null) } }
                delay(1000)
            }
        }
    }

    private fun failIfExited(run: Run) {
        if (run.childExited || run.startupRejected || !run.process.isAlive) {
            throw IOException(run.diagnostic.ifBlank { "代理内核在启动时退出" }.let { "代理内核已退出：$it" }.take(1000))
        }
    }

    private suspend fun requireRoot() {
        val result = ports.rootShell.run("id -u", timing.rootPromptMs)
        val granted = result.code == 0 && result.output.trim().lineSequence().lastOrNull()?.trim() == "0"
        mutableState.update { it.copy(capabilities = it.capabilities.copy(root = if (granted) RootStatus.GRANTED else RootStatus.DENIED)) }
        check(granted) { "未获得 Root 授权，代理未启动" }
    }

    private fun enterConflict(conflict: ProxyConflict) {
        mutableState.update { it.copy(phase = ProxyPhase.CONFLICT, progress = null, conflict = conflict,
            error = conflict.describe() + if (conflict.hard) "。请先在配置中改用其他网卡名/路由表/规则序号" else "。Workflow 不会接管其路由") }
    }

    private fun detectConflict(tun: TunSettings?, ownDevice: String?): ProxyConflict = runCatching {
        TunConflictDetector.detect(ports.networkProbe.interfaces(), ports.networkProbe.vpnActive(), ownDevice, tun?.explicitDevice)
    }.getOrDefault(ProxyConflict())

    /** Modification time and size of config.yaml; changes whenever the user saves it. */
    private fun configStamp(): String = files.config.let { "${it.lastModified()}:${it.length()}" }

    /** Root-free signs that a kernel from an earlier app process may still run with this config. */
    private fun orphanLikely(inspection: ConfigInspection?): Boolean {
        val endpoint = inspection?.controller?.endpoint
        if (endpoint != null && MihomoControllerClient.acceptsConnections(endpoint)) return true
        val device = inspection?.tun?.takeIf { it.enable }?.explicitDevice ?: return false
        return runCatching { ports.networkProbe.interfaces().any { it.name == device && it.up } }.getOrDefault(false)
    }

    private fun readInspection(): ConfigInspection? {
        if (!files.config.isFile) { inspected = null; mutableState.update { it.copy(config = ProxyConfigState()) }; return null }
        val stamp = configStamp()
        val result = inspected?.takeIf { it.first == stamp }?.second
            ?: runCatching { MihomoConfigInspector.inspect(files.readConfig().toString(Charsets.UTF_8)) }.also { inspected = stamp to it }
        mutableState.update { it.copy(config = ProxyConfigState(true, result.getOrNull(), result.exceptionOrNull()?.let { error ->
            (error as? IllegalArgumentException)?.message ?: (error as? IllegalStateException)?.message ?: "无法读取代理配置" })) }
        return result.getOrNull()
    }

    private suspend fun validate(inspection: Result<ConfigInspection>): ProxyConfigCheck {
        check(kernel.isFile && kernel.canExecute()) { "此设备尚未安装可执行的 Mihomo 内核" }
        files.ensureDirectory()
        val result = ports.kernelTool.validate(kernel, files.directory, files.config)
        val output = if (inspection.isFailure) {
            if (result.code == 0) "内核配置有效；应用无法安全解析此 YAML，控制功能不可用" else "内核配置校验失败；应用无法安全解析此 YAML，详细输出已隐藏"
        } else inspection.getOrThrow().secrets.redact(result.output)
        return ProxyConfigCheck(result.code == 0, output.trim().take(8000).ifBlank { if (result.code == 0) "配置有效" else "内核校验失败（${result.code}）" })
    }

    private fun progress(text: String) = mutableState.update { it.copy(progress = text) }

    private fun supervise(run: Run) {
        val output = scope.launch {
            try {
                runInterruptible {
                    run.process.inputStream.bufferedReader().useLines { lines -> lines.forEach { line -> handle(run, GuardianEvent.parse(line)) } }
                }
            } catch (error: Exception) {
                run.started.completeExceptionally(error)
                run.diagnostic = safeMessage(error, run.secret)
                // A broken control stream is not proof of exit. Closing stdin asks the guardian to stop its own child.
                runCatching { run.input.close() }
            }
        }
        val errors = scope.launch {
            try {
                runInterruptible {
                    run.process.errorStream.bufferedReader().use { reader ->
                        val buffer = CharArray(4096)
                        val redactor = run.secrets.stream()
                        fun capture(chunk: String) {
                            if (chunk.isEmpty()) return
                            run.diagnostic = (run.diagnostic + chunk).takeLast(2048)
                            files.appendLog(chunk)
                        }
                        while (true) {
                            val count = reader.read(buffer)
                            if (count < 0) { capture(redactor.append("", finished = true)); break }
                            capture(if (run.secret == null) "[内核日志已隐藏：无法安全解析控制接口配置]\n" else redactor.append(String(buffer, 0, count)))
                        }
                    }
                }
            } catch (_: IOException) { /* pipe closed during owned-process shutdown */ }
        }
        scope.launch {
            val code = runInterruptible { run.process.waitFor() }
            if (withTimeoutOrNull(1500) { output.join(); true } != true) { output.cancel(); runCatching { run.process.inputStream.close() } }
            if (withTimeoutOrNull(1500) { errors.join(); true } != true) { errors.cancel(); runCatching { run.process.errorStream.close() } }
            val confirmed = run.childExited || run.startupRejected
            run.started.completeExceptionally(IOException(run.diagnostic.ifBlank { "代理守护进程已退出（$code）" }))
            run.jobs.forEach { it.cancel() }
            if (active === run) {
                var recordError: String? = null
                if (confirmed) {
                    recordError = runCatching { files.removeIdentity(run.id) }.exceptionOrNull()?.let { safeMessage(it, run.secret) }
                    active = null
                }
                val unexpected = !run.stopRequested && mutableState.value.phase == ProxyPhase.RUNNING
                mutableState.update {
                    it.copy(stopUnconfirmed = !confirmed, controllerReachable = false, traffic = null, tun = null,
                        pid = if (confirmed) null else run.identity?.pid,
                        phase = if (unexpected || !confirmed) ProxyPhase.ERROR else if (connectionClosed && confirmed) ProxyPhase.STOPPED else it.phase,
                        progress = if (connectionClosed && confirmed) null else it.progress,
                        error = recordError ?: when {
                            run.cleanupFailed -> "内核已结束，但未能确认 TUN 路由清理；请检查网络状态"
                            !confirmed -> "守护通道已退出，内核停止状态尚未确认；请检查设备网络"
                            unexpected -> "代理内核已退出（$code）" + lastLine(run.diagnostic)?.let { line -> "：$line" }.orEmpty()
                            else -> it.error
                        })
                }
            }
            try { ports.lease.release(run.leaseId) } finally { run.exited.complete(code) }
        }
    }

    private fun handle(run: Run, event: GuardianEvent) {
        when (event) {
            is GuardianEvent.Started -> {
                check(run.identity == null && !run.childExited && !run.startupRejected) { "守护进程重复或错序启动记录" }
                val identity = ProxyProcessIdentity.fromGuardian(event.fields, run.id, kernel.canonicalPath, files.directory.path, files.config.path)
                run.identity = identity
                files.persistIdentity(identity)
                run.started.complete(identity)
            }
            is GuardianEvent.Error -> {
                run.diagnostic = event.message
                if (event.message == "Owned TUN policy-rule cleanup could not be confirmed") run.cleanupFailed = true
                if (event.fatal) {
                    run.startupRejected = run.identity == null
                    run.started.completeExceptionally(IOException(event.message))
                }
            }
            is GuardianEvent.Exit -> {
                check(!run.childExited) { "守护进程重复退出记录" }
                val identity = checkNotNull(run.identity) { "守护进程未启动即报告退出" }
                val (_, forced) = verifyGuardianExit(event.fields, identity)
                run.forced = forced
                run.childExited = true
            }
        }
    }

    private suspend fun stopRun(run: Run) {
        run.stopRequested = true
        run.jobs.forEach { it.cancel() }
        if (run.exited.isCompleted) {
            if (!run.childExited && !run.startupRejected) confirmLostRun(run)
            return
        }
        val identity = run.identity
        var writeFailure: Throwable? = null
        try {
            if (identity != null && !run.childExited) synchronized(run.input) {
                // The guardian verifies its own unreaped child; this is never a bare kill of a recorded PID.
                run.input.write(identity.stopCommand()); run.input.flush()
            }
        } catch (error: Exception) {
            writeFailure = error
        } finally {
            // EOF is also an owned-run stop request, including cancellation during the startup handshake.
            runCatching { run.input.close() }
        }
        val completed = withTimeoutOrNull(timing.stopMs) { run.exited.await(); true } == true
        if (!completed) {
            mutableState.update { it.copy(stopUnconfirmed = true) }
            throw IOException("等待守护进程停止超时，内核状态尚未确认", writeFailure)
        }
        if (!run.childExited && !run.startupRejected) confirmLostRun(run)
    }

    /**
     * The guardian channel ended without an exit record (guardian killed, su session lost). Only a read-only
     * root check of the recorded identity can confirm the kernel is gone; nothing is signalled.
     */
    private suspend fun confirmLostRun(run: Run) {
        try {
            requireRoot()
            rejectLivePreviousRecord()
        } catch (error: Exception) {
            mutableState.update { it.copy(stopUnconfirmed = true) }
            throw IOException("守护通道已退出，未收到内核停止确认：${error.message}", error)
        }
        if (active === run) active = null
    }

    /** No signal is sent. A lost guardian channel must not be replaced with a racy PID-based kill. */
    private suspend fun rejectLivePreviousRecord() {
        val previous = files.readIdentity() ?: run {
            if (files.hasRecord()) throw IOException("上次进程记录损坏；请先检查，应用不会按未知 PID 操作")
            return
        }
        val marker = "WORKFLOW_PROXY_EXECUTABLE_BOUNDARY"
        val script = "if [ ! -e /proc/${previous.pid}/stat ]; then exit 3; fi; cat /proc/${previous.pid}/stat || exit 4; printf '\\n$marker\\n'; readlink /proc/${previous.pid}/exe"
        val result = ports.rootShell.run(script, 10_000)
        if (result.code == 3) { files.removeIdentity(previous.runId); return }
        check(result.code == 0) { "无法核对上次代理身份；不会启动重复进程或发送停止信号" }
        val boundary = "\n$marker\n"
        val split = result.output.indexOf(boundary)
        check(split >= 0) { "无法解析上次代理身份；保留进程记录供检查" }
        val stat = result.output.substring(0, split)
        val actualExe = result.output.substring(split + boundary.length).trimEnd('\n', '\r')
        check(ProxyProcessIdentity.startTimeFromStat(stat) != null && actualExe.startsWith('/')) { "无法解析上次代理身份；保留进程记录供检查" }
        if (!previous.matchesObservedProcess(stat, actualExe)) { files.removeIdentity(previous.runId); return }
        throw IOException("上次内核仍存在，但控制通道已断开；请等待其守护进程退出。应用不会仅凭 PID 强制停止进程")
    }

    private suspend fun <T> outcome(action: suspend () -> T): Result<T> = withContext(Dispatchers.IO) {
        try { Result.success(action()) }
        catch (cancelled: CancellationException) { throw cancelled }
        catch (error: Exception) { Result.failure(error) }
    }

    private fun safeMessage(error: Throwable, secret: String?): String =
        (active?.secrets ?: inspected?.second?.getOrNull()?.secrets ?: ProxySecrets.empty()).redact(
            redactSecret(error.message ?: "代理操作失败", secret.orEmpty())).take(2000)

    private fun lastLine(text: String): String? = text.trim().lineSequence().lastOrNull()?.trim()?.takeIf { it.isNotEmpty() }?.take(300)
}
