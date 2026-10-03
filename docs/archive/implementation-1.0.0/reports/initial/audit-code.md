# Code and architecture audit, 0.2.1 baseline

Date: 2026-09-26. Auditor role: code and architecture, and the only Gradle owner in this phase.
Scope: `app/src` (Kotlin, C/C++, assets, manifest), `runtime/`, `tools/`, `web/`, build files.
The source tree matches the baseline snapshot (`artifacts/baseline/repo.git`). The only differences are new docs written by other agents.
Evidence is in `artifacts/audit-code/`: `gradle-build.log`, `junit/`, `lint-results-debug.sarif`, `python-tests.log`, `web-tests.log`, `native/native-tests.log`.

## 0. Executive summary

- **The build is healthy.** All tests pass: JVM 74/74, Python 12/12, web 15/15, native host 21/21. Lint reports 0 errors and 35 warnings. There are no Kotlin compiler warnings.
- **The low-level pieces are good and worth keeping.** These are the JNI PTY (`pty_process.c`), the root guardian (`proxy_guard.c`), `StreamingUtf8Decoder`, `TerminalOutputWindow` and `TerminalDeltaTracker`, `CodexNetworkBridge`, `LocalRuntimeService` leases, `LocalProxyConfig`, the strict `Json` codec, and the Session/Working Resource *policy*: `SessionPolicy` plus the archive-decision rules, covered by 43 core tests.
- **The layer between those pieces and the UI is rotten and should be rewritten.** This covers `WorkflowController` (334 lines) and `RuntimeUi` (535 lines), which are god objects:
  - They do main-thread disk IO.
  - They keep five parallel draft/state mirrors.
  - Agent state lives in a ViewModel, so background deltas are dropped after the ViewModel is recreated.
  - The Codex protocol, the legacy HTTP daemon, terminals, account handling and persistence are all mixed together.
- **`WorkspaceRepository` is correct but has the wrong execution model.** Every call re-reads the whole state (manifest, every draft file, every composer file) and re-writes it with an fsync and a backup copy. The UI calls it synchronously on the main thread, and calls it on every keystroke.
- **The code does not have the structures the product needs:**
  - no **Stack**. `Layout` is a flat panel list with MAIN, SIDE or BOTTOM positions.
  - no **Claude Code** backend, and no agent-backend abstraction at all.
  - no consumer of `container.json`.
  - no dedicated UI for 8 of the 11 Codex server requests.
- **Termux/HTTP is still wired into the product.** This includes the Settings guide, the `RUN_COMMAND` permission, `<queries com.termux>`, an exported runtime bundle asset, `WorkflowRuntimeClient`, the HTTP polling branch in `RuntimeUi`, and the config fields `daemonUrl`/`daemonToken`/`codexEndpoint`/`codexTransport=http`. **Recommendation: delete the Python daemon and all Termux/HTTP paths outright.** Keep only the image tooling (see §1.4).
- **About 700 MB of third-party binaries sit untracked in the source tree and are not git-ignored.** These are `app/src/main/jniLibs/arm64-v8a/libcodex.so` (247 MB) and `libmihomo.so` (64 MB), plus a duplicate `runtime/vendor/` (376 MB). They should be fetched by a Gradle task verified by SHA-256 into `build/`.
- **Dependencies are essentially at latest.** Behind are Codex 0.157.0 (0.157.1 was released today) and NDK 30.0.15729638 (30.0.16248370 is on the stable channel). `material-icons-extended` 1.7.8 is frozen upstream.

## 1. Component inventory and verdicts

Legend: **KEEP**: use as-is, maybe moved. **REFACTOR**: keep the logic or tests, restructure. **REWRITE**: replace; the old code is reference only. **DELETE**: remove.

### 1.1 `app/src/main/java/.../core` (Android-free today; keeps that property)

| File | Lines | Purpose / quality | Tests | Verdict |
| --- | --- | --- | --- | --- |
| `WorkspaceModels.kt` | 186 | Session, Layout, Panel, WorkingResource, config models, and `SessionPolicy`: 1 and 7 day retention, latest-live-reference protection, persistent ranking by log-normalized frequency and recency. The policy is clean and deterministic. | 19 + 12 + 6 | **KEEP** `SessionPolicy`, `Session`, `WorkingResource`. **REWRITE** `Layout`/`Panel`/`PanelPosition` into Paradigm → Region → Stack → Panel, with state schema v2 and a migration. Remove dead config fields (§5.1). |
| `WorkspaceCodec.kt` | 100 | Hand-written map codec. Keeps unknown keys in `config.json`/`container.json` and validates enums and paths. | covered | **KEEP**, then extend for v2. |
| `Json.kt` | 149 | Strict JSON parser and writer. Rejects duplicate keys and trailing data, and limits depth to 100. Good for user-editable declarative files. | covered | **KEEP** for disk and declarative files. Do not use it for protocol frames (§4). |
| `ConversationComposer.kt` | 27 | Revisioned unsent draft. Acknowledgement is scoped to the owner and the exact revision. | 6 + 12 | **KEEP**. |
| `WorkspaceRepository.kt` | 530 | Path confinement (canonical paths, symlink and sibling-prefix safe). Atomic write with fsync plus `.bak`. Recovery keeps the damaged bytes. Content-addressed immutable draft files. Revision-checked composer files. Archive decisions (SAVE/KEEP/DISCARD, with composer SAVE rejected). External-edit conflict detection by base digest. | 19 core + 12 archive + 6 recovery; strong | **REFACTOR** (§3.1). Keep the on-disk format, confinement, atomic IO, recovery and every test. Replace the execution model with a single in-memory store that has one writer, async mutations and change flows. Split it into ManifestStore, DraftStore, ComposerStore, ConversationStore and UserFileStore. |

The tests that encode the **archive protection policy** are `ComposerArchiveTest` (12 cases, including "only latest live reference … blocks automatic archive" and "file and composer protections are combined") and `WorkspaceRepositoryTest` (retention boundaries, latest-reference protection, deterministic tie-break, manual decisions). They must move unchanged into the new `:core` module and keep passing.

### 1.2 `ui/` (Compose), `MainActivity`, `platform/`

