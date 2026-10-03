# Kotlin reference implementations

This source set preserves the Kotlin workspace writer and environment planner as test oracles.
Gradle publishes it through `java-test-fixtures`; it is not part of the production core JAR or APK.
Tests opt in through `testFixtures(project(":core"))` and retain the original Kotlin package names.

Under `kotlin/top/flysoftbeta/workflow/core/`, `store/` holds `ReferenceWorkspaceStore`, its state
paths and the disk-backed trash writer. `io/` contains the blocking filesystem port, JVM/memory
implementations and watcher. `environment/` contains the earlier installer, planner and state
writer. The production equivalents live in `engine/server/` and `engine/environment/runtime/`; Android
reference tests supply their filesystem adapter from `app/src/androidTest/`.

Keep fixes that preserve oracle validity here, and keep real product behavior in the Rust owner.
Report reference-test results separately from protocol, real-image and device acceptance. See
the [core module guide](../../README.md) for production contracts and the normal test command.
