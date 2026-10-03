# Appendix: Chat (`workflow-chat`)

Status: verified round-2 port plan; round-1 JVM service retained. Owner: Engine contributor. Updated: 2026-10-03.

This appendix plans the port of chat from the Kotlin guest service to a pure Rust crate at `engine/chat/`. The inventory was rechecked against `f1ed8994ae513bd1e04106bbf162df56919cf2b6`, including the guest service, shared model, adapter fixtures and App projection. The Environment, Server-domains and App appendices now settle the shared contracts. This entire Rust port is round 2: round 1 retains `engine/chat/` unchanged and only types/moves its Rust bridge and runtime supervision.

## Key decisions

1. `workflow-chat` is a library linked into the Server binary. No JVM, JRE payload or private stdio RPC remains. Agent processes are spawned through the Environment runtime, and persistence uses the Environment's typed `.workspace/` store.
2. Everything the crate needs from other domains is a **port trait** defined in `workflow-chat` and implemented by `workflow-server`: the runtime, guest-home files, tool provisioning, typed documents, preferences, and a workspace bridge for composer acknowledgement, attachment bytes and conversation removal. The crate depends only on `workflow-environment`, as the layering rule requires.
3. The rollout has **two phases**. Phase 1 ports the domain logic and keeps today's `chat.*` wire format byte-compatible, so the unchanged Android client and the existing device tests prove parity before any Kotlin is deleted. Phase 2, done jointly with the Server and App planners, replaces client-side event reduction with Engine-reduced projection changes.
4. Parity is proven against Kotlin-generated golden files: reducer outputs, adapter replay scripts and document encodings. After deletion, those files remain as frozen Rust regression data.
5. Concurrency uses standard threads, channels and condition variables, like the current Server. No async runtime is added.

## 1. Current state and inventory

### 1.1 How chat is built, packaged and launched today

- **Build.** The Gradle project `:engine-chat` (`engine/chat/build.gradle.kts`, Kotlin/JVM 17, `application`) depends on `:agent` (which pulls in `:agent-model`) and `:core`. The `serviceJar` task produces a fat `workflow-chat.jar`.
- **Package.** `engine/tools/package.py --architecture amd64|arm64 --jar … --output …` combines the pinned Codex binary, a Temurin JRE (`third_party/jre/manifest.json`), the JAR and the license notices into `tools.zip` and `tools.json`. That catalog lists the mandatory tools `codex`, `jre` and `chat`, plus optional `claude`. On Android, `PrepareEngineToolsTask` in `app/build.gradle.kts` depends on `:engine-chat:serviceJar` and writes `assets/environment/tools/`, and `tools/build-release.sh` requires `jre/bin/java` and `chat/workflow-chat.jar`. A standalone host Server receives the same pair through `--tools DIR`.
- **Install.** `engine/server/src/tools.rs` requires exactly those three mandatory entries, extracts them to a content-addressed payload bound at `/opt/workflow/tools`, and checks readiness. The JRE readiness check runs `java -version` with the service's JVM flags; the `chat` entry reports ready without a probe.
- **Launch.** Host and Android behave the same way. On the first `chat.*` request with a usable environment, `engine/server/src/chat.rs` runs `/opt/workflow/tools/jre/bin/java -Xms16m -Xmx256m … -jar /opt/workflow/tools/chat/workflow-chat.jar` through `Environment::process_command(argv, "/workspace", {})`, using `setsid` and `PR_SET_PDEATHSIG`. Rust forwards `chat.snapshot`, `chat.watch` and `chat.command` with a 240 s timeout and at most 64 requests in flight. It answers the JVM's allowlisted callbacks: `hello`, `workspace.snapshot/watch/command`, `files.read`, `environment.tools.status/install`, and `documents.*` in namespace `chat`. The JVM is killed on environment restart and when the transport closes. Inside the guest, the JVM starts `codex app-server` and `claude -p …` with `ProcessBuilder`.

### 1.2 Current → target mapping

**`engine/chat` (`:engine-chat`, about 1,100 lines)**

| Kotlin | Target | Notes |
| --- | --- | --- |
| `Main.kt` (private JSON-RPC, `FrameWriter`, `ResponseInputStream`, `readFrame`) | deleted | Chat calls become in-process. |
| `ChatService.kt` | `service/mod.rs` (the `Chat` facade and command handlers), `service/tools.rs` (provisioning policy), `service/launch_env.rs` (guest environment filtering) | `readGuestFile` is split between the `GuestHome` port (transcripts) and `WorkspaceBridge::read_attachment` (attachments). |
| `AgentHub.kt` | `service/hub.rs` | Backend registry, the resumed-thread set, per-conversation locks, detection of SIGSYS-killed launches, and the index maintenance thread. |
| `ChatJournal.kt` | `journal.rs` | The frozen chunked snapshot transfer moves to the Server (§3.8). |
| `ConversationIndexFile.kt` (with its codec) | `service/index.rs`, `model/conversation.rs` | |
| `SendLedger.kt` | `service/ledger.rs` | |
| `GuestProcessLauncher.kt` | deleted | Replaced by the `AgentRuntime` port. |
| `AgentPaths.kt` | `service/paths.rs` | Only `toAgent` and `toWorkspace` are ported. `parseLink` is used only by the client's own copy. |

**`agent-model` (about 1,900 lines)**

| Kotlin | Target |
| --- | --- |
| `model/{Items,Events,State,Requests,Inputs,Account,Catalog,ConversationIndex}.kt`, `SendMode.kt` | `model/*.rs` (data only) |
| `AgentReducer.kt` | `reducer.rs`, which mutates state in place (`fn reduce(&mut AgentState, &AgentEvent)`) |
| `StreamText.kt` | A plain `String`. In-place appends are already amortized O(1), and the value serializes as a string. |
| `rpc/ChatWire.kt` (`ChatMetadata`, `ChatSnapshot`, `ChatUpdate`, `ChatSnapshotChunk`) | `model/wire_compat.rs` and `journal.rs`. The chunk type moves to the Server. |
| `ConversationIndexing.refresh` | `service/index.rs` |
| `ConversationIndexing.search/sorted`, `ModelCatalog.slider/resolve/supports` | Client-side presentation logic for the App planner. Rust keeps only the data. |
| `json/JsonAccess.kt`, `AgentFrames.kt`, `InMemoryConversationIndex` | deleted. Typed serde and `OpaqueJson` replace them. `sanitizeDisplayText` moves to `claude/requests.rs`. |

