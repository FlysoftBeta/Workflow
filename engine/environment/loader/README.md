# Freestanding ELF loader

The loader maps guest ELF executables and starts them with the stack and auxiliary-vector contract
expected by the container runtime. It is Rust `no_std`/`no_main`, with no dynamic interpreter, and
uses an explicit freestanding build. Android packages it as `libworkflow-loader.so` so
it can be executed from `nativeLibraryDir`.

`src/main.rs` supplies freestanding support and panic behavior, with the entry stubs in
`src/x86_64.S` and `src/aarch64.S`. `src/arch.rs` contains raw syscall support, and `src/elf.rs`
implements ELF mapping and handoff. `build.sh` invokes `rustc` with a static PIE link and rejects output containing runtime
relocations or an interpreter. Android linking uses the pinned NDK and a 16 KiB maximum page size.

Build through the Engine's host or Android wrapper. The low-level `build.sh` expects a target and
an output path and assumes its caller holds `artifacts/.gradle.lock`; it does not acquire a second
lock. Output belongs under ignored `artifacts/`, not this source directory.

Loader behavior is exercised with the real runtime by
[`runtime/tests/`](../runtime/tests/README.md), including static and dynamic guest workloads.
Successful linking alone is not acceptance on Android. See the
[runtime guide](../../../docs/implementation/container-runtime.md) and
[testing guide](../../../docs/development/testing.md) for the platform matrix.

The loader is also registered as the `workflow-loader` Cargo workspace member.
Its binary requires the explicit `freestanding` feature so ordinary workspace
unit tests do not link a no-std executable with the host test harness. The
supported production path remains `build.sh`, which selects the target linker,
uses panic-abort and static PIE flags, and rejects relocations and an interpreter.
For an explicit Cargo build, use `cargo rustc -p workflow-loader --features
freestanding -- -C panic=abort`; `build.rs` supplies the same linker flags.
