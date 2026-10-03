package top.flysoftbeta.workflow.core.environment

import top.flysoftbeta.workflow.core.json.Json
import top.flysoftbeta.workflow.core.json.jsonObject
import top.flysoftbeta.workflow.core.json.list
import java.io.File
import java.io.FileOutputStream
import java.nio.channels.FileChannel
import java.nio.file.Files
import java.nio.file.LinkOption
import java.nio.file.Path
import java.nio.file.Paths
import java.nio.file.StandardCopyOption
import java.nio.file.StandardOpenOption
import java.time.Instant
import java.util.UUID

/**
 * Reconcile-owned state under `.workspace/.workflow/engine/` (docs/engine/environment.md §4, §5.4):
 * activation pointers, generation records, the toolchain store's profiles and managed versions,
 * store seeding, failure memory and garbage collection. Inside a generation only `generation.json`
 * belongs to reconcile; everything else is the engine's. Blocking IO: call off the main thread.
 * Methods are synchronized, so the runtime may switch `active` while a build runs.
 */
class EngineStore(val root: File, private val clock: () -> Instant = Instant::now) {
    val generationsDirectory = File(root, "generations")
    val logsDirectory = File(root, "logs")
    val temporaryDirectory = File(root, "tmp")
    val homeDirectory = File(root, "home/work")
    val toolchainsDirectory = File(root, "toolchains")
    val profilesDirectory = File(toolchainsDirectory, "profiles")
    private val activeLink = File(toolchainsDirectory, "active")
    private val currentFile = File(root, "current.json")
    private val previousFile = File(root, "previous.json")
    private val reconcileFile = File(root, "reconcile.json")
    private val managedFile = File(root, "toolchains.json")

    fun generationDirectory(id: Long) = File(generationsDirectory, id.toString())
    fun partialDirectory(id: Long) = File(generationsDirectory, "$id.partial")
    fun logFile(id: String) = File(logsDirectory, "build-$id.log")
    fun storeDirectory(store: String) = File(root, store)
    fun now(): String = clock().toString()

    @Synchronized fun current(): ActiveEnvironment? = environment(currentFile)
    @Synchronized fun previous(): ActiveEnvironment? = environment(previousFile)

    @Synchronized fun generation(id: Long): GenerationRecord? = runCatching {
        GenerationRecord.fromJson(Json.parse(File(generationDirectory(id), "generation.json").readText()).jsonObject())
            .takeIf { it.id == id }
    }.getOrNull()

    private fun environment(file: File): ActiveEnvironment? {
        val activation = runCatching { Activation.fromJson(Json.parse(file.readText()).jsonObject()) }.getOrNull() ?: return null
        if (!File(profilesDirectory, activation.profile.name).isDirectory) return null
        return generation(activation.generation)?.let { ActiveEnvironment(activation, it) }
    }

    /** Next generation number; never reuses a number that still has a directory or a reference. */
    @Synchronized fun allocateGeneration(): Long {
        check(generationsDirectory.mkdirs() || generationsDirectory.isDirectory)
        val used = generationsDirectory.list().orEmpty().mapNotNull { it.removeSuffix(".partial").toLongOrNull() } +
            listOfNotNull(current()?.activation?.generation, previous()?.activation?.generation)
        return (used.maxOrNull() ?: 0) + 1
    }

    @Synchronized fun writeGeneration(record: GenerationRecord) {
        val directory = partialDirectory(record.id)
        check(directory.isDirectory) { "No partial generation ${record.id}" }
        atomicWrite(File(directory, "generation.json"), Json.stringify(record.toJson()))
    }

    /** `<n>.partial` → `<n>`, then the parent directory is synced. */
    @Synchronized fun commitGeneration(id: Long) {
        check(!File(partialDirectory(id), SEEDS).exists()) { "Seeds of generation $id were not merged" }
        Files.move(partialDirectory(id).toPath(), generationDirectory(id).toPath(), StandardCopyOption.ATOMIC_MOVE)
        syncDirectory(generationsDirectory)
    }

    @Synchronized fun discardPartial(id: Long) = deleteTree(partialDirectory(id))

    /**
     * Moves `<partial>/seeds/<store>` into the persistent stores (§2.5): `if-absent` only when the store
     * does not exist yet, `merge` adds entries the store lacks (existing entries win). Renames only.
     */
    @Synchronized fun mergeSeeds(partial: File, stores: List<StoreSeed>) {
        val seeds = File(partial, SEEDS)
        for (store in stores) {
            val source = File(seeds, store.store)
            if (!exists(source)) continue
            val target = storeDirectory(store.store)
            when (store.policy) {
                SeedPolicy.IF_ABSENT -> if (!exists(target)) move(source, target)
                SeedPolicy.MERGE -> merge(source, target)
            }
        }
        deleteTree(seeds)
    }

    private fun merge(source: File, target: File) {
        if (!exists(target)) {
            move(source, target)
        } else if (isRealDirectory(source) && isRealDirectory(target)) {
            source.list().orEmpty().sorted().forEach { merge(File(source, it), File(target, it)) }
        }
    }