**`agent` (about 4,000 lines of code, plus protocol schemas)**

| Kotlin | Target |
| --- | --- |
| `AgentBackend.kt` (`AgentEventSink`, `AgentStateStore`, `ThreadOptions`, `AgentBackend`) | `backend.rs`, with a crate-private `trait Backend`. The sink is the journal handle. `listThreads` and `ThreadSummary` are dropped because nothing in production calls them. `UnknownRequestPolicy` is dropped and its production value, ask-user, becomes fixed behavior. |
| `process/ProcessLauncher.kt` | the `AgentRuntime` and `AgentProcess` ports (§3.2) |
| `transport/JsonLines.kt`, `JsonRpcConnection.kt`, `ControlConnection.kt` | `transport/lines.rs`, `jsonrpc.rs`, `control.rs` |
| `codex/CodexBackend.kt`, `CodexEvents.kt`, `CodexItems.kt`, `CodexParams.kt` | `codex/backend.rs`, `events.rs`, `items.rs`, `params.rs`, plus a new `codex/wire.rs` with typed serde models |
| `codex/CodexProtocol.kt` (generated) | `codex/methods.rs`, generated by `tools/update-protocol.py --rust` |
| `claude/ClaudeBackend.kt`, `ClaudeLaunch.kt`, `ClaudeMapper.kt`, `ClaudeRequests.kt`, `ClaudeTranscript.kt` | `claude/backend.rs`, `launch.rs`, `mapper.rs`, `requests.rs`, `transcript.rs`, plus a new `claude/wire.rs` |
| `ProtocolCoverage.kt`, `agent/protocol-coverage.md` | `tests/coverage.rs`, `engine/chat/protocol-coverage.md` (a golden file regenerated with `UPDATE_GOLDEN=1`) |
| `src/main/resources/protocol/**` | `engine/chat/protocol/**`. This is test and tool data, not compiled in. |

### 1.3 Tests and fixtures

| Kotlin test | Rust destination | How |
| --- | --- | --- |
| `AgentReducerTest` (17 cases), `ModelSupportTest` | `tests/reducer.rs` | Ported one-to-one. The `StreamText` compaction case is dropped; search and ordering go to the client. |
| `ChatWireTest` | `tests/wire_compat.rs` | Every variant round-trips. Raw request IDs (`9007199254740993` versus `"9007199254740993"`) remain distinct. |
| `TransportTest`, `AgentFramesTest` | `tests/transport.rs` | One-to-one |
| `CodexBackendReplayTest`, `CodexEventsTest`, `CodexLoginTest`, `CodexQueueTest` | `tests/codex_*.rs` | Ported, and also driven by the parity scripts (§5) |
| `ClaudeBackendReplayTest`, `ClaudeUnitTest` | `tests/claude_*.rs` | Same as Codex |
| `ProtocolCoverageTest` | `tests/coverage.rs` | Golden table |
| `ChatJournalTest` | `tests/journal.rs` (two cases) | The frozen-chunk case moves to the Server's tests. |
| `ChatServiceTest` | `tests/service_tools.rs` | The five tool-install policy cases are ported. The frame-reader case is deleted with `Main.kt`. |
| `ConversationIndexFileTest`, `SendLedgerTest` | `tests/index.rs`, `tests/ledger.rs` | One-to-one: 4 and 6 cases |
| `testing/{FakeProcess,Fixtures,ScriptedServer}.kt` | `src/testing/` behind a `testing` feature | `ScriptedServer` keeps its step-matching and ID-rewriting rules. |
| `SmokeMain`, `JvmProcessLauncher` | not ported as host launchers | An `#[ignore]` smoke test may run through the real Environment runtime when an image is present. It never runs the agent directly on the host. |

The fixtures in `agent/src/test/resources/fixtures/{codex,claude}/*.jsonl` (already redacted) move to `engine/chat/tests/fixtures/`. During the parity window, the Kotlin build reads them from there, so only one copy exists.

### 1.4 What the Android client consumes today (for the App planner)

The production client depends on `:agent-model`. Its test and androidTest source sets also depend on `:agent`.

- `platform/agent/AgentHub.kt` uses `ChatWire`, `ChatSnapshot`, `ChatUpdate`, `ChatSnapshotChunk` and `ChatMetadata`, and runs `AgentReducer.reduceAll` on journal events. It derives `attention` from open requests and tracks `processEpochs` for each displayed request.
- Rendering in `feature/chat/*` and `transcript/*` uses `AgentState`, `BackendStatus`, `ThreadState`, `Turn`, every `Item` subclass, `ItemStatus`, `MessagePhase`, `MarkerKind`, `FileDelta`, `FileChangeKind`, `CommandAction`, `PlanStepStatus`, `Notice`, `NoticeLevel`, `TurnStatus`, `TurnError`, `PendingRequest`, `RequestKey`, `RequestKind`, `RequestStatus`, `Decision`, `DecisionKind`, `Question`, `ProcessState`, `LoginState`, `RateLimitWindow`, `ConversationEntry`, `ThreadKey` and `StreamText`.
- Commands send `RequestResponse`, `TurnSettings`, `PermissionPreset`, `LoginMethod`, `UserPart`, `SendMode` and `BackendKind`, and receive `LoginFlow` and `ModelCatalog`.
- Client-side logic also includes `ModelCatalog.slider/resolve` and `SliderPosition` in `Composer.kt` and `ConversationController.kt`, `ConversationIndexing.search` in `ChatRail.kt` and `AccountPanes.kt`, and the `JsonAccess` helpers in `RequestCards.kt` and `NativeTranscript.kt`.
- The client keeps its own `AgentPaths`, which duplicates the Engine's. androidTest's `EngineProcessLauncher` and `GuestProcessFixture` use `agent.process.*`.