| File | Lines | Findings | Verdict |
| --- | --- | --- | --- |
| `ui/WorkflowController.kt` | 334 | This is a god object. It holds Compose state, navigation, persistence calls, the composer state machine, the draft overlay and the archive transaction. `repository.load()`/`autoArchive()` run in the constructor on the main thread; the constructor runs from `Activity.onCreate`. `perform{}` runs repository writes synchronously on the main thread. `applyState(repository.load())` runs on `Dispatchers.Main.immediate` after each background edit. It keeps five parallel sources of truth: `state`, `pendingEdits`, `composers`, `composerPersistedRevisions` and `acknowledgedSaves`. It depends on error strings (`notice.startsWith("File already exists")` in WorkbenchScreen). | **REWRITE** as feature ViewModels over a process-scoped `WorkspaceStore`. |
| `ui/RuntimeUi.kt` | 535 | Mixes the Codex JSON-RPC client, the HTTP daemon poller, conversation persistence (the whole transcript every 400 ms), terminal lifecycle and mapping file, account/login and the model list. It lives in `WorkflowHost` (a ViewModel). When a new instance starts, `threadResources` is empty, so deltas for conversations it has not loaded are dropped (`resource ?: return`). Frames with a blank `threadId` are attributed to whatever conversation is loaded. It handles only 11 notification methods; everything else goes to a 200-entry debug list. It hard-codes `version "0.1.0"` in `initialize`. It reads the terminal mapping file and conversation JSON on the main thread. `completedTurns`/`resumedThreads` grow without bound. It uses `org.json`, so it cannot be tested on the JVM. | **REWRITE**: an `:agent` module (backend-neutral model plus Codex and Claude adapters) and a process-scoped `AgentHub`. |
| `ui/WorkflowApp.kt` | 99 | Uses NavigationBar/NavigationRail, which contradicts the brief ("不要用 NavigationBar"). Navigation is a hand-rolled destination enum plus history list. | **REWRITE** (UI agent). |
| `ui/WorkbenchScreen.kt` | 293 | Two hard-coded composables (FileEditorPanel and ChatPanel) swap between main and side. There is no Stack, no splits and no panel drag between regions. `c.fileList()` does disk IO inside `remember` in composition. `createFile` runs on click on the main thread. There is a 56 dp header, a 44 dp tab row and a status row (repeated chrome). | **REWRITE**. |
| `ui/ChatPanel.kt` | 285 | Creates one **WebView per message** (`MarkdownContent` inside `LazyColumn`), and each WebView loads marked, KaTeX and DOMPurify. Server requests appear as a single "需要你的确认" chip plus an AlertDialog. Only command/file approval and requestUserInput have forms; the other 8 need hand-typed JSON. There is **no auto-approval**. The drag-and-drop permission handling (`requestDragAndDropPermissions`, release after import) is correct and worth reusing. | **REWRITE**; reuse the DnD import and the "explicit decision" semantics. |
| `ui/SoraEditor.kt` | 185 | The conflict-refresh logic is careful (edit generation, acknowledged-save digest). But each keystroke does `text.toString()` on the whole document, then `editDraft` → full repository round trip (§3.1). | **REFACTOR**: move buffer and conflict state into an `EditorDocument` controller with a debounced draft flush. |
| `ui/FileChanges.kt` | 45 | A FileObserver on the parent directory survives atomic renames; debounced 100 ms. | **KEEP**; move it into a platform file-watch service that feeds the store. |
| `ui/FilePreview.kt` | 55 | Image decode on IO with sampling; FileProvider open-with. | **KEEP**. |
| `ui/FileActions.kt` | 92 | Shared import for camera, gallery and files; attachment ownership is captured before launch (good). It reads up to 16 MB into memory, then `c.refresh()` on main. | **REFACTOR**: stream to a temp file and add it to the store asynchronously. |
| `ui/LauncherScreens.kt` / `SettingsScreens.kt` / `ProxyScreen.kt` / `AccountScreen.kt` / `Components.kt` / `WorkspaceDragShadow.kt` / `theme/*` | 219 / 233 / 257 / 85 / 88 / 35 / 75 | Launcher calls `queryIntentActivities` + `loadLabel` inside `remember` (main thread). About half of Settings is the Termux guide, runtime export and `RUN_COMMAND` request. The theme uses `MaterialExpressiveTheme`, but no screen uses Expressive components. | **REWRITE** Settings (drop Termux). **REFACTOR** the others (UI agent owns the look). **KEEP** `WorkspaceDragShadow`. |
| `ui/web/ResourceWebViews.kt` | 239 | Security posture is correct: `WebViewAssetLoader` on the appassets origin, all other requests get 403, no file or content access, mixed content blocked, CSP in the HTML, and the bridge only calls `post{}` callbacks. Weaknesses: the markdown and terminal views share one bridge class; the whole terminal window string goes through Compose `update` on every chunk. | **REFACTOR**: split `MarkdownView` (one per transcript, or native) from `TerminalView`, and feed the terminal from a chunk flow. |
| `ui/web/TerminalDeltaTracker.kt` | 27 | Absolute UTF-16 offsets, reset on gap. | **KEEP** (3 tests). |
| `MainActivity.kt` | 47 | Thin; handles the HOME intent and restores navigation. | **KEEP/REFACTOR**. |
| `platform/DeviceCapabilities.kt` | 183 | Uses real capability checks and verifies after acting (grayscale reads the value back; lock checks `isInteractive`), which follows AGENTS.md. `RootCommands` duplicates the `su` runner in LocalProxyManager, and the two root-state caches can diverge. Grayscale restore state uses non-fsynced `writeText`+`renameTo`. | **REFACTOR**: one `RootShell` service shared by proxy, grayscale and lock. |
| `platform/WorkflowOverlayService.kt` | 378 | Service, permission-watch, keyguard and dock mechanics work. The UI is imperative Views with hard-coded RGB colours, Unicode glyph "icons" and RTL-hardcoded gravity (4 lint warnings). It constructs a **second `WorkspaceRepository`** on the same root: `@Synchronized` is per-instance, so the two instances do not serialize against each other. | **REFACTOR**: keep the service mechanics, rebuild the panel UI (Compose in overlay or themed views), and read state from the shared store. |

### 1.3 `runtime/` Kotlin (`app/.../runtime`) and native code

