# Connection and local Engine hosting

The Android application selects a connection profile, starts the embedded Engine when needed and presents its authoritative state. Pure JVM connection and JSON-RPC contracts live in `app/client`; Android bootstrap, foreground services and platform integration live in `app/android`. The [protocol](../engine/protocol.md) requires an exact version match. Remote and SSH are reserved abstractions only and explicitly fail as unsupported in version 1.0.0.

`AppGraph` composes the bootstrapper, transport, workspace projection, service ports and local executors. The selected profile is saved before startup. Embedded bootstrap alone resolves the local workspace root and paths to bundled Server, runtime, loader, images and tools. The active connection exposes protocol identity and ports, not a host workspace `File`; features cannot bypass Engine file policy.

The client uses generated `EngineBindings` views from the Server schema. These retain original JSON, including unknown fields, precise numeric IDs and the distinction between absent and explicitly null values. Existing `WorkspaceWire` mapping is isolated in a compatibility adapter, so moving source and adding typed access do not silently change established wire encodings.

## Startup, failure and reconnect

`WorkspaceConnectionConfig`, `WorkspaceBootstrapper` and `WorkspaceTransport` separate connection choice from transport mechanics. Startup performs `hello`, loads the authoritative workspace snapshot and `client.config`, then creates connection-scoped consumers. Workspace state is a disposable `StateFlow` projection. Accepted command replies replace it after Engine persistence succeeds.

An attempt reports its phase as `ConnectPhase`: starting the Engine, loading the workspace, and synchronizing configuration. A failed first connection is reported at once with the Engine's diagnostic tail for Details. Failure of a connection that was online retires it, cancels its consumers, marks its projection failed and releases its foreground-service leases. `WorkspaceConnectionManager` then reconnects automatically as `Reconnecting`, waiting 1, 2, 5, 10 and 30 seconds before successive attempts. Retry now and the app returning to the foreground end a wait early. After the last attempt, or a failure that retrying cannot fix such as `IncompatibleWorkspaceException`, the status becomes a non-retrying `Failed`. Explicit disconnection cancels pending attempts.

While a lost connection is restored, `MainActivity` keeps the previous `ShellViewModel` composed but inert beneath the reconnect card, and `AppGraph` stays bound to the lost session, so accessors never throw mid-transition and that session's services fail as a closed connection. Only a new session or explicit disconnection releases them. Workspace mutation remains disabled until a new connection loads fresh Engine state. Late callbacks from an obsolete instance cannot redirect work into another workspace or authorize a new executor. Already committed drafts survive on Engine; unacknowledged client work is not presented as durable.

`WorkspaceRpc` fails a request on a closed connection with `WorkspaceClosedException` and a request that receives no answer in time with `WorkspaceTimeoutException`. Both are failures rather than coroutine cancellations, so a timeout can no longer silently end a caller's loop. `Failure.of` classifies these and `WorkspaceRpcException` kinds as lost, transient, blocked, rejected or unexpected for presentation.

The embedded stdio Server is not a detached daemon. Closing its transport stops Chat and owned runtime processes. Reconnect rehydrates committed resource metadata and vendor history; it does not promise uninterrupted PTYs or preserved output rings. There is no local-store, host-agent or Android-shell fallback when bootstrap or Engine fails.

## Android lifecycle

`LocalRuntimeService` coordinates local connection and capability lifetimes. Foreground leases follow the work that requires them, including the guardian of a live local proxy. Startup must be confirmed before a queued release can stop a service. A retired service instance or stale configuration cannot authorize new work after reconnect.

Connection profiles are client-authored durable data. Workspace-delivered appearance and local settings may be cached by connection identity and revision, as described in [configuration](configuration.md); sessions, drafts, indexes and private Engine state may not be recovered from that cache. Application data is excluded from Android system backup.

Feature packages communicate through injected panel/service ports. Test fixtures use scoped providers rather than a process-global substitute. Imports, attachments, images and editor operations use Engine APIs, even when their source is an Android URI. Platform staging is bounded, disposable and tied to the connection that initiated it.

## Verification

Source architecture checks reject production vendor/process dependencies and exposure of the host workspace root. `ConnectionBoundaryAcceptanceTest` exercises connection retirement and recovery; `EngineIntegrationTest` covers the actual bundled Engine and guest lifecycle. Fixture process tests and a successful APK build cannot replace that device evidence. [Testing](../development/testing.md) defines frozen APK identity and isolated AVD use; [status](../status.md) records the proven matrix.
