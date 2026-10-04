# App

The App connects to a Workspace Engine, mirrors workspace-delivered configuration, presents the Workbench and executes local Android capabilities. Engine owns sessions, layout, files, drafts, the environment and service state. Connection profiles are the only durable data for which the App is the authority; other cached configuration is a revisioned projection. [Engine](../engine/README.md), [protocol](../engine/protocol.md), [product](../product/README.md) and [UX](../ux/README.md) define those boundaries.

| Source and Gradle module | Responsibility |
| --- | --- |
| `app/android`, `:app:android` | Android application, Compose features, platform adapters, embedded Engine hosting and instrumentation |
| `app/client`, `:app:client` | Pure JVM protocol bindings, transport/connection abstractions, disposable projections and synchronization |
| `app/proxy`, `:app:proxy` | Pure JVM proxy model, controller and executor ports |
| `app/native` | Local proxy guardian and instrumentation-only PTY fixtures |
| `app/web` | Offline xterm source, assets tooling and JavaScript tests |

The old `:core` and `:agent-model` Gradle modules are merged into `:app:client`. Their Kotlin package identities, including `core.*` and `agent.*`, are retained initially to preserve wire and serialization compatibility; package names do not create separate module ownership. Android depends on the client and proxy modules. The client and proxy core have no mutual dependency or vendor adapter dependency, and feature packages do not import other features.

Chat runs only in the Engine's Rust `workflow-chat`; the former Kotlin `:agent` adapters and `:engine-chat` guest service have been deleted. Android has no vendor adapter or process launcher in production. `:app:client` test fixtures keep the small `agent.process` launch port that instrumentation uses to run guest commands through the Server process API. Replacing the client event reducer with Engine-reduced projections remains a coordinated [Chat](../engine/chat.md#rust-port-and-device-acceptance) protocol change.

Start with [connection](connection.md) for bootstrap and lifecycle, [configuration](configuration.md) for revisioned synchronization, [Workbench](workbench.md) for Compose/Sora/xterm adapters, and [proxy](proxy.md) for local execution and measured receipts. Only xterm uses WebView. File IO and expensive parsing remain off the main thread; connection failure disables mutations and reconnect reloads authoritative state.

Android 9/API 28 is a real target, with pinned offline assets and both customized image ABIs. Remote/SSH interfaces remain unimplemented. Source checks, compilation and fixture tests do not establish Android acceptance; use the [build](../development/building.md), [testing](../development/testing.md) and [status](../status.md) records.
