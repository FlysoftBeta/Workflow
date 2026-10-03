import java.io.File
import java.io.InputStream
import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.security.MessageDigest
import java.time.Duration
import java.util.zip.GZIPInputStream
import javax.inject.Inject
import org.gradle.process.ExecOperations
import org.gradle.work.DisableCachingByDefault

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

val repositoryRoot = rootProject.layout.projectDirectory
/** One directory per prebuilt: manifest.json (version, upstream URL, sha256 per ABI) and LICENSE. */
val prebuiltPackages = listOf("mihomo").map { repositoryRoot.dir("third_party/$it") }

abstract class BuildProxyGuardTask : DefaultTask() {
    @get:InputFile @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val script: RegularFileProperty
    @get:InputFile @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val source: RegularFileProperty
    @get:Input abstract val toolchainVersion: Property<String>
    @get:Internal abstract val ndkDirectory: DirectoryProperty
    @get:OutputDirectory abstract val outputDirectory: DirectoryProperty
    @get:Inject abstract val execOperations: ExecOperations

    @TaskAction fun compileGuard() {
        execOperations.exec {
            commandLine("bash", script.get().asFile.absolutePath, "--ndk",
                ndkDirectory.get().asFile.absolutePath, "--output-dir", outputDirectory.get().asFile.absolutePath)
        }
    }
}

/**
 * Makes each pinned prebuilt executable available as a generated jniLibs directory. Binaries live in
 * third_party/.cache/<name>/<version>/<abi>/ (git-ignored). A missing binary is downloaded from the
 * manifest's pinned upstream URL; archive and binary SHA-256 must both match or the build fails.
 * A cached file with a wrong digest is never replaced silently.
 */
@DisableCachingByDefault(because = "Links large pinned binaries from the local prebuilt cache")
abstract class FetchPrebuiltsTask : DefaultTask() {
    @get:InputFiles @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val manifests: ConfigurableFileCollection
    @get:Input abstract val abis: ListProperty<String>
    @get:Internal abstract val cacheDirectory: DirectoryProperty
    @get:OutputDirectory abstract val outputDirectory: DirectoryProperty

    @TaskAction fun fetch() {
        val output = outputDirectory.get().asFile
        output.deleteRecursively()
        for (manifestFile in manifests.files.sortedBy { it.path }) {
            val manifest = groovy.json.JsonSlurper().parse(manifestFile) as Map<*, *>
            val name = safeName(manifest["name"], manifestFile)
            val version = safeName(manifest["version"], manifestFile)
            val packagedAs = safeName(manifest["packagedAs"], manifestFile)
            val entries = manifest["abis"] as? Map<*, *> ?: throw GradleException("$manifestFile: missing abis")
            for (abi in abis.get()) {
                val entry = entries[abi] as? Map<*, *> ?: throw GradleException("$name $version has no pinned binary for $abi")
                val cached = cacheDirectory.get().asFile.resolve("$name/$version/$abi/$packagedAs")
                ensureCached("$name $version $abi", entry, cached)
                val target = output.resolve("$abi/$packagedAs")
                target.parentFile.mkdirs()
                // A hard link avoids another few hundred MB per build; fall back to a copy across filesystems.
                try { Files.createLink(target.toPath(), cached.toPath()) }
                catch (_: Exception) { Files.copy(cached.toPath(), target.toPath(), StandardCopyOption.REPLACE_EXISTING) }
            }
        }
    }

