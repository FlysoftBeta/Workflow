# Workflow App

The App connects to a workspace and hosts the bundled Engine for local connections, mirrors workspace-delivered configuration, renders the Workbench, and executes the local proxy in Root or VpnService mode. Connection profiles are its only independent durable source of truth. Remote/SSH remains an extension point without an implementation.

| Directory | Gradle module | Ownership |
| --- | --- | --- |
| [client](client/README.md) | `:app:client` | Pure JVM typed Engine bindings, JSON-RPC, transport interfaces, configuration synchronization and disposable projections |
| [proxy](proxy/README.md) | `:app:proxy` | Pure JVM proxy controller, local executor and disposable staging |
| [android](android/README.md) | `:app:android` | Android lifecycle and services, Compose Workbench, Sora and xterm adapters |
| [native](native/README.md) | Android build inputs | Proxy guardian and instrumentation-only PTY |
| [web](web/README.md) | Offline build inputs | Maintained xterm adapter sources, packaging and tests |

`client` and `proxy` have no Android or mutual dependency. The Android application composes them. Feature packages communicate through App interfaces rather than importing one another. Historical persistence and host PTY implementations remain test fixtures. Retained Kotlin vendor adapters and the guest service are a temporary Engine parity oracle and production fallback, never an Android production dependency.

The [App guide](../docs/app/README.md), [protocol](../docs/engine/protocol.md), and [testing guide](../docs/development/testing.md) describe the contracts and verification boundaries.
