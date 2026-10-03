# Rust Workspace Engine

The Engine is the sole owner of workspace sessions, layouts, files, drafts, configuration,
environment generations and managed processes. Android connects through the exact
`workflow.workspace/1` JSONL protocol. [`protocol/contract.json`](protocol/contract.json) inventories
that contract; the [protocol guide](../docs/implementation/protocol.md) describes its semantics.

[`server/`](server/README.md) builds `workflow-engine`, packaged as `libworkflow-engine.so`, and
dispatches workspace RPC. [`runtime/`](runtime/README.md) builds the container compatibility
runtime `workflow-runtime`, packaged as `libworkflow-runtime.so`. [`loader/`](loader/README.md)
contains the freestanding Rust ELF loader, packaged as `libworkflow-loader.so`. Those filenames
allow Android to extract executable programs into `nativeLibraryDir`; they are not JNI libraries.

`Cargo.toml` includes server and runtime. The loader is built separately with `rustc` because it has
no standard library or dynamic interpreter. Top-level `build-android.sh` stages all three
executables; `runtime/build-host.sh` stages the host runtime and loader, while Cargo builds the
host Server. Build products go into ignored `target/` and `artifacts/` directories.
The maintained Rust runtime is platform-specialized; the frozen C implementation in
`docs/archive/native-engine/` is historical reference and is never a production build input.

Run workspace unit tests from the repository root:

```sh
flock artifacts/.gradle.lock cargo test --manifest-path engine/Cargo.toml -j 2
```

All heavy Cargo and Gradle builds share `artifacts/.gradle.lock`, with Cargo jobs limited to two.
The top-level and runtime build scripts acquire it; callers already holding it set
`WORKFLOW_BUILD_LOCK_HELD=1`. The loader's low-level build script assumes its caller holds the lock.
See the [architecture](../docs/implementation/architecture.md),
[workspace ownership](../docs/implementation/workspace-engine.md) and
[testing guide](../docs/development/testing.md). Runtime probes and workload oracles live under
`runtime/tests/`; host unit tests alone do not establish Android acceptance.
