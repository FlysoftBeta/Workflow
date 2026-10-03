# Workspace client contracts

`:core` is pure Kotlin/JVM. It contains the client connection protocol, workspace models and
codecs, layout semantics and terminal stream utilities. It has no Android dependency and does
not write production workspace files. [`RemoteWorkspaceStore`](src/main/kotlin/top/flysoftbeta/workflow/core/connection/RemoteWorkspaceStore.kt)
implements the store port through Engine RPC; the Rust Server supplies authoritative snapshots
and commits every workspace mutation.

Under `src/main/kotlin/top/flysoftbeta/workflow/core/`, `connection/` contains RPC, transports and
the remote projection; `store/` contains the store contract, state and wire codec; `layout/`,
`session/`, `resource/` and `config/` define shared values and pure operations. `environment/`
contains only status values. `terminal/` handles bounded output, UTF-8, OSC and key/link semantics.
`io/` holds path/name helpers, hashes and directory-entry values used by the protocol.

`src/testFixtures/` is a separate Gradle artifact for the historical Kotlin writer and environment
planner. The blocking `FileSystem` port, its memory/JVM implementations and `WorkspaceTrash` live
there with the reference store. They are absent from the production core JAR. JVM and Android
tests can depend on `testFixtures(project(":core"))`; production modules must not. The package
names stay stable so the existing reference and differential tests continue to exercise the same
semantics.

`src/test/` tests both production models and the explicitly imported reference fixtures. Passing
the old writer's tests does not establish that the production Rust owner passed acceptance. Run
the suite from the repository root:

```sh
flock artifacts/.gradle.lock ./gradlew :core:test
```

See the [protocol](../docs/implementation/protocol.md),
[workspace behavior](../docs/implementation/workspace.md),
[architecture](../docs/implementation/architecture.md) and
[testing guide](../docs/development/testing.md). Build output belongs in the ignored `build/`.
