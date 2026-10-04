# Module appendix: Environment, runtime and loader

Status: decided; round-1 foundation implemented, remaining work gated. Owner: Engine contributor; runtime extraction belongs to the separate `runtime-dedup` task. Updated: 2026-10-03.

This appendix plans `workflow-environment`, `workflow-runtime` and the loader for the [module reorganization](../module-reorganization.md). The draft inventory was checked against `f1ed8994ae513bd1e04106bbf162df56919cf2b6`. Historical counts are approximate: the four current source files total 2,714 lines including tests (`environment` 1,020, `process` 418, `tools` 814 and `home_stage` 462). API sketches below describe the eventual design; the concrete round-1 API is the crate source and the handoff, not every proposed convenience wrapper.

The inventories and task sketches below record the baseline used for planning; they are not a claim that every sketched API exists. Later user steering assigns configuration and service domain logic to Environment, agent defaults to Chat and terminal settings to Terminal, superseding the old Server/services allocation in that inventory. Runtime extraction is separately owned and excluded from the current round-2 source scope.

## Key decisions

- **On-disk compatibility is preserved byte for byte where it matters.** Environment declaration/lifecycle paths, persisted field names and both environment fingerprints stay identical, because version 1.0.0 has no migration. A path change on the user's tablet would orphan the persistent home (which holds agent logins) and force a rebuild. The proxy is the explicit user-directed exception: all canonical proxy files use `.workspace/proxy/`, retaining `services.proxy` identifiers, without migration from the previous service directory.
- **The declaration is still `.workspace/env.json`.** The user and the skeleton say `environment.json`, but today that name belongs to the private lifecycle record at `.workspace/environment/environment.json`. The typed model is called `EnvironmentDeclaration`. Round 1 deliberately retains that name and documents the distinction; no rename or migration is introduced.
- **`workflow-environment` is the foundation crate.** Every domain crate already depends on it, so it hosts the shared primitives: strict JSON, `Identifier`, SHA-256 helpers, atomic file publication and `DiskVersion`.
- **`workflow-runtime` stays a separate executable.** A ptrace tracer must not run inside the multi-threaded Server, and the runtime explicitly sets `PR_SET_PDEATHSIG` and supervises process-tree convergence. The Rust API that drives it, including PTYs, supervision and stop, is `workflow_environment::runtime`.
- **The three runtime trees are not proven safe to merge in round 1.** They are related c2rust outputs, but target-specific libc declarations, errno accessors, register layouts and legacy syscall branches remain. Both `target_os` and `target_arch` are necessary because Linux uses glibc and Android uses bionic. Similar text and archived C ancestry are evidence for a future extraction, not equivalence proof. Move all three trees intact; ENV-6 requires a per-target semantic/equivalence report, host oracles and Android checks before it can ship.

## 1. Inventory and current-to-target mapping

### 1.1 Server files owned by the environment

| Current file (lines) | What it does | Target |
| --- | --- | --- |
| `engine/server/src/environment.rs` (986) | `Options`; loads, recovers and persists `.workspace/environment/environment.json` as a raw `Value`; validates `env.json`; reconcile thread; generation build (image install, seed merge, apt, Python/Node, `verify-many`, tool verification, post-scripts); activation; restart; the `workflow-runtime run` argv builder (`guest_command`) including binds, hides and guest `PATH`; the bounded subprocess helper `checked`; image extraction from the APK | Split across `lifecycle`, `provision`, `image`, `bundle`, `declaration` and `runtime::{cli,mounts}` |
| `engine/server/src/process.rs` (418) | Process registry (128 entries), spawn thread that owns `PDEATHSIG`, `openpty`/`TIOCSCTTY`, 4 MiB output rings, write, resize, TERM/KILL to the process group, wait, `stop_all`, a shared `running` counter | `runtime::supervisor`, `runtime::pty` and `runtime::output` |
| `engine/server/src/tools.rs` (673) | Engine tools payload: APK or directory source, catalog validation, safe ZIP extraction into a content-addressed directory, a global `VERIFIED` cache, guest binds and the Codex launcher, measured tool status in `tools/state.json`, version probes, the optional Claude download job | `tools::{catalog,payload,status,optional}`; the binds move to `runtime::mounts` |
| `engine/server/src/home_stage.rs` (343) | Transactional post-script home: snapshot, baseline file, three-way merge check, `RENAME_EXCHANGE` publication, cleanup | `provision::home_stage`, with typed entry versions |
| `engine/server/guest/codex` | Managed guest launcher that adds `--no-daemon` | `engine/environment/guest/codex` |

### 1.2 Environment pieces in files owned by other planners

| Location | Environment piece | Target API |
| --- | --- | --- |
| `storage.rs` | `atomic`, `create_atomic`, `publish_new`, `read_json`, `write_json`, `hash`, `identifier`, `now` | `persist::{fsx,json,Identifier}`. `path`, `version`, `same`, `missing` and `copy_tree` go to filework. `default_config`, `config`, `app_ref` and `merge` go to workspace |
| `main.rs` `serve` | Creates `.workspace`, takes the `flock` on `.workspace/engine.lock` | `WorkspaceDir::open` |
| `main.rs` `options` | `serve --root --runtime --loader --apk --tools --image --image-index` | The Server parses argv into `EnvironmentOptions`; the flags are unchanged because the App's `EmbeddedWorkspaceBootstrapper` and `EngineIntegrationTest` pass them |
| `main.rs` `environment.*`, `process.*` | Restart choreography: stop chat, `stop_all`, a ten-second wait, `restart`, terminal restore | `Environment::restart(grace)` plus `RuntimeEvent`s; the Server keeps only the wire mapping |
| `main.rs` `services.report` | Writes guest DNS to `.workspace/environment/network/resolv.conf` with mode 0644 | `Environment::set_guest_dns` |
| `main.rs` `documents.*`, `service_document` | Opaque revisioned documents in `.workspace/documents/<ns>/<key>.json`; service files in `.workspace/services/<svc>/<key>` with sidecars in `.workspace/documents/services.<svc>/`; quarantine to `.workspace/corrupt/` | `persist::OpaqueDocuments` and `persist::ConfigFileHandle`; the Server keeps the routing of the `services.` namespace |
| `workspace.rs` `load` | Creates the `.workspace` subdirectories, purges stale uploads, writes the default `env.json`, persists `workspace.json` with a backup and recovery | `WorkspaceDir::open` creates the layout; `EnvironmentDeclaration::DEFAULT_DOCUMENT`; `StateFile<WorkspaceRecord>` (type owned by workspace) |
| `workspace.rs` `validate_text`, `layout.rs` `valid_path`/`service_path`, `imports.rs` `validate` | Which `.workspace` paths are editable, and `env.json` validation | `WorkspaceDir::classify` and the registered `ConfigValidator`s |
| `terminal.rs` | `terminals.json` persistence; calls into `Processes` | `StateFile<TerminalsRecord>` and the `runtime` API |
| `chat.rs` `Host::spawn` | A fourth copy of spawn, `PDEATHSIG` and reaping, plus its own `running` increment | `Runtime::spawn` with `Capture::Stream` |
| `local_services.rs` | Proxy executor intent and receipts | **Not environment.** It belongs to the Server/services planner; it uses `StateFile` and `ConfigFileHandle` |