    private fun ensureCached(label: String, entry: Map<*, *>, cached: File) {
        val sha256 = entry["sha256"] as String
        val bytes = (entry["bytes"] as Number).toLong()
        if (cached.isFile) {
            if (cached.length() == bytes && digest(cached) == sha256) return
            throw GradleException("$label: cached ${cached.path} does not match the pinned SHA-256. Inspect and remove it to re-fetch.")
        }
        val url = entry["url"] as String
        require(url.startsWith("https://")) { "$label: pinned URL must use HTTPS" }
        val archive = cached.parentFile.resolve(url.substringAfterLast('/'))
        val archiveSha256 = entry["archiveSha256"] as String
        cached.parentFile.mkdirs()
        if (!archive.isFile) download(label, url, archive, archiveSha256)
        else check(digest(archive) == archiveSha256) { "$label: cached ${archive.path} does not match the pinned archive SHA-256" }
        val partial = File(cached.parentFile, ".${cached.name}.part")
        try {
            archive.inputStream().buffered().use { raw ->
                when (entry["archive"]) {
                    "gz" -> GZIPInputStream(raw).use { copyTo(it, partial) }
                    "tar.gz" -> GZIPInputStream(raw).use { copyTarMember(it, entry["member"] as String, partial) }
                    else -> throw GradleException("$label: unsupported archive type ${entry["archive"]}")
                }
            }
            check(partial.length() == bytes && digest(partial) == sha256) { "$label: extracted binary does not match the pinned SHA-256" }
            partial.setExecutable(true, false)
            Files.move(partial.toPath(), cached.toPath(), StandardCopyOption.ATOMIC_MOVE)
        } finally { partial.delete() }
    }

    private fun download(label: String, url: String, archive: File, sha256: String) {
        logger.lifecycle("Downloading $label from $url")
        val partial = File(archive.parentFile, ".${archive.name}.part")
        try {
            val client = HttpClient.newBuilder().followRedirects(HttpClient.Redirect.NORMAL).connectTimeout(Duration.ofSeconds(30)).build()
            val request = HttpRequest.newBuilder(URI(url)).timeout(Duration.ofMinutes(30)).GET().build()
            val response = client.send(request, HttpResponse.BodyHandlers.ofFile(partial.toPath()))
            check(response.statusCode() == 200) { "$label: download failed with HTTP ${response.statusCode()}" }
            check(digest(partial) == sha256) { "$label: downloaded archive does not match the pinned SHA-256" }
            Files.move(partial.toPath(), archive.toPath(), StandardCopyOption.ATOMIC_MOVE)
        } finally { partial.delete() }
    }

    /** Minimal ustar/GNU reader: the pinned archives contain one regular file member. */
    private fun copyTarMember(input: InputStream, member: String, target: File) {
        val header = ByteArray(512)
        var longName: String? = null
        while (true) {
            if (input.readNBytes(header, 0, 512) != 512) break
            if (header.all { it.toInt() == 0 }) break
            fun field(offset: Int, length: Int) = String(header, offset, length, Charsets.UTF_8).substringBefore('\u0000')
            val size = field(124, 12).trim().ifEmpty { "0" }.toLong(8)
            val type = header[156].toInt().toChar()
            val prefix = if (field(257, 6).startsWith("ustar")) field(345, 155) else ""
            val name = longName ?: (if (prefix.isEmpty()) field(0, 100) else "$prefix/${field(0, 100)}")
            longName = null
            val padded = (size + 511) / 512 * 512
            when {
                type == 'L' -> { longName = String(input.readNBytes(size.toInt()), Charsets.UTF_8).substringBefore('\u0000'); input.skipNBytes(padded - size) }
                (type == '0' || type == '\u0000') && name.removePrefix("./") == member -> {
                    target.outputStream().use { output ->
                        val buffer = ByteArray(1 shl 16)
                        var remaining = size
                        while (remaining > 0) {
                            val count = input.read(buffer, 0, minOf(buffer.size.toLong(), remaining).toInt())
                            check(count > 0) { "Truncated tar member $member" }
                            output.write(buffer, 0, count); remaining -= count
                        }
                    }
                    return
                }
                else -> input.skipNBytes(padded)
            }
        }
        throw GradleException("Archive does not contain $member")
    }

    private fun copyTo(input: InputStream, target: File) = target.outputStream().use { input.copyTo(it, 1 shl 16) }

    private fun safeName(value: Any?, manifest: File): String = (value as? String)?.takeIf { it.matches(Regex("[A-Za-z0-9._-]+")) }
        ?: throw GradleException("$manifest: invalid name/version/packagedAs")

    private fun digest(file: File): String {
        val sha = MessageDigest.getInstance("SHA-256")
        file.inputStream().use { input ->
            val buffer = ByteArray(1 shl 16)
            while (true) { val count = input.read(buffer); if (count < 0) break; sha.update(buffer, 0, count) }
        }
        return sha.digest().joinToString("") { "%02x".format(it.toInt() and 255) }
    }
}

