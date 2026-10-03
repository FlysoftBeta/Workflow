// Shared by :app:client, :agent and :app:proxy (applied after the Kotlin JVM plugin).
// Fails the build if a pure module references Android APIs in source or pulls an Android artifact
// onto its classpath. The module graph already keeps android.jar off the classpath; this check also
// catches androidx/org.json usage that would only surface when the module is consumed by :app.

abstract class VerifyPlatformIndependence : DefaultTask() {
    @get:InputFiles @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val sources: ConfigurableFileCollection

    @get:Input
    abstract val compileRoot: Property<org.gradle.api.artifacts.result.ResolvedComponentResult>

    @get:Input
    abstract val runtimeRoot: Property<org.gradle.api.artifacts.result.ResolvedComponentResult>

    @get:OutputFile
    abstract val report: RegularFileProperty

    @TaskAction
    fun verify() {
        // Not preceded by an identifier character or a dot, so "top.example.android.x" is not matched.
        val forbiddenReference = Regex("""(?<![\w.])(android|androidx|dalvik|org\.json)\.[A-Za-z_]""")
        val forbiddenGroup = Regex("""^(androidx\..*|android\..*|com\.android\..*|com\.google\.android\..*|org\.json)$""")
        val problems = mutableListOf<String>()
        sources.asFileTree.files.sortedBy { it.path }.forEach { file ->
            file.readLines().forEachIndexed { index, line ->
                val code = line.substringBefore("//")
                forbiddenReference.find(code)?.let { problems += "${file.path}:${index + 1}: references ${it.value.dropLast(1)}" }
            }
        }
        val modules = sortedSetOf<String>()
        for (root in listOf(compileRoot.get(), runtimeRoot.get())) {
            val seen = HashSet<org.gradle.api.artifacts.component.ComponentIdentifier>()
            val queue = ArrayDeque(listOf(root))
            while (queue.isNotEmpty()) {
                val component = queue.removeFirst()
                if (!seen.add(component.id)) continue
                (component.id as? org.gradle.api.artifacts.component.ModuleComponentIdentifier)?.let { id ->
                    modules += "${id.group}:${id.module}:${id.version}"
                    if (forbiddenGroup.matches(id.group)) problems += "classpath contains Android artifact ${id.group}:${id.module}"
                }
                component.dependencies.filterIsInstance<org.gradle.api.artifacts.result.ResolvedDependencyResult>().forEach { queue += it.selected }
            }
        }
        report.get().asFile.writeText((listOf("modules:") + modules + listOf("problems:") + problems).joinToString("\n") + "\n")
        if (problems.isNotEmpty()) throw GradleException("Pure JVM module must stay Android-free:\n" + problems.joinToString("\n"))
    }
}

val verifyPlatformIndependence = tasks.register<VerifyPlatformIndependence>("verifyPlatformIndependence") {
    group = "verification"
    description = "Fails if this pure JVM module references android.*, androidx.*, dalvik.* or org.json."
    sources.from(fileTree("src") { include("**/*.kt", "**/*.java", "**/*.kts") })
    compileRoot.set(configurations.named("compileClasspath").flatMap { it.incoming.resolutionResult.rootComponent })
    runtimeRoot.set(configurations.named("runtimeClasspath").flatMap { it.incoming.resolutionResult.rootComponent })
    report.set(layout.buildDirectory.file("reports/platform-independence.txt"))
}
tasks.named("check") { dependsOn(verifyPlatformIndependence) }
tasks.named("test") { dependsOn(verifyPlatformIndependence) }