### 1.3 Runtime, loader and build scripts

`engine/runtime` (crate `workflow-runtime`, dependencies `libc` and `zstd-sys`) has 69,977 lines in `src/`. Of these, 949 are the generated `syscalls.rs`, and three trees, `host/` (23,784), `android_x86_64/` (23,390) and `android_aarch64/` (21,703), contain the same 15 modules. `main.rs` selects a tree with `cfg(all(target_os, target_arch))`. Measured with `diff`:

- Android x86_64 differs from host in 2,092 of 23,390 lines. Most differences are extern declarations (glibc and bionic parameter names, `__errno_location` against `__errno`), type aliases, constants and `stat` layouts. Genuine logic differences are the `#if defined(__ANDROID__)` system binds in `guest.rs` and a few dozen lines in `install.rs`, `meta.rs` and `tracer.rs`.
- Android aarch64 differs from Android x86_64 in 669 of its lines, plus 2,356 x86-only lines. Those are mostly legacy syscall handling (`open`, `stat` and similar) in `sys.rs` and `tracer.rs`, and the register names in `arch.rs`.
- Function bodies are identical between the two Android trees in 9 of 14 modules. `json.rs`, `sha256.rs` and `mem.rs` are byte-identical.
- The archived C source (`docs/archive/native-engine/src`, 7,072 lines) has about 65 preprocessor conditionals, mostly in `sys.c`, `arch.c` and `tracer.c`. That confirms one program with target conditionals.

About 27,000 lines are unique, so roughly 43,000 lines are duplicated. Each tree also redeclares libc types per module (451 type and struct declarations in the host tree, 73 of them duplicated names).

The rest of the area:

- `engine/runtime/{build-host.sh,build-android.sh,test-host.sh,test-android.sh,test-app-sandbox.sh}` build and test under the shared lock with two Cargo jobs.
- `inventory/` and `tools/generate-syscalls.py` produce `syscalls.rs`.
- `tests/native` and `tests/device` contain C probes and oracles. `tests/android-harness` is a standalone Kotlin Gradle test app.
- `engine/loader` (1,308 lines): a `no_std` static-PIE ELF loader built by `build.sh` with `rustc` outside Cargo.
- `engine/build-android.sh` builds the runtime, the loader and `workflow-engine` for both ABIs.
- `app/build.gradle.kts` `BuildEngineTask` globs `engine/**` and packages `libworkflow-{engine,runtime,loader}.so`. `PrepareEnvironmentImageTask` packages `artifacts/image/<arch>/image.{tar.zst,json}` as `assets/environment/`. `PrepareEngineToolsTask` runs `engine/tools/package.py` (Codex, the Temurin JRE and the chat JAR) into `assets/environment/tools/`.
- `image/` holds the image builder (`wfimage` Python package, guest `provision.sh`, `envctl`, tests). It is build tooling for the environment and stays at the repository root.
- `native/pty` (instrumentation-only JNI PTY) and `native/proxy-guard` (root proxy guardian) are **not** environment code. They belong to the App planner.

## 2. Violations found

| Kind | Count | Representative locations |
| --- | --- | --- |
| `Value` indexing (`v["key"]`) in environment-owned files | 259 (environment.rs 129, tools.rs 106, process.rs 12, home_stage.rs 12); storage.rs adds 36 | `environment.rs:151-176` derives status from `state["status"]`/`["stage"]`/`["steps"]`; `tools.rs:308-327` uses `catalog["tools"]...unwrap()`; `environment.rs:749-787` mutates `spec[lang]` and unwraps array lengths |
| `json!` construction of persisted or derived documents | 91 in those four files | `environment.rs:43` (fresh record), `:915` (activation record), `tools.rs:484, :535` |
| Direct `std::fs` calls | 82 in those four files (home_stage.rs 38, environment.rs 23, tools.rs 20, process.rs 1); storage.rs 12, main.rs 12, workspace.rs 18 | `environment.rs:39-41`, `:530-537`; `home_stage.rs` throughout |
| Ad-hoc `.workspace` path building | About 90 literal occurrences in 7 files; 52 `.join(` in environment.rs alone | `".workspace/environment/generations"` is spelled 6 times in environment.rs and once in tools.rs; the guest hide list (7 entries) lives only in `environment.rs:553-566` and is not derived from the layout |
| Spawn with `PDEATHSIG`, pipe draining and reaping | 4 implementations | `environment.rs:586` `checked`, `process.rs:63` `spawn`, `tools.rs:423` `probe`, `chat.rs:73` `Host::spawn` |
| Streaming SHA-256 of a file | 4 | `tools.rs:34`, `home_stage.rs:40` and `:159`, `storage.rs:113` |
| Temporary-file-then-rename publication | 6 | `storage.rs:25` and `:50`, `environment.rs:682` and `:975`, `tools.rs:100` and `:286` |
| Quarantine to `.workspace/corrupt/` | 5 | `environment.rs:56`, `tools.rs:393`, `workspace.rs:105`, `main.rs:120` and `:398` |
| APK asset extraction | 2 | `environment.rs:651-696`, `tools.rs:67-116` |
| Recursive tree copy, each with different semantics | 3 | `environment.rs:718` `merge_seed`, `home_stage.rs:83`, `storage.rs:182` |
| Mandatory tool list | 3 copies in one file | `tools.rs:179`, `:219`, `:237` (plus the status list at `:484`) |
| Architecture name and guest `PATH` | 2 each | `environment.rs:171` and `tools.rs:27`; `environment.rs:575` and `:890` |
| Process-wide mutable statics | 2 | `tools.rs:22-23` (`VERIFIED`, `STATES`, keyed by root) |
| Error kinds as free strings | about 45 distinct `kind` values | `Error::business("busy", ...)` and others; they are wire-visible and must survive the change |
| Runtime tree duplication | about 43,000 lines | See §1.3 |

