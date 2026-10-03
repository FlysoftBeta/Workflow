# Rust Workspace Engine

- `server/`: the JSONL workspace owner (`workflow-engine`, APK `libworkflow-engine.so`).
- `runtime/`: the container compatibility runtime (`workflow-runtime`, APK `libworkflow-runtime.so`).
- `loader/`: the standalone no_std ELF loader (APK `libworkflow-loader.so`).
- `protocol/contract.json`: the current `workflow.workspace/1` contract inventory.

The root Cargo workspace contains server and runtime; loader has a separate freestanding build.
Heavy Cargo and Gradle builds share `artifacts/.gradle.lock`, with Cargo jobs limited to two.
Runtime/loader build scripts acquire the lock themselves; callers already holding it pass
`WORKFLOW_BUILD_LOCK_HELD=1`.

See [architecture](../docs/architecture.md), [protocol](../docs/protocol.md),
[workspace ownership](../docs/workspace-engine.md), [runtime](../docs/container-runtime.md), and
[test commands](../docs/testing.md). Current runtime acceptance probes/oracles are under
`runtime/tests/`; the archived C implementation in `docs/archive/native-engine/` is not a build input.