| File | Lines | Findings | Verdict |
| --- | --- | --- | --- |
| `runtime/WorkflowRuntimeClient.kt` | 104 | Client for the loopback HTTP daemon (Termux era). Its container, proxy and capabilities methods are never called. | **DELETE**. |
| `local/NativePty.kt` | 18 | JNI surface with byte-array UTF-8 arguments. | **KEEP**. |
| `local/StreamingUtf8Decoder.kt` | 46 | One decoder per stream. Handles ICU's internal-prefix behaviour. EOF is explicit. | **KEEP** (6 tests). Move to pure `:core`. |
| `local/TerminalOutputWindow.kt` | 16 | Bounded window with a surrogate-safe trim and `addExact` offsets. O(window) per chunk, which is acceptable. | **KEEP** (8 tests). Move to `:core`. |
| `local/LocalTerminalSession.kt` | 139 | Correct reader/waiter/drain/release ordering. Writes are ordered through a Mutex on the caller dispatcher. Exposes a 1 M-char `StateFlow<String>`, which gets copied per chunk. | **KEEP**; put it behind a `TerminalBackend` interface and add a chunk `SharedFlow`. |
| `local/LocalRuntime.kt` | 119 | Workspace-confined cwd; env sanitisation (`LD_PRELOAD` removed, mkshrc PS1 workaround); caps of 8 active and 8 exited terminals. It computes the workspace path itself (one of 6 copies). The backend is always `android-system-shell`. | **REFACTOR**: pluggable backend (host shell, opt-in proot, future engine) with the workspace root injected. |
| `local/LocalRuntimeService.kt` | 153 | The FGS lease model is right: owners retain and release, and a failed start is observable. | **KEEP**. |
| `local/LocalCodexSession.kt` | 251 | Real stdio app-server. Removes API-key env vars. Bounded event channel (backpressure, nothing dropped). Generation counter. JSONL framing with an 8 MiB cap. It is Codex-specific and tied to `org.json`. If the collector is cancelled (a transport switch), the channel fills and Codex stalls. `stderrTail` is kept but never shown. | **REFACTOR** into a generic `StdioJsonlProcess` in `:agent`, with the Android launcher in platform. Reuse it for Claude Code. |
| `local/CodexNetworkBridge.kt` | 216 | Authenticated loopback CONNECT-only proxy gives musl Codex Android DNS. System CA export. Constant-time auth compare. Bounded headers. Half-close preserved. A good piece of engineering. | **KEEP** (platform); unit-test the header parser on the JVM. |
| `local/LocalProxyConfig.kt` | 149 | SnakeYAML `SafeConstructor` with limits; controller must be loopback; secret is ASCII-only; stat-parser; guardian record validation; bounded reader; chunk-safe secret redaction. | **KEEP** (13 tests). Move to `:proxy`. |
| `local/LocalProxyManager.kt` | 546 | Careful ownership semantics: it never signals a persisted PID, identity is verified via the guardian, stop is only confirmed by the guardian, and fixtures are isolated. But one class does five jobs (process lifecycle, REST client, JSONL guardian protocol, log files, UI state). It uses `org.json`, which blocks JVM tests. Mihomo runs as root with `-d .workflow/proxy`, so it will create **root-owned** files (cache.db, geodata) inside app storage. `.process.json`/`runtime.log` show up in the user file tree. | **REFACTOR**: split into `proxy.controller` (REST, pure), `proxy.guardian` (protocol, pure), `platform.RootProcess` (Android) and a state machine. Give Mihomo its own `-d` home under `.workflow/proxy/run/`, hidden from the tree. |
| `cpp/pty_process.c` + `.h` | 366 | posix_openpt, IUTF8, CLOEXEC error pipe, async-signal-safe child, `waitid(WNOWAIT)` before signalling (no PID reuse), foreground-pgrp SIGHUP/SIGKILL escalation, dup-per-op fd safety. Host test covers 7 scenarios. High quality. | **KEEP**. |
| `cpp/pty_jni.cpp` | 146 | Handle table using `shared_ptr` (release is safe during concurrent calls). A minor JNI misuse: after `fail()` on a null array it keeps calling JNI with a pending exception (`strings()` → `GetArrayLength`; `write()` with null data). Kotlin never passes null, but CheckJNI would abort. | **KEEP**; fix the null and pending-exception paths. |
| `cpp/CMakeLists.txt` | 8 | `-Werror`, hidden visibility, 16 KiB pages. | **KEEP**; add a host-test target (§4). |
| `cpp/proxy_guard/proxy_guard.c` | 399 | Root guardian: child unreaped during signalling, `/proc` start-time and exe identity, fixed-string errors (no secrets echoed), stdin EOF means stop, `close_range` fallback, the host-test define fails to compile on Android. 13 host tests plus a parser test. High quality. | **KEEP**. |
| `cpp/proxy_guard/build.sh` | 27 | Hard-codes the `linux-x86_64` NDK host tag and duplicates the NDK version string with `build.gradle.kts`. | **REFACTOR** (detect host tag; one version source). |

### 1.4 `runtime/` (non-Kotlin), `tools/`, `web/`