Tests: `process.rs` has none. Process supervision is covered only by `engine/server/tools/test-environment.py` and device tests.

## 3. Target design

### 3.1 Crate layout

```
engine/environment/
  Cargo.toml            # workflow-environment: serde, serde_json, libc, sha2, uuid, zip; tempfile (dev)
  guest/codex
  src/lib.rs            # re-exports the public API below
  src/error.rs          # EnvError, PersistError, RuntimeError with stable kind()
  src/persist/          # WorkspaceDir, layout table, StateFile, OpaqueDocuments, ConfigFileHandle, fsx, json, ids
  src/declaration.rs    # EnvironmentDeclaration and its validation
  src/model/            # LifecycleRecord, Activation, ImageIndex, ToolCatalog, ToolState, HomeEntry
  src/bundle.rs         # APK or directory asset sources; one bounded copy with digest checking
  src/image/            # ImageStore
  src/tools/            # payload, catalog, status, optional installs
  src/provision/        # build pipeline, seed merge, home_stage
  src/lifecycle/        # state machine, Environment facade, declaration monitor
  src/runtime/          # cli (runtime argv), mounts, supervisor, pty, output, events
  src/settings.rs       # domain API for a settings panel
  runtime/              # workflow-runtime (binary), moved from engine/runtime
  loader/               # freestanding loader, moved from engine/loader
```

The workspace members become `environment`, `environment/runtime`, `environment/loader`, `workspace`, `filework`, `terminal` and `server`. Cargo accepts nested member packages. `zip` moves from the Server manifest; pinned `schemars` derives the Server export from actual serde types. `serde_json` features (`raw_value`, `arbitrary_precision`) are declared once at the workspace level, because feature unification would otherwise change number handling in only some crates.

### 3.2 `.workspace/` layout

Paths are unchanged. One table in `persist::layout` drives directory creation, path classification, the guest hide list and ownership.

| Path under `.workspace/` | Class | Owner (via API) | Guest |
| --- | --- | --- | --- |
| `config.json` | Editable config (settings v2) | workspace registers the validator | visible |
| `env.json` | Editable config (declaration v1) | environment | visible |
| `proxy/<key>` and other `services/<svc>/<key>` | Editable service file, with a sidecar revision | Environment/services | visible |
| `engine.lock` | Private; `flock` held for the Engine's lifetime | environment | hidden |
| `state/workspace.json` (+`.bak`) | Private state | workspace | hidden |
| `state/terminals.json` | Private state | terminal | hidden |
| `state/services/<id>.json`, `state/local-services/<id>.json` | Private state | Server/services | hidden |
| `documents/<ns>/<key>.json` | Private opaque documents (for example the chat send ledger) | environment API, used by chat | hidden |
| `documents/services.<svc>/<key>.json` | Sidecars for service files | environment | hidden |
| `uploads/`, `trash/` | Private areas | filework | hidden |
| `corrupt/` | Quarantined originals | environment | hidden |
| `environment/environment.json` (+`.bak`) | Lifecycle record | environment | hidden |
| `environment/generations/<job>/` | `rootfs`, `seeds`, `post-home`, `home-baseline.json`, `workspace-activation.json`, `home-applied` | environment | hidden |
| `environment/stores/{home/work,toolchains}` | Persistent home and toolchains (`active` symlink) | environment | bound at `/home/work` and `/opt/toolchains` |
| `environment/image/<sha>.{tar.zst,json}` | Extracted image cache | environment | hidden |
| `environment/tools/{archives,payloads,optional,state.json}` | Tool payloads and status | environment | payload bound at `/opt/workflow/tools` |
| `environment/{launchers/codex,network/resolv.conf}` | Generated guest files | environment | bound at `/usr/local/bin/codex` and `/etc/resolv.conf` |

The one deliberate exception outside `.workspace/` is the runtime socket directory, `$TMPDIR/wf-sock-<12 hex>`, because of the 108-byte Unix socket path limit. It moves into `persist::layout` as well. Generation and payload garbage collection remain unimplemented; the stores expose their contents so that a later collector can account for references.

### 3.3 Persistence API used by other crates