## 2. Violations found

**Untyped JSON**

- `engine/server/src/chat.rs:157-188,283` indexes values for frame routing and error decoding (`frame["jsonrpc"]`, `error["data"]["kind"]`, `params["namespace"]`). `main.rs:181` gates chat on `status()["usable"] != true`.
- `ChatService.kt:67-80` reads tool status as `List<*>` and maps (`it["id"]`, `get("phase") == "ready"`). Command arguments are pulled out of a `JsonObject` by name, as in `a.string("id")` and `a["settings"]`.
- `SendLedger.kt:29-39` builds and parses ledger records by hand. `ConversationIndexFile.kt:62` decodes the index with `item["id"].str`.
- All adapters read known vendor messages by indexing paths: about 1,500 lines such as `CodexEvents.kt:88` (`p["threadId"].str`), `CodexItems.kt`, `ClaudeMapper.kt` and `ClaudeRequests.kt`.
- `tools.rs:484` assembles tool status with `json!` and compares `t["id"] == "chat"`.

**Direct file IO outside an owner**

- `ChatService.kt:84-88` creates agent homes and sets their permissions, and checks `canExecute`. `ChatService.kt:109` reads attachments and transcripts with `File(...).canonicalFile`.
- `main.rs:62,393` implements `documents.*` with direct `fs` access below `.workspace/documents/`, including a rename into `corrupt/`. `workspace.rs` (`updateConfig`) writes `config.json` directly.

**Duplicated logic**

- The environment-variable deny list appears three times, with a difference: `ChatService.kt:100`, `CodexBackend.kt:83` (no `CLAUDECODE`) and `ClaudeLaunch.kt:48`.
- MIME guessing appears twice (`AgentHub.kt:470`, `ClaudeRequests.kt:250`). Login methods are listed in `AgentHub.loginMethods` and again in each backend. Tool binary paths are repeated in `ChatService`, `tools.rs`, `package.py` and `chat.rs`. The `hello` capability JSON appears twice in `main.rs`.
- Three line-framing implementations exist: `protocol::line` and `strict_json` in Rust, `Main.readFrame` with a 32 MiB limit, and `LineReader` with a 64 MiB limit. The whole private RPC layer duplicates JSON-RPC only to reach Engine state.
- `AgentReducer` runs on both sides, and `AgentPaths` exists in the Engine and in the client.

**Safety and behavior defects to fix in the port**

- *Raw-console reviewer gap.* `CodexParams.REVIEWER_METHODS` (`CodexParams.kt:26`) covers `thread/start`, `thread/resume`, `thread/fork` and `turn/start`. However, `thread/settings/update` and `turn/settings/update` also accept `approvalsReviewer` (see `ThreadSettingsUpdateParams.json` and `TurnSettingsUpdateParams.json`), so the advanced console can change the reviewer on those two methods. The port adds both methods to the enforced set. This is a deliberate parity deviation (§6, Q6).
- *Unbounded ledger.* Each send writes one `send-<sha>.json` record that is never removed, not even when its conversation is deleted.
- *Engine-authored copy.* The Engine produces Chinese user-facing strings, such as the `（分叉）` fork-title suffix and the error that tells the user to stop the running answer first.

## 3. Target design

### 3.1 Crate layout

```
engine/chat/                       crate workflow-chat (library)
  Cargo.toml                       serde, serde_json (raw_value, arbitrary_precision), sha2, uuid, indexmap (pinned), workflow-environment
  protocol/{codex,claude}/…        pinned vendor schemas and inventories (data only)
  protocol-coverage.md             golden coverage table
  src/
    lib.rs                         pub use of Chat, ChatPorts, SendRequest, ChatError and model::*
    error.rs                       ChatError { kind, message }; messages go to the requester only, never to logs
    ports.rs                       AgentRuntime, AgentProcess, GuestHome, AgentTools, ChatStore, AgentPreferencesStore, WorkspaceBridge, Clock, IdSource
    model/                         ids, opaque, items, events, state, requests, inputs, account, catalog, conversation, wire_compat
    reducer.rs                     pure, in-place fold
    journal.rs                     revisioned log, metadata, process epochs, waits
    backend.rs                     crate-private Backend trait, ThreadOptions
    transport/                     lines.rs, jsonrpc.rs, control.rs
    codex/                         backend, events, items, params, wire, methods (generated)
    claude/                        backend, launch, mapper, requests, transcript, wire
    service/                       mod (Chat), hub, tools, index, ledger, paths, launch_env
    testing/                       feature "testing": fake runtime and pipes, ScriptedServer, in-memory ports
  tests/                           fixtures/, parity/{scripts,golden,rust-wire}/, *.rs
```

### 3.2 Ports (requirements on Environment, Server, Filework and Workspace)

`workflow-chat` defines the following ports, and the Server implements them by adapting `workflow-environment`, `workflow-filework` and `workflow-workspace`. If the Environment appendix defines equivalent types, the Environment's types replace these and the traits become thin adapters.