| Path | Size | Purpose | Verdict |
| --- | --- | --- | --- |
| `runtime/workflow_daemon/{server,processes,events,workspace,proxy,pty_child,codex_guest,__main__,__init__}.py`, `pyproject.toml` | ~660 lines | Loopback HTTP + pairing-token daemon for Termux: files, PTY, Codex bridge, event cursor, proxy. The user rejected this architecture. Nothing in the default product path needs it. Its container.json validation disagrees with Kotlin (numeric versions and `user == work` here, versus any `[A-Za-z0-9._-]+` and any user in Kotlin). | **DELETE** outright. History is in git and the baseline repo. |
| `runtime/workflow_daemon/engine.py` | 132 | proot reconcile for a Termux host. | **DELETE**; the design notes live in `docs/native-engine-design.md`. |
| `runtime/workflow_daemon/image.py` + `runtime/tests/test_image.py` | 120 + 70 | Reference installer for the private `image.tar.zst` format: safe member paths, deferred links, uid/gid/mode into xattr. It is the only executable spec of the image format. | **REFACTOR**: move to `image/` next to the Dockerfile and packer as host tooling and tests, until the Android engine implements install. |
| `runtime/tests/test_runtime.py` | 208 | 9 of the 12 Python tests; all daemon-only. | **DELETE** with the daemon. |
| `runtime/install-termux.sh`, `runtime/proxy/install-proxy-termux.sh` | — | Termux installers. | **DELETE**. |
| `runtime/proxy/config.example.yaml`, `controller.example.json` | — | Examples. | **KEEP** the YAML as the single source for the in-app template, which is duplicated inline in `LocalProxyManager.createTemplate`. **DELETE** the JSON. |
| `runtime/schemas/` | 9.1 MB | Codex 0.157.0 JSON schema, TypeScript, `inventory.json` (167 client requests, 11 server requests, 83 notifications). The app consumes none of it at build time. | **KEEP**; move under `:agent` (`protocol/codex/`). Generate Kotlin method constants and the server-request registry from it. `typescript/` can be dropped because nothing consumes it. |
| `runtime/reference/` | 276 KB | Apache-licensed Rust source snapshots plus JSON "result" evidence files. | **KEEP** sources and license. Move the `*-result.json` evidence to `artifacts/`. |
| `runtime/vendor/` | 376 MB | Codex tarball plus extracted binary, Mihomo gz ×2 plus source, manifests, licenses. Not git-ignored. | **REFACTOR**: keep only `manifest.json` and `LICENSE` (in `third_party/`). The binaries come from a Gradle `fetchPrebuilts` task that checks SHA-256 and caches under `artifacts/prebuilt/`. |
| `runtime/image/Dockerfile` | 24 | Debian 13 image definition. | **KEEP** (in `image/`). |
| `runtime/android/tests/*` | ~530 | Host tests for PTY and guardian (C plus Python harness). | **KEEP**; move to `native/*/tests`, wired into CTest. |
| `app/src/main/jniLibs/arm64-v8a/lib{codex,mihomo}.so` | 311 MB | Prebuilt executables. | **REFACTOR**: generated jniLibs source dir from `fetchPrebuilts`. It follows the same pattern already used for `buildProxyGuard`. |
| `app/src/main/assets/runtime/` | 17 KB | Exported Termux runtime bundle. | **DELETE** (with `BundledRuntimeTest`). |
| `tools/update-protocol.py` | — | Regenerates schema and inventory from the host `codex`. | **KEEP**; extend it to emit Kotlin constants. |
| `tools/smoke-codex.py` | — | Imports `workflow_daemon.server.Runtime`. | **REWRITE** as a standalone stdio smoke (about 40 lines). |
| `tools/package-runtime.sh`, `package-proxy.sh`, `run-mihomo.sh` | — | Termux packaging and a host root launcher. | **DELETE**. |
| `tools/build-image.sh`, `pack-image.py` | — | Image build. | **KEEP** (into `image/`). |
| `tools/vendor-editor-grammars.py` | — | TextMate grammars and licenses into assets. | **KEEP**. |
| `tools/engine-probes/` | — | ptrace and xattr host probes. | **KEEP** (into `native/engine/probes`). |
| `web/package.json`, lock, `vendor.mjs`, `*.test.mjs` | — | Pinned npm deps; vendors to `assets/web/vendor` with a SHA manifest and esbuild to chrome66. The tests import the asset JS directly. | **KEEP**. |
| `app/src/main/assets/web/{markdown,terminal}.{html,js,css}` | ~9 KB | Hand-written renderers (the source of truth lives in assets). | **REFACTOR**: move sources to `web/src/`, and have `npm run build` copy them into assets. |
| `app/src/main/assets/web/android-input.js` | 5 KB | Monkey-patches xterm 6.0.0's private `CompositionHelper`. It is guarded by function fingerprints and fails closed. | **KEEP**, but note that it **pins xterm**: any upgrade needs re-verification on Android 9. |
| `app/src/main/assets/textmate/*` | 856 KB | Six grammars, two themes, licenses. | **KEEP**. |

### 1.5 Tests

| Suite | Count | Verdict |
| --- | --- | --- |
| `core/*Test` | 43 | **KEEP**; move to `:core`. They must stay green through the refactor. |
| `LocalProxyConfigTest` | 13 | **KEEP** in `:proxy`. |
| `StreamingUtf8DecoderTest`, `TerminalOutputWindowTest`, `TerminalDeltaTrackerTest` | 17 | **KEEP** in `:core`. |
| `ExampleUnitTest`, `ExampleInstrumentedTest` | 2 | **DELETE** (template). |
| `BundledRuntimeTest` | 1 | **DELETE** (Termux bundle). |
| androidTest `LocalRuntimeInstrumentedTest`, `CodexNetworkBridge*`, `CodexNetworkTls*`, `LocalProxy*`, `LocalCodexExecutable*`, `LocalCodexTransport*`, `OfflineRendererTest` | 16 | **KEEP** (device-only coverage of the real PTY, bridge, guardian and WebView). |
| androidTest `RuntimeConversationTest` | 2 | **REWRITE** as JVM replay tests in `:agent`. The two properties it tests (per-thread attribution; unknown server request keeps its numeric ID and is not approved) must stay. |
| Compose UI tests | 0 | Missing. Add them for Paradigm, Stack, DnD and the archive dialog after the rewrite. |

## 2. Build health (run 2026-09-26, this host)

| Command | Result |
| --- | --- |
| `./gradlew :app:assembleDebug :app:testDebugUnitTest :app:lintDebug` (one invocation, 1 m 40 s) | **BUILD SUCCESSFUL**. 74 JVM tests, 0 failures (73 real plus 1 template). Lint: 0 errors, 35 warnings. No Kotlin `w:` warnings. |
| `PYTHONPATH=runtime python3 -m unittest discover -s runtime/tests -v` | **OK, 12 tests** (9 of them are daemon-only). |
| `cd web && npm test` (node_modules present) | **15/15 pass**. |
| Native host tests (built manually into `artifacts/audit-code/native/`, same flags as `runtime/android/tests/*.sh`) | PTY: **7/7 PASS**. Stat parser: **1/1**. Guardian: **13/13 OK**. |

Warnings worth fixing:

