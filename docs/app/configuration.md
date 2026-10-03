# Workspace-delivered configuration

Engine is the source of truth for workspace configuration. `app/client` provides the pure JVM bindings and synchronization logic; `app/android` caches permitted local projections and connects them to platform settings. Only connection profiles are App-authored durable state. Cached theme, palette, fonts, terminal preferences and proxy files cannot become an independent workspace repository.

Environment owns appearance, overlay/client configuration and service/proxy intent and receipts. Chat owns agent/backend defaults; Terminal owns terminal settings. Server composes these domain types into the stable [protocol](../engine/protocol.md) and persists the revisioned configuration through Environment's store. A future settings panel consumes those same APIs rather than inventing another file format or owner.

## Revisions and reconnect

After `hello`, initial connection reads the authoritative snapshot and `client.config {clientId}`. The latter returns `{revision,config}` with `appearance`, `overlay`, `launcher` and `terminal` settings. A permitted cache is bound to its connection/workspace identity and last accepted revision. It can supply appearance while selecting a connection, but cannot restore sessions, drafts, service intent or private Engine state.

Local edits submit the last observed `expectedRevision`. On conflict, the client first accepts the returned authoritative projection, then reapplies the user's intended transformation to that current state before retrying. Repeating the same stale whole document would overwrite unrelated changes and is not allowed. Unknown configuration fields survive typed decode/encode and updates.

Reconnect replaces stale projections. Failed reads remain failures; old cache contents cannot be used as proof of current service configuration or measured capability. A connection switch cancels its consumers and staging work, and late results must be checked against the originating connection instance.

## Proxy files and receipts

The canonical proxy root is `.workspace/proxy/`. `services.proxy` remains its document namespace: `documents.read` with key `config.yaml` and the editor path `.workspace/proxy/config.yaml` reach the same original bytes. Provider assets and the bounded redacted `runtime.log` use that same directory. Version 1.0.0 does not migrate `.workspace/services/proxy/` or other historical layouts.

Sidecars contain a content hash and revision, not a second document body. Reading or compare-and-set writing observes the original file first, so external/editor changes invalidate stale revisions. Initial template creation, import and log publication use Engine-owned service commands; secrets are created there and are not written into logs, fixtures or backups.

The Android executor stages current configuration and provider bytes under a disposable connection cache. It cannot substitute stale files after an authoritative read fails. Each local operation uses a boot-bound executor epoch and Engine ticket; only a matching measured completion acknowledges its outcome. Unknown or ambiguous outcomes are never replayed automatically. [Proxy](proxy.md) defines the lifecycle and cleanup details.

## Validation boundary

Codec and synchronization checks must cover preserved unknown fields, Rust contract examples, concurrent revision conflicts, reconnect replacement and late callbacks from retired connections. Service checks also cover stale epochs, external file changes and failed measured completion. Integrated device acceptance is required for platform application of settings and actual proxy execution; relocation into `app/client` alone proves neither. Follow [testing](../development/testing.md) and record actual evidence in [status](../status.md).
