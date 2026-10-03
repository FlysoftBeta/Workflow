# Testing and evidence

Choose checks by the boundary a change affects. Pure model and protocol tests are fast feedback; they do not replace an Android app-UID test for rendering, process execution, foreground services or root capabilities. The [current status](../status.md) records accepted scope and known limits, and reports preserve the source and artifact identity for each result.

## Host checks

The supported development host is Linux, with Git, Python 3.11 or newer, JDK 25, Rust 1.93.1, the Android SDK/build-tools 37, the pinned NDK and CMake, Node.js, and zstd. The [build guide](building.md) gives exact toolchain setup, customized-image prerequisites, Debug/Release commands, and output paths. Exact dependency manifests and the Gradle wrapper are checked into the repository. Configure the Android SDK through `local.properties` or the SDK environment variables; do not commit machine-local paths. Install web test dependencies with `npm ci --ignore-scripts` in `app/web/` before its first check.

`tools/workflow check NAME` runs a named suite in the coordinator checkout. Add `--task NAME` to run in an isolated task checkout. The catalog is executable configuration in `tools/workflow-suites.json`, so commands and output locations can be reviewed together.

The `app-unit`, `lint`, and `android-apk` suites select the `x86_64` flavor for the default test target. The `arm64` and `x86_64` flavor names identify Android ABIs `arm64-v8a` and `x86_64`, independent of physical hardware or emulation. Standard `assembleDebug` and `assembleRelease` aggregate both; use the explicit ABI tasks in the build guide when only one APK is needed.

| Suite | What it establishes |
| --- | --- |
| `client`, `proxy` | `:app:client` protocol/workspace contracts and `:app:proxy` local execution. Reference-store tests explicitly use test fixtures. |
| `agent` | Retained client codec/model tests, temporary Engine-only Kotlin adapter/service tests and the guest service JAR build while Rust Chat cutover is incomplete. |
| `app-unit` | Client/proxy tests, the retained Kotlin adapter/service oracle tests, and Android formatting, rendering projections, interaction models and adapter helpers. |
| `rust-server` | Authoritative workspace, layout, imports, tools, terminal resources, local-service tickets, environment and protocol behavior in Rust. |
| `runtime-host` | The Rust container runtime against the real host syscall and filesystem oracles. |
| `native` | Remaining native PTY and proxy guardian checks. |
| `image` | Image format, manifest, schema and rejection cases. |
| `web` | Offline terminal input, links and retained rendering behavior. Chat is native Compose. |
| `infrastructure` | Real temporary Git worktrees, ownership, locks, source-bound results, integration recovery, device-result parsing and source architecture guards for the Android/Engine boundary. |
| `documentation` | Maintained English prose, balanced fences and local links; frozen historical originals are excluded. |
| `lint` | Android lint, including API-level and Compose integration checks. |
| `android-apk` | A successful app/test build plus an immutable, digested APK pair; this is preparation, not device acceptance. |

Run only the checks needed by the change, then broaden when another boundary or a failure justifies it. Every result includes its source fingerprint. Changing source during a check taints that run; use a new run rather than editing its record. Logs and selected reports live alongside `run.json` under `artifacts/workflow/runs/`.

The runtime host oracle needs a pristine `artifacts/engine/rootfs-amd64` and the customized image. It creates its own disposable working copy. `ENGINE_HOST_BUILD` selects its output directory, while `ENGINE_ROOTFS` can select a different pristine input. The Android harness's ARM64 fixture is now `artifacts/engine/fixtures/debian-13-slim-arm64`, not the retired research directory.

Additional Server black-box, differential layout and real-image lifecycle checks are in `engine/server/tools/`. Their arguments and proven scope are documented in the [Server implementation](../engine/server.md). The source-level Kotlin reducer is an oracle for Rust behavior, not a production state writer. Keep fixture process tests distinct from real runtime execution.

The separation worker's `agent` run `20261003T083733Z-4df7e37c` passed 67 JVM tests: 53 adapter tests, two shared codec tests and twelve service tests, and built the service JAR. That evidence belongs to its recorded source fingerprint and establishes host contracts, not final merged-source or guest/device acceptance. The service cases cover bounded journals and snapshot transfers, process epochs, submission deduplication and ambiguous outcomes, and index concurrency/recovery. Final runtime and device validation must use the integrated source and frozen artifacts.

`tools/tests/test_client_boundary.py` guards the production dependency graph, vendor process/import boundaries, absence of client vendor paths/installers, host-PTY fixture isolation, terminal lifecycle ownership, workspace-root encapsulation and guest-only Codex packaging. These source checks complement behavior tests; they cannot prove a JRE executes under Android 9's app UID.

## Reorganization gates

The App Gradle paths are `:app:android`, `:app:client` and `:app:proxy`. The former `core` suite is now `client`; other suite labels select their relocated catalog commands. Run the source-bound catalog, not historical commands copied from reports. Kotlin client fixtures must be compared with the Rust-exported catalog/schema/goldens; moving source folders does not establish binding parity.

