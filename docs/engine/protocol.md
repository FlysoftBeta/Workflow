# Workspace Engine protocol

This document and the generated [`engine/protocol/contract.json`](../../engine/protocol/contract.json) catalog define the cross-component contract for Workflow 1.0.0. New workspace configuration and private state live in `<workspace-root>/.workspace/`. There is no legacy data import, migration, or cross-version protocol adapter.

## Transport and handshake

The implemented transport is bidirectional stdio to a local process. Messages are UTF-8 JSON-RPC 2.0 objects, one per line; batches are unsupported. Request IDs must be returned unchanged. Stdin and stdout are exclusively protocol channels, while credential-free diagnostics go to stderr. A frame may contain at most 32 MiB; binary data is base64-encoded in chunks of at most 65,536 raw bytes.

The first request must be `hello` with `{protocol:"workflow.workspace/1",clientId:"..."}`. Its result contains the same protocol string, `engineVersion`, `workspaceRoot`, and `capabilities`. A protocol mismatch is rejected immediately. Unknown methods return `-32601`, invalid parameters return `-32602`, and business failures return `-32000` with a structured `data.kind`; failures must never appear as silent success.

The Server's command line is:

```text
workflow-engine serve --root <user-file-root>
  [--runtime <workflow-runtime>] [--loader <workflow-loader>]
  [--apk <Android-APK>]
  [--image <image.tar.zst> --image-index <image.json>]
  [--tools <directory-containing-tools.json-and-tools.zip>]
```

Android packages the Server as `libworkflow-engine.so`, the container runtime as `libworkflow-runtime.so`, and the loader as `libworkflow-loader.so`. The APK carries the same architecture-specific tools pair under `assets/environment/tools/`; standalone execution supplies `--tools`. Codex is a guest tool, not an Android JNI library. Runtime CLI commands such as `run`, `install`, `verify`, `fsck`, and `probe` retain their separate runtime contract and acceptance workloads.

## Authoritative state

`workspace.snapshot {}` returns `{revision,state}`. `workspace.watch {afterRevision,timeoutMs}` returns the same shape, waiting for a newer revision or the timeout. `timeoutMs` is in `0..30000`, with a default of 30000. Waiting releases the state lock and does not prevent other commands from completing.

`workspace.command {name,args}` returns `{revision,state,value}`. The Server serializes mutations and returns the authoritative snapshot after a successful commit. The client's `StateFlow` is a disposable projection; Android does not persist another workspace repository or perform file I/O on its main thread.

`state` contains `status` (`ready` or `failed`), `failure`, `sessions`, `activeSessionId`, `pinned`, `drafts` keyed by path, `composers` keyed by resource ID, `disk` keyed by path, `config`, `configProblem`, `notices`, and `writeError`. The Rust export defines the contract; the Session, Workbench, PanelTarget, PanelView, DiskVersion, draft and composer encodings in [`StateCodec.kt`](../../app/client/src/main/kotlin/top/flysoftbeta/workflow/core/store/StateCodec.kt) implement its Kotlin bindings. They are definitions of this protocol, not an import format for old state. Vendor-specific adapter indexes use opaque documents rather than entering Server core.

The following are protocol command identifiers, not names dynamically dispatched through reflection. Results retain the semantics of the corresponding `WorkspaceStore` operation.

| Command | Arguments |
| --- | --- |
| `enterWorkbench` | `{}` |
| `createSession` | `{name?}` |
| `openInSeparateSession` | `{target}` |
| `activateSession`, `restoreSession`, `unpinSession` | `{id}` |
| `renameSession` | `{id,name}` |
| `archiveSession` | `{id,decision?}`, where `decision` is `save_all`, `keep_drafts`, or `discard` |
| `pinSession` | `{id,index}` |
| `runMaintenance`, `flush` | `{}` |
| `dismissNotice` | `{id}` |
| `applyLayout` | `{sessionId?,op}`; omitted session means the active session |
| `openFile` | `{path}` |
| `editFile` | `{path,text,shown}` |
| `saveFile` | `{path,text?}` |
| `resolveConflict` | `{path,resolution}`, where resolution is `keep_mine` or `use_disk` |
| `discardDraft`, `deletePath`, `trashPath`, `createDirectory` | `{path}` |
| `movePath`, `copyPath` | `{from,to}` |
| `restoreFromTrash` | `{id}` |
| `purgeTrash` | `{}` |
| `createFile` | `{path,data}` with base64 data; use upload for large files |
| `listDirectory` | `{path,showHidden}` |
| `editComposer` | `{draft,expectedRevision}` |
| `acknowledgeComposer` | `{submitted}` |
| `discardComposer`, `removeConversation` | `{conversationId}`; removal requires the user's confirmed action |
| `updateConfig` | `{config,expectedRevision}` |