- **Lint:**
  - `UseTomlInstead` ×5: webkit, sora BOM, editor, textmate and icons-extended are hard-coded in `app/build.gradle.kts`. Move them to `libs.versions.toml`.
  - `UnusedResources` ×8: template `colors.xml`.
  - `RtlHardcoded` ×4, `SetTextI18n` ×3, `ClickableViewAccessibility`: overlay service.
  - `MissingOnRenderProcessGone` ×2: flagged at `ResourceWebViews.kt:77` although it is overridden at :97. Re-check after the split.
  - `StaticFieldLeak`: the `LocalProxyManager` singleton. It holds an application context, but it should become an injected, process-scoped object.
  - `UseKtx` ×7, `AutoboxingStateCreation` ×2, `EmptySuperCall`, `RedundantLabel`.
- **Gradle:** one deprecation (`Configuration.setVisible`). It comes from `com.android.internal.application` (AGP internal), so it is not ours to fix.
- **Release build:**
  - `optimization { enable = false }`, so the APK has no R8. Debug `classes.dex` is 42 MB, largely `material-icons-extended`.
  - There is no signing config.
  - The APK is 208 MB (388 MB uncompressed; 311 MB of that is Codex and Mihomo).
- **ABIs:** `abiFilters` includes `x86_64`, but Codex and Mihomo are packaged only for arm64. An x86_64 or emulator build therefore silently lacks the agent and the proxy, even though `runtime/vendor` has a Mihomo amd64 build. Either drop x86_64 or fetch both ABIs.
- **Reproducibility:**
  - CMake 4.3.0 comes from `cmake.dir=/usr`; the SDK only offers up to 4.1.2. Pin the CMake version to one the SDK offers, or document the host requirement.
  - The NDK version string is duplicated in three places: `ndkVersion`, `toolchainVersion` and `build.sh`.
  - Native host tests are not part of any documented command. Add them to README as `native/run-host-tests.sh`, or to a CTest target.

## 3. Structural problems

### 3.1 Main-thread IO, per-keystroke disk churn, and stale parallel snapshots

- **Main thread:**
  - `WorkflowController` construction runs `load()` and `autoArchive()`, which does writes.
  - Every `perform{}` runs on the main thread: ensureSession, newSession, activate, updateLayout (on each tab click and each paradigm switch), openFile, newConversation (writes a file), editConfig, registerApp, unregisterApp, renameSession and createFile.
  - `applyState(repository.load())` runs after edits, saves and archive.
  - `fileList()` runs inside `remember` in composition.
  - `RuntimeUi` construction reads `local-terminals.json`.
  - `conversationFor()` calls `repository.readFile()`, which is a full `load()`.
  - Launcher calls `queryIntentActivities`/`loadLabel`.
  - `WorkspaceCorruptException` from `load()` in the controller constructor is not caught. An invalid user edit to `container.json` or `config.json` that has no `.bak` therefore crash-loops the app.
- **Every keystroke** follows this path:
  1. `CodeEditor.text.toString()`, which is O(document).
  2. A Compose `state` copy.
  3. `repository.editFile`, which does:
     1. a full `load()` (parses the manifest, **every** draft file and **every** composer file);
     2. a SHA-256 of the source;
     3. a new content-addressed draft file with fsync;
     4. a manifest write with a backup copy (two fsyncs);
     5. a prune pass.
  4. `load()` again on the main thread.
- **While Codex streams**, `updateConversation` does a full `load()`, rewrites the entire transcript and copies a `.bak` every 400 ms. Each delta also does `frames + frame.toString()` and list copies on the main thread.
- **Parallel snapshots:**
  - Controller: `state`, `pendingEdits`, `composers`, `composerPersistedRevisions`, `acknowledgedSaves`.
  - RuntimeUi: `messages`/`busy`/`threadId` mirror `Conversation`; `terminalOutput`/`terminalBuffers`/`terminalOffsets` mirror `LocalTerminalSession.state`; `codexTransport` mirrors config.
  - SoraEditor: `editingBaseDigest`, `lastSourceDigest`, `previousBuffer`.
  - Overlay: a second repository instance.
  - The `SettingsScreens` capability snapshot.

  This is exactly the "parallel stale UI snapshot" pattern AGENTS.md forbids.

**Fix (target):** one process-scoped `WorkspaceStore`:

- It is created in `Application`, loaded once on a dedicated single-thread IO dispatcher, and exposes `StateFlow<WorkspaceState>`.
- Mutations are `suspend` commands run by one writer. Only draft text is debounced (about 300 ms), with one draft file per resource and atomic replace.
- Save and archive remain transactional and still re-check source digests.
- ViewModels only map flows to UI state.
- The current on-disk format, recovery and tests carry over.
- Declarative files (`config.json`, `container.json`) get "last-good applied" semantics: an invalid edit is reported and the previously applied version stays in effect. Today, `readRecovering` silently reverts the user's file from `.bak`.

### 3.2 Layout model cannot express the product

`Layout(paradigm, panels[], focusedPanelId, sidePanelOpen, splitFraction)` with `PanelPosition.{MAIN,SIDE,BOTTOM}` has no **Stack**, no splits, no per-region focus and no Solo paradigm. `docs/product.md` §5 requires Stack tab rows as the only toolbar, splits, panel moves between stacks, and multiple terminals as tabs. Recommended model, pure and in `:core`:

```
Session.layout: LayoutState(paradigm, regions: Map<RegionRole, RegionNode>, focusedPanelId)
RegionNode = Split(orientation, fraction, a, b) | Stack(id, panelIds, activePanelId)
Panel(id, kind, resourceRef | featureRef, viewState)
```

`withParadigm` swaps region roles and keeps each paradigm's last arrangement, so switching back restores it. All DnD outcomes are pure `LayoutOp` transactions (`Reorder`, `MoveToStack`, `SplitEdge`, `Close`), each with a JVM unit test. Bump the state schema to v2 and migrate v1 (main panels → editor stack; chat → chat region; terminal → bottom stack).

### 3.3 Agent layer

- There is **no Claude Code integration**, although the brief and `docs/product.md` require one with "完整" (complete) coverage.
- Codex handling is ad hoc:
  - 11 notifications are handled.
  - Server requests are a flat `List<JSONObject>`.
  - The transcript is a flat `ChatEntry(role, text, kind)`. It loses structured items: diffs, command output streams, reasoning summaries, plans, MCP calls.
  - There is no `thread/read` hydration.
- Agent state is ViewModel-scoped, so background conversations lose events when the ViewModel is recreated.

