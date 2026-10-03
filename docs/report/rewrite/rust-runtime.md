# Rust runtime / loader rewrite

## Delivery

The runtime implementation is now compiled Rust, with no production link to or execution of the frozen C engine. The initial mechanical translation used local C2Rust 0.22.1 (Compiler Explorer build 20260924), followed by stable Rust 1.93.1 adaptation and behavior fixes. No repository source was sent to a remote translation service. This is a behavior-preserving unsafe Rust port, not a claim that raw pointer ownership has been made memory-safe.

`engine/runtime/src/{host,android_x86_64,android_aarch64}` preserve explicit libc and register layouts for each supported platform. `logging.rs` replaces C variadic function definitions with Rust macros plus libc formatting. CLI uses `args_os` so arbitrary Unix path bytes and UTF-8 are preserved. `sysinv.rs` is a shared immutable Rust inventory generated from pinned Linux v7.2.8 inputs under `inventory/`; no fallback permits unknown syscalls. Compression alone uses pinned zstd-sys 2.0.16 / zstd 1.5.7.

`engine/loader` is Rust no_std with small raw-syscall/startup/stack-handoff assembly. Loader artifact checks also confirm no NEEDED libraries and 16 KiB alignment for every PT_LOAD (`artifacts/rust-port/loader-artifact-check.log`). The build checks absence of runtime relocations and a dynamic interpreter. LTO removes compiler panic formatting dependencies; the handoff is a PC-relative assembly branch, avoiding a loader GOT relocation. Loader storage for its wire plan has explicit 16-byte alignment. Both ELF segment mapping strategies, page-size simulation, auxv, scratch handshake, interpreter and stack rules remain.

Production integration entrypoint:

```
ENGINE_ANDROID_OUT=<output> engine/runtime/build-android.sh [arm64-v8a x86_64]
<output>/<abi>/libworkflow-runtime.so
<output>/<abi>/libworkflow-loader.so
```

The script takes the common Gradle/Cargo lock and limits Cargo jobs to two. A caller that already holds it sets `WORKFLOW_BUILD_LOCK_HELD=1`. `build-host.sh OUT` emits `OUT/engine` and `OUT/loader`. Loader can also be built directly with `engine/loader/build.sh TARGET OUTPUT` while holding the same lock.

## Findings fixed during parity validation

- The old resolver did not initialize `exists` for relative paths consisting only of `.` / `..`. C stack contents happened to produce 1 in the frozen tests; deterministic Rust initialization exposed a 0. `find .` then treated its open directory as a new file and rewrote its xattr to mode 000. The Rust resolver initializes the already-existing cwd/root to 1 and resets to 0 only on an explicitly permitted missing final component. All metadata goldens are unchanged.
- ELF headers read into a byte buffer need `read_unaligned` before typed access. Debug Rust caught the old C alignment assumption. All three runtime platforms now copy that header into aligned storage before parsing.

## Verification

- Optimized host runtime and Rust loader: **68/68**, frozen path (77 cases), metadata and identity oracles unchanged, legacy entry/SIGSYS modes, app-filter emulation, fast/plain ptrace, hardlink crash recovery, concurrent instances. Log: `artifacts/rust-port/host-suite-final.log`.
- Debug runtime (Rust alignment and overflow checks) and Rust loader: **68/68**. Log: `artifacts/rust-port/host-suite-debug.log`.
- Real workspace image install, generation verify, PTY and lifecycle: **9/9** phase checks, including 14 PTY assertions, SIGTERM grace/reaping, PTRACE EXITKILL, interrupted install cleanup, clone/verify and busy-generation refusal. Logs: `artifacts/rust-port/accept-host.log`, `artifacts/engine/rust-accept/pty.log`.
- Inventory policy test checks every entry of both ABI tables against the TSV and verifies unknown/gap bounds default deny. Pinned source hash and complete classification check: `python3 engine/runtime/tools/generate-syscalls.py --check`.
- Both Android ABI builds produced API 28 executables and relocation-free, interpreter-free, 16 KiB-aligned Rust loaders. Log: `artifacts/rust-port/build-android-final.log`; exact ELF headers, sizes and SHA-256 values in `artifacts/rust-port/final-binaries.json`.

- API 28 x86_64 real app sandbox (`workflow-tablet-api28`, emulator-5830): initial and final runs each **24/24 baseline + 29/29 M5**. M5 includes image installation, native metadata/identity oracles, crash/fsck, gcc/Python/Node/git, AF_UNIX, bionic-in-guest and apt network. The harness verifies all staged executable hashes in `nativeLibraryDir`; measured seccomp filter results match the runtime SIGSYS emulation probe. Log: `artifacts/rust-port/app-sandbox-final.log`; evidence directories: `artifacts/engine/device/workflow-tablet-api28-rust{,-m5}`. The final run uses the shared immutable syscall inventory.
- Targeted CLI checks pass UTF-8 argv/filenames, arbitrary non-UTF-8 Unix argv bytes, pack-only `--no-default-binds`, ordinary default binds, primary-process exit tree cleanup and `--wait-all`. Log: `artifacts/rust-port/contract.log`; executable test: `engine/runtime/tools/check-contract.py`.
- Invalid compressed image and missing input return 65 / 66 respectively and leave no target directory. Evidence: `artifacts/rust-port/install-negative.json`.

## Reference and limitations

The old C implementation remains under `native/engine/src`, `native/engine/loader`, `native/engine/include` as a frozen reference while the top-level integration moves its production build path. The Rust build does not read those engine/loader C files. C guest probes and Android/JNI code outside the container engine remain test/platform code. The harness uses a test-only filename alias `libworkflow-engine.so` for the Rust runtime to keep its original case files unchanged; production packages the server at that name and runtime at `libworkflow-runtime.so`.

This compatibility layer remains same-UID, not a security sandbox; original TOCTOU/proc and virtual-ID kernel limitations remain. The 16 KiB simulated static ELF cases pass, but 4 KiB-linked amd64 glibc libraries still reject 16 KiB runtime pages. No claim is made for arm64 daily-tablet, f2fs, Android API 29+ or real 16 KiB device acceptance in this delivery unless separately listed. No user tablet, routes, DNS/system settings, other apps, credentials or device PIN were touched.

The first emulator wrapper revealed an inherited-lock issue: if adb daemonizes after FD 9 is opened, the daemon keeps the device flock after emulator teardown. The leaked descriptor in the task-created adb daemon was closed via a brief debugger attach (`close(9)` returned 0); the daemon and device connections were preserved. Final acceptance then completed. The permanent wrapper fix was reported to the parent integrator; the runtime has no device-lock implementation of its own. Evidence: `artifacts/rust-port/release-inherited-lock.log`.
