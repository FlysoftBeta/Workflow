# Workspace client contracts

`:app:client` is pure Kotlin/JVM. It contains generated typed protocol bindings, JSON-RPC transport, configuration synchronization,
workspace/chat projections and terminal stream utilities. `client/protocol/EngineBindings.kt` is generated
from the Server export by `tools/generate-client-protocol.py`; immutable typed views retain original
unknown fields, precise numeric IDs and absent/null distinctions. `client/sync/` supplies connection-bound
configuration projections and revision-aware documents. Legacy package names remain for the existing
UI and parity codecs; they do not denote separate Gradle modules. It has no Android dependency and does
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
there with the reference store. They are absent from the production client JAR. JVM and Android
tests can depend on `testFixtures(project(":app:client"))`; production modules must not. The package
names stay stable so the existing reference and differential tests continue to exercise the same
semantics.

`src/test/` tests both production models and the explicitly imported reference fixtures. Passing
the old writer's tests does not establish that the production Rust owner passed acceptance. Run
the suite from the repository root:

```sh
tools/workflow check client --task module-reorg-r2
```

See the [protocol](../../docs/engine/protocol.md),
[workspace behavior](../../docs/engine/workspace.md),
[architecture](../../docs/app/README.md) and
[testing guide](../../docs/development/testing.md). Build output belongs in the ignored `build/`.