Rust Chat policy, port and model tests establish only their implemented scope. Production cutover and removal of Kotlin/JRE/JAR require reducer/codec, adapter replay and service parity plus the zero-skip isolated API 28 guest Chat matrix in the [Chat reference](../engine/chat.md#rust-port-and-cutover-gate). Retain the production JVM path while any part of that gate remains unmet. Replacing the client event reducer is a separate coordinated projection-protocol change.

Runtime deduplication belongs to the separate `runtime-dedup` task, outside the current round-2 implementation. It requires the real `runtime-host` oracle, Engine cross-builds for both ABIs and isolated API 28 guest exec, PTY and stop. Keep all three existing trees intact if the full extraction cannot pass; do not hand off a partial merge. State physical ARM64 device proof as remaining, without using the daily tablet.

Terminal URL/file navigation, touch scrollbar and visible selection handles require joint device interaction checks while streaming and after wrapping, resize, rotation and IME changes. Offline web tests verify logic but cannot establish these Android touch behaviors.

## Android acceptance

Build with `tools/workflow check android-apk`, then pass its printed run directory to `tools/workflow device`. The default suite builds the x86_64 Debug app and instrumentation APKs. The driver verifies both APK digests and the source snapshot before queuing. It uses the shared emulator wrapper, installs the pair, selects the emulator explicitly for every adb command, and saves instrumentation output and logcat. A suite with skipped tests does not pass by default.

```sh
tools/workflow check android-apk
tools/workflow device --apk-run /absolute/path/to/the/build/run \
  --class top.flysoftbeta.workflow.app.WorkbenchEngineAcceptanceTest
```

The default AVD is `workflow-tablet-api28`, an API 28 x86_64 device at 1920×1200 and 261 dpi. Set `WORKFLOW_AVD` to an existing disposable AVD when another matrix entry is needed. `tools/with-emulator.sh` is the only entry point for an emulator: it owns the global device lease, starts one 1536 MiB/two-core emulator, puts writable overlays on disk, and cleans up its own process. Never edit a running or queued script.

The main acceptance surfaces are `ConnectionBoundaryAcceptanceTest`, `WorkbenchEngineAcceptanceTest`, `EditorEngineAcceptanceTest`, `EngineIntegrationTest`, `TerminalAttachmentTest`, `NativeTranscriptTest`, and `OfflineRendererTest`. Together they exercise real connection retirement and recovery, files and drafts, layout interactions, Sora reading position, customized environment lifecycle, native streaming markup, and Android 9 xterm input. `TerminalAttachmentTest` uses a managed fixture to test repeated EOF, another client's restart, metadata/output resets and cancellation without stopping a resource. JNI Android-shell fixtures remain instrumentation-only. Neither those fixtures nor a host JRE probe establishes the real guest service path. The integrated matrix must verify guest Java startup, the service handshake, chat snapshots/watches, optional tool jobs, terminal attachment/restart, import connection identity and executor ticket retirement on an isolated API 28 target. Reference-store UI tests cover focused interaction regressions and are named separately in reports.

`ProxyExecutableTest`, `ProxyEmulatorRootTest`, `ProxyPanelEmulatorTest`, `DeviceControlsEmulatorTest`, and `OverlayEmulatorTest` establish actual local-service behavior. Root tests require `--root` plus their instrumentation arguments and also check emulator hardware and shell UID. The tests preserve foreign routes and clean up their own interfaces and processes. Android 9 accessibility nodes may retain old coordinates after rotation; geometry acceptance reads the settled overlay window, preserving the existing strict position tolerance.

The daily tablet is not a disposable test device. Do not uninstall the app, enter its PIN, change another application's configuration, or alter its routes, DNS or system settings. A rooted emulator does not establish that an OEM `su` flow or grayscale action works on that tablet. API 28 x86_64 results likewise do not establish physical ARM64, newer Android or real 16 KiB glibc workloads.

## Resource leases and release validation

Heavy commands share the primary checkout's `artifacts/.gradle.lock`; Cargo uses at most two jobs. Use `tools/with-build-lock.sh COMMAND ...` for an extra heavy command that is not in the suite catalog. Task checkouts link their conventional lock filenames to that same resource. Do not hold the build lease while waiting for a device lease, and do not wrap a command that already owns its build lock.

Release signing remains in the coordinator checkout. Follow the [Release build procedure](building.md#build-and-verify-release-apks) for key initialization and APK paths. `tools/init-release-key.sh` creates a local key only when absent; `tools/build-release.sh` acquires the build lease, packages both customized images, copies outputs under that lease, and verifies signatures, ABI, assets, version and digests. Signing keys, APKs and source checkpoints are ignored local material. A release still needs its own startup/connection smoke test because a Debug instrumentation result does not exercise the Release package flags.

Record a result only for what ran. State the actual device/API/ABI, selected classes, source identity, artifact hashes, failures and skipped cases, and whether a backend used real credentials or a fixture. Keep raw logs and screenshots outside Git; put a concise English explanation in `docs/report/` when the evidence matters to a maintained claim.
