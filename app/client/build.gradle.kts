import org.jetbrains.kotlin.gradle.dsl.JvmTarget

// Pure JVM client contracts/models and terminal utilities; historical writers are test fixtures.
plugins {
    `java-test-fixtures`
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
    api(libs.kotlinx.coroutines.core)
    api(libs.kotlinx.serialization.json)
    testFixturesImplementation(libs.kotlinx.coroutines.core)
    testImplementation(libs.junit)
    testImplementation(libs.kotlinx.coroutines.test)
}

sourceSets.test { resources.srcDir(rootProject.file("engine/protocol")) }