Recommendation, a pure-JVM `:agent` module:

- **`agent.api`**: `AgentBackend` (start, stop, capabilities, `newThread`, `resume`, `send`, `interrupt`, `respond(ServerRequest, Decision)`) plus a backend-neutral event model: Thread, Turn, Item {Message, Reasoning, Command, FileChange, ToolCall, Plan, Unknown(raw)}, ServerRequest {kind, rawId, rawParams}.
- **`agent.transport`**: `StdioJsonlProcess` (framing, size caps, generation, backpressure) and `JsonRpcPeer` (our request IDs; server request IDs preserved as raw `JsonElement`; unknown methods passed through to an `Unknown` event, never dropped).
- **`agent.codex`**: an App Server adapter driven by generated method constants from `runtime/schemas/inventory.json`, plus a coverage table checked in CI (schema method → handled / UI / raw-only).
- **`agent.claude`**: a Claude Code adapter over its stream-json / SDK control protocol (tool-permission requests map to `ServerRequest`). Which binary and runtime it needs (Node or native, host or guest) must be verified by the runtime agent. The interface must not assume Codex.
- **Persistence**: a `ConversationStore` in `:core` holds items keyed by resource ID, append-only with snapshot compaction. It replaces whole-transcript rewrites.
- **Ownership**: a process-scoped `AgentHub` in `:app` owns backends and routes events by thread → resource, including threads that are not visible.

### 3.4 Workspace path and confinement duplication

- `File(filesDir, ".workspace")` is computed in 6 places: WorkflowController, LocalRuntime, LocalCodexSession, DeviceCapabilities, the overlay (twice), and indirectly LocalProxyManager.
- Path confinement is implemented 5 times with different rules:
  - `WorkspaceRepository.userFile` reserves `.state` and `.workflow/*` except proxy.
  - `ChatPanel.addPath` blocks only `.state`.
  - `RuntimeUi.send` uses string `startsWith`.
  - `RuntimeUi.openTerminalFile`.
  - `LocalRuntime.createTerminal`.
- Fix: a single `WorkspacePaths` / `WorkspaceRoot` value in `:core` (`resolveUser`, `resolveState`, `isReserved`), injected everywhere.

### 3.5 Other duplication

- **Atomic write**, 6 variants. Only the repository one fsyncs and keeps a backup. `LocalProxyManager` uses `renameTo` without directory fsync; the terminal-mapping, grayscale and overlay-position writes use `writeText`/`renameTo`. Put one `AtomicFiles` in `:core`.
- **`su` runner and root probe**: `RootCommands.run` and `LocalProxyManager.command(su -c id -u)`, with two root-state caches.
- **SHA-256 hex helper**: 2 copies.
- **Bounded-output process runner**: 2 copies.
- **Codex `initialize` payload**: 3 copies (local, http, smoke script). It hard-codes version `0.1.0`.
- **Ctrl-key mapping**: in both Kotlin `TerminalContent` and xterm.
- **container.json validation**: Kotlin and Python disagree.
- **Mihomo template**: an inline Kotlin string and `runtime/proxy/config.example.yaml`.

## 4. Proposed module layout

Keep it pragmatic: 4 Gradle modules plus 3 non-Gradle source roots. The pure modules are there to enforce "Android-free", give fast parallel tests (the brief asks for independent MVP modules) and allow codegen. They are not there for ceremony.

```
settings.gradle.kts: include(":core", ":agent", ":proxy", ":app")

:core   (kotlin("jvm"), coroutines only)
        top.flysoftbeta.workflow.core.{workspace, session, layout, draft, composer,
          conversation, config, json, io, terminal}
:agent  (kotlin("jvm"), coroutines + kotlinx-serialization-json) → :core
        …agent.{api, transport, codex, claude, coverage}; resources: protocol/codex/{schema,inventory.json}
:proxy  (kotlin("jvm"), snakeyaml + okhttp) → :core
        …proxy.{config, controller, guardian, redact}
:app    (com.android.application) → :core, :agent, :proxy
        …app (Application, AppGraph manual DI, MainActivity, navigation)
        …feature.{launcher, sessions, workbench, files, editor, terminal, chat, proxy, settings, overlay}
        …platform.{pty, process, root, network, service, capabilities, filewatch}
        …ui.{design, web}
native/                     (CMake; AGP externalNativeBuild points here)
  pty/          pty_process.{c,h}, pty_jni.cpp, CMakeLists (android .so + host test), tests/
  proxy-guard/  proxy_guard.c, build (per-ABI PIE task), tests/ (fake kernel, harness)
  engine/       README, probes/ (from tools/engine-probes); future loader/ptrace
web/            src/ (renderer html/js/css), test/, vendor.mjs → app/src/main/assets/web
image/          Dockerfile, build-image.sh, pack-image.py, reference installer + tests
third_party/    codex/, mihomo/ (manifest.json + LICENSE only); Gradle fetchPrebuilts → build/
tools/          update-protocol.py (+ Kotlin codegen), vendor-editor-grammars.py, smoke-codex.py
```

Dependency rules, enforced by the module graph and one small Gradle check:

1. `:core` depends on nothing internal and on no Android, `org.json`, OkHttp or WebView types.
2. `:agent` and `:proxy` may depend on `:core`, never on each other, and never on Android. Android behaviour enters only through ports:
   - `ProcessLauncher` (agent)
   - `GuardianLauncher` and `RootShell` (proxy)
   - `TerminalBackend` (core)
3. `:app/platform` implements those ports (JNI PTY, stdio process with NetworkBridge env, su guardian, FGS leases).
4. `:app/feature.*` packages depend on `core`/`agent`/`proxy` APIs, `ui.design` and `app.navigation`, and do not depend on each other. Cross-feature actions go through store commands or navigation contracts (for example "attach file to conversation" or "open path in editor").
5. Long-lived owners (`WorkspaceStore`, `LocalRuntime`, `AgentHub`, `ProxyService`) are process-scoped in `AppGraph`. ViewModels hold no durable state.
6. Protocol frames use `kotlinx.serialization.json.JsonElement`; unknown fields survive. Disk and declarative files keep the strict `core.json` codec, because kotlinx does not reject duplicate keys.

Where existing files move:

