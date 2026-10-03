import org.jetbrains.kotlin.gradle.dsl.JvmTarget

// Pure Kotlin/JVM: agent backend API, Codex App-Server / Claude Code adapters, protocol transport.
plugins {
    alias(libs.plugins.kotlin.jvm)
    alias(libs.plugins.kotlin.serialization)
    alias(libs.plugins.android.lint)
}
apply(from = rootProject.file("gradle/pure-jvm-module.gradle.kts"))

java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}
kotlin {
    compilerOptions { jvmTarget = JvmTarget.JVM_17 }
}

dependencies {
    api(project(":app:client"))
    // Retained Engine parity oracle consumes the client wire types until Rust Chat cutover.
    api(libs.kotlinx.coroutines.core)
    api(libs.kotlinx.serialization.json)
    testImplementation(libs.junit)
    testImplementation(libs.kotlinx.coroutines.test)
}

tasks.test {
    // Golden files (agent/protocol-coverage.md) are verified, or rewritten with -Pagent.updateGolden=true.
    systemProperty("agent.projectDir", layout.projectDirectory.asFile.absolutePath)
    systemProperty("agent.updateGolden", providers.gradleProperty("agent.updateGolden").getOrElse("false"))
    inputs.files(layout.projectDirectory.file("protocol-coverage.md")).optional()
}

// Host smoke runs against real binaries; not part of `test`. See SmokeMain.kt.
tasks.register<JavaExec>("smoke") {
    group = "verification"
    description = "Drives a real Codex App-Server or Claude Code CLI through the :agent backends."
    classpath = sourceSets["test"].runtimeClasspath
    mainClass.set("top.flysoftbeta.workflow.agent.smoke.SmokeMainKt")
    args(providers.gradleProperty("smokeArgs").getOrElse("").split(" ").filter { it.isNotBlank() })
}
