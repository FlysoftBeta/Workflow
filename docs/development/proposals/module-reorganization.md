# Module reorganization: Engine domains and a thin App

Status: decided; implementation is partial. Owner: coordinator. Updated: 2026-10-04.

## Problem and intended outcome

The pre-reorganization Engine worked, but its organization no longer matched its responsibilities. `engine/server` was a flat set of files that mixed protocol handling, environment provisioning, terminal management, chat bridging and persistence. That baseline read JSON by indexing untyped values and touches files directly, so persisted state and wire messages lacked a single typed definition. Chat still runs as a Kotlin program under `engine/chat`, which mixes Rust and JVM code on the server side and keeps coding-agent adapters in shared JVM modules that the Android client also builds. The old source and documentation layout obscured the client/server boundary. The domain and documentation moves address that problem without treating the incomplete Rust Chat port as a production cutover.

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
| `engine/environment/` | `workflow-environment` | The environment that runs a workspace. Owns the `.workspace/` directory layout and its typed, atomic persistence API; the typed `env.json` declaration model and dependency provisioning (for example installing the optional Claude Code tool); container image storage, verification and installation; the environment lifecycle state machine. It also owns workspace-delivered appearance/client configuration and local service intent, executor epochs and measured receipts. Its `runtime` module is the only Rust API for interacting with the concrete environment: start, stop, run a command, open and resize a PTY, signal, wait, and report status. Configuration it owns can be exposed to a front-end settings panel through the Server. |
| `engine/environment/runtime/` | `workflow-runtime` | The isolation layer itself: the ptrace tracer binary and its library entry points. Anything that touches the guest directly belongs here. Architecture differences live in `cfg(target_arch)` modules rather than parallel copies of the whole tree. |
| `engine/environment/loader/` | `workflow-loader` | Guest loader used by the runtime. |
| `engine/chat/` | `workflow-chat` | Multi-paradigm coding-agent back ends (Codex app-server, Claude Code). Process supervision through the environment runtime, conversation journal, send ledger and composer-revision de-duplication, approval pass-through, preservation of unknown agent traffic. Rust is the decided replacement for the current Kotlin program; replacement remains blocked by parity and isolated device gates. Chat also owns agent/backend defaults. |
| `engine/workspace/` | `workflow-workspace` | Sessions, panel layout and panel state, paradigm selection. It does not own file contents or drafts. |
| `engine/filework/` | `workflow-filework` | File listing, reading and writing with revisions, drafts (Working Resources), diff, imports and the archive-protection policy. |
| `engine/terminal/` | `workflow-terminal` | Terminal settings and sessions on top of runtime PTYs: lifecycle, scrollback, resize, working-directory tracking and the path resolution needed to open files from terminal output. |
| `engine/server/` | `workflow-server` | Remote interaction. Owns the JSON-RPC 2.0 protocol (typed request, response, notification and error definitions plus the exported contract), transport, client connections and subscriptions, and composition of all domain crates. It maps protocol types to domain APIs and is the only crate that depends on wire formats. |

Rules that apply to every crate:

1. No `serde_json::Value` indexing for structures we know. Known documents and messages are serde types; where unknown fields must be preserved, the type carries an explicit flattened remainder. `Value` appears only behind a named opaque type.
2. No private-state filesystem access or `.workspace/` path construction outside `workflow-environment`; no workspace-file IO outside `workflow-filework`; no direct guest interaction outside the runtime.
3. Each crate exposes a small public API and keeps its internals private. Domain crates do not depend on each other except through `workflow-environment`.
4. No Kotlin, Java or JVM process under `engine/`.

## Target App structure

The App has only these responsibilities:

