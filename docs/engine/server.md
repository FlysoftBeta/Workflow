# Server

`engine/server` is package `workflow-server`, the composition root of the Engine. Its executable remains `workflow-engine`, packaged on Android as `libworkflow-engine.so`. Server owns typed JSON-RPC envelopes, request validation, transport lifetime, bounded worker scheduling, subscriptions and operations spanning domain crates. It maps the [protocol](protocol.md) to their APIs. Domain configuration, service policy, file IO and concrete guest execution belong to their owners rather than the transport layer.

The local command is `workflow-engine serve --root <user-file-root>`. Runtime, loader, customized image and verified tools payload locations are supplied explicitly. Missing components cause explicit failure; there is no host-shell or historical Kotlin workspace-store fallback.

## Domain composition

[Workspace](workspace.md) owns sessions and layouts, [FileWork](filework.md) owns files and Working Resources, [Terminal](terminal.md) owns terminal identities and settings, and [Environment](environment.md) owns the store, lifecycle, workspace-delivered configuration and local service state machine. [Chat](chat.md) owns conversation behavior and backend defaults. Domain structs can derive serde/schema without knowing method names, JSON-RPC IDs or transport policy.

Server composes sessions and drafts into one persisted `state/workspace.json` transaction. A file move succeeds through FileWork before Server rebases session targets and attachment references. Archive operations consult dirty resources and file versions before changing session state. Closing a panel changes its layout only. Configuration combines Environment-owned appearance/client settings, Chat preferences and Terminal settings in the existing wire shape; that composition does not make Server the domain owner of those settings.

`main.rs` handles bootstrap and dispatch, protocol modules define frames and method types, and Server state code coordinates domain results and transaction publication. The retained Chat bridge supervises the guest Kotlin service while Rust Chat parity is incomplete. It uses typed private envelopes around explicitly opaque service bodies; it must not be mistaken for a completed Rust adapter port.

## Transactions and recovery

Environment supplies the exclusive workspace process lock and typed atomic store. Publication writes and synchronizes a temporary file, renames it atomically, and synchronizes the containing directory before Server acknowledges the new revision. Workspace settings live in `.workspace/config.json`; the environment declaration lives in `.workspace/env.json`.

Recovery retains a valid backup and preserves corrupt original bytes under `.workspace/corrupt/`. Unknown state, draft and composer formats stay read-only rather than being migrated. A damaged environment record does not block access to ordinary files and sessions. Invalid external configuration leaves the last valid configuration in use and exposes `configProblem`; a parse failure does not authorize overwriting it.

Layout operations preserve the [Workspace invariants](workspace.md). A composer acknowledgement clears only the exact submitted owner, revision, text and attachments, while an empty tombstone retains monotonic revision identity. Archive save preflight and file publication follow [FileWork](filework.md).

## Services and execution

Environment owns service intent, executor epochs, operation receipts and resolver generation. Server exposes those domain operations through `services.*` and revisioned `documents.*` methods. A proxy ticket is committed before local Android execution; only a matching measured completion can finish it. Reports alone cannot complete pending intent. Proxy files have the canonical root `.workspace/proxy/`, while the document namespace remains `services.proxy`. Network reports generate a resolver file bound into the guest and never change host settings.

Server orchestrates restart across Chat, Terminal and Environment. It stops managed processes, waits for actual convergence, activates a verified pending generation, then restores previously running terminals through their domain API. With no verified pending generation, restart leaves live resources untouched. Environment builds and process waits release the workspace transaction lock.

The production Chat bridge still starts `:engine-chat` on the bundled guest JRE. That service owns vendor processes, conversation history, index bookkeeping and the send ledger. Allowlisted private callbacks reach domain APIs without a writable private-state mount. Removing the bridge, JRE or Kotlin implementation requires the [Chat cutover gate](chat.md#rust-port-and-cutover-gate).

The embedded Server exits on stdio EOF and stops its owned process groups. A persistent reaper thread holds the parent-death relationship instead of attaching it to a short-lived request thread. Metadata can rehydrate after restart; process output rings are not durable.

## Contract export and bounds

Rust request, response, notification and error types generate [`engine/protocol/contract.json`](../../engine/protocol/contract.json), schema and golden examples. Run `workflow-engine export-contract --out engine/protocol` to regenerate them. The Rust drift test compares checked-in artifacts with current type-derived output. The generated `EngineBindings` views in `app/client` derive from the Server schema and preserve unknown fields, null/missing distinctions and numeric ID precision. The retained `WorkspaceWire` adapter preserves existing client projections. Source-bound fixture and catalog checks must still establish codec and method coverage; relocation alone is not parity evidence.

| Resource | Bound |
| --- | --- |
| Protocol frame | 32 MiB |
| Raw blob chunk | 64 KiB |
| Editable text and service document | 16 MiB |
| Persisted workspace state | 24 MiB |
| Frozen Chat snapshot | 64 MiB |
| Process output ring, per stream | 4 MiB |
| Pending long requests / active processes | 128 / 128 |
| Incomplete uploads | 16, each at most 8 GiB |

A combined reply that exceeds its frame bound fails explicitly with `too_large`. Large binary data uses chunked file APIs. Bounded journals require a fresh snapshot when a cursor is no longer retained. These limits do not establish acceptance for every extreme combination.

`tools/workflow check rust-server` covers the domain and protocol contracts, including export drift. The additional black-box and real-image lifecycle harnesses are under `engine/server/tools/`. Historical [Server](../archive/implementation-1.0.0/reports/rewrite/rust-workspace.md) and [integration](../archive/implementation-1.0.0/reports/rewrite/integration.md) reports substantiate only their recorded source and device matrix. Current limits are in [status](../status.md).