    /** Toolchain versions installed by reconcile (or seeded by an image); only these are ever removed. */
    @Synchronized fun managedToolchains(): Pair<Set<String>, Set<String>> = runCatching {
        val value = Json.parse(managedFile.readText()).jsonObject()
        value.list("python").map { it as String }.toSet() to value.list("node").map { it as String }.toSet()
    }.getOrDefault(emptySet<String>() to emptySet())

    @Synchronized fun recordManaged(python: String? = null, node: String? = null) {
        val (pythons, nodes) = managedToolchains()
        val updated = (pythons + listOfNotNull(python)) to (nodes + listOfNotNull(node))
        if (updated != pythons to nodes) writeManaged(updated.first, updated.second)
    }

    private fun writeManaged(pythons: Set<String>, nodes: Set<String>) =
        atomicWrite(managedFile, Json.stringify(linkedMapOf("python" to pythons.sorted(), "node" to nodes.sorted())))

    /** Points `/opt/toolchains/active` at [profile] (atomic rename). Call only before an instance starts. */
    @Synchronized fun switchActive(profile: ToolchainProfile) {
        check(File(profilesDirectory, profile.name).isDirectory) { "Profile ${profile.name} does not exist" }
        val temporary = File(toolchainsDirectory, ".active.${UUID.randomUUID()}").toPath()
        Files.createSymbolicLink(temporary, Paths.get("profiles", profile.name))
        try {
            Files.move(temporary, activeLink.toPath(), StandardCopyOption.ATOMIC_MOVE, StandardCopyOption.REPLACE_EXISTING)
            syncDirectory(toolchainsDirectory)
        } finally {
            Files.deleteIfExists(temporary)
        }
    }

    @Synchronized fun activeProfile(): String? = runCatching { Files.readSymbolicLink(activeLink.toPath()).fileName.toString() }.getOrNull()

    /**
     * Writes a new activation. `previous` becomes what the user can fall back to: the running
     * instance's activation when it differs from the old current (an intermediate activation that
     * never ran is dropped), otherwise the old current. So at most one previous generation is kept.
     */
    @Synchronized fun activate(generation: Long, profile: ToolchainProfile, config: ResolvedSpec, reason: ActivationReason,
                               running: Activation? = null): Activation {
        requireNotNull(generation(generation)) { "Generation $generation is not committed" }
        check(File(profilesDirectory, profile.name).isDirectory) { "Profile ${profile.name} does not exist" }
        val old = current()?.activation
        val fallback = running?.takeIf { it.id != old?.id && generation(it.generation) != null } ?: old
        val id = maxOf(old?.id ?: 0, previous()?.activation?.id ?: 0, running?.id ?: 0) + 1
        val activation = Activation(id, generation, profile, config, reason, now())
        if (fallback != null) atomicWrite(previousFile, Json.stringify(fallback.toJson())) else previousFile.delete()
        atomicWrite(currentFile, Json.stringify(activation.toJson()))
        return activation
    }

    /** Re-activates the previous activation; the old current becomes previous. */
    @Synchronized fun rollback(running: Activation? = null): Activation? {
        val target = previous()?.activation ?: return null
        return activate(target.generation, target.profile, target.config, ActivationReason.ROLLBACK, running)
    }

    /**
     * An instance started from the current activation: the restart succeeded, so a previous
     * activation on another rootfs generation is released (its generation is pruned by [collectGarbage]).
     * Toolchain-only fallbacks stay: rolling them back costs nothing.
     */
    @Synchronized fun startedCurrent(running: Activation) {
        val current = current()?.activation ?: return
        if (running.id != current.id) return
        val previous = previous()?.activation ?: return
        if (previous.generation != current.generation) previousFile.delete()
    }

    @Synchronized fun failure(): FailureRecord? = runCatching {
        (Json.parse(reconcileFile.readText()).jsonObject()["failure"] as? Map<*, *>)?.let {
            @Suppress("UNCHECKED_CAST")
            FailureRecord.fromJson(it as Map<String, Any?>)
        }
    }.getOrNull()

    @Synchronized fun recordFailure(failure: FailureRecord) = atomicWrite(reconcileFile, Json.stringify(mapOf("failure" to failure.toJson())))

    @Synchronized fun clearFailure() {
        if (reconcileFile.exists()) atomicWrite(reconcileFile, Json.stringify(mapOf("failure" to null)))
    }

    data class Recovery(val removedPartials: List<String>, val restoredFromPrevious: Boolean, val collected: Collected)

    /** Startup recovery (§5.4); call before any environment instance starts. */
    @Synchronized fun recover(): Recovery {
        val partials = generationsDirectory.listFiles().orEmpty().filter { it.name.endsWith(".partial") }
        partials.forEach(::deleteTree)
        temporaryDirectory.listFiles().orEmpty().forEach(::deleteTree)
        deleteTree(File(toolchainsDirectory, ".tmp"))
        toolchainsDirectory.listFiles().orEmpty().filter { it.name.startsWith(".active.") }.forEach { it.delete() }
        profilesDirectory.listFiles().orEmpty().filter { it.name.startsWith(".new.") }.forEach(::deleteTree)
        var restored = false
        if (current() == null) {
            val previous = previous()
            if (previous != null) {
                atomicWrite(currentFile, Json.stringify(previous.activation.toJson()))
                restored = true
            } else currentFile.delete()
            previousFile.delete()
        } else if (previous() == null) previousFile.delete()
        return Recovery(partials.map { it.name }, restored, collectGarbage(null))
    }

