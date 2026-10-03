# Rust Workspace Engine

The Engine owns sessions, layouts, files, drafts, configuration, environments and managed processes. Android connects through `workflow.workspace/1`. The [generated contract](protocol/contract.json), schema and golden fixtures are exported from Rust types; the [protocol guide](../docs/implementation/protocol.md) describes their semantics.

The Cargo workspace contains these packages:

| Package | Path | Ownership |
| --- | --- | --- |
| `workflow-environment` | [environment](environment/README.md) | Typed `.workspace/` store, image/tool provisioning, lifecycle and runtime process API |
| `workflow-runtime` | [environment/runtime](environment/runtime/README.md) | Concrete ptrace isolation executable |
| `workflow-loader` | [environment/loader](environment/loader/README.md) | Freestanding guest ELF loader |
| `workflow-workspace` | [workspace](workspace/README.md) | Sessions, panels and layout |
| `workflow-filework` | [filework](filework/README.md) | Files, versions, drafts, diff, imports and archive policy |
| `workflow-terminal` | [terminal](terminal/README.md) | Terminal identity/lifecycle over runtime PTYs |
| `workflow-server` | [server](server/README.md) | Protocol, transport and domain composition |

The Server binary remains `workflow-engine`. Android packages the executables as `libworkflow-engine.so`, `libworkflow-runtime.so` and `libworkflow-loader.so`; these are executable programs, not JNI libraries. `build-android.sh` builds the complete distribution. The loader's specialized build retains its freestanding static-PIE flags; its Cargo target is feature-gated to avoid linking it with the ordinary host test harness.

The unchanged Kotlin service under `chat/` is the round-1 exception to the Rust-only target. Server supervises it through the Environment runtime until the round-2 Chat port passes adapter/reducer/service parity and guest-device acceptance. No host execution fallback is added. The runtime's three platform trees remain separate; the archived C implementation never becomes a production input.

Run `tools/workflow check rust-server --task engine-reorg-r1` for all domain tests, Server protocol tests and generated-contract drift. Run `runtime-host`, `native`, `image`, `infrastructure`, `documentation` and `android-apk` as described in the [testing guide](../docs/development/testing.md). Heavy commands share the primary build lease and use at most two Cargo jobs. Build products remain in ignored `target/` and `artifacts/` directories. Compilation and host checks do not establish Android device acceptance.