```rust
pub struct WorkspaceDir { /* canonical root, lock fd, claims, validators */ }
impl WorkspaceDir {
    pub fn open(root: &Path) -> Result<Arc<Self>, PersistError>;      // creates layout; Busy if locked
    pub fn root(&self) -> &Path;                                       // user files belong to filework
    pub fn classify(&self, path: &RelativePath) -> PathClass;
    pub fn guest_hidden(&self) -> &'static [&'static str];             // relative to /workspace
    pub fn claim<T: StateDocument>(&self, at: StateLocation) -> Result<StateFile<T>, PersistError>;
    pub fn documents(&self) -> OpaqueDocuments<'_>;
    pub fn config(&self, file: ConfigFile) -> ConfigFileHandle<'_>;
    pub fn register_validator(&self, kind: ConfigKind, v: Arc<dyn ConfigValidator>);
    pub fn area(&self, area: Area) -> AreaDir;                         // Uploads | Trash, for filework
    pub fn quarantine(&self, source: &Path, label: &str) -> Result<PathBuf, PersistError>;
}
pub enum PathClass { User, Config(ConfigFile), Private }
pub enum ConfigFile { Workspace, Environment, Service { service: Identifier, key: Identifier } }
pub enum ConfigKind { Workspace, Environment, Service }
pub trait ConfigValidator: Send + Sync { fn validate(&self, text: &str) -> Result<(), ValidationError>; }
pub enum Area { Uploads, Trash }
pub struct AreaDir { /* .. */ }
impl AreaDir { pub fn path(&self) -> &Path; pub fn child(&self, id: &Identifier) -> PathBuf; }
pub struct RelativePath(String);    // workspace-relative, no "", ".", "..", "\\", NUL or leading "/"

pub enum StateLocation { Workspace, Terminals, ServiceReport(Identifier), ServiceControl(Identifier),
                         EnvironmentRecord, ToolState }
pub struct Policy { pub format: Option<u64>, pub backup: bool, pub max_bytes: usize,
                    pub on_unsupported: OnUnsupported, pub on_corrupt: OnCorrupt }
pub enum OnUnsupported { ReadOnly, Quarantine }
pub enum OnCorrupt { RestoreBackup, Fresh }
pub trait StateDocument: Serialize + DeserializeOwned + Send + 'static {
    const POLICY: Policy;
    fn fresh() -> Self;
}
pub enum Loaded<T> { Fresh(T), Clean(T),
                     Recovered { value: T, original: PathBuf, from_backup: bool },
                     Unsupported { found: Option<u64> } }                // the caller stays read-only
pub struct StateFile<T> { /* .. */ }
impl<T: StateDocument> StateFile<T> {
    pub fn load(&self) -> Result<Loaded<T>, PersistError>;   // strict JSON; probes `format` first
    pub fn save(&self, value: &T) -> Result<(), PersistError>; // .bak, temp, fsync, rename, fsync dir
}

pub struct OpaqueDocument { pub revision: u64, pub text: Option<String> }
impl OpaqueDocuments<'_> {
    pub fn read(&self, ns: &Identifier, key: &Identifier) -> Result<OpaqueDocument, PersistError>;
    pub fn write(&self, ns: &Identifier, key: &Identifier, text: &str, expected: Option<u64>)
        -> Result<u64, PersistError>;                         // Conflict on mismatch; 16 MiB cap
    pub fn quarantine(&self, ns: &Identifier, key: &Identifier) -> Result<(), PersistError>;
}
pub struct ConfigText { pub text: Option<String>, pub version: DiskVersion, pub revision: Option<u64> }
pub enum Expect { Any, Revision(u64), Version(DiskVersion) }
impl ConfigFileHandle<'_> {
    pub fn read(&self) -> Result<ConfigText, PersistError>;   // bumps the sidecar if the content changed externally
    pub fn write(&self, text: &str, expect: Expect) -> Result<ConfigText, PersistError>; // validates first
    pub fn ensure(&self, default: &str) -> Result<(), PersistError>;                    // create, never replace
    pub fn quarantine(&self) -> Result<ConfigText, PersistError>;
}

pub mod fsx {   // also used by filework for user files
    pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()>;   // keeps mode
    pub fn create_atomic(path: &Path, bytes: &[u8]) -> io::Result<()>;  // RENAME_NOREPLACE
    pub fn publish_new(from: &Path, to: &Path) -> io::Result<()>;
    pub fn exchange(a: &Path, b: &Path) -> io::Result<()>;              // RENAME_EXCHANGE
    pub fn sha256_file(path: &Path) -> io::Result<Sha256Hex>;
}
pub mod json { pub fn from_slice_strict<T: DeserializeOwned>(b: &[u8]) -> Result<T, JsonError>; }
pub struct Identifier(String);     // 1..=160 bytes of [A-Za-z0-9._-], not "." or ".."
pub struct Sha256Hex([u8; 32]);    // lowercase hex on the wire and on disk
pub struct DiskVersion { pub exists: bool, pub size: u64, pub modified_at: u64, pub sha256: Option<Sha256Hex> }
pub struct UnknownFields(serde_json::Map<String, serde_json::Value>);   // opaque, flattened, preserved
```

`claim` fails if a location is already claimed, so each document has exactly one owning crate. Owners serialize their own access. Policies reproduce current behavior: `workspace.json` and `environment.json` keep a backup and become read-only on an unknown format, `tools/state.json` has no format field and is quarantined on a wrong shape, and `terminals.json` becomes read-only. `from_slice_strict` keeps today's duplicate-key rejection and depth limit of 128.

The rule "no `.workspace` path construction outside the environment" is met as follows. Filework receives `AreaDir`s for `uploads` and `trash` and performs ordinary file IO inside them. It writes editable configuration through `ConfigFileHandle`, which runs the validator registered by the owner. Environment owns configuration validation and Server composes the Chat and Terminal sections, so FileWork does not depend on those domains.

### 3.4 Typed declaration (`env.json`)

```rust
pub struct EnvironmentDeclaration {
    pub version: DeclarationVersion,                  // only 1
    pub python: Option<Vec<PythonSpec>>,              // None: image default; Some([]): disabled
    pub node: Option<Vec<NodeSpec>>,
    pub packages: Option<Vec<PackageName>>,
    pub env: Option<BTreeMap<EnvKey, EnvValue>>,
    pub post_scripts: Option<Vec<PostScript>>,
    pub unknown: UnknownFields,
}
pub struct PostScript { pub id: Identifier, pub run: ScriptBody, pub user: GuestUser, pub unknown: UnknownFields }
pub enum GuestUser { Work, Root }
pub struct PythonSpec(VersionSpec);   // 1-3 numeric u16 parts, no leading zeros, major == 3
pub struct NodeSpec(VersionSpec);     // major >= 1
pub struct PackageName(String);       // [a-z0-9+.:-], starting alphanumeric
pub struct EnvKey(String);            // [A-Za-z_][A-Za-z0-9_]*
pub struct EnvValue(String);          // no NUL
impl EnvironmentDeclaration {
    pub const DEFAULT_DOCUMENT: &'static str; // {"version":1,"packages":[],"env":{},"post_scripts":[]}
    pub fn parse(text: &str) -> Result<Self, DeclarationError>;
    pub fn input_fingerprint(&self) -> Fingerprint;          // used for failure suppression
    pub fn resolve(&self, defaults: &LanguageDefaults) -> ResolvedDeclaration;
}
pub struct ResolvedDeclaration { /* every field present, defaults applied, unknown preserved */ }
impl ResolvedDeclaration { pub fn fingerprint(&self) -> Fingerprint; }    // activation identity
pub struct DeclarationError { pub problems: Vec<Problem> }    // Display = first problem's current text
pub struct Problem { pub field: String, pub message: String }
```

Compatibility rules:

