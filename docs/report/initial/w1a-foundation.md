# W1a foundation: module restructure (report)

Date: 2026-09-27. Role: foundation engineer, Gradle owner for W1a. Status: **done, build green**.
Evidence is in `artifacts/w1a/`: `gradle-snapshot.log`, `gradle-full.log`, `native-final.log`, `web-final.log`, `python-after.log`, lint and APK listings, and warning diffs.

**Start signal for other workstreams.** The module layout from `docs/architecture.md` §1 exists and builds. Always invoke Gradle as `flock artifacts/.gradle.lock ./gradlew …`.

**Warning about the live `:agent` tree.** W4 is writing into `agent/src/**` concurrently. At 06:15 its `agent/transport/ControlConnection.kt:84-86` did not compile (UNSAFE_CALL). While that is true, any build of the live tree that includes `:agent` or `:app` fails. The full verification was therefore run on a snapshot copy of the tree. The snapshot's `agent/src` contains only the W1a skeleton; everything else is identical (see §7). W4 owns fixing its files. W1a did not edit them.

## 1. Modules and build

| Module | Kind | Dependencies | Packages |
| --- | --- | --- | --- |
| `:core` | Kotlin JVM | `api` kotlinx-coroutines-core | `core.workspace`, `core.json`, `core.terminal` |
| `:agent` | Kotlin JVM + serialization plugin | `api` `:core`, coroutines, kotlinx-serialization-json | `agent` (W1a skeleton: `AgentFrames` + 1 test; W4 is adding more) |
| `:proxy` | Kotlin JVM | `api` `:core`, OkHttp; `implementation` SnakeYAML | `proxy.config`, `proxy.controller`, `proxy.guardian`, `proxy.io`, `proxy.redact` |
| `:app` | Android | `:core`, `:agent`, `:proxy` | unchanged `ui.*`; new `platform.{pty, process, network, root, service}` |

Build details:

- **Kotlin and JVM target.** The pure modules use `org.jetbrains.kotlin.jvm` 2.4.20. This is the same KGP that AGP's built-in Kotlin resolves. They target JVM 17 for both Java and Kotlin, matching `:app` `compileOptions`.
- **Version catalog.** `gradle/libs.versions.toml` is the only place for versions. New entries:
  - plugins: `kotlin-jvm`, `kotlin-serialization` and `android-lint` (all pinned to kotlin/agp);
  - libraries: `kotlinx-serialization-json` 1.11.0 and `kotlinx-coroutines-{core,test}`;
  - versions: `ndk` and `cmake`;
  - the formerly hard-coded webkit, sora BOM/editor/textmate and icons-extended. This clears lint `UseTomlInstead` ×5.
- **Coroutines pin.** Coroutines is pinned to **1.9.0**, the version `:app` already resolved through AndroidX (checked with dependencyInsight). Declaring 1.11.0 would silently upgrade the app. Lint now reports `NewerVersionAvailable` for it, deliberately.
- **Android-free check.** `gradle/pure-jvm-module.gradle.kts` adds `verifyPlatformIndependence` to `test` and `check` in each pure module. It fails the build if:
  - source references `android.`, `androidx.`, `dalvik.` or `org.json.`; or
  - the compile or runtime classpath contains `androidx.*`, `android.*`, `com.android.*`, `com.google.android.*` or `org.json`.

  I confirmed it fails when a probe file references `android.util.Log`. It writes `build/reports/platform-independence.txt`.
- **Lint.** `:app` sets `lint.checkDependencies = true`, and the pure modules apply `com.android.lint`, so their sources are linted inside `:app:lintDebug`.

## 2. What moved where

Nothing in the moved code was redesigned. The only changes are package lines, imports, `internal` → `public` where `:app` needs access, and two warning-only simplifications with identical semantics in `:proxy`.

