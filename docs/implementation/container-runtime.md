# Container runtime

The Workspace Server invokes `workflow-runtime` to run guest processes and manage image generations. Android packages it as `libworkflow-runtime.so`; `libworkflow-engine.so` names the separate Server. The runtime is a user-space compatibility layer running under the ordinary application UID, not a security boundary for hostile code. Product terminals and agents always use a configured environment. A rootless `run -- CMD` form remains available to native tests, not as a product host fallback.

## Implementation

`engine/environment/runtime` contains the Rust CLI, ptrace scheduler, path resolution, virtual identity and permission handling, xattr metadata, hard-link journal and fsck, exec planning, and streaming installer. Linux x86_64, Android x86_64, and Android aarch64 retain explicit platform modules for their libc and register layouts. Low-level pointer operations still use unsafe Rust. Replacing the old implementation did not by itself establish complete memory safety.

`engine/environment/runtime/src/sysinv.rs` supplies shared read-only syscall classifications, backed by pinned Linux v7.2.8 source digests in `inventory/`. Unknown numbers, holes, and x32 syscall numbers are rejected. `engine/environment/runtime/tools/generate-syscalls.py --check` verifies the source digests, complete classification, and generated output.

`engine/environment/loader` is a Rust `no_std` ELF loader. Assembly is limited to syscall entry and the final stack transfer. The executable has no libc, dynamic interpreter, or runtime relocations; segments are aligned to 16 KiB. The loader obtains the actual page size from `AT_PAGESZ` and retains both file-mapping and anonymous-copy loading paths.

The runtime links the platform libc and exactly `zstd-sys = 2.0.16`, incorporating zstd 1.5.7. It neither links nor invokes the archived C engine. Those sources and licenses remain in [the native-engine archive](../archive/native-engine/ARCHIVE.md) as historical material, outside production build inputs.

## Execution contract

`run`, `install`, `clone`, `verify`, `remove`, `fsck`, and `probe` preserve the frozen CLI argument, output, and exit-code contract used by the native acceptance workloads. The Server invokes them with separate argv elements. `run` supports a generation `--root`, repeated `--bind HOST:GUEST` and `--hide`, `--cwd`, virtual `--user work|root` or `--uid`/`--gid`, a loader path, socket directory, binfmt, seccomp, and process-tree policy. Guest and host ABIs must match; this is not a cross-architecture CPU emulator.

Default bindings expose `/proc`, `/sys`, necessary devices, and Android bionic paths. Explicit tooling such as image packaging may use `--no-default-binds` and provide all required bindings itself. Ordinary terminals and agents retain the default binding behavior. The Server supplies individual `--hide` entries for private workspace state, while the protocol controls explicit configuration and service-file access.

Every guest exec actually executes the loader, preserving close-on-exec handling, thread reaping, signal reset, and command-line semantics. Exec plans retain virtual UID/GID, `AT_SECURE`, shebang/binfmt handling, and argv. Android x86_64 rewrites legacy syscalls when app sandbox SIGSYS behavior requires it; virtual identity and denied classes are emulated directly.

Within a rootfs, `.workflow-engine/` stores hidden metadata and instance locks, hard-link objects and journals, and FIFO backing objects. It is a runtime implementation detail, distinct from `<workspace-root>/.workspace/`. Environment owns the outer generations and lifecycle; Server composes the domain APIs. Default exit behavior converges the whole guest process tree; `--wait-all` provides an explicit alternative waiting mode.

## Image installation

The installer streams zstd and tar data, verifies digests and attributes, extracts relative to directory file descriptors, removes failed staging generations, and respects seed directories and generation-use locks. The [image format](image-format.md) defines the archive and metadata validation in detail.

Installer exit codes distinguish invalid data (65), missing input (66), failure to create a destination (73), I/O failure (74), and insufficient space (75). Removal and repairing fsck return 3 when a generation is still in use. A successful image build does not establish that its tools execute correctly on an Android device.

## Build and acceptance entry points

`engine/environment/runtime/build-host.sh OUT` produces `OUT/engine` and `OUT/loader`; `ENGINE_RUST_PROFILE=debug` enables the debug runtime configuration. `ENGINE_ANDROID_OUT=OUT engine/environment/runtime/build-android.sh [arm64-v8a x86_64]` produces each ABI's runtime and loader at API 28. Heavy builders share `artifacts/.gradle.lock`, limit Cargo to two jobs, and use `WORKFLOW_BUILD_LOCK_HELD=1` for nested builders when their caller already owns the lock.

`ENGINE_HOST_BUILD=OUT engine/environment/runtime/test-host.sh` runs the retained 68-case host acceptance workload against the Rust runtime. C probes and frozen oracles in `engine/environment/runtime/tests/native/` are test inputs, not another runtime. `engine/environment/runtime/test-android.sh` stages the Rust binaries and runs the 24 baseline and 29 M5 cases through `tools/with-emulator.sh` in the API 28 x86_64 app sandbox. The test harness's historical filename for the runtime is local to that harness; it does not change the production executable names.

The [runtime report](../archive/implementation-1.0.0/reports/rewrite/rust-runtime.md) records the verified matrix. ARM64 compilation does not constitute ARM64 device acceptance. Real 16 KiB systems still have limitations with 4 KiB-linked amd64 glibc shared libraries; static ELF simulations do not prove complete dynamic-library compatibility. The [testing guide](../development/testing.md) governs new validation, and [status](../status.md) preserves these limits.
