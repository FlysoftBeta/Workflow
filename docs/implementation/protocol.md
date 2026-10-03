# Workspace Engine protocol

This document and [`engine/protocol/contract.json`](../../engine/protocol/contract.json) define the cross-component contract for Workflow 1.0.0. New workspace configuration and private state live in `<workspace-root>/.workspace/`. There is no legacy data import, migration, or cross-version protocol adapter.

## Transport and handshake

The implemented transport is bidirectional stdio to a local process. Messages are UTF-8 JSON-RPC 2.0 objects, one per line; batches are unsupported. Request IDs must be returned unchanged. Stdin and stdout are exclusively protocol channels, while credential-free diagnostics go to stderr. A frame may contain at most 32 MiB; binary data is base64-encoded in chunks of at most 65,536 raw bytes.

The first request must be `hello` with `{protocol:"workflow.workspace/1",clientId:"..."}`. Its result contains the same protocol string, `engineVersion`, `workspaceRoot`, and `capabilities`. A protocol mismatch is rejected immediately. Unknown methods return `-32601`, invalid parameters return `-32602`, and business failures return `-32000` with a structured `data.kind`; failures must never appear as silent success.

The Server's command line is:

```text
workflow-engine serve --root <user-file-root>
  [--runtime <workflow-runtime>] [--loader <workflow-loader>]
  [--apk <Android-APK>] [--native-dir <nativeLibraryDir>]
  [--image <image.tar.zst> --image-index <image.json>]
```

Android packages the Server as `libworkflow-engine.so`, the container runtime as `libworkflow-runtime.so`, and the loader as `libworkflow-loader.so`. Runtime CLI commands such as `run`, `install`, `verify`, `fsck`, and `probe` retain their separate runtime contract and acceptance workloads.

## Authoritative state

`workspace.snapshot {}` returns `{revision,state}`. `workspace.watch {afterRevision,timeoutMs}` returns the same shape, waiting for a newer revision or the timeout. `timeoutMs` is in `0..30000`, with a default of 30000. Waiting releases the state lock and does not prevent other commands from completing.

`workspace.command {name,args}` returns `{revision,state,value}`. The Server serializes mutations and returns the authoritative snapshot after a successful commit. The client's `StateFlow` is a disposable projection; Android does not persist another workspace repository or perform file I/O on its main thread.

`state` contains `status` (`ready` or `failed`), `failure`, `sessions`, `activeSessionId`, `pinned`, `drafts` keyed by path, `composers` keyed by resource ID, `disk` keyed by path, `config`, `configProblem`, `notices`, and `writeError`. The Session, Workbench, PanelTarget, PanelView, DiskVersion, draft, and composer encodings in [`StateCodec.kt`](../../core/src/main/kotlin/top/flysoftbeta/workflow/core/store/StateCodec.kt) define the current wire shapes. They are definitions of this protocol, not an import format for old state. Vendor-specific adapter indexes use opaque documents rather than entering Server core.

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

`ResourceRef` is `{kind:"file",path}` or `{kind:"conversation",id}`. `openFile` returns `{path,disk,diskText,binary,tooLarge,draft}`. `listDirectory` returns an array of `{path,isDirectory,size,modifiedAt}` entries.

Layout operation `type` values use lower camel case: `open`, `focus`, `focusStack`, `close`, `move`, `splitStack`, `resizeSplit`, `resetSplit`, `resizeRegion`, `setRegionCollapsed`, `toggleRegion`, `setMaximized`, `switchParadigm`, `promoteConversation`, `returnToFiles`, `enterSolo`, `setChatSideStack`, `retarget`, `updateView`, `updateExplorer`, and `renamePath`. Fields follow the corresponding Kotlin constructor parameters; enum values are lowercase. Placement tags are `auto`, `inStack`, and `splitEdge`; drop tags are `center`, `tab`, `edge`, and `editorEdge`. The Server must preserve the [layout invariants and archive protections](workspace.md) rather than trust a client-computed layout.

## Files and uploads

`files.read {path,offset,length}` returns `{data,nextOffset,eof,size}`, with `length` at most 65,536. Paths must remain beneath the workspace root and cannot traverse symlinks or private state. Explicit configuration editing permits `.workspace/config.json`, `.workspace/env.json`, and safe paths beneath `.workspace/services/<serviceId>/`.