/** Ships each prebuilt's manifest and upstream license as assets/notices/<name>-{manifest.json,LICENSE}. */
abstract class PrebuiltNoticesTask : DefaultTask() {
    @get:InputFiles @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val packages: ConfigurableFileCollection
    @get:OutputDirectory abstract val outputDirectory: DirectoryProperty

    @TaskAction fun copy() {
        val notices = outputDirectory.get().asFile.resolve("notices")
        notices.deleteRecursively(); notices.mkdirs()
        for (directory in packages.files.sortedBy { it.name }) {
            for ((source, suffix) in listOf("manifest.json" to "manifest.json", "LICENSE" to "LICENSE")) {
                val file = directory.resolve(source)
                check(file.isFile) { "Missing ${file.path}" }
                file.copyTo(notices.resolve("${directory.name}-$suffix"))
            }
        }
    }
}

/**
 * Cross-builds the Rust server, runtime and loader with engine/build-android.sh as jniLibs (docs/engine.md §2: everything the app executes lives in
 * nativeLibraryDir; useLegacyPackaging extracts them there). The script's test helpers are not packaged.
 */
abstract class BuildEngineTask : DefaultTask() {
    @get:InputFiles @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val sources: ConfigurableFileCollection
    @get:Input abstract val abis: ListProperty<String>
    @get:Input abstract val toolchainVersion: Property<String>
    @get:Internal abstract val script: RegularFileProperty
    @get:Internal abstract val ndkDirectory: DirectoryProperty
    @get:Internal abstract val workDirectory: DirectoryProperty
    @get:OutputDirectory abstract val outputDirectory: DirectoryProperty
    @get:Inject abstract val execOperations: ExecOperations

    @TaskAction fun build() {
        val work = workDirectory.get().asFile
        work.deleteRecursively()
        work.mkdirs()
        execOperations.exec {
            commandLine(listOf("bash", script.get().asFile.absolutePath) + abis.get())
            environment("ANDROID_NDK", ndkDirectory.get().asFile.absolutePath)
            environment("ENGINE_ANDROID_OUT", work.absolutePath)
            environment("WORKFLOW_BUILD_LOCK_HELD", "1")
        }
        val output = outputDirectory.get().asFile
        output.deleteRecursively()
        for (abi in abis.get()) {
            for (name in listOf("libworkflow-engine.so", "libworkflow-runtime.so", "libworkflow-loader.so")) {
                val from = work.resolve("$abi/$name")
                if (!from.isFile) throw GradleException("The engine build produced no $abi/$name")
                val to = output.resolve("$abi/$name")
                to.parentFile.mkdirs()
                from.copyTo(to, overwrite = true)
                to.setExecutable(true, false)
            }
        }
    }
}

/** Verifies and packages the pre-provisioned custom image for each APK architecture. */
@DisableCachingByDefault(because = "Links large image files from artifacts/")
abstract class PrepareEnvironmentImageTask : DefaultTask() {
    @get:Input abstract val selection: Property<String>
    @get:Input abstract val architecture: Property<String>
    @get:InputFiles @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val candidates: ConfigurableFileCollection
    @get:Internal abstract val imageDirectory: DirectoryProperty
    @get:OutputDirectory abstract val outputDirectory: DirectoryProperty