```rust
pub trait AgentRuntime: Send + Sync {
    /// Spawns inside the active verified generation with the workspace layout: /workspace bound,
    /// private .workspace state hidden, /home/work from the home store. Fails with
    /// EnvironmentUnavailable and never falls back to the host. The child is registered in the
    /// environment's process registry so a restart stops and waits for it, and it dies with the Server.
    fn spawn(&self, spec: SpawnSpec) -> Result<Box<dyn AgentProcess>, RuntimeError>;
    /// The guest base environment (PATH, LANG, TMPDIR, …) that chat filters and extends.
    fn base_env(&self) -> BTreeMap<String, String>;
}
pub struct SpawnSpec { pub argv: Vec<String>, pub cwd: GuestPath, pub env: BTreeMap<String, String>, pub label: String }
pub trait AgentProcess: Send + Sync {
    fn take_stdio(&mut self) -> Option<AgentStdio>;          // piped stdin (Write) and stdout/stderr (Read)
    fn signal(&self, signal: StopSignal) -> io::Result<()>;  // Term or Kill, delivered to the whole guest tree; idempotent
    fn wait(&self) -> ExitStatus;                           // blocking
    fn try_status(&self) -> Option<ExitStatus>;
}
pub enum ExitStatus { Code(i32), Signal(i32) }              // Signal(SIGSYS) before Ready marks the backend unsupported

pub trait GuestHome: Send + Sync {                          // backed by the environment home store
    fn ensure_private_dir(&self, path: &GuestPath) -> Result<(), StoreError>;          // only below /home/work, mode 0700
    fn read(&self, path: &GuestPath, max: u64) -> Result<Option<Vec<u8>>, StoreError>; // only below /home/work, no symlink escape
}
pub trait AgentTools: Send + Sync {
    fn status(&self) -> ToolsStatus;                        // typed {revision, generation, tools: [ToolStatus {id, phase, binary, error}]}
    fn install(&self, tool: ToolId, retry: bool) -> Result<ToolsStatus, ToolError>;
}
pub trait ChatStore: Send + Sync {                          // Environment typed documents, namespace "chat"
    fn read_index(&self) -> Result<Option<ConversationIndexDoc>, StoreError>; // NewerFormat leaves the file untouched
    fn write_index(&self, doc: &ConversationIndexDoc) -> Result<(), StoreError>; // atomic
    fn quarantine_index(&self) -> Result<(), StoreError>;
    fn read_send(&self, key: &SendKey) -> Result<Option<SendRecordDoc>, StoreError>;
    fn write_send(&self, key: &SendKey, doc: &SendRecordDoc) -> Result<(), StoreError>;
    fn remove_sends(&self, conversation: &ConversationId) -> Result<(), StoreError>; // requires key listing in the store
}
pub trait AgentPreferencesStore: Send + Sync {              // the config.json "agent" section (owner: Q2)
    fn get(&self) -> AgentPreferences;
    fn update(&self, f: &mut dyn FnMut(&mut AgentPreferences)) -> Result<(), StoreError>;
}
pub trait WorkspaceBridge: Send + Sync {                    // Server → filework/workspace
    fn read_attachment(&self, path: &WorkspacePath, max: u64) -> Result<Option<Vec<u8>>, BridgeError>;
    fn acknowledge_composer(&self, submitted: &SubmittedComposer) -> Result<(), BridgeError>; // clears only an exact match
    fn remove_conversation(&self, id: &ConversationId) -> Result<(), BridgeError>; // closes panels and clears the composer
}
```

The Server's `SIGSYS` and `PDEATHSIG` behavior, the group kill, and the `running` counter that `environment.restart` waits on all move behind `AgentRuntime`. Chat does not use `std::fs`, `std::process` or `libc`, and a source guard test enforces this.

### 3.3 Public API used by `workflow-server`

All methods block and are called from Server worker threads.

```rust
pub struct Chat { /* Arc<Inner> */ }
pub struct ChatPorts { pub runtime: Arc<dyn AgentRuntime>, pub guest_home: Arc<dyn GuestHome>, pub tools: Arc<dyn AgentTools>,
    pub store: Arc<dyn ChatStore>, pub preferences: Arc<dyn AgentPreferencesStore>, pub workspace: Arc<dyn WorkspaceBridge>,
    pub clock: Arc<dyn Clock>, pub ids: Arc<dyn IdSource> }

impl Chat {
    pub fn start(ports: ChatPorts) -> Result<Chat, ChatError>;       // loads the index; starts the maintenance and tool threads
    pub fn shutdown(&self);                                          // stops agents; their requests expire through events
    pub fn environment_changed(&self);                               // after activation: reset install demand, re-measure tools
    // Projection reads
    pub fn snapshot(&self) -> ChatSnapshot;                          // {epoch, revision, state (Arc-shared), metadata}
    pub fn changes(&self, after: Cursor, wait: Duration) -> ChangeBatch; // long-poll; Resnapshot when the cursor is gone
    // Conversations
    pub fn create_conversation(&self, backend: Option<BackendKind>, id: Option<ConversationId>) -> Result<ConversationId, ChatError>;
    pub fn ensure_conversation(&self, id: &ConversationId) -> Result<ConversationEntry, ChatError>;
    pub fn open(&self, id: &ConversationId) -> Result<(), ChatError>;
    pub fn load_earlier(&self, id: &ConversationId) -> Result<(), ChatError>;
    pub fn set_backend(&self, id: &ConversationId, kind: BackendKind) -> Result<(), ChatError>;
    pub fn rename(&self, id: &ConversationId, title: &str) -> Result<(), ChatError>;
    pub fn archive(&self, id: &ConversationId, archived: bool) -> Result<(), ChatError>;
    pub fn delete_conversation(&self, id: &ConversationId) -> Result<(), ChatError>; // vendor delete, bridge removal, index removal
    pub fn fork(&self, id: &ConversationId, at_turn: Option<&TurnId>) -> Result<ConversationId, ChatError>;
    pub fn compact(&self, id: &ConversationId) -> Result<(), ChatError>;
    pub fn set_permissions(&self, id: &ConversationId, preset: PermissionPreset) -> Result<(), ChatError>;
    pub fn remember_selection(&self, id: &ConversationId, model: Option<String>, effort: Option<String>) -> Result<(), ChatError>;
    // Turns and requests
    pub fn send(&self, request: SendRequest) -> Result<ClientMessageId, ChatError>;
    pub fn interrupt(&self, id: &ConversationId) -> Result<(), ChatError>;
    pub fn cancel_queued(&self, id: &ConversationId, message: &ClientMessageId) -> Result<(), ChatError>;
    pub fn respond(&self, key: &RequestKey, epoch: &ProcessEpoch, response: RequestResponse) -> Result<(), ChatError>;
    pub fn raw_request(&self, id: &ConversationId, method: &str, params: Option<OpaqueJson>) -> Result<OpaqueJson, ChatError>;
    // Backends and accounts
    pub fn warm_up(&self, kind: BackendKind) -> Result<(), ChatError>;
    pub fn refresh_account(&self, kind: BackendKind) -> Result<(), ChatError>;
    pub fn refresh_usage(&self, kind: BackendKind) -> Result<(), ChatError>;
    pub fn refresh_models(&self, kind: BackendKind) -> Result<ModelCatalog, ChatError>;
    pub fn login(&self, kind: BackendKind, method: LoginMethod, secret: Option<Secret>) -> Result<LoginFlow, ChatError>;
    pub fn cancel_login(&self, kind: BackendKind, login_id: &str) -> Result<(), ChatError>;
    pub fn logout(&self, kind: BackendKind) -> Result<(), ChatError>;
    pub fn flush(&self) -> Result<(), ChatError>;
}
pub struct SendRequest { pub operation: OperationId /* correlation only, ≤200 chars */, pub submitted: SubmittedComposer,
    pub settings: TurnSettings, pub mode: SendMode }
pub struct SubmittedComposer { pub conversation: ConversationId, pub revision: u64, pub text: String, pub attachments: Vec<ComposerAttachment> }
pub enum ChatErrorKind { InvalidArgument, NotFound, BackendUnavailable, RequestExpired, DecisionNotOffered, TurnActive,
    SendConflict, SendAmbiguous, Vendor, Store, Closed }
```

