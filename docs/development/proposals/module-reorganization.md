# Module reorganization: Engine domains and a thin App

Status: draft (coordinator skeleton; module appendices pending). Owner: coordinator. Updated: 2026-10-03.

## Problem and intended outcome

The Engine works, but its organization no longer matches its responsibilities. `engine/server` is a flat set of files that mixes protocol handling, environment provisioning, terminal management, chat bridging and persistence. Code throughout reads JSON by indexing untyped values and touches files directly, so persisted state and wire messages have no single typed definition. Chat runs as a Kotlin program under `engine/chat`, which mixes Rust and JVM code on the server side and keeps coding-agent adapters in shared JVM modules that the Android client also builds. The boundary between client and server is therefore not visible in the source tree, and the documentation still follows the period when the Engine lived inside the app.

The intended outcome is a source tree in which every responsibility has one owner and the client/server split is obvious from the top-level directories:

- `engine/` contains only Rust and only server-side responsibilities, divided into the domains below.
- `app/` contains only the client: connecting to a workspace, hosting a local Engine, synchronizing workspace-delivered configuration, the Workbench UI and the local proxy.
- Every persisted document and every protocol message has a typed definition. Untyped JSON is allowed only for explicitly opaque pass-through payloads, such as unknown coding-agent methods that must be preserved.
- File-system access happens only behind the owning module's typed API.
- The documentation tree mirrors the source tree.

Behavior that must stay dependable: the Engine remains the source of truth for sessions, layout, files, drafts, the environment and service state; workspace configuration and private state remain under `<workspace-root>/.workspace/`; agent requests are never auto-approved; unknown protocol methods, notifications, fields and server request IDs are preserved; Codex always receives the explicit user-reviews-approvals setting; version stays 1.0.0 with no legacy migration and no cross-version adapter; Android 9 (minSdk 28) remains a real target.

## Target Engine structure

The Engine is one Cargo workspace. Crates use the `workflow-` prefix. Arrows point from a crate to the crates it may depend on.

```
workflow-server ──► workflow-chat ─────┐
       │        ──► workflow-workspace ├──► workflow-environment ──► (spawns) workflow-runtime binary
       │        ──► workflow-filework ─┤
       │        ──► workflow-terminal ─┘
       └── owns the JSON-RPC protocol; no other crate knows wire types
```

| Directory | Crate | Responsibility |
| --- | --- | --- |
| `engine/environment/` | `workflow-environment` | The environment that runs a workspace. Owns the `.workspace/` directory layout and its typed, atomic persistence API; the typed `environment.json` model and dependency provisioning (for example installing the optional Claude Code tool); container image storage, verification and installation; the environment lifecycle state machine. Its `runtime` module is the only Rust API for interacting with the concrete environment: start, stop, run a command, open and resize a PTY, signal, wait, and report status. Configuration it owns can be exposed to a front-end settings panel through the Server. |
| `engine/environment/runtime/` | `workflow-runtime` | The isolation layer itself: the ptrace tracer binary and its library entry points. Anything that touches the guest directly belongs here. Architecture differences live in `cfg(target_arch)` modules rather than parallel copies of the whole tree. |
| `engine/environment/loader/` | (existing loader) | Guest loader used by the runtime. |
| `engine/chat/` | `workflow-chat` | Multi-paradigm coding-agent back ends (Codex app-server, Claude Code). Process supervision through the environment runtime, conversation journal, send ledger and composer-revision de-duplication, approval pass-through, preservation of unknown agent traffic. Rust replaces the current Kotlin program. |
| `engine/workspace/` | `workflow-workspace` | Sessions, panel layout and panel state, paradigm selection. It does not own file contents or drafts. |
| `engine/filework/` | `workflow-filework` | File listing, reading and writing with revisions, drafts (Working Resources), diff, imports and the archive-protection policy. |
| `engine/terminal/` | `workflow-terminal` | Terminal sessions on top of runtime PTYs: lifecycle, scrollback, resize, working-directory tracking and the path resolution needed to open files from terminal output. |
| `engine/server/` | `workflow-server` | Remote interaction. Owns the JSON-RPC 2.0 protocol (typed request, response, notification and error definitions plus the exported contract), transport, client connections and subscriptions, and composition of all domain crates. It maps protocol types to domain APIs and is the only crate that depends on wire formats. |

