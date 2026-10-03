# Testing and evidence

Choose checks by the boundary a change affects. Pure model and protocol tests are fast feedback; they do not replace an Android app-UID test for rendering, process execution, foreground services or root capabilities. The [current status](../status.md) records accepted scope and known limits, and reports preserve the source and artifact identity for each result.

## Host checks

The supported development host is Linux, with Git, Python 3.11 or newer, JDK 25, Rust 1.93.1, the Android SDK/build-tools 37, the pinned NDK and CMake, Node.js, and zstd. Exact dependency manifests and the Gradle wrapper are checked into the repository. Configure the Android SDK through `local.properties` or the SDK environment variables; do not commit machine-local paths. Install web test dependencies with `npm ci --ignore-scripts` in `web/` before its first check.

`tools/workflow check NAME` runs a named suite in the coordinator checkout. Add `--task NAME` to run in an isolated task checkout. The catalog is executable configuration in `tools/workflow-suites.json`, so commands and output locations can be reviewed together.

| Suite | What it establishes |
| --- | --- |
| `core`, `agent`, `proxy` | JVM contracts, models and adapters in their respective modules. Reference-store tests explicitly use test fixtures. |
| `app-unit` | Pure app-level formatting, rendering projections, interaction models and adapter helpers. |
| `rust-server` | Authoritative workspace, layout, file, environment and protocol behavior in Rust. |
| `runtime-host` | The Rust container runtime against the real host syscall and filesystem oracles. |
| `native` | Remaining native PTY and proxy guardian checks. |
| `image` | Image format, manifest, schema and rejection cases. |
| `web` | Offline terminal input, links and retained rendering behavior. Chat is native Compose. |
| `infrastructure` | Real temporary Git worktrees, ownership, locks, source-bound results, integration recovery and device-result parsing. |
| `documentation` | Maintained English prose, balanced fences and local links; frozen historical originals are excluded. |
| `lint` | Android lint, including API-level and Compose integration checks. |
| `android-apk` | A successful app/test build plus an immutable, digested APK pair; this is preparation, not device acceptance. |

Run only the checks needed by the change, then broaden when another boundary or a failure justifies it. Every result includes its source fingerprint. Changing source during a check taints that run; use a new run rather than editing its record. Logs and selected reports live alongside `run.json` under `artifacts/workflow/runs/`.

The runtime host oracle needs a pristine `artifacts/engine/rootfs-amd64` and the customized image. It creates its own disposable working copy. `ENGINE_HOST_BUILD` selects its output directory, while `ENGINE_ROOTFS` can select a different pristine input. The Android harness's ARM64 fixture is now `artifacts/engine/fixtures/debian-13-slim-arm64`, not the retired research directory.

Additional Server black-box, differential layout and real-image lifecycle checks are in `engine/server/tools/`. Their arguments and proven scope are documented in the [Server implementation](../implementation/workspace-engine.md). The source-level Kotlin reducer is an oracle for Rust behavior, not a production state writer. Keep fixture process tests distinct from real runtime execution.

## Android acceptance

Build with `tools/workflow check android-apk`, then pass its printed run directory to `tools/workflow device`. The driver verifies both APK digests and the source snapshot before queuing. It uses the shared emulator wrapper, installs the pair, selects the emulator explicitly for every adb command, and saves instrumentation output and logcat. A suite with skipped tests does not pass by default.

```sh
tools/workflow check android-apk
tools/workflow device --apk-run /absolute/path/to/the/build/run \
  --class top.flysoftbeta.workflow.app.WorkbenchEngineAcceptanceTest
```

The default AVD is `workflow-tablet-api28`, an API 28 x86_64 device at 1920×1200 and 261 dpi. Set `WORKFLOW_AVD` to an existing disposable AVD when another matrix entry is needed. `tools/with-emulator.sh` is the only entry point for an emulator: it owns the global device lease, starts one 1536 MiB/two-core emulator, puts writable overlays on disk, and cleans up its own process. Never edit a running or queued script.

The main acceptance surfaces are `ConnectionBoundaryAcceptanceTest`, `WorkbenchEngineAcceptanceTest`, `EditorEngineAcceptanceTest`, `EngineIntegrationTest`, `NativeTranscriptTest`, and `OfflineRendererTest`. Together they exercise real connection retirement and recovery, files and drafts, layout interactions, Sora reading position, customized environment lifecycle, native streaming markup, and Android 9 xterm input. Reference-store UI tests cover focused interaction regressions and are named separately in reports.

`ProxyExecutableTest`, `ProxyEmulatorRootTest`, `ProxyPanelEmulatorTest`, `DeviceControlsEmulatorTest`, and `OverlayEmulatorTest` establish actual local-service behavior. Root tests require `--root` plus their instrumentation arguments and also check emulator hardware and shell UID. The tests preserve foreign routes and clean up their own interfaces and processes. Android 9 accessibility nodes may retain old coordinates after rotation; geometry acceptance reads the settled overlay window, preserving the existing strict position tolerance.

The daily tablet is not a disposable test device. Do not uninstall the app, enter its PIN, change another application's configuration, or alter its routes, DNS or system settings. A rooted emulator does not establish that an OEM `su` flow or grayscale action works on that tablet. API 28 x86_64 results likewise do not establish physical ARM64, newer Android or real 16 KiB glibc workloads.

## Resource leases and release validation

Heavy commands share the primary checkout's `artifacts/.gradle.lock`; Cargo uses at most two jobs. Use `tools/with-build-lock.sh COMMAND ...` for an extra heavy command that is not in the suite catalog. Task checkouts link their conventional lock filenames to that same resource. Do not hold the build lease while waiting for a device lease, and do not wrap a command that already owns its build lock.

Release signing remains in the coordinator checkout. `tools/init-release-key.sh` creates a local key only when absent; `tools/build-release.sh` acquires the build lease, packages both customized images, copies outputs under that lease, and verifies signatures, ABI, assets, version and digests. Signing keys, APKs and source checkpoints are ignored local material. A release still needs its own startup/connection smoke test because a Debug instrumentation result does not exercise the Release package flags.

Record a result only for what ran. State the actual device/API/ABI, selected classes, source identity, artifact hashes, failures and skipped cases, and whether a backend used real credentials or a fixture. Keep raw logs and screenshots outside Git; put a concise English explanation in `docs/report/` when the evidence matters to a maintained claim.