Results are tagged by `kind`:

| Operation | Result variants |
| --- | --- |
| Save | `saved{version,text}`, `unchanged{version}`, `conflict{disk}`, `invalid{message}`, `failed{message}` |
| Archive | `archived{savedPaths}`, `needsDecision{resources}`, `saveConflict{paths}`, `invalid{path,message}`, `failed{message}`, `notFound` |
| File operation | `done`, `failed{message}` |
| Trash | `trashed{entry}`, `failed` |
| Restore | `restored{path}`, `failed` |
| Configuration | `updated{config}`, `conflict{revision}`, `blocked{problem}`, `failed` |

An archive with dirty resources and no decision returns `needsDecision`. A configuration revision conflict is a successful command envelope containing a conflict value: the client first accepts its authoritative snapshot, then reapplies the user's transformation to that configuration before retrying. It must not resend a stale full document unchanged.

`ResourceRef` is `{kind:"file",path}` or `{kind:"conversation",id}`. `openFile` returns `{path,disk,diskText,binary,tooLarge,draft}`. `listDirectory` returns an array of `{path,isDirectory,size,modifiedAt}` entries. Listing the root always includes the protected `.workspace` folder, regardless of `showHidden`. A `.workspace` listing contains only entries that Environment's allowlist makes visible; private state and agent credentials never appear, even with `showHidden`. The entry shape is unchanged, and the [FileWork reference](filework.md#paths-and-explicit-configuration) lists which visible entries are editable, read-only or fixed folders.

Layout operation `type` values use lower camel case: `open`, `focus`, `focusStack`, `close`, `move`, `splitStack`, `resizeSplit`, `resetSplit`, `resizeRegion`, `setRegionCollapsed`, `toggleRegion`, `setMaximized`, `switchParadigm`, `promoteConversation`, `returnToFiles`, `enterSolo`, `setChatSideStack`, `retarget`, `updateView`, `updateExplorer`, and `renamePath`. Fields follow the corresponding Kotlin constructor parameters; enum values are lowercase. Placement tags are `auto`, `inStack`, and `splitEdge`; drop tags are `center`, `tab`, `edge`, and `editorEdge`. The Server must preserve the [layout invariants and archive protections](workspace.md) rather than trust a client-computed layout.

## Files and uploads

`files.read {path,offset,length}` returns `{data,nextOffset,eof,size}`, with `length` at most 65,536. Paths must remain beneath the workspace root and cannot traverse symlinks or private state. File commands accept visible `.workspace` entries. Writes are limited to editable configuration: `.workspace/config.json`, safe paths beneath `.workspace/proxy/`, the classified directories of other services and the allowlisted entries of the Codex and Claude Code homes below `.workspace/agents/`. Writing a fixed folder fails with `read_only`; private paths, including `.workspace/cache/`, fail as invalid parameters.

`files.upload.begin` accepts `{directory,name,size}` for an import whose final destination must be allocated by the Engine. `directory` is workspace-relative and `name` is a single filename. The Engine validates both and returns `{uploadId}`. The exact-destination form `{path,size}` remains available for file creation and explicit service assets; it rejects an existing destination rather than allocating a variant.