| New home | Existing files |
| --- | --- |
| `:core` | `core/*.kt`; `runtime/local/StreamingUtf8Decoder.kt`, `TerminalOutputWindow.kt`; `ui/web/TerminalDeltaTracker.kt`; all corresponding tests (60 tests) |
| `:proxy` | `LocalProxyConfig.kt`; REST, guardian-event and template parts of `LocalProxyManager.kt`; `LocalProxyConfigTest` |
| `:agent` | Framing from `LocalCodexSession.readFrames`/`send`; RPC/pending/`processFrame` logic from `RuntimeUi` (rewritten); `runtime/schemas`; logic of `RuntimeConversationTest` as JVM replay fixtures |
| `:app/platform` | `NativePty`, `LocalTerminalSession`, `LocalRuntime`, `LocalRuntimeService`, `CodexNetworkBridge`, process half of `LocalCodexSession`/`LocalProxyManager`, `DeviceCapabilities`, `WorkflowOverlayService` (service part), `FileChanges` |
| `:app/feature`, `ui` | Rewritten screens; `ResourceWebViews` split; `SoraEditor` refactored; `FileActions`, `FilePreview`, `WorkspaceDragShadow`, theme |
| `native/` | `app/src/main/cpp/**`, `runtime/android/tests/**` |
| delete | See §1.3 and §1.4. Also update AGENTS.md, README and `docs/runtime.md`, which still describe HTTP/Termux as optional compatibility |

## 5. Dead code, unsafe patterns, contradictions

### 5.1 Dead or unused code

- **Config fields:**
  - `WorkspaceConfig.codexEndpoint`, `proxyMode`, `activationComplete` and `theme` are read or written only by the codec.
  - `daemonUrl`/`daemonToken`/`codexTransport=http` are legacy.
- **Models:** `Panel.scrollOffset` is never written. `PanelKind.APPS` is unused.
- **Repository and runtime:**
  - `WorkspaceRepository.readPath`, `discardWorkingResource`, `updateContainer` and `createDirectory` are never called from main code; there is no "new folder" UI.
  - `ContainerConfig` is loaded but nothing consumes it.
  - `LocalCodexState.stderrTail` is collected but never shown.
  - `LocalRuntime.backend` duplicates `LocalTerminalSession.backend`.
- **HTTP path:** `WorkflowRuntimeClient` (whole file); `RuntimeUi.startPolling`/`resetDaemon`/`client()`/HTTP attachment upload; `WorkflowController.daemonConnected`/`daemonStatus`/`health`; `ChatPanel` `health.workspace` path stripping.
- **Termux in Settings and manifest:** the Termux guide, export and permission in `SettingsScreens`; `com.termux.permission.RUN_COMMAND` and `<package com.termux/>` in the manifest; `assets/runtime/*`; `BundledRuntimeTest`.
- **Templates:** `ExampleUnitTest`, `ExampleInstrumentedTest`, `res/values/colors.xml`, the template `backup_rules.xml`/`keepRules`.
- **Python and tools:** the daemon, Termux installers, `tools/package-*.sh`, `run-mihomo.sh`; `runtime/schemas/typescript` (not consumed).

### 5.2 Unsafe or fragile patterns

- **Auto-approval: none found.** Approvals are sent only from explicit button taps (`ServerRequestDialog`). Keep this invariant in the new agent layer, with a test.
- **The agent can write app state.** Codex runs with `cwd = .workspace` and `sandbox: workspace-write`, so `.workspace/.state` (session manifest, drafts, `codex/auth.json`) is inside the agent's writable root. On Android 9 (kernel 4.14, no Landlock) the Codex sandbox probably offers no confinement at all; this is unverified. An agent command can corrupt the workspace state. The planner must decide between two options:
  - Put agent cwd and writable roots at a user subtree and keep app state out of it.
  - Accept the risk explicitly.

  This decision interacts with the rule "all durable data under `.workspace`".
- **Secrets are attachable to chat.** `config.json` (holds `daemonToken`) and `.workflow/proxy/config.yaml` (holds the Mihomo `secret` and subscription URLs) are ordinary user files and can be attached to a model conversation. Deleting the daemon removes the first. For the proxy config, warn or redact before attaching.
- **Device transfer (verify).** `allowBackup=false`, but `data_extraction_rules.xml` is the untouched template. For targetSdk ≥ 31, device-to-device transfer is governed by `dataExtractionRules`, so Codex auth and proxy secrets may migrate. Add explicit `<device-transfer><exclude …/>` rules for `.workspace/.state` and `.workflow/proxy`.
- **FileProvider** exposes `files-path .workspace/` (the whole tree, including `.state`). Only app-created URIs are granted, but narrow it to user paths.
- **Root-owned files.** Mihomo runs as root with its home inside app storage, so it will leave root-owned `cache.db`/geodata files the app cannot replace. `.process.json` and `runtime.log` are visible and editable in the file tree.
- **Blocked Codex.** `LocalCodexSession` events use a single bounded channel. If the collector is cancelled (transport switch, or ViewModel cleared before re-collection), Codex blocks on stdout.
- **JNI** pending-exception misuse on null inputs (§1.3).
- **Error-string coupling** (`"File already exists"`) and Chinese UI strings hard-coded in Kotlin. There is no resources or i18n layer; the lint `SetTextI18n` warnings are a symptom.
- **Logging:** only `Log.w("WorkflowWeb", …)` of renderer console errors (truncated to 500 chars). No tokens or messages are logged. That is fine.

### 5.3 Doc and code contradictions

