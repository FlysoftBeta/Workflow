# Kotlin chat deletion

Branch `cleanup-kotlin-chat`, based on `9fce582`. Environment: cloud Linux container with Rust, Java, the Android SDK at `/opt/android-sdk` and Gradle; no emulator, customized image, CMake or Engine Android build. `tools/with-build-lock.sh` wrapped the heavy Cargo and Gradle commands; Cargo used two jobs.

## Decision

The user explicitly decided to delete the Kotlin chat path now instead of after isolated API 28 guest Chat acceptance. This overrides the deletion gate that `AGENTS.md`, the Chat reference and the module-reorganization proposal previously stated. Server had already switched to the in-process Rust `workflow-chat` ([cutover report](2026-10-04-rust-chat-cutover.md)). Retired source was not copied into `docs/archive/`; git history is its archive.

## Change

- Deleted the Kotlin chat service and its tests (`engine/chat/src/{main,test}/kotlin`, `engine/chat/build.gradle.kts`), the `:engine-chat` and `:agent` Gradle projects, the `agent/` tree with its Codex/Claude protocol assets, coverage golden and Kotlin fixtures, and `third_party/jre`. The Rust crate keeps its own fixtures and goldens under `engine/chat/tests/`.
- Moved the only remaining `:agent` consumer dependency, the `agent.process` launch port used by `EngineProcessLauncher` and `GuestProcessFixture`, into `:app:client` test fixtures with its package identity. Android instrumentation already depended on those fixtures.
- Removed the JAR and JRE from `engine/tools/package.py`, the APK tools task and notices, `tools/build-release.sh` and Environment's tool catalog validation, verification commands and status. The mandatory payload is now Codex and the Codex and Claude Code notices; Claude stays optional. The instrumentation tool check expects only `codex`.
- Replaced the `agent` suite with a Cargo `chat` suite, removed the deleted projects from `app-unit` and `tools/check-app-unit.sh`, deleted the Kotlin-only `tools/update-protocol.py` and `tools/update-claude-protocol.py`, and turned the boundary test that required `engine/chat/build.gradle.kts` into one that rejects the retired paths.
- Fixed the five remaining `workflow-chat` Clippy warnings. Checked every non-test `serde_json::Value` use in Server, Chat and Environment: the `OpaqueJson` definition, the exact-token vendor projection in `wire.rs`, a JSON-RPC null result and test support remain, all named opaque values. The stale `chat.command` comment now says that Chat decodes the arguments; the exported schema and Kotlin bindings were regenerated.

## Checks

- `cargo test --manifest-path engine/Cargo.toml --locked -j 2`: 223 passed, 0 failed.
- `cargo clippy -j 2 -p workflow-chat -p workflow-server -p workflow-environment --all-targets`: no `workflow-chat` warnings. Existing Server (five) and Environment (four) warnings were not introduced here and were left alone.
- `./gradlew :app:client:test :app:android:testDebugUnitTest :app:android:compileX86_64DebugAndroidTestKotlin`: passed, so Gradle configuration succeeds without the deleted projects.
- `python3 tools/generate-client-protocol.py --check`: bindings match. `python3 tools/check-docs.py`: passed.
- `tools/tests/test_engine_boundaries.py` and `test_workflow.py`: passed. `test_client_boundary.py`: one failure that predates this change and is unrelated to it, because production `AgentPaths.kt` contains the `/home/work/.codex` and `/home/work/.claude` guest paths that the vendor-path rule rejects.

## Limits

The APK was not built: the customized images, CMake and the Engine Android cross-build are missing here, so the reduced tools payload has not been packaged or run. Isolated API 28 acceptance of the Rust chat is still pending, as are real accounts and physical ARM64. The Codex App-Server schema bundle and protocol-coverage golden went with `:agent`; no Rust inventory or coverage check replaces them yet.