`files.upload.chunk {uploadId,offset,data}` must use the previous `nextOffset` and returns the next one. `files.upload.commit {uploadId}` requires the declared byte count, synchronizes the staging file, and publishes without replacing an existing destination. For the allocating form, the Engine chooses a free filename variant while committing, including when another writer races publication. Success is `{kind:"done",path:<final-workspace-relative-path>}`. Clients use that returned path instead of predicting a name. `files.upload.cancel {uploadId}` abandons staging. Incomplete uploads remain private and never become user files merely because a connection ends.

Android supplies a length and stream factory and sends 64 KiB chunks. URI/camera staging is capped at 512 MiB per import, is disposable, and is bound to the connection instance that opened the picker. A source-length change, failure, cancellation or connection switch cancels or rejects the operation; it cannot redirect a result into another Workspace. The client does not list filenames or decide collision policy locally.

## Documents and local services

`documents.read {namespace,key}` returns `{document,revision}`, where `document` is a string or null. `documents.write {namespace,key,document,expectedRevision?}` returns `{revision}`; `documents.quarantine` addresses the same namespace and key. Server maps safe identifiers to Environment's document API, which owns path construction. These calls do not accept arbitrary filesystem paths. The conversation index uses namespace `chat` and key `conversations`.

For namespace `services.<serviceId>`, a key is a safe single filename and the document is the original service file selected by Environment. The proxy namespace remains `services.proxy`, with `config.yaml` addressing exactly the same text as the explicit editor path `.workspace/proxy/config.yaml`; its providers and logs use `.workspace/proxy/` too. Other service files remain under `.workspace/services/<serviceId>/`. There is no migration from the earlier proxy directory. A sidecar stores its content hash and revision, not a second copy of its body. Reads and compare-and-set writes inspect the original file first, so editor or external changes invalidate stale revisions. Service documents are UTF-8 text of at most 16 MiB, with format interpreted by the service. Identifiers reject slashes and `.` or `..`. The explicit file API additionally permits safe multi-segment service asset paths, including binary assets, but still rejects symlinks and parent traversal.

`services.status {}` returns `{services,desired,control}`. `services` contains measured reports, `desired` retains workspace service configuration, and `control.proxy` contains the Engine's proxy control projection. Desired state and successful execution are separate facts.

The proxy executor obtains a boot-bound lease through `services.executor.register {serviceId:"proxy",executorId}` → `{epoch,control}`. Re-registering the same active executor ID is idempotent; a different executor or Engine process retires its predecessor. `services.executor.retire {serviceId:"proxy",epoch}` → `{control}` invalidates the lease. Epochs fence stale instances; they are not an authentication mechanism.

`services.command {serviceId:"proxy",epoch,name,args}` commits an intent before returning a ticket `{operation:{id,name,args,config?},control}`. Supported device operations are `start`, `stop`, `checkConfig`, `refreshProviders`, `setMode {mode}`, `select {group,node}`, `testNode {name,url?,timeoutMs?}`, and `testGroup {group,url?,timeoutMs?}`. Modes are `rule`, `global` or `direct`; test timeouts are 1–60000 ms. Start, configuration check and provider refresh receive current canonical `{text,revision}` configuration in the ticket. Only one device operation may remain pending; an uncertain operation is not automatically replayed.

Configuration-only commands run in Engine: `ensureConfig {}` and `importConfig {text,expectedRevision}` return `{operation:null,config:{text,revision},control}`. The Engine creates template secrets and performs compare-and-set import. `publishLog {text,expectedRevision}` publishes up to 1 MiB of already-redacted log text and returns `{operation:null,revision,control}`.

`services.complete {serviceId:"proxy",epoch,operationId,success,state}` requires the current lease and pending operation ID, records measured state, and returns `{serviceId,state,revision,control}`. A successful start requires a running phase and positive PID; stop requires a stopped phase, no PID and no unconfirmed stop; mode and node selection require matching measurements. A failed completion preserves desired intent while recording failure. `services.report {serviceId:"proxy",epoch,state}` publishes measurements with the same receipt shape but never completes a ticket. Proxy reports are bounded to 1 MiB and exclude secrets.