`Secret` has a redacted `Debug` implementation and is never stored. The Claude API key stays in Server memory, as it stayed in JVM memory before. The current wire repeats `text` and `attachments` beside `submitted`; the Server rejects a mismatch, and the domain accepts only `submitted`. Gating chat on the environment being prepared remains a Server decision, made before it calls `Chat`.

### 3.4 Concurrency

- Each agent process has a stdout reader thread, a stderr drain thread (keeping a 32 KiB tail that is never logged) and an inbound dispatcher. Outbound writes are serialized by a mutex.
- Outbound requests wait on a one-shot channel. They fail with `Closed` on process exit and time out after 240 s, matching today's bound.
- Supporting threads: login polling every 2 s up to 15 min, index maintenance with 250 ms coalescing, and a tool-status refresh every 3 s and on `environment_changed`.
- The journal holds a single mutex and condition variable. Reduction happens in place. Threads are stored as `Arc<ThreadState>` and copied on write, so snapshots are cheap pointer copies.

### 3.5 Event model and journal

- **Domain events.** `AgentEvent` is ported exactly, with 35 variants in five groups: process and backend (`ProcessChanged`, `ServerInfo`, `AccountChanged`, `LoginChanged`, `RateLimitsChanged`, `ModelsChanged`, `McpServerChanged`, `BackendNotice`, `Unknown`), thread (`ThreadUpserted` … `HistoryLoaded`, `QueueUpdated`), turn (`TurnSubmitted`, `TurnBound`, `TurnStarted`, `TurnCancelled`, `TurnCompleted`, plan, diff, progress, notice), item (`ItemStarted`, `ItemUpdated(ItemDelta)`, `ItemCompleted`, `ItemDeclined`), and request (`RequestOpened`, `RequestClosed`). The adapters emit them. The reducer is their only consumer of meaning and never answers a request.
- **Journal.** A per-Server `epoch` (UUID) and a monotonic `revision`. Each entry is `{revision, event | metadata-only, metadata: Arc<ChatMetadata>, bytes}`, with retention of 4,096 entries and 8 MiB and batches of at most 512 entries and 1 MiB. An expired cursor or a foreign epoch returns `Resnapshot`. Metadata (conversations, available backends, login methods, default permissions, default backend, process epochs) is recomputed after every command and on index or tool changes, and is recorded only when it changes.
- **Process epochs.** Each `ProcessChanged(Starting)` assigns a new epoch for that backend. `respond` requires the caller's epoch to equal the current one and the request to be open, so a vendor that reuses an ID cannot be answered across processes.
- **Environment restart.** The Rust service may survive as an object, but must rotate its journal epoch and expire open requests/process epochs when the old guest generation stops. The existing JVM process restart already invalidates that identity. Reconnect requires a fresh snapshot; keeping the old epoch is not an accepted parity deviation. Install demand resets for the new generation.
- **Phase 2 (`ChatChange`).** The journal records Engine-reduced changes instead of events: `Backend{kind,status}`, `ThreadHeader{key,header}` (a thread without turns), `TurnOrder{key,turn_ids}`, `Turn{key,turn}` (without items, with item order), `Item{key,turn,item}`, `TextAppended{key,turn,item,field,text}` (the streaming fast path), `Request{request}` and `Metadata`. The reducer reports which entities each event touched. Text deltas become `TextAppended`, and command-output truncation becomes a full `Item`. The client applies changes by key and runs no reducer.

### 3.6 Vendor traffic: typed models and preservation

- Each line is parsed once into `Box<RawValue>`, then into a typed envelope: Codex `{id: Option<RequestId>, method, params, result, error}`, or Claude `{type, request_id, request, response}`. Known methods deserialize into structs in `codex/wire.rs` and `claude/wire.rs` whose fields are optional or defaulted, so parsing stays as tolerant as Kotlin. A known method that fails to deserialize becomes `AgentEvent::Unknown` with its raw frame, and is never dropped.
- `RequestId` is `Number(serde_json::Number) | String(String)`, with arbitrary precision preserved. It is echoed exactly and kept in separate inbound and outbound maps.
- Unknown data uses named opaque types. `OpaqueJson(Box<RawValue>)` preserves exact bytes and compares by bytes; `OpaqueObject` holds a JSON object. They cover every Kotlin `raw`, `ServerInfo.info`, `UnknownRecord`, `UnknownItem`, `RequestKind::Unknown.params`, `PermissionsApproval.permissions`, tool arguments and results, elicitation schemas, raw-console parameters and results, and `RawResult`. No code indexes into `Value`.
- Kotlin length limits count UTF-16 code units: titles and previews take 200, command output keeps 512 Ki. Rust keeps that unit, cut at character boundaries, so the golden files match.