    @TaskAction fun prepare() {
        val output = outputDirectory.get().asFile
        output.deleteRecursively()
        val environment = output.resolve("environment")
        val directory = imageDirectory.get().asFile
        val arch = architecture.get()
        check(selection.get() in setOf("workspace", "auto")) { "APKs require a customized workspace image" }
        val profile = "workspace"
        val image = directory.resolve("image.tar.zst")
        val index = directory.resolve("image.json")
        if (!image.isFile || !index.isFile) {
            throw GradleException("Missing ${image.path} (+ ${index.name}). Build it with image/build.sh --arch $arch")
        }
        val parsed = groovy.json.JsonSlurper().parse(index) as Map<*, *>
        val metadata = parsed["metadata"] as? Map<*, *> ?: throw GradleException("$index: no metadata")
        check(metadata["architecture"] == arch) { "$index: architecture ${metadata["architecture"]} != $arch" }
        check(metadata["profile"] == profile) { "$index: profile ${metadata["profile"]} != $profile" }
        check((parsed["size"] as Number).toLong() == image.length()) { "${image.path}: size differs from ${index.name}" }
        val digest = MessageDigest.getInstance("SHA-256")
        image.inputStream().use { input ->
            val buffer = ByteArray(1 shl 20)
            while (true) { val count = input.read(buffer); if (count < 0) break; digest.update(buffer, 0, count) }
        }
        val actual = digest.digest().joinToString("") { "%02x".format(it.toInt() and 255) }
        if (actual != parsed["sha256"]) throw GradleException("${image.path}: sha256 $actual does not match ${index.name}")
        environment.mkdirs()
        val target = environment.resolve("image.tar.zst")
        // A hard link avoids copying ~200 MB per build; fall back to a copy across filesystems.
        try { Files.createLink(target.toPath(), image.toPath()) }
        catch (_: Exception) { Files.copy(image.toPath(), target.toPath(), StandardCopyOption.REPLACE_EXISTING) }
        index.copyTo(environment.resolve("image.json"))
        logger.lifecycle("Environment image: $profile $arch ${actual.take(12)} (${image.length() / 1_000_000} MB)")

    }
}

val buildProxyGuard = tasks.register<BuildProxyGuardTask>("buildProxyGuard") {
    script.set(repositoryRoot.file("native/proxy-guard/build.sh"))
    source.set(repositoryRoot.file("native/proxy-guard/proxy_guard.c"))
    toolchainVersion.set(libs.versions.ndk)
    ndkDirectory.set(androidComponents.sdkComponents.ndkDirectory)
    outputDirectory.set(layout.buildDirectory.dir("generated/proxy-guard-jni"))
}

val fetchPrebuilts = tasks.register<FetchPrebuiltsTask>("fetchPrebuilts") {
    group = "build setup"
    description = "Ensures the pinned Mihomo executable is cached, verified and exposed as jniLibs."
    manifests.from(prebuiltPackages.map { it.file("manifest.json") })
    // Platform capability executables remain Android native payloads.
    abis.set(listOf("arm64-v8a", "x86_64"))
    cacheDirectory.set(repositoryRoot.dir("third_party/.cache"))
    outputDirectory.set(layout.buildDirectory.dir("generated/prebuilt-jni"))
}

val buildEngine = tasks.register<BuildEngineTask>("buildEngine") {
    group = "build setup"
    description = "Cross-builds the Rust Workspace server, runtime and loader for the packaged ABIs."
    val engine = repositoryRoot.dir("engine")
    sources.from(engine.asFileTree.matching {
        exclude("target/**", "**/target/**", "**/build/**", "chat/**")
        include("**/*.rs", "**/Cargo.toml", "Cargo.lock", "**/*.sh", "**/*.json", "**/*.S", "environment/guest/**", "server/resources/**")
    })
    abis.set(listOf("arm64-v8a", "x86_64"))
    toolchainVersion.set(libs.versions.ndk)
    script.set(engine.file("build-android.sh"))
    ndkDirectory.set(androidComponents.sdkComponents.ndkDirectory)
    workDirectory.set(layout.buildDirectory.dir("intermediates/engine-build"))
    outputDirectory.set(layout.buildDirectory.dir("generated/engine-jni"))
}

/** ABI flavor → Debian architecture of its customized image. */
val flavorArchitectures = mapOf("arm64" to "arm64", "x86_64" to "amd64")
val environmentImages = flavorArchitectures.mapValues { (flavor, arch) ->
    tasks.register<PrepareEnvironmentImageTask>("prepare${flavor.replaceFirstChar(Char::titlecase)}EnvironmentImage") {
        group = "build setup"
        description = "Verifies and packages the $arch environment image for the $flavor flavor."
        val directory = repositoryRoot.dir("artifacts/image/$arch")
        selection.set(providers.gradleProperty("workflow.image.$flavor").orElse(providers.gradleProperty("workflow.image")).orElse("auto"))
        architecture.set(arch)
        imageDirectory.set(directory)
        candidates.from(listOf("image.tar.zst", "image.json").map { directory.file(it) })
        outputDirectory.set(layout.buildDirectory.dir("generated/environment-image/$flavor"))
    }
}