Rules that apply to every crate:

1. No `serde_json::Value` indexing for structures we know. Known documents and messages are serde types; where unknown fields must be preserved, the type carries an explicit flattened remainder. `Value` appears only behind a named opaque type.
2. No `std::fs` or path construction for `.workspace/` outside `workflow-environment`; no workspace-file IO outside `workflow-filework`; no direct guest interaction outside the runtime.
3. Each crate exposes a small public API and keeps its internals private. Domain crates do not depend on each other except through `workflow-environment`.
4. No Kotlin, Java or JVM process under `engine/`.

## Target App structure

The App has only these responsibilities:

1. **Connection.** Connect to a workspace Engine. For local use it also owns the lifecycle of the bundled Engine. Remote/SSH transport keeps abstractions only, with no implementation.
2. **Configuration sync.** Workspace-delivered configuration is mirrored locally and local edits can be pushed back, but the workspace remains the source of truth. This infrastructure currently serves the proxy (Clash uses the workspace's `.workspace/proxy` configuration and workspace-produced data can flow back) and App UI configuration such as theme, palette and fonts. Connection profiles are the only data for which the App itself is the source of truth.
3. **Workbench.** The front end, built with native Compose; only the offline xterm adapter stays in WebView.
4. **Proxy.** Runs locally on the device in Root or VpnService mode.

All client code moves under `app/`. The exact Gradle module split is settled by the App appendix, under these constraints: a pure-JVM client module owns the typed protocol bindings, JSON-RPC client, transport abstractions and synchronization logic; the proxy core stays pure JVM; Android-specific code stays in the Android application module; `core`, `agent`, `agent-model` and `engine/chat` cease to exist as client or server dependencies once their responsibilities have moved. Kotlin protocol types are checked against the contract exported by `workflow-server`.

## Terminal defects in scope

- Links in terminal output can be opened.
- File paths in terminal output can be opened in the Workbench, resolved against the terminal's working directory by the Engine.
- The scrollbar is usable by touch: a large draggable thumb and predictable fast scrolling.
- Text can be selected and copied by touch, with visible selection handles.

## Documentation structure

The documentation mirrors the code: `docs/product/` and `docs/ux/` stay; `docs/engine/` documents each Engine crate plus the protocol; `docs/app/` documents connection, synchronization, Workbench and proxy; `docs/development/`, `docs/status.md`, `docs/report/` and `docs/archive/` keep their roles. `docs/implementation/` and the top-level navigation stubs are retired after links are updated, and a relocation map is added to `docs/archive/`.

## Implementation and ownership

Module appendices live in `docs/development/proposals/module-reorganization/`:

- `environment.md` — environment, runtime, loader, images, `.workspace/` persistence.
- `chat.md` — porting chat and the coding-agent adapters to Rust.
- `server-domains.md` — server and protocol, workspace, filework, terminal (including the Engine side of the terminal defects).
- `app.md` — App module layout, connection, synchronization, Workbench, proxy, the client side of the terminal defects, and the documentation reorganization.

Each appendix maps current source to its target, defines the public interfaces the other modules rely on, and lists implementation tasks with owned paths, checks and dependencies. The coordinator settles cross-appendix contracts before implementation starts.

## Verification and rollout

To be completed from the appendices. Compilation is not acceptance: Engine behavior needs host suites, and user-visible behavior needs the documented device matrix through `tools/workflow device`.

## Decision and completion

Pending review of the appendices.