### 3.7 Persistence documents

| Document | Location (via the Environment store) | Schema |
| --- | --- | --- |
| Conversation index | `chat/conversations` (today `.workspace/documents/chat/conversations.json`) | `ConversationIndexDoc {format: 1, conversations: [{id, backend: "codex"\|"claude", thread?, title?, cwd, createdAt, updatedAt, archived? (only when true), forkedFrom?, forkedAt?, model?, effort?, preview?}]}`. Entries without an ID or a known backend are skipped, and duplicate IDs keep the first entry. A newer format is refused without touching the file; a malformed one is quarantined and replaced by an empty index. Concurrent edits are serialized by an edit lock, and a refresh never overwrites a user's title or archive state. |
| Send record | `chat/send-<sha256({"conversationId","revision"})>` (unchanged key) | `SendRecordDoc {fingerprint, state: pending\|accepted, clientMessageId?, conversationId, revision}`. The last two fields are new, so records can be removed when their conversation is deleted. A pending record is written before dispatch and is never erased or replayed. An accepted record returns the original ID and acknowledges again. A fingerprint mismatch fails. |
| Agent preferences | the `agent` section of `config.json` (owner: Q2) | `AgentPreferences {backend, backends: {id: {model?, effort?}}, permissions?: ask\|auto_edit\|plan\|deny_unlisted}` |
| Agent homes | guest `/home/work/.codex` (`CODEX_HOME`) and `/home/work/.claude` (`CLAUDE_CONFIG_DIR`) in the environment home store | Owned by the vendors. Chat only creates them (mode 0700) and reads Claude transcripts at `/home/work/.claude/projects/-workspace/<session>.jsonl`, up to 16 MiB. |

`AgentState` and the journal stay in memory, and history is rehydrated from the vendors. The fingerprint becomes a SHA-256 over a canonical, typed `SendFingerprint {submitted, settings, mode}`. A record that Kotlin wrote before the switch therefore fails explicitly when resent instead of being dispatched twice, which is the safe direction.

### 3.8 What the protocol must carry (for the Server planner)

- **Phase 1 (unchanged wire).** `chat.snapshot {transferId?, offset?}`, `chat.watch {epoch, afterRevision, timeoutMs}` and `chat.command {name, args}` for the 24 existing names listed under "Chat resources" in `docs/implementation/protocol.md`. They use the `ChatWire` shape: an `_type` tag, uppercase enum names, `encodeDefaults` behavior (absent optional fields are written as `null`), structured map keys as alternating key/value arrays, and `BackendKind`-keyed maps as objects. The domain types derive serde in exactly this shape. The Server owns the frozen chunked transfer (above 1 MiB, 64 KiB raw chunks, a 64 MiB cap, two transfers retained) because it is a framing concern.
- **Phase 2.** Typed methods replace `chat.command`:
  - `chat.snapshot` and `chat.watch`, which return `ChatChange[]` plus metadata or `resnapshot`.
  - Conversation methods: `chat.conversation.{create, ensure, open, loadEarlier, setBackend, rename, archive, delete, fork, compact, setPermissions, rememberSelection}`.
  - `chat.send`, `chat.interrupt`, `chat.cancelQueued`, `chat.respond {key, processEpoch, response}` and `chat.raw`.
  - Backend methods: `chat.backend.{warmUp, refreshAccount, refreshUsage, refreshModels, login, cancelLogin, logout}`.
  - `chat.flush`.

  Errors map from `ChatErrorKind`. Vendor error messages go to the requester only and are never logged. The Server must not synthesize any answer to a request.

### 3.9 Where the invariants live

| Invariant | Location and test |
| --- | --- |
| Never auto-approve | Only `Chat::respond` writes an answer to an approval request. The only automatic answers are factual or negative capability results: `currentTime/read`, dynamic `item/tool/call` → `success:false`, and auth-refresh/attestation → error. Each is in the coverage table as `auto` or `reject`. Tests assert that no frame answers an open request ID before the user does. |
| Preserve unknown traffic and IDs | `OpaqueJson`, `RequestId`, unknown methods become cards with a single `reject` decision, undeclared Claude dialogs are never answered, and Claude echoes of our own responses are ignored. Transport and replay tests cover this. |
| `approvalsReviewer: "user"` | `codex/params.rs` adds it to start, resume, fork and turn-start requests, to the permission update, and in the raw console for every method that accepts it. The adapter blocks sending if the server reports another reviewer. |
| Composer-revision deduplication | `service/ledger.rs` plus `WorkspaceBridge::acknowledge_composer`. The six ledger cases are ported. |
| Environment required | Only `AgentRuntime::spawn` starts processes and it has no host fallback. Claude modes are limited to `default`, `acceptEdits`, `plan` and `dontAsk`. |
| Optional Claude install | `service/tools.rs`: install once per generation when the default backend or a demanded conversation is Claude. A failed install stays failed until an explicit tools retry. The five policy cases are ported. |

## 4. What happens to `agent/`, `agent-model/` and `:engine-chat`

- **`:engine-chat`** is deleted entirely, together with its Gradle include, the `serviceJar` packaging, the JRE payload and `third_party/jre`, and the `guestChat` ownership entry in `contract.json` (CH-7, CH-8).
- **`agent/`** is deleted entirely. All of it is server-side. Its fixtures and protocol assets move to `engine/chat/` first (CH-0). The App planner must replace the app's `testImplementation` and `androidTestImplementation(project(":agent"))` dependencies, whose only users are `EngineProcessLauncher` and `GuestProcessFixture` with `agent.process.*`, before CH-8 can remove the module.
- **`agent-model/`** shrinks and then ceases to exist. Its server half (reducer, index, frames, JSON access) is replaced by Rust. Its client half (the §1.4 types, slider and search helpers, and, in phase 1, the reducer) moves into the App's pure-JVM client module as typed chat protocol bindings, which are checked against the contract exported by `workflow-server`. After phase 2 the client reducer is replaced by a change applier. The App planner owns the module name, its location under `app/` and the timing. Chat's requirement is only that nothing under `engine/` depends on it after CH-6.

