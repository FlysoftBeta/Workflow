# Complete Engine and client separation

Status: implemented with targeted integrated acceptance. Owner: coordinator. Updated: 2026-10-03.

## Problem and intended outcome

The originating ownership gap was that Android still owned agent adapters, conversation orchestration, tool installation and terminal resource lifecycle despite the Engine boundary. The user explicitly requests completing that boundary. Android retains presentation, connection profiles, disposable projections and platform capability executors configured by the Workspace. Engine owns all workspace business operations and resource lifecycles. No remote or SSH implementation is introduced.

## Implementation agreement

Rust remains the public Workspace RPC and persistence authority. A pure JVM chat component under `engine/chat` reuses the tested vendor adapters in `agent`, supervised by Rust and executed inside the environment. A verified offline Linux JRE is a prerequisite; there is no Android-JVM or host execution fallback. Shared immutable models, their wire codec and disposable reducers live in `agent-model`. Android's production dependency graph excludes vendor adapters, vendor process launchers and the chat service.

The Engine distribution supplies an architecture-specific tools payload independently of Android native libraries. It includes Codex, the guest JRE and chat service; Claude remains a pinned optional Engine-managed install. APK assets may transport the Engine distribution, but Android does not choose tool versions, download URLs, executable paths or lifecycle policy. Existing environment generations receive managed tool bindings without rerunning user post-scripts or importing legacy state.

### Chat RPC contract

Public methods are `chat.snapshot`, `chat.watch` and `chat.command`. The command request is `{name,args}`. The service implements the existing hub operations with the same lower-camel-case names: `newConversation`, `ensureConversation`, `open`, `loadEarlier`, `send`, `interrupt`, `cancelQueued`, `respond`, `rename`, `archive`, `deleteConversation`, `compact`, `refreshUsage`, `rawRequest`, `fork`, `setPermissions`, `rememberSelection`, `setBackend`, `login`, `cancelLogin`, `logout`, `refreshAccount`, `warmUp` and `flush`. Engine determines process setup, availability and login methods. Mutating replies return the command value; projections arrive through snapshots and watches.

The shared `top.flysoftbeta.workflow.agent.rpc.ChatWire` codec defines the exact tagged DTOs and uses lossless JSON elements for raw vendor values. `ChatSnapshot` contains a service epoch, monotonic revision, normalized `AgentState`, conversation entries, measured availability, login methods and permission defaults. `ChatUpdate` carries the service epoch, next revision, ordered authoritative `AgentEvent` changes and replacement metadata. Watches take `{epoch,afterRevision,timeoutMs}`; an epoch mismatch or expired cursor requires a fresh snapshot. Events are batched and bounded rather than sending full transcripts per token. Large initial snapshots use frozen, bounded chunk transfer when they cannot fit one frame. Request responses include a process epoch so stale approval cards cannot answer reused vendor IDs.

Private service IPC is bidirectional JSON-RPC 2.0 over stdio. Rust sends the three chat methods; the JVM service issues Engine callbacks with separate string IDs for the existing Workspace handshake, snapshots, commands, files and documents. Rust routes callbacks without holding the workspace mutex. Guest private directories remain masked: the service persists metadata through these callbacks, never through a writable private-state mount. Callback permissions are allowlisted and cannot recursively invoke chat. Neither side logs frame bodies or credentials.

Accepted sends use a client operation ID and the submitted composer revision. The Engine service records the intent and outcome, deduplicates retries and acknowledges only the submitted draft after backend acceptance. Ambiguous vendor submissions are not automatically replayed. Conversation index semantics, thread mapping, lifecycle bookkeeping and approval validation now reside in the Engine service.

### Other resources and local capabilities

Terminal IDs, process attachment, status, retained output, restart policy and reference cleanup belong to Engine. Android retains xterm and input/rendering adapters. A no-op environment restart must leave running resources alone. Import allocation and collision decisions are atomic Engine operations; Android handles URI grants and bounded streaming, tied to the originating connection. Local device executors retain only Android-specific permission, root, service and hardware operations; Workspace owns desired configuration and business decisions.

The current embedded stdio server ends when its transport closes. This work must represent resulting resource termination and rehydration honestly; moving code out of Android does not by itself establish a detached server or remote transport.

## Verification and completion

The adapter replay suites, codec round trips, stale-epoch rejection, durable send deduplication and resource ownership checks were retained or extended. Actual Linux JRE subprocess behavior was verified through the runtime, followed by integrated Android 9 execution. The final isolated API 28 x86_64 run passed 13 tests with zero skips, covering guest chat startup, client reattachment, generation restart, terminal resources, imports and local proxy execution.

The [separation report](../../report/2026-10-03-engine-client-separation.md) identifies source-bound records and the remaining limits. No real account/model turn, physical ARM64 acceptance or detached/remote server is claimed. The daily tablet was not used as a test fixture.
