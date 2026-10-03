package top.flysoftbeta.workflow.core.environment

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import top.flysoftbeta.workflow.core.json.Json
import top.flysoftbeta.workflow.core.json.jsonObject
import java.io.File

/**
 * Frozen reference port for the former Kotlin environment writer. Every call is
 * cancellable: cancelling [run] must kill the guest process group before returning.
 */
interface EnvironmentEngine {
    /**
     * Installs the bundled image into [target] (which must not exist), verifying its sha256 (§2.5):
     * the rootfs goes to `target/rootfs` plus the engine's attribute store; rows below a
     * `metadata.stores` prefix go to `target/seeds/<store>/` as plain files.
     */
    suspend fun install(image: ImageInfo, target: File, log: (String) -> Unit)

    /** Copies a committed generation (rootfs and attribute store) into [target]. */
    suspend fun clone(source: File, target: File, log: (String) -> Unit)

    /**
     * Runs one build step with [root] as the guest generation and the toolchain store bound at
     * `/opt/toolchains`; network allowed, no `/workspace`, no persistent home. Stdout goes to
     * [stdout], stderr to [log]. Returns the exit status.
     */
    suspend fun run(root: File, command: GuestCommand, environment: Map<String, String>, log: (String) -> Unit, stdout: StringBuilder): Int
}

sealed interface DesiredConfig {
    data class Valid(val spec: ContainerSpec) : DesiredConfig
    /** container.json is invalid; reconcile keeps using [lastValid] (or the defaults) and reports the problem. */
    data class Invalid(val error: ContainerSpecException, val lastValid: ContainerSpec?) : DesiredConfig
}

/**
 * Reconcile loop (docs/engine/environment.md §5). Feed desired states with `collectLatest { reconcile(it) }` so
 * that a newer container.json cancels an in-flight build; call [start] once on a background dispatcher.
 * Instance lifecycle: [prepareStart] before starting an environment instance, [instanceStarted] once it
 * runs, [instanceStopped] when its last process exits.
 */