## 5. Implementation tasks

Every task updates the maintained documentation it affects in the same change (`docs/implementation/agents.md`, `protocol.md`, `workspace-engine.md`, `environment.md`, `testing.md`, or their successors after the documentation reorganization). Heavy Cargo and Gradle commands go through the suites. The new suite is `"chat": ["cargo","test","--manifest-path","engine/Cargo.toml","--locked","-p","workflow-chat","-j","2"]` (heavy).

| ID | Objective | Owned paths | Depends on | Tier | Checks |
| --- | --- | --- | --- | --- | --- |
| CH-0 | Crate skeleton and asset move. Add `engine/chat/Cargo.toml` and the workspace member, `error.rs` and `ports.rs` stubs, and the `chat` suite. `git mv` the protocol assets, the coverage golden and the fixtures into `engine/chat/`. Point `agent/build.gradle.kts` at them so classpath paths stay `/protocol/…` and `/fixtures/…`. Repoint `tools/update-protocol.py` and `update-claude-protocol.py`, and add `--rust` output to `codex/methods.rs`. | `engine/chat/{Cargo.toml,src/*.rs,protocol/**,protocol-coverage.md,tests/**}`, the `engine/Cargo.toml` member list, `engine/Cargo.lock`, `agent/build.gradle.kts`, `agent/src/{main,test}/resources/**`, `agent/protocol-coverage.md`, `tools/update-*protocol.py`, `tools/workflow-suites.json` (the new entry only) | Coordinate the `engine/Cargo.toml` edit with the ENV and SRV skeleton tasks | sol-high | agent, chat, infrastructure, documentation |
| CH-P | Kotlin parity oracle (temporary). Data-driven replay scripts mirror the Kotlin replay, login and queue tests: `tests/parity/scripts/*.json` with operations such as `start`, `startThread`, `send`, `respond`, `interrupt` and `fork`. A Kotlin interpreter writes `golden/<script>.json` with the outbound frames and a state checkpoint for each step. Also: reducer goldens (event sequence → state), index codec goldens, and a Kotlin test that decodes the Rust samples in `tests/parity/rust-wire/`. | `agent/src/test/kotlin/…/agent/parity/**`, `engine/chat/tests/parity/{scripts,golden}/**` | CH-0 | sol-high | agent |
| CH-1 | Domain model, in-place reducer and `ChatWire`-compatible serde, including the key/value-array maps, `RequestId` and `OpaqueJson`. Port the reducer, model and wire tests. Emit the `rust-wire` samples. | `engine/chat/src/{model/**,reducer.rs}`, `tests/{reducer,wire_compat}.rs`, `tests/parity/rust-wire/**` | CH-0, CH-P | astra-max | chat, agent |
| CH-2 | Transports: the bounded line reader (CRLF, oversize, invalid UTF-8, missing final newline), JSON-RPC with two ID spaces and verbatim server IDs, the Claude control connection, the fake process and `ScriptedServer`. | `engine/chat/src/{transport/**,testing/{process,scripted}.rs}`, `tests/transport.rs` | CH-0 (parallel with CH-1) | astra-max | chat |
| CH-3 | Codex adapter: typed wire models, event and item mapping, parameters with reviewer enforcement (including the raw-console hardening), the backend (lifecycle, login polling and timeout, queue synchronization, send modes, response validation), and the coverage golden. | `engine/chat/src/codex/**`, `tests/codex_*.rs`, `tests/coverage.rs`, `protocol-coverage.md` | CH-1, CH-2, CH-P | astra-max | chat |
| CH-4 | Claude adapter: launch argv/env and the mode guard, requests and answers (suggestions, `suppress_always_allow_rule`, user-interaction rules), attachments through the bridge (20 MiB inline limit), mapper, transcript hydration, the session and spare-process model, and hooks (crate-private and test-only; production registers none). | `engine/chat/src/claude/**`, `tests/claude_*.rs` | CH-1, CH-2, CH-P (parallel with CH-3) | astra-max | chat |
| CH-5 | Chat service: the `Chat` facade with all 24 operations, hub, journal, index, ledger, tool policy, environment filtering (one shared deny list including `CLAUDECODE`), agent-home preparation, SIGSYS detection and metadata. Provide in-memory port fakes and a source guard against `std::fs`, `std::process` and `libc`. | `engine/chat/src/{lib.rs,journal.rs,backend.rs,ports.rs,service/**,testing/fakes.rs}`, `tests/{service*,journal,index,ledger}.rs` | CH-1 (can begin with stub backends), CH-3, CH-4; the port signatures are agreed with ENV | astra-max | chat |
| CH-6 | Server wiring (cross-module). Implement the ports over the Environment runtime, store and tools and the workspace/filework functions. If ENV has not landed, use temporary adapters over `Environment::process_command`, `tools::status/install` and the current document storage, recorded as an ENV follow-up. Delete the JVM supervisor and callback allowlist. Map `chat.*` with the phase 1 wire, and move the chunked transfer and its test into the Server. Call `shutdown`/`environment_changed` around restart and `shutdown` on transport close. | `engine/server/src/chat.rs` (or the chat module of the new Server layout), the `engine/server/Cargo.toml` dependency, the chat dispatch lines in `main.rs` (shared, so coordinate), chat cases in `engine/server/src/tests.rs` | CH-5; ENV runtime/store/tools; SRV layout if it has landed | astra-max | rust-server, chat, infrastructure, android-apk, then `tools/workflow device` |
| CH-7 | Remove the JVM payload (cross-module). The tools payload contains only Codex, with Claude optional. Update the mandatory lists in `tools.rs` or its ENV successor, `package.py` and its README, `third_party/jre` and the notices, `PrepareEngineToolsTask`/`prebuiltNotices`, `build-release.sh`, `contract.json`, and the `jre`/`chat` tool assertions in `EngineIntegrationTest`. | the paths named, each with the agreement of its ENV or APP owner | CH-6 accepted on a device | sol-high | rust-server, infrastructure, android-apk, device |
| CH-8 | Delete the Kotlin chat and adapters. Remove the `engine/chat` Kotlin sources and Gradle file and `agent/**`, replace the `agent` suite with `chat`, make `test_client_boundary.py` assert that `engine/` contains no `*.kt`, `*.java` or `*.gradle.kts`, and update the docs and `status.md`. | `engine/chat/{src/main,src/test,build.gradle.kts}`, `agent/**`, the `settings.gradle.kts` includes, `tools/workflow-suites.json`, `tools/tests/test_client_boundary.py`, chat docs | Parity evidence from CH-3, CH-4 and CH-5; CH-6 on a device; the APP task that removes the app's `:agent` test dependencies | sol-high | chat, infrastructure, documentation |
| CH-9 | Projection protocol (joint with SRV and APP). The journal records `ChatChange`, the Server exposes typed chat methods, and the client applies changes. After that, APP removes the client reducer and `agent-model`. | Chat side: `engine/chat/src/{journal.rs,model/change.rs}`, `tests/changes.rs`; SRV and APP paths per their appendices | CH-6, SRV protocol typing, the APP client module | astra-max | chat, rust-server, app-unit, device |

