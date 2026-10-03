# Architecture

The Rust Workspace Engine is the single authority for sessions, layout, files, drafts, configuration, environments, and published service state. Android is a connection and presentation shell. It may keep connection profiles and local configuration delivered by the workspace, but its workspace state is a disposable projection of the Engine's snapshot. This boundary lets a reconnect recover the same workspace without making the client a second writer.

The [product specification](../product/README.md) defines observable behavior, the [UX specification](../ux/README.md) defines presentation, and the [protocol](protocol.md) defines communication. This document explains where those responsibilities live in the implementation.

## Production components

`engine/server` implements the JSONL Workspace Server, state transactions, file operations, service documents, environment lifecycle, and managed processes. `engine/runtime` implements the container CLI, ptrace scheduling, path and identity translation, image installation, and generation metadata. `engine/loader` is the `no_std` ELF loader used by the runtime. These are separate executables: the Android package contains `libworkflow-engine.so`, `libworkflow-runtime.so`, and `libworkflow-loader.so`, respectively.

The JVM modules separate platform-independent responsibilities. `:core` contains protocol and connection types, workspace models, reference layout semantics, and terminal stream utilities. `:agent` contains the Codex and Claude adapters, transport machinery, neutral conversation model, and approval constraints. `:proxy` contains Mihomo configuration, controller access, and guardian protocol handling. None of these modules references Android; `:agent` and `:proxy` do not depend on one another. Historical Kotlin workspace writers and their blocking filesystem/trash helpers are test fixtures, not production stores. Production core retains the file-entry value model and hashing utility needed by connection APIs without exposing the old writer implementation.

`:app` contains the Android connection shell, independent feature packages, Compose UI, Sora and xterm adapters, and local capability executors. Feature packages communicate through injected service ports and panel navigation contracts; they do not import each other. `AppGraph` assembles connections, snapshot projections, and platform executors. A ViewModel can coordinate an interaction, but cannot become an owner of durable workspace state.

`native/` holds Android PTY/JNI and the Root proxy guardian, not the container implementation. `web/` holds offline xterm assets, input adaptation, and tests. `image/` builds the customized environment images. `third_party/` records pinned downloads, hashes, provenance, and licenses; its ignored cache holds the downloaded binaries.

## Workspace storage

The user selects a workspace root. User files live directly beneath that root, which appears as `/workspace` inside the environment. The Engine keeps its configuration and private data in the root's `.workspace/` directory:

```text
<workspace-root>/
  .workspace/
    config.json                 workspace settings
    env.json                    environment declaration
    state/workspace.json        sessions, layouts, file and composer drafts
    state/services/             measured local service reports
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

Every product terminal and coding-agent process is started by the Server through the configured Rust runtime and a verified environment generation. The process API carries PTY or stdio bytes. There is no Android shell, host agent, or old repository fallback. The Server does not parse or answer vendor approval requests; the agent adapters preserve them for the user.

`RuntimeService` maintains the embedded connection and local capability lifecycle. Root Mihomo is a separate local executor, outside the development container. Its desired configuration and published status still belong to the Engine, as described in the [proxy document](proxy.md).

Chat is rendered by native Compose components: CommonMark supplies the Markdown model, native elements render text, tables, and code, and Canvas renders mathematical notation. Scroll state is sent back through panel commands. Only the offline xterm terminal uses WebView, including compatibility assets for Android 9's Chromium 66. The [Android client](android-client.md) explains rendering, input, and platform adapters.

Archived C runtime code, the old WebView chat, and obsolete designs live under [docs/archive](../archive/README.md). Production builds do not use them. Historical test results and current acceptance limits are distinguished in [status](../status.md) and the [testing guide](../development/testing.md).