- Both fingerprints are `sha256(serde_json::to_vec(&serde_json::to_value(x)?))`. Going through `Value` sorts keys exactly as today's `BTreeMap`-backed values do.
- Absent optional fields are skipped, and unknown top-level and post-script fields are included. An existing activation therefore matches its declaration and does not rebuild.
- `requestedSpecHash` remains `"<size>:<sha256 of raw bytes>"`.
- Validation collects every problem for a settings panel. Its `Display` keeps the first message string that clients see today, for example `"node: duplicate or invalid version"`.

The private record keeps its current field names through explicit serde renames: `LifecycleRecord { format, active, pending, previous?, failure, status, stage, step, steps, jobId?, requestedSpecHash?, unknown }`, `Activation { generation, profile, config, environment, fingerprint, imageSha256, verified, verifiedAt }`, `Failure { stage, message, fingerprint? }`. Optional keys that are absent today stay absent. In particular, the presence of `requestedSpecHash` still means the environment is enrolled for monitoring.

### 3.5 Bundle and image store

`EnvironmentOptions { runtime, loader, image: Option<ImageSource>, tools: Option<ToolSource> }`, with `ImageSource::{Files { archive, index }, Apk(path)}` and `ToolSource::{Directory(path), Apk(path)}`.

`bundle` replaces both APK readers with one bounded read and one bounded copy that hashes while copying. Hashing during the copy is new for the image: a corrupt APK image then fails early with `invalid_image` instead of exit code 65 from the runtime. It costs no extra read.

`ImageStore` validates the typed `ImageIndex`/`ImageMetadata`: `workflow-image` format 2, `debian-trixie` type 1, profile `workspace`, and an architecture equal to `Architecture::current()`, a check that is new on the Server side. It extracts into `environment/image/`, installs generations through the typed `runtime install` argv, and exposes `bundled_defaults()` so the settings panel can show default language versions without extracting anything. Both ABIs keep shipping customized images; the base profile is still rejected.

### 3.6 Tools and provisioning

`Tools` owns the payload (typed `ToolCatalog`, safe extraction, a per-instance verification cache instead of the process-wide statics), the binds and launcher, measured `ToolState`, and optional installs. Its public API is `status()`, `install(ToolId, retry)` and `binary(ToolId)`. Probes and the Claude download run through `Runtime` and still count as running processes. The mandatory set is one constant. It drops `jre` and `chat` once Rust chat lands (task ENV-7).

`provision::build` keeps today's step order, progress counts and messages: image install, seed policy, packages, each Python and Node version, `verify-many`, tool verification, home staging, post-scripts, activation record and generation verification. Build steps run on the candidate generation with `ProcessOwner::Build`. That owner is excluded from the running count but stopped on shutdown, just as today's `checked` children die with `PDEATHSIG`. `home_stage` keeps its algorithm and its `home-baseline.json` shape, so a pending generation built before the change still activates.

### 3.7 Lifecycle state machine

```
                 reconcile (changed or retry)
 Unavailable ───────────────┐                      restart: stop_all ok, activate
 Ready ─────────────────────┼──► Building{job,stage,step,steps} ──► PendingRestart ──────────► Ready
 Failed{failure,fp} ────────┘          │ success, running == 0 ──────────────────────────────► Ready
                                       │ failure ──► Failed (active generation still usable)
                                       │ declaration changed mid-build ──► previous state, then reconcile again
 load: status=applying ──► Ready or Unavailable with failure "interrupted"
 load: unknown format  ──► ReadOnly (never written)
 restart: activation fails ──► Failed, pending cleared, `toolchains/active` symlink restored
```

`Phase` (`not_installed`, `installing`, `building`, `ready`, `needs_restart`, `failed`) and `usable` are derived from this state. They are not stored separately. An unchanged failed fingerprint suppresses automatic rebuilds until an explicit retry. Files-only connections never extract an image, because monitoring starts only after the first explicit reconcile.

```rust
#[derive(Clone)] pub struct Environment(Arc<Shared>);   // Send + Sync; callers never lock it
impl Environment {
    pub fn open(dir: Arc<WorkspaceDir>, options: EnvironmentOptions) -> Result<Self, EnvError>;
    pub fn status(&self) -> EnvironmentStatus;
    pub fn wait_changed(&self, after_revision: u64, timeout: Duration) -> u64;
    pub fn reconcile(&self, retry: bool) -> Result<EnvironmentStatus, EnvError>;  // explicit; enrolls
    pub fn poll_declaration(&self);            // the monitor thread calls this every 750 ms
    pub fn restart(&self, grace: Duration) -> Result<RestartOutcome, EnvError>;
    pub fn set_guest_dns(&self, servers: &[IpAddr]) -> Result<(), EnvError>;
    pub fn runtime(&self) -> &Runtime;
    pub fn tools(&self) -> &Tools;
    pub fn settings(&self) -> EnvironmentSettings<'_>;
    pub fn shutdown(&self);                     // stops the monitor and kills every process
}
pub struct EnvironmentStatus { pub revision: u64, pub record: LifecycleRecord, pub phase: Phase,
    pub usable: bool, pub progress: Option<f64>, pub error: Option<String>,
    pub running_processes: usize, pub architecture: Architecture }
pub enum RestartOutcome { NothingPending(EnvironmentStatus),
                          Activated { status: EnvironmentStatus, stopped: StopReport } }
pub enum Architecture { Arm64, Amd64 }       // the single cfg-derived definition
```

`EnvironmentStatus` carries the full record so the Server can reproduce today's `environment.status` result exactly. The real-image harness reads `active.generation` and `active.verified`.

### 3.8 Runtime API consumed by chat and terminal