| From (`app/src/...`) | To |
| --- | --- |
| `core/{WorkspaceModels,WorkspaceCodec,WorkspaceRepository,ConversationComposer}.kt` | `core/src/main/kotlin/.../core/workspace/` |
| `core/Json.kt` | `.../core/json/Json.kt` |
| `runtime/local/{StreamingUtf8Decoder,TerminalOutputWindow}.kt`, `ui/web/TerminalDeltaTracker.kt` | `.../core/terminal/` |
| `test/.../core/*Test` (43), `StreamingUtf8DecoderTest`, `TerminalOutputWindowTest`, `TerminalDeltaTrackerTest` (17) | `core/src/test/...` (same subpackages), 60 tests unchanged |
| `runtime/local/LocalProxyConfig.kt` | split into `proxy.config.LocalProxyConfig` (+`ProxyControllerSettings`), `proxy.guardian.GuardianProtocol.kt` (`ProxyProcessIdentity`, `verifyGuardianExit`, `guardianInteger`), `proxy.io.readProxyBytes`, `proxy.redact.ProxyLogRedactor` |
| Pure parts of `LocalProxyManager.kt` | `proxy.config.MihomoConfigTemplate` (template + secret), `proxy.controller.MihomoControllerClient` (OkHttp REST transport, bounded body, auth header, path encoding, port probe), `proxy.guardian.GuardianCommand` (su script quoting) |
| `LocalProxyConfigTest` (13) | `proxy/src/test/.../proxy/LocalProxyConfigTest.kt`, plus the new `ProxyExtractionTest` (4) for the extracted parts |
| `runtime/local/{NativePty,LocalTerminalSession,LocalRuntime}.kt` | `app/.../platform/pty/` (JNI symbols renamed to `Java_top_flysoftbeta_workflow_platform_pty_NativePty_*`; checked with `llvm-nm` on both ABIs) |
| `runtime/local/LocalRuntimeService.kt` | `app/.../platform/service/` (manifest updated) |
| `runtime/local/CodexNetworkBridge.kt` | `app/.../platform/network/` |
| `runtime/local/LocalCodexSession.kt` | `app/.../platform/process/` |
| `runtime/local/LocalProxyManager.kt` | `app/.../platform/root/`. It keeps the Android and process half and `org.json`, and delegates to `:proxy` |
| androidTest `runtime/local/*` | `androidTest/.../platform/{network,pty,root}/` |
| `app/src/main/cpp/{pty_process.c,.h,pty_jni.cpp,CMakeLists.txt}` | `native/pty/` |
| `app/src/main/cpp/proxy_guard/{proxy_guard.c,build.sh,README.md}` | `native/proxy-guard/` |
| `runtime/android/tests/pty_process_test.c` | `native/pty/tests/` |
| `runtime/android/tests/proxy_guard_{fake_kernel.c,parser_test.c,test.py}` | `native/proxy-guard/tests/` |

Details of the platform moves:

- `platform/DeviceCapabilities.kt` and `platform/WorkflowOverlayService.kt` stay in `platform`. Moving them would change the component name of `WorkflowDeviceAdminReceiver`, which would silently revoke an already-granted device admin.
- `ui/*` stays for the UI rewrite.
- `ExampleUnitTest` stays in `:app` so the test count does not drop.

Native build wiring:

- `externalNativeBuild` points at `native/pty/CMakeLists.txt`, and `buildProxyGuard` at `native/proxy-guard/build.sh`. The NDK and CMake versions come from the catalog.
- `native/test-host.sh [pty|proxy-guard]` replaces `runtime/android/tests/*.sh`. It builds with the system `cc` into `artifacts/native-host/`.
- `native/engine/README.md` is a placeholder and says the engine is not implemented.

## 3. Prebuilt binaries (`third_party/`)

- `third_party/{codex,mihomo}/` contain `manifest.json` and `LICENSE`. The license texts were copied byte-identical from `runtime/vendor`.
  - Each manifest records version, license, source, `packagedAs` and, per ABI: upstream `url`, archive type and member, `archiveSha256`/`archiveBytes`, and the binary `sha256`/`bytes`.
  - Codex 0.157.0: arm64-v8a.
  - Mihomo 1.19.31: arm64-v8a and x86_64. The x86_64 entry is recorded but not packaged. Mihomo also records its GPL source archive.