| Doc claim | Code reality |
| --- | --- |
| AGENTS.md, README, `runtime.md`: Termux/HTTP is "optional compatibility". User: the architecture is rejected. | It is still in Settings, the manifest, assets, `RuntimeUi` and `WorkflowRuntimeClient`. The docs must be updated when it is deleted. |
| `architecture.md`: "`Stack` 管理同组 panels…" (Stack manages a group's panels) | No Stack type exists. |
| `architecture.md` and brief: one toolbar layer; no NavigationBar | Uses NavigationBar/NavigationRail. Workbench has a header, a tab row and a status row. |
| `architecture.md`: container.json "构建成功后才激活" (activated only after a successful build) | Nothing builds or applies it on Android. Invalid edits are silently reverted from `.bak`, or crash startup when no backup exists. |
| `runtime.md`: "The Kotlin adapter is `WorkflowRuntimeClient`" | The default path is `LocalCodexSession` plus `RuntimeUi`. The HTTP client is legacy. |
| `runtime.md`: approvals, permissions, elicitation, dynamic tools, auth refresh, attestation and clock are "client-owned" | True that nothing is auto-granted. But 8 of 11 server request kinds need hand-typed JSON, so `currentTime/read`, `item/tool/call` and `attestation/generate` effectively stall a turn. |
| `implementation-status.md`: "68 项 JVM 测试" (68 JVM tests), "核心 19" (19 core tests) | 74 JVM tests (73 plus the template); 43 core tests. |
| Brief and `product.md`: Codex **and Claude Code**, "完整" (complete) | No Claude Code code at all. |
| `dependencies.md`: Codex 0.157.0 "官方当前稳定版" (official current stable) | 0.157.1 was released 2026-09-26. |
| README: the build needs no special host setup beyond the SDK | It requires host CMake 4.3.0 (`cmake.dir=/usr`) and the `linux-x86_64` NDK host tag in `build.sh`. |

## 6. Dependency freshness (checked 2026-09-26 against Google Maven, Maven Central, the Gradle plugin portal, npm, GitHub releases and the SDK repository XML)

| Component | Current | Latest stable (newest pre-release) | Upgrade risk / note |
| --- | --- | --- | --- |
| Gradle wrapper | 9.8.0 | 9.8.0 (released 09-24) | Current |
| AGP | 9.4.1 | 9.4.1 (9.5.0-alpha07) | Current |
| Kotlin (compose plugin; AGP built-in Kotlin) | 2.4.20 | 2.4.20 (2.5.0-Beta1) | Current. Pure modules will need `kotlin("jvm")` 2.4.20 plus the serialization plugin 2.4.20 |
| foojay resolver | 1.0.0 | 1.0.0 | Current |
| Compose BOM | 2026.09.00 | 2026.09.00 | Current (ui/foundation 1.12.1, material3 1.4.0) |
| material3 | 1.5.0-alpha29 (overrides BOM) | stable 1.4.0; alpha29 is newest | Required for Expressive. Alpha API churn is the main risk; keep opt-ins localised |
| material-icons-extended | 1.7.8 (via BOM) | 1.7.8, frozen upstream | Large and unmaintained. Replace with a vendored subset of Material Symbols vectors; this also shrinks dex |
| androidx.core:core-ktx | 1.19.1 | 1.19.1 | Current |
| activity-compose | 1.13.0 | 1.13.0 (1.14.0-alpha03) | Current |
| lifecycle-runtime-ktx | 2.11.0 | 2.11.0 (2.12.0-alpha04) | Current. Add `lifecycle-viewmodel-compose`/`runtime-compose` 2.11.0 for `collectAsStateWithLifecycle` |
| androidx.webkit | 1.17.1 (hard-coded) | 1.17.1 (1.18.0-alpha02) | Current; move to TOML |
| Sora editor BOM / editor / language-textmate | 0.24.6 (hard-coded) | 0.24.6 (GitHub marks it pre-release) | Current; move to TOML |
| OkHttp | 5.5.0 | 5.5.0 | Current |
| SnakeYAML | 2.7 | 2.7 (alternative: snakeyaml-engine 3.1.1, YAML 1.2) | Current |
| kotlinx-coroutines | transitive (not declared) | 1.11.0 | Declare explicitly in the pure modules |
| kotlinx-serialization-json | not used | 1.11.0 (1.12.0-RC) | Proposed for `:agent`/`:proxy` frames |
| junit | 4.13.2 | 4.13.2 | Current (JUnit 4 line) |
| androidx.test ext-junit / espresso / runner | 1.3.0 / 3.7.0 / (implicit) | 1.3.0 / 3.7.0 / 1.7.0 | Current; declare the runner |
| NDK | 30.0.15729638 | **30.0.16248370** (channel-0) | Low risk. Update `ndkVersion`, `toolchainVersion` and `build.sh` together |
| CMake | 4.3.0 (host `/usr`) | SDK offers 4.1.2; host 4.3.0 | Reproducibility, not freshness |
| compileSdk / targetSdk | 37 | platform 37.2 available; build-tools 37.0.0 | Optional minor-SDK bump |
| @xterm/xterm / addon-fit / addon-web-links | 6.0.0 / 0.11.0 / 0.12.0 | same (6.1.0-beta.304) | Current. **Upgrades are gated by `android-input.js` fingerprints**; re-run the Android 9 IME test |
| marked / katex / dompurify | 18.0.14 / 0.18.9 / 3.4.16 | same (released 09-22 and 09-23) | Current |
| core-js-bundle / esbuild / jsdom | 3.50.0 / 0.28.2 / 30.1.1 | same | Current |
| Codex CLI (bundled) | 0.157.0 | **0.157.1** (2026-09-26; 0.158/0.159 are alpha) | Low to medium. Regenerate the schema and inventory, and re-run device transport tests |
| Mihomo (bundled) | 1.19.31 | 1.19.31 | Current. The binary links only unversioned bionic LIBC symbols, so API 28 is OK |

## 7. Recommended order (for the planner)

1. **Module skeleton.** Create the `:core`/`:agent`/`:proxy` skeleton and move the pure files with their tests unchanged. There is no behaviour change; the gate is 73 tests passing in the pure modules (60 in `:core`, 13 in `:proxy`).
2. **Prebuilts and Termux removal.** Add `fetchPrebuilts`, take the binaries out of the tree and extend `.gitignore`. Delete the Termux/HTTP paths and the Python daemon. Update AGENTS.md, README and `runtime.md`.
3. **`WorkspaceStore`.** Build the single writer with async commands and flows. Add layout v2 with Stack and migration, plus pure `LayoutOp` tests. Add last-good declarative config.
4. **Agent layer.** Build `:agent` transport and the Codex adapter with replay fixtures and a coverage table. Then add the Claude Code adapter, which depends on the runtime and engine decisions. Add a process-scoped `AgentHub`.
5. **UI rewrite.** Rewrite the feature UIs over ViewModels (UI agent), reusing the kept WebView, editor and DnD pieces.
6. **Native folder.** Move `native/` with CTest, fix the JNI null path, and bump the NDK.