**Acceptance evidence and the parity proof required before Kotlin is deleted**

1. **Reducer parity (CH-1).** Every reducer golden matches exactly after normalization: identifiers drawn from the injected ID source are renamed by order of first appearance, and `*AtMs`/`receivedAtMs` values are masked. All ported reducer and wire cases pass. The Kotlin `ChatWire` decodes every Rust sample and re-encodes it to an equal value.
2. **Adapter parity (CH-3, CH-4).** For every script, the outbound frames of each step are equal as multisets within the step, and every state checkpoint is equal. That includes `{"id":0,"result":{"decision":"accept"}}`, `approvalsReviewer` on every reviewer method, the Claude `--permission-mode` argv, and attachment base64. The only allowed differences are listed in the harness header: the raw-console reviewer hardening, and `thread/list` becoming `generic` in the coverage table. The regenerated coverage table differs from Kotlin only in those listed rows.
3. **Service parity (CH-5).** The five tool-policy, two journal, four index and six ledger cases are ported with identical assertions. Index encoding matches the codec goldens byte for byte. Each of the 24 operations has a command-level test against the fakes. The source guard passes.
4. **Device acceptance (CH-6) with the unchanged client.** Run `tools/workflow check android-apk`, then `tools/workflow device --class …EngineIntegrationTest` on the isolated API 28 x86_64 AVD. The chat cases must pass with zero skips: snapshot and watch, Codex start and account state without credentials, client reattachment, Claude selection, install and initialization, and rehydration after a generation restart. The record must state that no real-account turn was made.
5. **Deletion gate (CH-8).** The parity runs from points 1 to 3 and the device run from point 4 are recorded against the integrated source. After CH-8, the golden files stay as frozen Rust regression data.

## 6. Resolved cross-appendix decisions

Phase 1 preserves the existing `ChatWire` shape; the projection protocol is a separate coordinated phase after parity. Chat owns the typed `agent` preferences section, Environment owns `config.json` publication, and Server composes revision checks and projections. Domain structs can derive serde/schema, but only Server defines RPC envelopes, method names, errors and framing. The Environment store keeps `documents/<namespace>/<key>.json` with its format/revision/document envelope; it adds bounded typed access rather than moving existing indexes or ledgers. Standard threads/channels remain the concurrency model.

The port must preserve the explicit Codex user reviewer setting and extend raw-console enforcement to the two settings methods. That hardening is an intentional recorded difference, never an approval bypass. Existing Engine-authored text stays for initial parity; localization/error-kind projection is a separate client protocol change. A login flow requiring a terminal becomes a user action routed by Server to Terminal, without Chat depending on Terminal. Client presentation can parse links, but authoritative workspace path resolution belongs to Server composition with FileWork and Environment.

Two draft risks require correction before the port: a changed send fingerprint must never replay a Kotlin pending/ambiguous submission, and keeping a Rust service object across restart must not keep stale approval/journal identities. Golden cases must cover both. Counts in the inventory are estimates; the latest source-bound service run documented in testing contains twelve service tests, and the complete parity gate is all discovered current cases, not a frozen numerical count from the initial draft.

CH-6 owns Server wiring with coordinator-assigned paths; CH-7 removes the JRE only after accepted guest execution; CH-8 waits for App test dependencies and fixtures to move. No round-1 change deletes vendor schemas, Kotlin test oracles, the JAR, JRE notices or approval behavior.

## 7. Later-round deliveries

| Round/task | Owned paths | Dependencies | Checks |
| --- | --- | --- | --- |
| R2 CH-0/CH-P/CH-1/CH-2 | Rust Chat crate, vendor fixture/schema relocation, Kotlin parity harness | Environment runtime/store signatures, pinned Rust dependencies | All discovered reducer, transport and wire cases; Kotlin decodes Rust goldens |
| R2 CH-3/CH-4/CH-5 | Rust adapters, journal, index, ledger and service | Replay goldens; no-auto-approval and raw-ID preservation tests | Adapter outgoing frames/state parity, service deduplication and restart identity tests |
| R2 CH-6 | Server Chat composition/bridge and contract | CH-5 plus integrated Environment and domain APIs | Rust suites, unchanged client APK, isolated API 28 guest Chat and reconnect acceptance with zero skips |
| R2 CH-7/CH-8 | JRE/JAR packaging, Kotlin Engine/agent modules, App fixture references | Parity and accepted guest/device cutover; App owner agreement | Both ABI builds, tools measurements, dependency guards and documentation |
| R3 CH-9 | Chat change journal, Server methods/schema and App change applier | Stable Rust Chat and typed Kotlin client | Projection replay, subscription resnapshot, streaming/approval device matrix and accurate reports |