```rust
pub struct Runtime { /* registry, lifecycle gate, mount planner */ }
pub struct ProcessSpec { pub argv: Vec<String>, pub cwd: GuestPath, pub user: GuestUser,
    pub env: BTreeMap<EnvKey, EnvValue>, pub io: ProcessIo, pub owner: ProcessOwner, pub label: Option<String> }
pub enum ProcessIo { Pipes { capture: Capture }, Pty { size: PtySize, capture: Capture } }
pub enum Capture { Ring { capacity: usize }, Stream }        // Stream: the caller takes the readers
pub enum ProcessOwner { Terminal, Chat, ToolProbe, Api }     // Build is crate-private
pub struct PtySize { rows: u16, columns: u16 }               // PtySize::new rejects 0 or > 1000
pub struct GuestPath(String);                                // normalized absolute guest path
pub enum GuestSignal { Interrupt, Terminate, Kill, Hangup }  // sent to the tracer's process group
impl Runtime {
    pub fn spawn(&self, spec: ProcessSpec) -> Result<ProcessHandle, RuntimeError>;
    pub fn get(&self, id: &ProcessId) -> Option<ProcessHandle>;
    pub fn running(&self) -> usize;
    pub fn stop_all(&self, grace: Duration) -> Result<StopReport, RuntimeError>; // TERM, then KILL; spawns wait
    pub fn run_to_completion(&self, spec: ProcessSpec, limits: OutputLimits, timeout: Duration)
        -> Result<Completed, RuntimeError>;
    pub fn locate(&self, path: &GuestPath) -> GuestLocation;  // Workspace(RelativePath) | Home(String) | Other
    pub fn subscribe(&self, listener: Box<dyn Fn(&RuntimeEvent) + Send + Sync>);
}
pub enum RuntimeEvent { Stopping, Activated { generation: Identifier }, Exited { id: ProcessId, status: ExitStatus } }
#[derive(Clone)] pub struct ProcessHandle(Arc<Process>);
impl ProcessHandle {
    pub fn id(&self) -> &ProcessId;
    pub fn status(&self) -> ProcessStatus;                        // Running | Exited(ExitStatus)
    pub fn read(&self, stream: Stream, offset: u64, max: usize, wait: Duration) -> Result<Chunk, RuntimeError>;
    pub fn take_streams(&self) -> Option<ProcessStreams>;         // Capture::Stream only
    pub fn write(&self, bytes: &[u8]) -> Result<usize, RuntimeError>;
    pub fn close_input(&self);
    pub fn resize(&self, size: PtySize) -> Result<(), RuntimeError>;
    pub fn signal(&self, signal: GuestSignal) -> Result<(), RuntimeError>;
    pub fn wait(&self, timeout: Duration) -> ProcessStatus;
    pub fn clear_output(&self, stream: Stream);
}
pub struct Chunk { pub data: Vec<u8>, pub start: u64, pub next: u64, pub eof: bool, pub exit: Option<ExitStatus> }
```

The domain invariants are a 128-process cap, 4 MiB rings, a 1 MiB stdout and 64 KiB stderr cap for build steps, and the exit code `128 + signal`. Wire limits such as `waitMs <= 1000` and `timeoutMs <= 30000` stay in the Server. `spawn` fails with `environment_unavailable` when nothing is active, and `stopping` while `stop_all` holds the gate. That gate replaces the Server's `execution` mutex for the race between spawn and restart.

`runtime::cli` is the only place that builds `workflow-runtime` argv (`run`, `install`, `verify`). `runtime::mounts` derives binds and hides from the layout table and `Tools`. There are two modes: interactive (workspace, live home and hides) and build (candidate generation and `post-home`). Unit tests use a `#[cfg(test)]` launcher. Production has no host execution path.

### 3.9 Settings panel domain API

```rust
pub struct EnvironmentSettingsView { pub text: Option<String>, pub version: DiskVersion,
    pub declaration: Result<EnvironmentDeclaration, DeclarationError>,
    pub image_defaults: Option<LanguageDefaults>, pub architecture: Architecture,
    pub status: EnvironmentStatus, pub tools: ToolsSnapshot }
pub enum ApplyOutcome { Saved { version: DiskVersion, status: EnvironmentStatus }, Conflict { current: DiskVersion } }
impl EnvironmentSettings<'_> {
    pub fn view(&self) -> Result<EnvironmentSettingsView, EnvError>;
    pub fn validate(&self, proposed: &EnvironmentDeclaration) -> Result<(), DeclarationError>;
    pub fn apply(&self, proposed: &EnvironmentDeclaration, expected: &DiskVersion) -> Result<ApplyOutcome, EnvError>;
}
```

`apply` performs a compare-and-set write of `env.json` through `ConfigFileHandle`. It preserves unknown fields, writes canonical pretty JSON, and enrolls and reconciles the environment. Restart and optional tools are already part of `Environment` and `Tools`. An editor draft of `env.json` sees a panel save as an ordinary external change and reports a conflict. The Server planner defines the wire methods for this API.

### 3.10 Errors

`EnvError`, `PersistError` and `RuntimeError` are enums with `kind() -> &'static str` and `message()`. Every current kind string is kept, among them `busy`, `processes_running`, `unavailable`, `environment_unavailable`, `runtime_unavailable`, `unsupported_format`, `invalid_apk`, `missing_image`, `invalid_image`, `unsupported_image`, `too_large`, `build_failed`, `verification_failed`, `tool_version_mismatch`, `invalid_tools`, `home_stage`, `home_changed`, `home_conflict`, `limit`, `not_found`, `closed`, `not_terminal`, `invalid_offset`, `spawn_failed`, `stop_timeout` and `conflict`. The Server maps them mechanically to JSON-RPC errors.

### 3.11 Runtime tree merge

The target is one module set: `src/{cli,exec,guest,ident,install,json,mem,meta,path,scratch,sha256,sys,tracer}.rs`, plus `src/platform/{linux_gnu_x86_64,android_x86_64,android_aarch64}.rs` for libc declarations, struct layouts and the errno accessor, plus `src/arch/{x86_64,aarch64}.rs` for registers and legacy syscalls.

The merge takes the Android x86_64 tree as its base. Each divergent hunk becomes an item-level `#[cfg]` alternative drawn from a fixed vocabulary, such as `cfg(all(target_os = "android", target_arch = "aarch64"))`. Extern declarations that differ only in parameter names are unified.

A new `tools/check-platform-merge.py` evaluates those cfgs for each of the three targets. It compares the resulting item multiset with the original tree from a baseline Git revision after `rustfmt` and parameter-name normalization, and it rejects cfg forms outside the vocabulary.

Replacing the c2rust libc declarations with the `libc` crate is deferred, because it changes FFI surfaces that only device runs can validate.

## 4. Implementation tasks