abstract class PrepareEngineToolsTask : DefaultTask() {
    @get:Input abstract val architecture: Property<String>
    @get:InputFile @get:PathSensitive(PathSensitivity.RELATIVE) abstract val script: RegularFileProperty
    @get:InputFiles @get:PathSensitive(PathSensitivity.RELATIVE) abstract val sources: ConfigurableFileCollection
    @get:InputFile @get:PathSensitive(PathSensitivity.NONE) abstract val serviceJar: RegularFileProperty
    @get:OutputDirectory abstract val outputDirectory: DirectoryProperty
    @get:Inject abstract val execOperations: ExecOperations
    @TaskAction fun prepare() {
        execOperations.exec {
            commandLine("python3", script.get().asFile.absolutePath,
                "--architecture", architecture.get(), "--jar", serviceJar.get().asFile.absolutePath,
                "--output", outputDirectory.get().asFile.resolve("environment/tools").absolutePath)
        }
    }
}
val engineTools = flavorArchitectures.mapValues { (flavor, arch) ->
    tasks.register<PrepareEngineToolsTask>("prepare${flavor.replaceFirstChar(Char::titlecase)}EngineTools") {
        dependsOn(":engine-chat:serviceJar")
        architecture.set(arch)
        script.set(repositoryRoot.file("engine/tools/package.py"))
        sources.from(repositoryRoot.file("engine/tools/package.py"))
        sources.from(listOf("codex", "jre", "claude-code").flatMap { name ->
            listOf("manifest.json", "LICENSE").map { repositoryRoot.file("third_party/$name/$it") }
        })
        serviceJar.set(project(":engine-chat").layout.buildDirectory.file("libs/workflow-chat.jar"))
        outputDirectory.set(layout.buildDirectory.dir("generated/engine-tools/$flavor"))
    }
}

val prebuiltNotices = tasks.register<PrebuiltNoticesTask>("prebuiltNotices") {
    packages.from(prebuiltPackages)
    // Bundled design resources (ui.design): Material Symbols vectors and the JetBrains Mono NL font.
    packages.from(listOf("material-symbols", "jetbrains-mono", "claude-code", "codex", "jre").map { repositoryRoot.dir("third_party/$it") })
    outputDirectory.set(layout.buildDirectory.dir("generated/prebuilt-notices"))
}

