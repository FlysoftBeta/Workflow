# Frozen Kotlin environment reference

These classes retain the pre-rewrite planner, installer and state-writer tests.
They belong to `java-test-fixtures` and are not included in the production `:core` JAR or APK.
The production environment owner is `engine/server`; shared shell status types remain in
`core/src/main/kotlin/top/flysoftbeta/workflow/core/environment/EnvironmentStatus.kt`.
Historical design: the initial checkpoint under `artifacts/checkpoints/initial-1.0.0/`.