`control` contains `desired:{running,mode,selections}`, `operation` or null, `executor:{epoch,active}`, and `measuredConfirmed`. Operation status is `pending`, `completed`, `failed` or `interrupted`. Replacing or retiring an executor interrupts a pending operation and clears measured confirmation. An installed binary, a UI switch or a stale report cannot establish success.

Other local measurements use `services.report {serviceId,state}`. The Engine persists and publishes them with `{revision,serviceId,state}`.

The network executor reports `{serviceId:"network",state:{dnsServers:[...],connected:boolean}}`. Environment validates IPv4/IPv6 addresses, accepts at most 16 entries, and writes an internal resolver file with mode 0644, bound only to guest `/etc/resolv.conf`. It never changes Android or host DNS, routing, or other applications.

`client.config {clientId}` returns `{revision,config}` with workspace-delivered `appearance`, `overlay`, `launcher`, and `terminal` settings. Android may cache these settings, bound to the connection ID, together with connection profiles. It may use that appearance on the connection screen, but cannot reconstruct sessions, drafts, or workspace configuration from the cache. Logs must be redacted before a service submits them as documents.

## Environment lifecycle

Environment means the full runtime, including tools, packages, variables, mounts, processes, and lifecycle. The `environment` section of `.workspace/config.json` declares version 1 settings, including Python and Node arrays, packages, variables, and ordered `{id,run,user:"work"|"root"}` post-scripts. Missing language arrays use image defaults, empty arrays disable that managed language, and the first entry of a nonempty array is the default. Invalid or duplicate version specifications are rejected. `root` here is virtual guest root, not device root. The [environment document](environment.md) gives the full declaration semantics.

First use explicitly requests `environment.reconcile {retry?}`. Once enrolled, the Server monitors saved and external declaration changes, including after reconnect. A connection used only for files or local services does not extract an unused environment. Failed input is remembered and does not repeatedly run without a changed declaration or explicit retry.

`environment.status {}` reports `phase` (`not_installed`, `installing`, `building`, `ready`, `needs_restart`, or `failed`), `usable`, `progress` in `0..1` or null, `stage`, `error`, `runningProcesses`, and `architecture`. Builds publish queryable progress and leave file and session commands usable. They retain the old usable generation, home, user files, and toolchain cache. Post-scripts run only for a new configuration build, in order, and activation requires their success.

`environment.restart {}` activates only an already verified pending generation. If none exists, it returns status without stopping processes. Otherwise it stops Engine-owned process groups, waits up to ten seconds, and switches only after they exit. A successful build with no running processes can activate directly. Post-script home changes use private mode-0700 staging and content-version checks when merged at activation. Concurrent user changes that the script did not touch survive; a conflict leaves the old generation active and reports failure. Staged home contents do not belong in backups or release artifacts.

## Engine-managed tools

`environment.tools.status {}` returns `{revision,tools:[...]}`. Each tool has `id`, `version`, `architecture`, `binary`, `phase`, `progress`, `error`, and `operationId`; unavailable values can be null. The catalog includes the mandatory `codex` and optional `claude`. Phases are `not_installed`, `installing`, `verifying`, `ready` or `failed`.

`environment.tools.install {toolId:"claude",retry?:boolean}` starts or observes an Engine-owned asynchronous job and returns the current tools projection. Failed unchanged work requires explicit retry. The Engine selects the pinned download, validates length and SHA-256, runs the measured version check, and publishes its outcome. Android does not supply an executable path, release version, URL or script. A restart marks interrupted install/verification work failed rather than inventing completion.

## Chat resources

`chat.snapshot {}` returns `ChatSnapshot {epoch,revision,state,metadata}` using the shared [`ChatWire`](../../app/client/src/main/kotlin/top/flysoftbeta/workflow/agent/rpc/ChatWire.kt) codec. `metadata` contains `conversations`, `available`, `loginMethods`, `permissions`, `defaultBackend`, and `processEpochs`. Tagged classes use `_type`; enums use their shared uppercase names and maps with structured keys use alternating key/value arrays. Vendor values remain lossless JSON, including string versus numeric request IDs. Clients must use the shared codec rather than invent another encoding.