class EnvironmentReconciler(
    private val store: EngineStore,
    private val engine: EnvironmentEngine,
    private val image: ImageInfo,
) {
    private val variant = requireNotNull(ImageVariants.find(image.type, image.typeVersion)) { "Unsupported image variant" }
    private val mutex = Mutex()
    private val mutableStatus = MutableStateFlow<EnvironmentStatus>(EnvironmentStatus.Applying(Stage.INSTALL, 0, 0))
    val status: StateFlow<EnvironmentStatus> = mutableStatus.asStateFlow()

    @Volatile private var current: Activation? = null
    @Volatile private var running: Activation? = null
    @Volatile private var failure: FailureRecord? = null
    @Volatile private var configProblem: ContainerSpecException? = null
    @Volatile private var applying: EnvironmentStatus.Applying? = null

    /** The activation new environment instances start from. */
    val activation: Activation? get() = current

    /** Startup: recovery (§5.4) and cached state. Blocking IO. */
    suspend fun start(): EngineStore.Recovery = mutex.withLock {
        val recovery = store.recover()
        current = store.current()?.activation
        failure = store.failure()
        refresh()
        recovery
    }

    /** Points `/opt/toolchains/active` at the current profile; returns what the new instance must use. */
    fun prepareStart(): Activation? = store.current()?.activation?.also {
        store.switchActive(it.profile)
        current = it
    }

    /** An instance started from [activation] is running. A successful restart releases the old rootfs. */
    suspend fun instanceStarted(activation: Activation) {
        running = activation
        store.startedCurrent(activation)
        // Garbage collection must not race a build that has installed toolchains it has not referenced yet.
        if (mutex.tryLock()) {
            try { store.collectGarbage(activation) } finally { mutex.unlock() }
        }
        refresh()
    }

    fun instanceStopped() {
        running = null
        refresh()
    }

    suspend fun reconcile(desired: DesiredConfig, retry: Boolean = false): Unit = mutex.withLock {
        val spec = when (desired) {
            is DesiredConfig.Valid -> desired.spec.also { configProblem = null }
            is DesiredConfig.Invalid -> (desired.lastValid ?: ContainerSpec()).also { configProblem = desired.error }
        }
        val resolved = spec.resolve(image.defaults)
        val environment = store.current()
        when (val outcome = ReconcilePlanner.plan(resolved, image, environment)) {
            PlanOutcome.UpToDate -> clearFailure()
            is PlanOutcome.ActivateOnly -> {
                current = store.activate(environment!!.generation.id, environment.activation.profile, outcome.config,
                    ActivationReason.CONFIG, running)
                store.collectGarbage(running)
                clearFailure()
            }
            is PlanOutcome.Build -> {
                val known = failure
                if (retry || known == null || known.fingerprint != resolved.fingerprint || known.imageSha256 != image.sha256) {
                    build(outcome)
                }
            }
        }
        refresh()
    }

    /** Activates the previous activation. Returns the spec to write back to container.json. */
    suspend fun rollback(): ContainerSpec? = mutex.withLock {
        val activation = store.rollback(running) ?: return@withLock null
        current = activation
        store.collectGarbage(running)
        clearFailure()
        refresh()
        activation.config.toSpec()
    }

    private suspend fun build(plan: PlanOutcome.Build) {
        val newGeneration = when (val base = plan.base) {
            BuildBase.Image, is BuildBase.Clone -> store.allocateGeneration()
            is BuildBase.InPlace -> null
        }
        val root = when (val base = plan.base) {
            is BuildBase.InPlace -> store.generationDirectory(base.generation)
            else -> store.partialDirectory(newGeneration!!)
        }
        val logFile = store.logFile(newGeneration?.toString() ?: "p${System.currentTimeMillis()}")
        check(logFile.parentFile!!.mkdirs() || logFile.parentFile!!.isDirectory)
        val total = plan.steps.size + 2
        var stage = when (plan.base) {
            BuildBase.Image -> Stage.INSTALL
            is BuildBase.Clone -> Stage.CLONE
            is BuildBase.InPlace -> plan.steps.first().stage
        }
        logFile.bufferedWriter().use { writer ->
            val log: (String) -> Unit = { line -> writer.appendLine(line); writer.flush() }
            try {
                progress(stage, 1, total)
                log("# ${plan.reason.wire}: ${plan.base}, config ${plan.config.fingerprint}")
                when (val base = plan.base) {
                    BuildBase.Image -> {
                        engine.install(image, root, log)
                        store.mergeSeeds(root, image.stores)
                        store.recordManaged(image.profile.python, image.profile.node)
                    }
                    is BuildBase.Clone -> engine.clone(store.generationDirectory(base.generation), root, log)
                    is BuildBase.InPlace -> Unit
                }
                var verified: Verified? = null
                for ((index, step) in plan.steps.withIndex()) {
                    stage = step.stage
                    progress(stage, index + 2, total)
                    val command = variant.command(step)
                    log("$ " + command.argv.joinToString(" "))
                    val stdout = StringBuilder()
                    val status = engine.run(root, command, image.environment + plan.config.env, log, stdout)
                    if (status != 0) throw BuildFailure(stage, "${command.argv.drop(1).firstOrNull()} exited with status $status")
                    val output = if (step is Step.InstallPython || step is Step.InstallNode || step is Step.Verify) {
                        runCatching { Json.parse(stdout.toString().trim()).jsonObject() }
                            .getOrElse { throw BuildFailure(stage, "unreadable output of ${command.argv[1]}") }
                    } else null
                    when (step) {
                        is Step.InstallPython -> if (output!!["installed"] == true) store.recordManaged(python = output["python"] as? String)
                        is Step.InstallNode -> if (output!!["installed"] == true) store.recordManaged(node = output["node"] as? String)
                        is Step.Verify -> verified = try {
                            ReconcilePlanner.verified(output!!, plan.config)
                        } catch (error: IllegalArgumentException) {
                            throw BuildFailure(Stage.VERIFY, error.message ?: "verification failed")
                        }
                        else -> Unit
                    }
                }
                val result = verified ?: throw BuildFailure(Stage.VERIFY, "no verification step")
                stage = Stage.ACTIVATE
                progress(stage, total, total)
                val generation = when (val base = plan.base) {
                    is BuildBase.InPlace -> base.generation
                    else -> {
                        val parent = (base as? BuildBase.Clone)?.generation
                        store.writeGeneration(GenerationRecord(newGeneration!!, parent, store.now(), image.ref, result.packages))
                        store.commitGeneration(newGeneration)
                        newGeneration
                    }
                }
                current = store.activate(generation, result.profile, plan.config, plan.reason, running)
                clearFailure()
                store.collectGarbage(running)
                log("# activated generation $generation with ${result.profile.name}")
            } catch (cancelled: CancellationException) {
                withContext(NonCancellable) { newGeneration?.let(store::discardPartial) }
                throw cancelled
            } catch (error: Exception) {
                val message = error.message ?: error.javaClass.simpleName
                val failedStage = (error as? BuildFailure)?.stage ?: stage
                runCatching { log("# failed at ${failedStage.wire}: $message") }
                newGeneration?.let(store::discardPartial)
                FailureRecord(plan.config.fingerprint, image.sha256, failedStage, message, logFile.path).also {
                    failure = it
                    store.recordFailure(it)
                }
            } finally {
                applying = null
            }
        }
    }

    private fun clearFailure() {
        if (failure != null) store.clearFailure()
        failure = null
    }

    private fun progress(stage: Stage, step: Int, steps: Int) {
        applying = EnvironmentStatus.Applying(stage, step, steps)
        refresh()
    }

    private fun refresh() {
        val active = current
        val live = running
        val failed = failure
        val problem = configProblem
        mutableStatus.value = applying ?: when {
            problem != null -> EnvironmentStatus.Failed(Stage.CONFIG, problem.problems.joinToString("\n"), null, active != null)
            failed != null -> EnvironmentStatus.Failed(failed.stage, failed.message, failed.log, active != null)
            active == null -> EnvironmentStatus.Applying(Stage.INSTALL, 0, 0)
            live != null && live.id != active.id -> EnvironmentStatus.PendingRestart(active.reason)
            else -> EnvironmentStatus.UpToDate
        }
    }

    private class BuildFailure(val stage: Stage, message: String) : Exception(message)
}
