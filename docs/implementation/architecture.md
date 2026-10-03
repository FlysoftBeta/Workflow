# Architecture

The Rust Workspace Engine is the single authority for sessions, layout, files, drafts, configuration, environments, and published service state. Android is a connection and presentation shell. It may keep connection profiles and local configuration delivered by the workspace, but its workspace state is a disposable projection of the Engine's snapshot. This boundary lets a reconnect recover the same workspace without making the client a second writer.

The [product specification](../product/README.md) defines observable behavior, the [UX specification](../ux/README.md) defines presentation, and the [protocol](protocol.md) defines communication. This document explains where those responsibilities live in the implementation.

## Production components

`engine/server` implements the JSONL Workspace Server, state transactions, file operations, service documents, environment lifecycle, and managed processes. `engine/runtime` implements the container CLI, ptrace scheduling, path and identity translation, image installation, and generation metadata. `engine/loader` is the `no_std` ELF loader used by the runtime. These are separate executables: the Android package contains `libworkflow-engine.so`, `libworkflow-runtime.so`, and `libworkflow-loader.so`, respectively.

The JVM modules separate presentation data from execution. `:core` contains protocol and connection types, workspace models, reference layout semantics, and terminal stream utilities. `:agent-model` contains immutable conversation models, the shared `ChatWire` codec, JSON values, and the pure event reducer. Android depends on these shared definitions, not on vendor backends. `:agent` contains Codex and Claude adapters, transports, process ports, and approval enforcement and is an Engine-only production dependency. `:engine-chat`, rooted at `engine/chat`, owns conversation orchestration and runs inside the environment on the bundled Linux JRE. Rust supervises this service and mediates its private persistence callbacks. `:proxy` remains an independent pure JVM local-capability module; it does not depend on the agent modules.

`:app` contains the connection shell, independent feature packages, Compose UI, Sora and xterm adapters, and Android capability executors. `AppGraph` composes RPC clients and disposable projections. Android does not construct vendor backends, install tools, maintain conversation indexes, allocate terminal identities, or decide resource cleanup. Features submit semantic Engine commands through injected ports and do not import one another. Historical Kotlin workspace writers and host-shell adapters exist only in test fixtures or instrumentation sources.

The architecture-specific Engine tools distribution contains Codex, the guest JRE and the chat service JAR. APK assets transport the verified catalog and payload; they do not turn these tools into Android native libraries. Engine owns their verification, bindings and optional Claude installation. `native/` contains the local proxy guardian and isolated PTY test support, not the container implementation. `web/` contains offline xterm assets and tests, `image/` builds customized images, and `third_party/` retains pinned manifests and licenses. Generated binaries remain in ignored caches and build directories.

## Workspace storage

The user selects a workspace root. User files live directly beneath that root, which appears as `/workspace` inside the environment. The Engine keeps its configuration and private data in the root's `.workspace/` directory:

```text
<workspace-root>/
  .workspace/
    config.json                 workspace settings
    env.json                    environment declaration
    state/workspace.json        sessions, layouts, file and composer drafts
    state/services/             measured local service reports
    state/local-services/       desired proxy state and executor operation receipts
    state/terminals.json         terminal metadata and resource identities
    environment/                generations, home, toolchains, activation records
    services/<serviceId>/       service configuration, assets, redacted logs
    documents/                  opaque adapter documents and revision metadata
    uploads/                    incomplete, restricted uploads
    trash/                      reversible deletions
    corrupt/                    original data retained during recovery
```

The normal explorer always hides `.workspace`. Explicit configuration and service-file actions can open the paths allowed by the protocol; the file API rejects other private state. The Engine also masks private state directories in the guest environment. Runtime metadata inside a generation uses a separate `.workflow-engine/` directory and is not a legacy workspace configuration location.

The [Workspace Server](workspace-engine.md) owns atomic writes and recovery. Android does not reconstruct sessions or drafts from its cache when the connection fails, and application data is excluded from Android system backup. This version does not scan, import, or migrate old workspace layouts or configuration formats.

## Commands and connections

Startup first selects and saves a connection profile, then creates a bootstrapper and transport, performs the exact-version `hello`, and loads the authoritative snapshot. Only the embedded local implementation exists in 1.0.0. Remote and SSH connection types reserve an abstraction boundary; they explicitly fail as unsupported rather than establish a remote connection.

The Server serializes state-changing commands and returns the committed revision and complete state after persistence succeeds. Android replaces its `StateFlow` projection with that result. Layout changes are reduced by the Server, rather than accepted as layouts precomputed by a client. Kotlin's pure layout implementation remains useful for protocol encoding and the cross-language test oracle.

Sessions describe arrangements of panels; Working Resources hold unsaved file and composer content independently of those arrangements. Closing a panel or changing sessions therefore does not discard a draft. The detailed archive, conflict, and acknowledgement semantics are in the [workspace model](workspace.md). Watches, environment builds, process waits, and output reads do not retain the workspace state lock while they wait.

Disconnection disables workspace mutations, cancels the failed connection's consumers, and retires its service leases. Reconnection fetches fresh Engine state. Already committed drafts survive; unacknowledged client work cannot be presented as committed state.

## Execution and presentation

The Server starts product terminals and the guest chat service through the configured Rust runtime and a verified environment generation. The chat service launches vendor adapters inside that environment. Android receives terminal resources through `terminal.*` and chat projections through `chat.*`, rather than directing vendor process setup. There is no Android shell, Android-JVM chat-service, host-agent, or local-repository fallback. Vendor approval policy remains in the Engine adapters, and only an explicit user response can approve a request.

Terminal resources own stable IDs, titles, cwd, process association, output generation and cleanup policy. Closing a view removes only its UI attachment. The Engine observes references in live workspace sessions and ends unreferenced resources after a fifteen-second grace period. Environment activation restores previously running terminal resources; a restart with no verified pending generation does not disturb them.

The embedded stdio Server still ends when its transport closes and stops its processes. Durable resource metadata and drafts can be loaded on reconnect, but moving ownership into Engine does not imply a detached daemon, uninterrupted process survival, or remote transport.

`LocalRuntimeService` maintains the embedded connection and local capability lifecycle. Root Mihomo is a separate local executor, outside the development container. Its desired configuration and published status still belong to the Engine, as described in the [proxy document](proxy.md).

Chat is rendered by native Compose components: CommonMark supplies the Markdown model, native elements render text, tables, and code, and Canvas renders mathematical notation. Scroll state is sent back through panel commands. Only the offline xterm terminal uses WebView, including compatibility assets for Android 9's Chromium 66. The [Android client](android-client.md) explains rendering, input, and platform adapters.

Archived C runtime code, the old WebView chat, and obsolete designs live under [docs/archive](../archive/README.md). Production builds do not use them. Historical test results and current acceptance limits are distinguished in [status](../status.md) and the [testing guide](../development/testing.md).