android {
    namespace = "top.flysoftbeta.workflow"
    ndkVersion = libs.versions.ndk.get()
    compileSdk {
        version = release(37)
    }

    defaultConfig {
        applicationId = "top.flysoftbeta.workflow"
        minSdk = 28
        targetSdk = 37
        versionCode = 10000
        versionName = "1.0.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    // One APK per ABI because each carries its own environment image: jniLibs ABI
    // splits cannot split assets. Same applicationId, so either flavor updates an installed app in place.
    flavorDimensions += "abi"
    productFlavors {
        create("arm64") {
            dimension = "abi"
            ndk { abiFilters += "arm64-v8a" }
        }
        create("x86_64") {
            dimension = "abi"
            ndk { abiFilters += "x86_64" }
        }
    }

    signingConfigs {
        create("release") {
            // Created explicitly by tools/init-release-key.sh; never generated during a build.
            storeFile = rootProject.file(providers.gradleProperty("workflow.keystore")
                .orElse("artifacts/signing/Workflow-release.p12").get())
            storeType = "PKCS12"
            storePassword = ""
            keyAlias = "workflow"
            keyPassword = ""
            enableV1Signing = true
            enableV2Signing = true
            enableV3Signing = true
        }
    }

    buildTypes {
        release {
            signingConfig = signingConfigs.getByName("release")
            optimization {
                enable = false
            }
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    buildFeatures {
        compose = true
    }
    externalNativeBuild {
        cmake {
            path = repositoryRoot.file("native/pty/CMakeLists.txt").asFile
            version = libs.versions.cmake.get()
        }
    }
    packaging {
        jniLibs {
            useLegacyPackaging = true
            keepDebugSymbols += "**/libmihomo.so"
            // Shipped byte-identical to the engine build (the loader is a relocation-free static-pie).
            keepDebugSymbols += "**/libworkflow-engine.so"
            keepDebugSymbols += "**/libworkflow-loader.so"
            keepDebugSymbols += "**/libworkflow-runtime.so"
        }
    }
    androidResources {
        // The image is already zstd; stored entries stream from the APK without inflating.
        noCompress += "zst"
    }
    lint {
        // Code moved into :core/:agent/:proxy keeps the app's API-level (NewApi, minSdk 28) checks.
        checkDependencies = true
    }
}

composeCompiler {
    // :core / :agent / :proxy models are immutable data classes (docs/report/initial/w1b-core-state.md §3).
    stabilityConfigurationFiles.add(project.layout.projectDirectory.file("compose-stability.conf"))
}

androidComponents.onVariants { variant ->
    // The old JNI PTY is an instrumentation fixture, never a release execution path.
    if (variant.buildType == "release") variant.packaging.jniLibs.excludes.add("**/libworkflow_pty.so")
    variant.sources.jniLibs?.addGeneratedSourceDirectory(buildProxyGuard, BuildProxyGuardTask::outputDirectory)
    variant.sources.jniLibs?.addGeneratedSourceDirectory(fetchPrebuilts, FetchPrebuiltsTask::outputDirectory)
    variant.sources.jniLibs?.addGeneratedSourceDirectory(buildEngine, BuildEngineTask::outputDirectory)
    val image = environmentImages[variant.flavorName] ?: throw GradleException("No environment image for flavor ${variant.flavorName}")
    variant.sources.assets?.addGeneratedSourceDirectory(image, PrepareEnvironmentImageTask::outputDirectory)
    variant.sources.assets?.addGeneratedSourceDirectory((engineTools[variant.flavorName] ?: throw GradleException("No Engine tools for ${variant.flavorName}")), PrepareEngineToolsTask::outputDirectory)
    variant.sources.assets?.addGeneratedSourceDirectory(prebuiltNotices, PrebuiltNoticesTask::outputDirectory)
}

dependencies {
    implementation("org.commonmark:commonmark:0.30.0")
    implementation("org.commonmark:commonmark-ext-gfm-tables:0.30.0")
    implementation("org.commonmark:commonmark-ext-gfm-strikethrough:0.30.0")
    implementation("ru.noties:jlatexmath-android:0.2.0")
    implementation(project(":core"))
    implementation(project(":agent-model"))
    testImplementation(project(":agent"))
    androidTestImplementation(project(":agent"))
    androidTestImplementation(testFixtures(project(":proxy")))
    implementation(project(":proxy"))
    implementation(libs.androidx.webkit)
    implementation(platform(libs.sora.editor.bom))
    implementation(libs.sora.editor)
    implementation(libs.sora.language.textmate)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.compose.material3)
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.ui.graphics)
    implementation(libs.androidx.compose.ui.tooling.preview)
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.lifecycle.runtime.ktx)
    testImplementation(testFixtures(project(":core")))
    androidTestImplementation(testFixtures(project(":core")))
    testImplementation(libs.junit)
    androidTestImplementation(platform(libs.androidx.compose.bom))
    androidTestImplementation(libs.androidx.compose.ui.test.junit4)
    androidTestImplementation(libs.androidx.espresso.core)
    androidTestImplementation(libs.androidx.junit)
    debugImplementation(libs.androidx.compose.ui.test.manifest)
    debugImplementation(libs.androidx.compose.ui.tooling)
}

// Unqualified test/install aliases select x86_64 for the current AVD matrix. ABI names describe
// executable compatibility, not whether Android runs on physical or virtual hardware.
mapOf(
    "testDebugUnitTest" to "testX86_64DebugUnitTest",
    "connectedDebugAndroidTest" to "connectedX86_64DebugAndroidTest",
    "installDebug" to "installX86_64Debug",
    "lintDebug" to "lintX86_64Debug",
).forEach { (alias, target) ->
    tasks.register(alias) {
        group = "verification"
        description = "Alias of $target (x86_64 ABI)."
        dependsOn(target)
    }
}