- **`:app:fetchPrebuilts`** is wired through `variant.sources.jniLibs.addGeneratedSourceDirectory`, so it runs before `merge*JniLibFolders`. For each manifest and ABI (currently only `arm64-v8a`, exactly as before), it:
  1. ensures `third_party/.cache/<name>/<version>/<abi>/<packagedAs>` exists with the pinned size and SHA-256;
  2. if the binary is missing, extracts it from the cached archive after checking the archive SHA-256;
  3. if the archive is missing too, downloads it over HTTPS from the pinned URL and verifies it;
  4. hard-links the binary (falling back to a copy) into `app/build/generated/prebuilt-jni/`.

  A cached file whose digest is wrong fails the build. It is never overwritten.
- **Notices.** `:app:prebuiltNotices` generates `assets/notices/<name>-{manifest.json,LICENSE}` from `third_party/`. This replaces the hand-copied `app/src/main/assets/notices/`, which was deleted. No code reads these files. The manifest JSON is now in the new format.
- **Cache seeding.** The cache was seeded by moving files, and every hash was verified before any original was deleted (`artifacts/w1a/cache.sha256`):
  - `jniLibs/arm64-v8a/lib{codex,mihomo}.so`;
  - Codex `tar.gz`;
  - Mihomo arm64/amd64 `.gz` and the source tarball.

  `runtime/vendor/codex/codex-arm64` was a byte-identical duplicate of `libcodex.so` and was deleted after `cmp`. `app/src/main/jniLibs/` and `runtime/vendor/` no longer exist.
- `/third_party/.cache/` is in `.gitignore`.
- **What was tested.** I moved both extracted binaries aside and re-ran `fetchPrebuilts --rerun`. The tar.gz and gz extraction reproduced the pinned hashes, and the output is a hard link.
- **What was not tested.** The network download path was not exercised; all archives were already cached.
- **APK check.** In the new APK, `libcodex.so`, `libmihomo.so` and `libworkflow_proxy_guard.so` are byte-identical to the baseline APK.

## 4. Removed (Termux/HTTP path)

- `runtime/WorkflowRuntimeClient.kt` and every use of it:
  - `RuntimeUi`: the HTTP poller, `resetDaemon`, `client()`, transport switch, HTTP attachment upload and the `health` path mapping.
  - `WorkflowController`: `daemonConnected`, `daemonStatus`, `health`.
  - `WorkflowApp`: the polling effect.
  - `ChatPanel`: the "兼容服务尚未连接" state and the path prefix.
  - `AccountScreen`: the "兼容服务" label and the "高级兼容设置" row.
- `PermissionsScreen`: the "扩展与兼容" section, the Termux install/permission/export rows, endpoint and token fields, and the guide sheet.
- Behaviour of old conversations: a persisted conversation that has a thread but no `source` falls back to `"http:legacy"`. Such conversations are still refused with "请新建对话", as they were before in local mode.
- Manifest: the `com.termux.permission.RUN_COMMAND` permission and the `<package com.termux/>` query.
- Assets: `app/src/main/assets/runtime/{workflow-runtime.bundle,.sha256}`.
- Tests: androidTest `BundledRuntimeTest` (1 test; it only checked the Termux bundle).
- Scripts:
  - `runtime/install-termux.sh`, `runtime/proxy/install-proxy-termux.sh`;
  - `tools/package-runtime.sh`, `tools/package-proxy.sh`;
  - `tools/run-mihomo.sh`. This was only reachable through the Termux proxy package, and it read the deleted `runtime/vendor`.
