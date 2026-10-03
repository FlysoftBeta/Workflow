# Server, Workspace, FileWork and Terminal

Status: decided; round-1 domain split implemented, round-2 composition and terminal work pending integrated evidence. Owner: Engine contributor. Updated: 2026-10-03.

This appendix completes the [architecture](../module-reorganization.md). The source inventory was verified against `f1ed8994ae513bd1e04106bbf162df56919cf2b6`. Round 1 preserves the Android method names, JSON field names, executable names, version 1.0.0 and current state formats. It adds no remote transport or migration.

## Ownership and composition

`workflow-server` is the composition root. Its binary remains `workflow-engine`, packaged as `libworkflow-engine.so`. It owns JSON-RPC envelopes, method dispatch, request validation, transport lifetime, bounded worker scheduling, watches, and transactions that span domains. Server state composition combines session state from Workspace with Working Resources from FileWork; a single store publication preserves the existing `state/workspace.json` transaction boundary. Closing panels changes only session state. Persisting separate independently updated files for sessions and drafts would break archive atomicity and is deliberately avoided.

| Existing source | Owner after the split | Public responsibility |
| --- | --- | --- |
| `server/src/layout.rs` | `workspace` | Typed panel targets, views, stacks, splits, regions, layout operations and normalization |
| Session operations in `server/src/workspace.rs` | `workspace` | Session creation, activation, pin order, archive/restore, maintenance eligibility and resource references |
| File/draft operations in `workspace.rs`; `imports.rs`; file helpers in `storage.rs` | `filework` | Safe paths, versions, bounded reads, file writes, drafts/composers, imports, trash, diff and archive save preflight |
| `server/src/terminal.rs` | `terminal` | Metadata, terminal generations, runtime PTY attachment, output cursors, resize/stop/wait, OSC scanning and unreferenced cleanup |
| `main.rs`, `protocol.rs` | `server` | Protocol, composition and service-document routing |
| Configuration/service domain logic formerly in Server `state.rs` and `local_services.rs` | `environment` | Workspace-delivered appearance/client configuration, service intent, epochs and receipts |
| Agent/backend defaults and terminal settings formerly in Server `state.rs` | `chat`, `terminal` respectively | Domain configuration types and behavior, with Server composing wire projections |
| `server/src/chat.rs` | Server temporary bridge | Typed private envelopes around opaque Kotlin chat bodies until round 2 |

Each domain depends only on Environment. Domain structs can derive serde and JSON Schema for storage and export; they contain no method names, JSON-RPC IDs or transport decisions. The Server embeds those data types rather than maintaining duplicate representations. Cross-domain effects are explicit composition operations. In particular, FileWork returns a successful moved path before Server rebases Workspace references, and Server asks FileWork about dirty resources before changing a session's archive state.

Workspace's API takes typed state and operations and returns typed changes. FileWork takes typed Working Resources plus its root/store handles; it reports revisions and conflicts without accepting a client-computed layout. Terminal takes Environment runtime process handles, typed launch/read/resize options and terminal IDs. Its typed metadata includes generation, working directory, title, size and exit state. No domain accepts a raw method string as its business API.

The Environment store owns private document keys, bounded reads, atomic publication, backup, quarantine, upload staging and the workspace lock. FileWork owns ordinary workspace file IO and the no-symlink path policy. Server owns no direct workspace-file access. Configuration remains explicitly editable through FileWork with validators supplied by the owner; private state remains hidden.

The configuration/service extraction follows the later user-directed ownership decision. Server retains cross-domain transaction state and wire DTOs or re-exports; moving these types must keep the exported contract and goldens stable unless a rename is deliberate on both sides. Proxy configuration and assets use `.workspace/proxy/` with the `services.proxy` namespace and no migration.

## Typed JSON-RPC and contract export

The transport remains UTF-8 JSON objects separated by newlines, maximum 32 MiB, no batches, with a mandatory first `hello`. An envelope distinguishes a missing ID (notification) from an explicit null ID. String and arbitrary-precision numeric IDs are echoed without conversion through floating point, including the original numeric representation. Strict JSON validation rejects duplicate keys and trailing documents. Unknown envelope fields have a flattened named opaque remainder; unknown coding-agent bodies remain opaque and are never dropped or interpreted as approval.

Known method parameters and responses are serde structs or tagged enums. Protocol errors contain a numeric JSON-RPC code and typed data with a stable `kind`. Parse errors use -32700, invalid envelopes -32600, unknown methods -32601, invalid parameters -32602, and domain failures -32000. Domain errors carry domain meaning, and the Server maps them to this envelope. Failures cannot be encoded as an apparent successful method unless the existing operation explicitly has a successful conflict/result variant.