An upload begins with `files.upload.begin {path,size}`, which returns `{uploadId}`. Each `files.upload.chunk {uploadId,offset,data}` must use the previous `nextOffset` and returns the next one. `files.upload.commit {uploadId}` returns a file-operation result only after receiving the declared byte count and atomically publishing the new file without replacing an existing destination. `files.upload.cancel {uploadId}` abandons the staging file. Incomplete uploads remain in a restricted staging directory until cleanup; disconnection never turns a partial file into a user file.

The client's import port accepts a length and an `InputStream` factory. Known bytes or an already staged file provide their length directly and stream in 64 KiB chunks. A changed source length, exception, or cancellation cancels the upload. The client does not create another persistent copy of workspace file contents.

## Documents and local services

`documents.read {namespace,key}` returns `{document,revision}`, where `document` is a string or null. `documents.write {namespace,key,document,expectedRevision?}` returns `{revision}`; `documents.quarantine` addresses the same namespace and key. The Server constructs paths from safe identifiers. These calls do not accept arbitrary filesystem paths. The conversation index uses namespace `chat` and key `conversations`.

For namespace `services.<serviceId>`, a key is a safe single filename, and the document is the original file `.workspace/services/<serviceId>/<key>`. For example, `services.proxy` with `config.yaml` addresses exactly the same text as the explicit editor path `.workspace/services/proxy/config.yaml`. A sidecar stores its content hash and revision, not a second copy of its body. Reads and compare-and-set writes inspect the original file first, so editor or external changes invalidate stale revisions. Service documents are UTF-8 text of at most 16 MiB, with format interpreted by the service. Identifiers reject slashes and `.` or `..`. The explicit file API additionally permits safe multi-segment service asset paths, including binary assets, but still rejects symlinks and parent traversal.

`services.report {serviceId,state}` accepts measured results from a local capability executor. The Engine persists and publishes those results; a UI switch or an installed executable cannot establish that an operation succeeded. `services.status {}` returns `{services,desired}`, separating stored reports from the service configuration in the workspace document.

The network executor reports `{serviceId:"network",state:{dnsServers:[...],connected:boolean}}`. The Server validates IPv4/IPv6 addresses, accepts at most 16 entries, and writes an internal resolver file with mode 0644, bound only to guest `/etc/resolv.conf`. It never changes Android or host DNS, routing, or other applications.

`client.config {clientId}` returns `{revision,config}` with workspace-delivered `appearance`, `overlay`, `launcher`, and `terminal` settings. Android may cache these settings, bound to the connection ID, together with connection profiles. It may use that appearance on the connection screen, but cannot reconstruct sessions, drafts, or workspace configuration from the cache. Logs must be redacted before a service submits them as documents.

## Environment lifecycle

Environment means the full runtime, including tools, packages, variables, mounts, processes, and lifecycle. `.workspace/env.json` declares version 1 settings, including Python and Node arrays, packages, variables, and ordered `{id,run,user:"work"|"root"}` post-scripts. Missing language arrays use image defaults, empty arrays disable that managed language, and the first entry of a nonempty array is the default. Invalid or duplicate version specifications are rejected. `root` here is virtual guest root, not device root. The [environment document](environment.md) gives the full declaration semantics.

First use explicitly requests `environment.reconcile {retry?}`. Once enrolled, the Server monitors saved and external declaration changes, including after reconnect. A connection used only for files or local services does not extract an unused environment. Failed input is remembered and does not repeatedly run without a changed declaration or explicit retry.

`environment.status {}` reports `phase` (`not_installed`, `installing`, `building`, `ready`, `needs_restart`, or `failed`), `usable`, `progress` in `0..1` or null, `stage`, `error`, `runningProcesses`, and `architecture`. Builds publish queryable progress and leave file and session commands usable. They retain the old usable generation, home, user files, and toolchain cache. Post-scripts run only for a new configuration build, in order, and activation requires their success.

`environment.restart {}` activates only an already verified pending generation. If none exists, it returns status without stopping processes. Otherwise it stops Engine-owned process groups, waits up to ten seconds, and switches only after they exit. A successful build with no running processes can activate directly. Post-script home changes use private mode-0700 staging and content-version checks when merged at activation. Concurrent user changes that the script did not touch survive; a conflict leaves the old generation active and reports failure. Staged home contents do not belong in backups or release artifacts.

## Processes

All product terminals and coding agents execute inside the environment. The Engine uses the runtime and loader supplied by the APK through distinct argv elements, without shell concatenation or host execution fallback.

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
