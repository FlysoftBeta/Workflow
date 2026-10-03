// Engine device acceptance harness: an app whose instrumentation test runs libworkflow-engine.so from
// nativeLibraryDir inside the zygote-forked app process (app seccomp filter, untrusted_app domain).
// Build:  flock <repo>/artifacts/.gradle.lock <repo>/gradlew -p engine/runtime/tests/android-harness \
//             -PengineAbis=x86_64 assembleDebug assembleDebugAndroidTest
// Normally driven by engine/runtime/tests/device/run.sh.

plugins {
    alias(libs.plugins.android.application)
}

val repositoryRoot: File = rootDir.resolve("../../../..").canonicalFile
val engineOut: File = providers.gradleProperty("engineOut").map { File(it) }
    .getOrElse(repositoryRoot.resolve("artifacts/engine/rust-harness"))
/** ABIs to package; default = every ABI directory the engine build produced. */
val engineAbis: List<String> = providers.gradleProperty("engineAbis").map { it.split(',').map(String::trim).filter(String::isNotEmpty) }
    .getOrElse(listOf("arm64-v8a", "x86_64").filter { engineOut.resolve(it).isDirectory })

/**
 * Copies the engine build output into a generated jniLibs tree. Everything that is executed must live in
 * nativeLibraryDir (exec from app data is denied on API 29+), so helpers get lib*.so names:
 *   libworkflow-engine.so, libworkflow-loader.so     unchanged
 *   test/sigsys_helper  -> libwftest-sigsys.so        (host pass-through helper)
 *   test/fork_stress    -> libwftest-forkstress.so    (host pass-through helper)
 *   test/guest_static   -> libwftest-gueststatic.so   (copied into the rootfs by the case file's setup)
 * plus, when present, artifacts/engine/device/probe/<abi>/sysprobe -> libwftest-sysprobe.so (seccomp survey
 * probe built by engine/runtime/tests/device/run.sh from engine/runtime/tests/device/sysprobe.c).
 */
abstract class StageEngineLibsTask : DefaultTask() {
    @get:InputFiles @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val sources: ConfigurableFileCollection
    @get:Input abstract val engineOutPath: Property<String>
    @get:Input abstract val probeOutPath: Property<String>
    @get:Input abstract val abis: ListProperty<String>
    @get:OutputDirectory abstract val outputDirectory: DirectoryProperty

    @TaskAction fun stage() {
        val out = outputDirectory.get().asFile
        out.deleteRecursively()
        val names = linkedMapOf(
            "libworkflow-engine.so" to "libworkflow-engine.so",
            "libworkflow-loader.so" to "libworkflow-loader.so",
            "test/sigsys_helper" to "libwftest-sigsys.so",
            "test/fork_stress" to "libwftest-forkstress.so",
            "test/guest_static" to "libwftest-gueststatic.so",
        )
        if (abis.get().isEmpty()) throw GradleException("no engine ABI found under ${engineOutPath.get()}; run engine/runtime/test-android.sh")
        for (abi in abis.get()) {
            for ((src, dst) in names) {
                val from = File(engineOutPath.get(), "$abi/$src")
                if (!from.isFile) throw GradleException("missing $from; run engine/runtime/test-android.sh $abi")
                val to = out.resolve("$abi/$dst")
                to.parentFile.mkdirs()
                from.copyTo(to, overwrite = true)
                to.setExecutable(true, false)
            }
            val probe = File(probeOutPath.get(), "$abi/sysprobe")
            if (probe.isFile) {
                val to = out.resolve("$abi/libwftest-sysprobe.so")
                probe.copyTo(to, overwrite = true)
                to.setExecutable(true, false)
            }
        }
    }
}

val stageEngineLibs = tasks.register<StageEngineLibsTask>("stageEngineLibs") {
    val probeOut = repositoryRoot.resolve("artifacts/engine/device/probe")
    engineOutPath.set(engineOut.path)
    probeOutPath.set(probeOut.path)
    abis.set(engineAbis)
    sources.from(engineAbis.map { engineOut.resolve(it) })
    sources.from(engineAbis.map { probeOut.resolve(it) })
    outputDirectory.set(layout.buildDirectory.dir("generated/engine-jni"))
}

android {
    namespace = "top.flysoftbeta.workflow.engineharness"
    compileSdk {
        version = release(37)
    }

    defaultConfig {
        applicationId = "top.flysoftbeta.workflow.engineharness"
        minSdk = 28
        targetSdk = 37
        versionCode = 1
        versionName = "0.1"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        ndk {
            abiFilters += engineAbis
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    packaging {
        jniLibs {
            // extractNativeLibs=true: the executables must exist as files in nativeLibraryDir.
            useLegacyPackaging = true
            // Ship the engine byte-identical to the build output (the harness verifies sha256 on device).
            keepDebugSymbols += "**/*.so"
        }
    }
}

androidComponents.onVariants { variant ->
    variant.sources.jniLibs?.addGeneratedSourceDirectory(stageEngineLibs, StageEngineLibsTask::outputDirectory)
}

dependencies {
    androidTestImplementation(libs.junit)
    androidTestImplementation(libs.androidx.junit)
    androidTestImplementation("androidx.test:runner:1.7.0")
}