Snapshots larger than 1 MiB use frozen transfers: `{transferId,offset,data,nextOffset,eof}`, with base64 data in at most 65536-byte raw chunks. Continue with `chat.snapshot {transferId,offset:nextOffset}` and decode the complete UTF-8 snapshot. A snapshot is at most 64 MiB; two frozen transfers are retained. An expired transfer requires a new snapshot.

`chat.watch {epoch,afterRevision,timeoutMs?}` accepts 0–30000 ms and returns `ChatUpdate {epoch,revision,events,metadata,resnapshot}`. The journal retains up to 4096 changes and 8 MiB of event data; a reply contains at most 512 changes and 1 MiB of event data. Metadata corresponds to exactly the returned revision. A different service epoch, unavailable cursor or oversized single change requests `resnapshot:true`; the client replaces its projection rather than guessing missing changes.

`chat.command {name,args}` returns a direct command value, not a workspace snapshot. Operations are `newConversation`, `ensureConversation`, `open`, `loadEarlier`, `send`, `interrupt`, `cancelQueued`, `respond`, `rename`, `archive`, `deleteConversation`, `compact`, `refreshUsage`, `rawRequest`, `fork`, `setPermissions`, `rememberSelection`, `setBackend`, `login`, `cancelLogin`, `logout`, `refreshAccount`, `warmUp`, and `flush`. Conversation operations use `id`; backend operations use `kind`, while `newConversation` takes optional `backend` and `id`. Shared request/response, settings and login values use `ChatWire`. IDs are returned as strings, void operations as null, and the advanced console returns its original JSON value.

A send includes `{id,text,attachments,settings,mode,operationId,submitted}`. Attachments are `{path,mimeType?}`; `submitted` is the exact composer draft encoding, including revision. Engine records the operation intent and argument fingerprint before vendor dispatch, records acceptance, and acknowledges only that submitted draft. Retrying an accepted operation returns the original message ID; a reused ID with different arguments or ambiguous outcome fails without another vendor submission. Android never issues a separate successful-send acknowledgement.

`respond` includes `{key,response,processEpoch}`. The service checks that the process epoch matches and the original request is still open before forwarding the user's choice. Neither the Rust supervisor nor the shared client reducer auto-approves vendor requests. Service epoch changes and per-backend process epoch changes are distinct.

The Rust Chat service runs inside Server; there is no private chat RPC. Its errors keep the retained codes: invalid arguments, undecodable arguments and unoffered decisions are `-32602`, other failures `-32000`, both with error kind `chat`. Object members of vendor JSON are re-encoded in canonical key order; values, unknown members and exact number tokens are preserved.

## Terminal resources

Android addresses stable terminal resources, not generic process IDs. Metadata is `{id,ordinal,generation,cwd,title,customTitle,status,exitCode?,error?,rows,columns}`, with guest-absolute cwd and status describing the actual process. Terminal identity and metadata are Engine-owned and persisted; output is an Engine process-lifetime ring, not durable transcript storage.

| Method | Contract |
| --- | --- |
| `terminal.create` | `{directory,rows?,columns?}` with workspace-relative directory → metadata |
| `terminal.attach` | `{terminalId,rows?,columns?}` → metadata; attaches without restarting an existing live or exited process, and rehydrates a resource with no process after Server restart |
| `terminal.restart` | `{terminalId,rows?,columns?}` → metadata after an explicit resource restart |
| `terminal.status` | `{terminalId}` → metadata |
| `terminal.read` | `{terminalId,generation,offset,maxBytes?,waitMs?}` → `{data,startOffset,nextOffset,eof,exitCode?,reset,terminal}`; raw chunks at most 65536 bytes and waits at most 1000 ms |
| `terminal.write` | `{terminalId,data}` with bounded base64 bytes |
| `terminal.resize` | `{terminalId,rows,columns}` |
| `terminal.stop` | `{terminalId,force?}` |
| `terminal.wait` | `{terminalId,timeoutMs}` → `{running,exitCode?}` |
| `terminal.rename` | `{terminalId,title}` → metadata; null or blank removes the custom title |
| `terminal.clear` | `{terminalId}` → metadata after clearing retained output and advancing generation |
| `terminal.resolvePaths` | `{terminalId,generation,candidates:[string]}` → `{terminalId,generation,cwd,paths:[{text,path?,kind?,line?,column?}]}` |