The coordinator should freeze `engine/server/src/{environment,process,tools,home_stage}.rs` from the start of ENV-1 until ENV-4 lands, so the old and new implementations cannot drift. All Cargo work uses `tools/workflow check` or `tools/with-build-lock.sh`, with two jobs. No task adds a prebuilt; ENV-7 removes one (the JRE).

| ID | Objective | Depends on | Tier |
| --- | --- | --- | --- |
| ENV-1 | Crate foundation: `persist`, typed models, errors | coordinator answers to Q1–Q4 | astra-max |
| ENV-2 | `runtime` module: supervisor, PTY, argv, mounts, events | ENV-1 | astra-max |
| ENV-3 | Bundle, images, tools, provisioning, lifecycle, settings | ENV-1, ENV-2 | astra-max |
| ENV-4 | Server cutover to `workflow-environment` | ENV-3; before any server-domains split | astra-max |
| ENV-5 | Relocate the runtime, loader and tools packager | ENV-1 merged (shares `engine/Cargo.toml`) | sol-high |
| ENV-6 | Single-tree runtime with cfg platform modules | ENV-5 | astra-max |
| ENV-7 | Remove the JRE and chat JAR from the tools payload (done 2026-10-04, before device acceptance by user decision) | ENV-4, Rust chat cutover (chat appendix), App removal of `:engine-chat` | sol-high |
| ENV-8 | Boundary guard and environment documentation set | ENV-4, ENV-6, documentation reorganization (App appendix) | sol-high |

**ENV-1.**
- Owns `engine/environment/{Cargo.toml,README.md,src/lib.rs,src/error.rs,src/persist/**,src/declaration.rs,src/model/**}`, plus `engine/Cargo.toml` (members and workspace dependencies only) and `engine/Cargo.lock`.
- The coordinator adds the `rust-environment` suite: `cargo test --manifest-path engine/Cargo.toml --locked -p workflow-environment -j 2`, marked heavy.
- Checks: `rust-environment`, `rust-server`, `documentation`.
- Acceptance:
  - Fixtures captured from the current Server load and save without losing fields: `workspace.json`, `environment.json` with active, pending and previous generations, `tools/state.json`, `terminals.json`, an opaque document envelope and a service sidecar.
  - Corrupt input is quarantined and the backup restored, and unknown formats stay read-only. These mirror the existing `corrupt_state_*` and `environment_corruption_*` tests.
  - Fingerprint golden tests match the legacy `Value` algorithm, kept in the test, for at least 10 declarations, including unknown keys and absent arrays.
  - Each current `env.json` error message is reproduced, and `claim` rejects a second owner.

**ENV-2.**
- Owns `engine/environment/src/runtime/**`.
- Checks: `rust-environment`. Also run an `#[ignore]` host test against a real runtime built by `engine/environment/runtime/build-host.sh` under the lock, using `WORKFLOW_RUNTIME` and the pristine `artifacts/engine/rootfs-amd64`.
- Acceptance:
  - The generated argv equals today's `guest_command` output, byte for byte, for interactive and build modes, including the seven hides and the environment ordering.
  - Ring offset, eof and exit semantics match `process.rs`. A PTY child gets a controlling terminal and `TIOCSWINSZ` takes effect.
  - `stop_all` escalates from TERM to KILL within the grace period, spawns wait or fail during a stop, the 128-process cap is enforced, and `Capture::Stream` delivers bytes in order.
  - The real-runtime test runs a PTY shell, resizes it, signals it and waits for it.

**ENV-3.**
- Owns `engine/environment/src/{bundle.rs,image/**,tools/**,provision/**,lifecycle/**,settings.rs}`.
- Checks: `rust-environment`.
- Acceptance:
  - The 12 tests from `tools.rs`, `home_stage.rs` and `environment.rs` are ported and pass.
  - A transition test covers every edge in §3.7: interrupted build, change during a build, failure suppression and retry, pending restart, and activation failure with the symlink restored.
  - The settings `apply` compare-and-set is tested, and a corrupt image is rejected during the copy.
  - `grep -nE '\[\"[A-Za-z_]+\"\]' engine/environment/src` finds nothing outside `UnknownFields` helpers.

**ENV-4.**
- Owns the deletion of `engine/server/src/{environment,process,tools,home_stage}.rs`.
- Call-site edits in `engine/server/src/{main,storage,terminal,chat,workspace,layout,imports,tests}.rs` and `engine/server/Cargo.toml` are temporarily assigned by the coordinator.
- Also owns `git mv engine/server/guest engine/environment/guest`, `docs/engine/{environment,runtime,workspace,server}.md` and `docs/report/<date>-environment-crate.md`.
- Wire output stays unchanged.
- Checks:
  - `rust-server`, `rust-environment`, `documentation`, `infrastructure`.
  - `engine/server/tools/test-protocol.py` and `test-environment.py` with host-built binaries and `artifacts/image/amd64`.
  - `android-apk`, then `tools/workflow device` with `EngineIntegrationTest`, `WorkbenchEngineAcceptanceTest` and `TerminalAttachmentTest` on API 28 x86_64.
- Acceptance:
  - Golden wire captures from the pre-change Server (volatile fields masked) match for `environment.status`, `environment.tools.status`, `process.*` and `documents.*`.
  - The real lifecycle harness prints its PASS line.
  - A copied, populated `.workspace/` opens as `ready` with no new generation.
  - Device runs have zero skips.

**ENV-5.**
- Owns `git mv engine/runtime engine/environment/runtime` and `git mv engine/loader engine/environment/loader`, with path edits in their scripts, `tests/device/cases`, `tests/android-harness/build.gradle.kts` and `generate-syscalls.py`.
- Also owns `engine/build-android.sh`, `engine/tools/**` (moving to `image/tools-payload/**`, see Q8), the member path in `engine/Cargo.toml` and the path mentions in `docs/engine/runtime.md` and `docs/development/testing.md`.
- Requested edits: `tools/workflow-suites.json` `runtime-host` (coordinator) and the `package.py` path in `app/build.gradle.kts` (App planner).
- Remove the ignored `build/` and `.gradle/` directories of the old harness before moving it.
- Checks: `runtime-host`, `python3 engine/environment/runtime/tools/generate-syscalls.py --check`, `engine/build-android.sh` for both ABIs under the lock, `image`, `documentation`.
- Acceptance:
  - `git diff -M --stat` shows renames plus path edits only, and runtime-host passes 68/68.
  - Each ABI produces three libraries.
  - No active code/build link uses the old runtime or loader path. Baseline inventory and relocation prose may retain the old names explicitly.