The method catalog is declared in Rust alongside the request types. It covers `hello`, `workspace.*`, `files.*`, `documents.*`, `client.config`, `environment.*`, `process.*`, `terminal.*`, `services.*` and the temporary `chat.*` bridge. `workspace.command` remains a discriminated `name`/`args` union so the current Android client continues to work. Method renaming and new notification-only subscription protocols are outside round 1.

Schema export derives from the actual Rust serde types, not a manually maintained list of JSON fields. The exporter writes `engine/protocol/contract.json`, a JSON Schema and golden request/response fixtures. The catalog includes the wire limits and ownership notes. A source-bound Rust test compares freshly generated output byte-for-byte with checked-in artifacts; regeneration is explicit. Golden fixtures are serialized typed Rust instances, including nullable fields, missing defaults, enum tags, success/error envelopes and precise IDs. They are compatibility examples, not exhaustive behavior tests.

In round 2, the pure JVM client decodes every Rust golden, compares its re-encoding structurally with the original, validates emitted client examples against the schema, and tests ID/null/missing-field behavior. Its method coverage must equal the Rust catalog. Vendor payload examples remain opaque until the Rust Chat port supplies their types. Changes to a schema and its golden require client compatibility evidence in the same change.

## Watches and lifecycle

Round 1 retains bounded long-poll subscriptions: `workspace.watch` waits on revision and releases the transaction lock; `chat.watch` is forwarded to the guest journal with epoch and revision. PTY reads wait at most the current protocol limit and release terminal registry locks. Closing the connection wakes pending watches, stops Chat and runtime processes, and discards partial uploads. Reattachment reads authoritative snapshots; it does not resume old executor tickets or silently retry ambiguous chat sends.

Restart orchestration serializes spawn with stop, captures running terminal identities, stops the guest Chat service, waits for actual process convergence, activates a verified pending generation and restores terminals through Runtime. Domain status reports measured outcomes. The Server must not infer readiness from an installed package, toggle or executable path.

## Terminal defect boundaries

The current code already has URL callbacks, candidate-file detection, OSC 7 working-directory tracking and touch-selection code. Their existence does not establish usable behavior on the device. Round 1 moves these responsibilities without claiming the four defects fixed.

For file opening, Terminal must keep bounded incremental OSC parsing across output chunks and reset it on lost scrollback or generation changes. The authoritative current directory must come from shell OSC 7 where available, with the launch directory as fallback. The agreed `terminal.resolvePaths` request is `{terminalId,generation,candidates:[string]}`. Its result is `{terminalId,generation,cwd,paths:[{text,path?,kind?,line?,column?}]}`; rejected items contain only `text`, and a stale generation fails the request. Resolved items identify workspace-relative paths and optional line/column. Server composes Terminal's guest-path mapping with FileWork's safe existence checks. The client must not reconstruct the host root or probe guessed directory listings. Guest home/system paths are not workspace files; private `.workspace` paths remain denied. URI decoding, localhost/host checks, traversal, Unicode and colon-suffixed diagnostics need tests.

Scrolling requires a consistent `(terminalId,generation,startOffset,nextOffset)` stream, reset indication and bounded retained output. Engine byte cursors describe output replay, not rendered rows; xterm owns wrapping, alternate-screen state and viewport row geometry. A large draggable thumb and fast scrolling need no second Engine scrollback database. The client anchors the viewport to xterm rows, preserves it during streaming, and clearly resets selection/viewport when Engine retention or generation changes invalidate the anchor.

## Later rounds: delivery tasks

| Round/task | Owned paths | Dependencies | Checks |
| --- | --- | --- | --- |
| R2 protocol client | `app/client/**`, Server contract fixtures, Gradle wiring | App module move; frozen Rust catalog | JVM codec/fixture/schema coverage, `rust-server`, `app-unit`, `android-apk` |
| R2 Rust Chat wiring | `engine/server/src/chat.rs`, Chat composition and contract variants | Chat parity suites and Environment runtime API | `chat`, `rust-server`, isolated API 28 chat/reconnect tests |
| R2 terminal path resolution | `engine/terminal/**`, `engine/filework/**`, Server terminal methods and schema | Typed domain APIs, client path callback | Traversal/OSC/chunk/generation unit cases; `rust-server`, offline terminal tests |
| R2 terminal interaction | App terminal and web assets (App owner) | Path-resolution contract | Link, file, selection and scrollbar API 28 device cases; no tablet |
| R3 acceptance and pruning | Engine integration tests, `docs/engine/**`, `docs/status.md`, reports | Integrated R2 build and complete docs move | Full source-bound host suites, both Android ABI builds, isolated device matrix; report hardware limits |