A generation mismatch resets reading to offset zero. Clipped reads disclose the retained starting offset. The client applies a frame's metadata, generation reset and bytes together, and continues observing after EOF so another client's restart becomes visible. Closing a view cancels only its attachment. Engine examines references in live workspace sessions, retaining an unreferenced terminal for a fifteen-second grace period before cleanup. A real environment activation restores previously running terminal resources; a no-op restart leaves them unchanged.

`terminal.resolvePaths` resolves output candidates against the current Engine-owned cwd under terminal identity and generation checks. Server composes Terminal context with FileWork existence and path-safety checks. An accepted item contains the original `text`, workspace-relative `path`, its `kind` and optional zero-based line/column location. A rejected candidate returns only its `text`; a stale generation fails with `stale_terminal` instead of resolving against another process lifetime. Guest home/system paths, nonlocal file URIs, symlinks, parent escapes and private Engine paths do not become workspace targets. Android opens or reveals returned paths and does not guess candidates by calling `listDirectory`.

## Processes

The generic process API remains an Engine execution primitive. Product terminal UI uses terminal resources and chat UI uses chat commands. All product terminals and coding agents execute inside the environment. The Engine uses the runtime and loader supplied by the APK through distinct argv elements, without shell concatenation or host execution fallback.

| Method | Arguments and result |
| --- | --- |
| `process.spawn` | `{argv?,cwd:"/workspace",env:{},terminal:false,rows:24,columns:80,label?}` → `{processId}`; `terminal:true` allocates a PTY, and omitted argv starts a login bash for `work` |
| `process.read` | `{processId,stream:"stdout"|"stderr",offset,maxBytes,waitMs}` → `{data,startOffset,nextOffset,eof,exitCode?}`; `waitMs` is at most 1000 |
| `process.write` | `{processId,data}` |
| `process.resize` | `{processId,rows,columns}` |
| `process.stop` | `{processId,force?}` |
| `process.wait` | `{processId,timeoutMs}` → `{running,exitCode?}` |

Output streams use bounded ring buffers; a reader whose offset was clipped receives the actual `startOffset`. Bytes are transported unchanged. Neither the Server nor the runtime consumes or approves vendor protocol requests.

## Connection ownership

Android uses `WorkspaceConnectionConfig`, `WorkspaceBootstrapper`, and `WorkspaceTransport`. Embedded bootstrap implements that same interface. Remote and SSH are abstraction and configuration types only in 1.0.0; attempts to use them explicitly return unsupported.

On connection failure the client clears the current connection, cancels all its session consumers, retires the corresponding foreground-service leases, and marks the old projection failed. Workspace mutations remain disabled until reconnection obtains a fresh authoritative snapshot. There is no silent local-store fallback. Drafts already accepted by the Engine remain owned by it.

The embedded stdio transport is not a detached daemon: closing it shuts down the Server, chat service and owned processes. Reconnection loads committed state and rehydrates resource metadata; it does not promise uninterrupted process or output survival.

## Rust contract export

`workflow-server` owns the typed envelopes, method catalog and error mapping. Domain payload types derive serde and schema; they do not depend on JSON-RPC. Run the Server binary with `export-contract --out engine/protocol` to regenerate the catalog, JSON Schema and typed golden fixtures. The Rust drift test compares all generated bytes with the checked-in artifacts. `app/client` contains generated `EngineBindings` typed views derived from the complete Server schema. They retain the original JSON representation, preserving unknown fields, explicit null versus missing values and precise numeric IDs. The existing `WorkspaceWire` mapping remains inside a compatibility adapter, with Kotlin package and serialization identities unchanged. Generation and fixture checks establish only their tested scope; source-bound results must identify catalog coverage and round-trip checks. Notifications are distinguished from explicit null IDs, and arbitrary-precision request-ID tokens are echoed unchanged.