    data class Collected(val generations: List<Long>, val profiles: List<String>, val pythons: List<String>, val nodes: List<String>)

    /**
     * Deletes what neither current, previous, [running] nor `active` needs: rootfs generations, toolchain
     * profiles, and managed Python/Node versions no kept profile references. Unmanaged versions (installed
     * by the user) are never touched. Also prunes old build logs. Must not run while a build is in flight.
     */
    @Synchronized fun collectGarbage(running: Activation?): Collected {
        val kept = listOfNotNull(current()?.activation, previous()?.activation, running)
        val generations = kept.map { it.generation }.toSet()
        val removedGenerations = generationsDirectory.listFiles().orEmpty()
            .mapNotNull { file -> file.name.toLongOrNull()?.takeIf { it !in generations }?.also { deleteTree(file) } }
        val profiles = kept.map { it.profile.name }.toSet() + listOfNotNull(activeProfile())
        val removedProfiles = profilesDirectory.listFiles().orEmpty()
            .filter { ToolchainProfile.isName(it.name) && it.name !in profiles }
            .onEach(::deleteTree).map { it.name }
        val referenced = profiles.filter(ToolchainProfile::isName).map(ToolchainProfile::parse)
        val (pythons, nodes) = managedToolchains()
        val removedPythons = pythons.filter { version -> referenced.none { it.python == version } }
        val removedNodes = nodes.filter { version -> referenced.none { it.node == version } }
        val pythonDirectory = File(toolchainsDirectory, "uv/python")
        removedPythons.forEach { version ->
            pythonDirectory.listFiles().orEmpty()
                .filter { it.name.startsWith("cpython-$version-") && isRealDirectory(it) }.forEach(::deleteTree)
        }
        if (removedPythons.isNotEmpty()) {
            // uv keeps minor-version links (cpython-3.13-…); drop those left dangling.
            pythonDirectory.listFiles().orEmpty().filter { Files.isSymbolicLink(it.toPath()) && !it.exists() }.forEach { it.delete() }
        }
        removedNodes.forEach { deleteTree(File(toolchainsDirectory, "nvm/versions/node/v$it")) }
        if (removedPythons.isNotEmpty() || removedNodes.isNotEmpty()) writeManaged(pythons - removedPythons.toSet(), nodes - removedNodes.toSet())
        logsDirectory.listFiles().orEmpty().filter { it.name.startsWith("build-") }
            .sortedByDescending { it.lastModified() }.drop(KEEP_LOGS).forEach { it.delete() }
        return Collected(removedGenerations.sorted(), removedProfiles.sorted(), removedPythons.sorted(), removedNodes.sorted())
    }

    fun logTail(file: File, lines: Int = 20): String =
        if (!file.exists()) "" else file.readLines().takeLast(lines).joinToString("\n")

    private fun exists(file: File) = Files.exists(file.toPath(), LinkOption.NOFOLLOW_LINKS)
    private fun isRealDirectory(file: File) = Files.isDirectory(file.toPath(), LinkOption.NOFOLLOW_LINKS)

    private fun move(source: File, target: File) {
        check(target.parentFile!!.mkdirs() || target.parentFile!!.isDirectory)
        Files.move(source.toPath(), target.toPath(), StandardCopyOption.ATOMIC_MOVE)
    }

    /** Deletes a tree without following symlinks; read-only directories are made writable first. */
    private fun deleteTree(file: File) {
        val path: Path = file.toPath()
        if (!Files.exists(path, LinkOption.NOFOLLOW_LINKS)) return
        if (Files.isDirectory(path, LinkOption.NOFOLLOW_LINKS)) {
            file.setWritable(true, true)
            file.setExecutable(true, true)
            file.listFiles().orEmpty().forEach(::deleteTree)
        }
        Files.delete(path)
    }

    private fun atomicWrite(file: File, text: String) {
        check(file.parentFile!!.mkdirs() || file.parentFile!!.isDirectory)
        val temporary = File(file.parentFile, ".${file.name}.${UUID.randomUUID()}.tmp")
        try {
            FileOutputStream(temporary).use { output -> output.write(text.toByteArray(Charsets.UTF_8)); output.fd.sync() }
            Files.move(temporary.toPath(), file.toPath(), StandardCopyOption.ATOMIC_MOVE, StandardCopyOption.REPLACE_EXISTING)
            syncDirectory(file.parentFile!!)
        } finally {
            temporary.delete()
        }
    }

    private fun syncDirectory(directory: File) {
        runCatching { FileChannel.open(directory.toPath(), StandardOpenOption.READ).use { it.force(true) } }
    }

    companion object {
        const val SEEDS = "seeds"
        private const val KEEP_LOGS = 10
    }
}