**ENV-6.**
- Owns `engine/environment/runtime/src/**`, `engine/environment/runtime/tools/check-platform-merge.py`, the runtime README and `docs/engine/runtime.md`.
- Checks:
  - The equivalence check for all three targets against the ENV-5 commit.
  - `cargo check -p workflow-runtime` for the host, `x86_64-linux-android` and `aarch64-linux-android` under the lock.
  - `runtime-host` in optimized and debug builds.
  - `engine/environment/runtime/test-android.sh` (24 baseline and 29 M5 cases through `tools/with-emulator.sh`).
  - `android-apk` plus `EngineIntegrationTest`.
- Acceptance:
  - The equivalence report shows zero unexplained differences, and the source falls from about 70,000 to under 30,000 lines.
  - ARM64 evidence is limited to compilation and equivalence, and `docs/status.md` says so.
  - Any module that cannot be proven equivalent stays per-platform under `platform/`, and the report says which.

**ENV-7.**
- Owns `engine/environment/src/tools/**`, `image/tools-payload/**` and the removal of `third_party/jre/**`, coordinated with the App notices task.
- Owns the tool-list text in `docs/engine/{environment,protocol}.md`; the Server planner owns the wire change.
- Checks: `rust-environment`, `rust-server`, `android-apk`, `EngineIntegrationTest` together with the chat device acceptance.
- Acceptance:
  - The payload holds only Codex and the notices, and `tools.status` lists `codex` and `claude`.
  - Existing generations bind the new payload without a rebuild or post-script replay.

**ENV-8.**
- Owns `tools/tests/test_engine_boundaries.py` (new, coordinator approval), the environment pages under the new `docs/engine/` tree, and the environment section of `docs/status.md`.
- Checks: `infrastructure`, `documentation`.
- Acceptance: the guard rejects seeded violations and passes on the tree. It checks for:
  - a `.workspace` literal outside `engine/environment`;
  - `Command::new` or `openpty` outside `engine/environment/src/runtime`;
  - `Value` index syntax in `engine/environment/src`;
  - a second definition of `sha256_file` or `write_atomic`.

Cross-module assumptions:

- Server: maps wire types to this API after ENV-4 and owns `services.` namespace routing and settings wire methods. Local service intent, tickets, receipts and configuration domain behavior move to Environment.
- Filework: uses `classify`, `AreaDir`, `ConfigFileHandle`, `fsx` and `DiskVersion`.
- Workspace: supplies session/layout state for the combined Server transaction; configuration validation follows the Environment/Chat/Terminal ownership split.
- Terminal: uses PTY spawn, `locate`, `RuntimeEvent::Activated` for restore, and `StateLocation::Terminals`.
- Chat: uses `Capture::Stream`, `Tools`, and `OpaqueDocuments` (namespace `chat`) for the existing send ledgers.
- App: keeps the Engine CLI flags and library names unchanged.

## 5. Round-1 decisions and remaining gates

The declaration and persisted paths/field names/fingerprints stay compatible. `.workspace/` is not renamed. The foundation exposes a typed `Store` with strict bounded generic reads, atomic writes, backup, quarantine, directory listing, workspace lock and upload staging; each domain owns its document type and recovery policy. Public file operations remain in FileWork. Store-relative typed keys are centralized in Environment and validated; handing another crate an unrestricted private root is not the intended API.

Environment owns service executor tickets, intent, receipts, storage and resolver generation; Server owns their wire composition. Environment also owns appearance, overlay and client configuration; Chat owns backend defaults and Terminal owns terminal settings. Domain data structs derive serde/schema so Server can export them without duplicating every domain model. Environment uses standard threads and locks; no async runtime is added. The CLI and all three packaged native library names remain unchanged. Source tooling/tests can include Python, C probes and the existing Android harness; the Rust-only rule applies to production Engine implementation. Kotlin production Chat is the explicit round-1 exception, removed only after parity and device proof.

The runtime and loader move together. Loader remains a specialized `no_std` executable with its static-PIE build flags; adding a Cargo package must not replace its known Android linker settings. `engine/tools/package.py` remains build tooling during round 1, and moving it is coordinated with App/JRE removal. The draft proposed a new settings facade and runtime subscription API; these are later API refinements, not extra round-1 wire methods. Optional Claude provisioning continues through the existing explicit tools operation and guest service policy; adding a new declaration field requires schema/product agreement first.

The claim that the platform trees can be mechanically merged is withdrawn. No host-only run proves Android bionic ABI equivalence or ARM64 register behavior. The safe round-1 decision is relocation without deduplication. The separately owned `runtime-dedup` task must compare selected target behavior to the retained baseline, run `runtime-host`, cross-build Engine for both ABIs and obtain isolated API 28 x86_64 guest exec, PTY and stop acceptance. Physical ARM64 device proof remains a reported gap; the daily tablet is not available for it. Leave the full original trees intact if the extraction cannot complete, never a partial merge.

## 6. Later-round tasks

| Round/task | Owned paths | Dependencies | Checks |
| --- | --- | --- | --- |
| Separate `runtime-dedup` task | `engine/environment/runtime/src/**`, equivalence checker and runtime tests | Preserved round-1 baseline; audited cfg vocabulary for OS and architecture | Host oracle before/after, both Android targets, isolated Android runtime suites; report ARM64 limits |
| R2 Environment settings/API refinement | `engine/environment/src/**`, Server settings types/export, App settings owner | Typed store/declaration and config-sync revision contract | Declaration/default/unknown-field tests, conflict and restart tests, schema drift, isolated settings acceptance |
| R2 ENV-7 JVM removal | Environment tools, `engine/tools`, image tooling, JRE manifests/notices and packaging lines | Rust Chat parity and isolated device cutover | Tools catalog/measurement tests, image, both ABI APKs, real guest tool probes |
| R3 persistence/lifecycle audit | Environment store/runtime, domain call sites, boundary guard, `docs/engine/environment.md` and runtime/loader pages | Integrated domain and App moves | Malformed/newer-format preservation, interrupted atomic writes, stop convergence, private mount masking, infrastructure and documentation |