1. **Connection.** Connect to a workspace Engine. For local use it also owns the lifecycle of the bundled Engine. Remote/SSH transport keeps abstractions only, with no implementation.
2. **Configuration sync.** Workspace-delivered configuration is mirrored locally and local edits can be pushed back, but the workspace remains the source of truth. This infrastructure currently serves the proxy (Clash uses the workspace's `.workspace/proxy` configuration and workspace-produced data can flow back) and App UI configuration such as theme, palette and fonts. Connection profiles are the only data for which the App itself is the source of truth.
3. **Workbench.** The front end, built with native Compose; only the offline xterm adapter stays in WebView.
4. **Proxy.** Runs locally on the device in Root or VpnService mode.

All client code moves under `app/`. The decided Gradle modules are `:app:android`, `:app:client` and `:app:proxy`, with native adapters in `app/native` and terminal web tooling in `app/web`. The App appendix fixes these constraints: a pure-JVM client module owns the typed protocol bindings, JSON-RPC client, transport abstractions and synchronization logic; the proxy core stays pure JVM; Android-specific code stays in the Android application module; `core`, `agent`, `agent-model` and `engine/chat` cease to exist as client or server dependencies once their responsibilities have moved. Kotlin protocol types must be checked against the contract exported by `workflow-server`. The client merge retains existing Kotlin package identities for wire compatibility. Until Rust Chat cutover passes, the temporary Engine-only `:agent` and `:engine-chat` modules may depend on `:app:client`; they remain excluded from Android production dependencies.

## Terminal defects in scope

- Links in terminal output can be opened.
- File paths in terminal output can be opened in the Workbench, resolved against the terminal's working directory by the Engine.
- The scrollbar is usable by touch: a large draggable thumb and predictable fast scrolling.
- Text can be selected and copied by touch, with visible selection handles.

## Documentation structure

The documentation mirrors the code: `docs/product/` and `docs/ux/` stay; `docs/engine/` documents each Engine crate plus the protocol; `docs/app/` documents connection, synchronization, Workbench and proxy; `docs/development/`, `docs/status.md`, `docs/report/` and `docs/archive/` keep their roles. `docs/implementation/` and the top-level navigation stubs are retired. The [relocation map](../../archive/module-reorganization-map.json) records their maintained destinations and byte-preserved originals.

## Implementation and ownership

Module appendices live in `docs/development/proposals/module-reorganization/`:

- `environment.md` — environment, runtime, loader, images, `.workspace/` persistence.
- `chat.md` — porting chat and the coding-agent adapters to Rust.
- `server-domains.md` — server and protocol, workspace, filework, terminal (including the Engine side of the terminal defects).
- `app.md` — App module layout, connection, synchronization, Workbench, proxy, the client side of the terminal defects, and the documentation reorganization.

Each appendix maps current source to its target, defines the public interfaces the other modules rely on, and lists implementation tasks with owned paths, checks and dependencies. The decided common contracts are: typed domain models may derive serde/schema without owning wire methods; Server composes a single persisted workspace transaction; Environment owns the typed store and guest process API. The JVM Chat bridge is the explicit round-1 exception to the Rust-only target. Runtime target-tree deduplication is separately owned by the `runtime-dedup` task and is outside this round-2 implementation. Its acceptance must distinguish host, cross-build and device proof. The current public declaration remains `.workspace/env.json`, with the private lifecycle record at `.workspace/environment/environment.json`.

## Verification and rollout

Round 1 splits the Cargo domains, types storage/protocol, exports schema and golden fixtures, updates build paths and preserves current Android wire behavior. Its handoff requires source-bound `rust-server` (including domain tests and contract drift), `runtime-host`, `native`, `image`, `infrastructure`, `documentation` and `android-apk` results. Cargo uses at most two jobs under the shared build lease.

Round 2 develops Rust Chat with replay/codec/reducer parity as the required cutover gate, checks Kotlin bindings against the Rust export, relocates App modules, repairs terminal interactions and reorganizes documentation. Rust Chat replaced the Kotlin service on 2026-10-04, and the JRE/Kotlin path was then deleted by explicit user decision before its isolated API 28 acceptance. The integrated matrix must be recorded separately from contributor checks. The `runtime-dedup` task owns runtime extraction and its host, both-ABI build and isolated API 28 exec/PTY/stop gate; this round does not claim its implementation. Compilation is not acceptance: user-visible behavior needs the documented device matrix through `tools/workflow device`, using disposable AVDs only in this task. No result on host or API 28 x86_64 establishes physical ARM64 acceptance. Each appendix ends with owned paths, dependencies and checks.

## Decision and completion

The module layout and ownership decisions are accepted. Maintained references now live in [Engine](../../engine/README.md) and [App](../../app/README.md). Environment owns configuration and service state; Chat owns backend defaults; Terminal owns terminal settings; Server owns mapping and composition. Canonical proxy files use `.workspace/proxy/`, retaining `services.proxy` document identifiers with no migration.

Implementation remains partial. Rust Chat is the only chat implementation and passes host adapter, reducer and approval parity; the Kotlin service, `:agent`, `:engine-chat`, JAR and JRE were deleted before guest/device acceptance, which remains unproven. The client event reducer remains until a later projection protocol. Runtime deduplication is a separately owned task. Actual source identity and check results belong to contributor handoffs and the integrated report; this decided proposal is not an acceptance claim.