- **Kept, per brief:** `runtime/workflow_daemon`, `runtime/schemas`, `runtime/image`, `runtime/reference`, `runtime/tests`, `runtime/pyproject.toml`, `runtime/proxy/{config.example.yaml,controller.example.json}` (the daemon's `proxy.py` reads the JSON), `tools/engine-probes`, `tools/build-image.sh`, `tools/pack-image.py`, `tools/smoke-codex.py`.
- **Config fields kept.** `WorkspaceConfig.daemonUrl/daemonToken/codexEndpoint/codexTransport` still exist in `:core` because the moved tests encode them, but the app no longer reads them. Removing them belongs to the W1b config rewrite. An existing `codexTransport: "http"` still parses, and the app now always uses local Codex.

## 5. Backup and transfer

`backup_rules.xml` and `data_extraction_rules.xml` now exclude every domain: root, file, database, sharedpref, external and the `device_*` domains. They cover both `<cloud-backup>` and `<device-transfer>`. `allowBackup="false"` is unchanged. I checked both in the merged APK manifest.

## 6. README

The build/test commands and the directory section were rewritten (Chinese, concise). The obsolete "高级兼容服务" section was removed. No other docs were changed.

## 7. Verification

The snapshot is `artifacts/w1a/snapshot`: the tree at 06:14 minus W4's in-progress `agent/src` files, with the prebuilt cache symlinked. In it, one invocation of `flock … ./gradlew :core:test :agent:test :proxy:test :app:assembleDebug :app:testDebugUnitTest :app:lintDebug :app:compileDebugAndroidTestKotlin` gave **BUILD SUCCESSFUL**. The live tree gave the same result at 06:10, before W4's broken file appeared. `native/test-host.sh` and `cd web && npm test` were run on the live tree.

| Suite | Before (baseline) | After |
| --- | --- | --- |
| JVM unit tests | 74 (all in `:app`) | **79**: `:core` 60, `:proxy` 17 (13 moved + 4 new), `:agent` 1, `:app` 1 |
| Instrumented tests (compiled, not run) | 20 `@Test` | 19. Only `BundledRuntimeTest` (Termux) was removed |
| Native host | PTY 7, parser 1, guardian 13 = 21 | 21 (`native/test-host.sh`) |
| Web | 15 | 15 |
| Python (`runtime/tests`) | 12 | 12 |
| Lint | 0 errors, 33 warnings, 2 hints | 0 errors, 29 warnings, 1 hint |
| Kotlin warnings (`:app` full compile) | 30 (the audit's "none" was wrong) | 24, all a subset of the baseline set |

Lint changes:

- Removed: `UseTomlInstead` ×5; `UseKtx` ×1 and `AutoboxingStateCreation` ×1, both from deleted Termux UI code.
- Added: `NewerVersionAvailable` ×2, from the coroutines 1.9.0 pin.

APK diff against the baseline: `assets/runtime/*` is removed and kotlinx-serialization metadata is added. The native libs are identical, and the notices are present. Nothing was installed on any device.

## 8. Deferred and known gaps

- **API level of pure-module code is no longer checked.** Android lint's NewApi/API-level check does not apply to plain JVM modules. A probe calling `InputStream.readNBytes` (API 33) in `:proxy` was not flagged. Code moved out of `:app` loses its minSdk 28 API check. The moved code previously passed that check, but new code is unchecked. Fix: add an animal-sniffer check against Android API 28 signatures (gummy-bears) to `pure-jvm-module.gradle.kts`, or keep API-sensitive IO in `:app/platform`.
- Coroutines 1.9.0 → 1.11.0 and serialization 1.11.0 → 1.12 are upgrade decisions for later.
- x86_64 Codex/Mihomo are not packaged, as before. The Mihomo x86_64 hash is in the manifest and cache, ready for the emulator AVD.
- The `fetchPrebuilts` network path was not exercised.
- These items from the audit are not done:
  - `native/proxy-guard/build.sh` still hard-codes the `linux-x86_64` host tag;
  - no CTest target (host tests use `native/test-host.sh`);
  - the JNI null/pending-exception fix;
  - the NDK bump;
  - `tools/engine-probes` → `native/engine/probes`.
- Docs and README text that still mention Termux/HTTP or old paths: `docs/runtime.md`, `docs/android-local-runtime.md` (`runtime/android/tests/run-host-tests.sh`), `docs/android-ui-validation.md`, AGENTS.md. Per the brief, they were not rewritten.
