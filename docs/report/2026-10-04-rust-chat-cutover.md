# Rust chat cutover (host evidence)

Branch `replay-rust-chat-cutover`, based on `9a4765f`. Environment: cloud Linux container with Rust 1.97 (Cargo and Clippy), Java and Node; no Android SDK, emulator, Codex CLI or Claude CLI. `tools/workflow` and its artifact locks were not used; plain Cargo ran with `-j 2`.

## Change

The `workflow-chat` crate now contains the complete Engine chat implementation: typed wire projections over an exact-token vendor tree, threaded JSON-RPC and Claude control transports, the Codex App-Server backend with the documented login state machine, the Claude Code backend (sessions, spare process, resume, fork, transcript hydration, hooks for tests), and the `Chat` service facade with all `chat.command` operations, the journal, the conversation index and the composer-revision send ledger. The final commit switches `workflow-server` from the supervised Kotlin service to the in-process crate. Server implements Chat's ports over the Environment runtime, document store, tools and agent homes, FileWork attachments and the Workspace transaction. The Kotlin `:engine-chat`, `:agent`, JAR, JRE payload and packaging are unchanged and no longer launched.

The `chat.snapshot`, `chat.watch` and `chat.command` wire is unchanged: same methods, argument names, `ChatWire` result shapes, chunked snapshot transfer and error codes (`-32602` for invalid arguments and unoffered decisions, `-32000` otherwise, kind `chat`). The exported schema and golden fixtures are unchanged. Only the contract's ownership text changed, and `tools/generate-client-protocol.py` regenerated the Kotlin bindings digest; `--check` passes. Vendor JSON objects are re-encoded in canonical key order, so raw payloads equal the Kotlin output as values, not byte for byte.

## Commands and results

- `cargo test --manifest-path engine/Cargo.toml --locked -j 2`: 223 passed, 0 failed, 0 ignored across the workspace. `workflow-chat` alone: 84 passed.
- `cargo clippy -p workflow-chat -p workflow-server --all-targets`: no new warnings in Server chat code. Remaining chat warnings are two complex types, one enum-size note, a pre-existing reducer `unwrap` and one in test support. Other crates' existing collapsible-if warnings were not touched.
- `python3 tools/generate-client-protocol.py --check`: bindings match. `python3 tools/check-docs.py`: passed.

Parity evidence in `engine/chat/tests`: the 26 frozen Kotlin reducer cases and 18 approval-card cases; `codex_replay` (recorded Codex 0.157 session: markdown and math, command approval answered only by the user, interrupt, history, rename, fork, archive, reviewer invariants, auto-reviewer send block, unavailable environment); `codex_login` (18 login races, including completion before or after polls, null reads after completion, unattributed completion, routing-discovery failure with erroring reads, single held read with cancellation, cancellation while starting, start failure, timeout, process exit, sign-out and startup read failure); `codex_events` (usage-limit and inbound replays, decisions, dispositions, parameters, queue paging); `claude_replay` (recorded Claude 2.1.283 session and fork, hook callback, allow and deny, mode, model and effort switches, interrupt, not-logged-in run, process exit expiring cards); `claude_unit`; `chat_service` (commands, metadata, watch, ledger deduplication, epoch-checked responses, unsupported launches); transport, journal, ledger, index and tool-policy cases; and a source guard.

## Not verified here

No Android build, Kotlin test, emulator or device run was possible, so `tools/workflow check agent`, `android-apk` and the isolated API 28 `EngineIntegrationTest` chat cases (snapshot and watch, unauthenticated Codex startup, reattachment, Claude install and initialization, generation restart) did not run. The guest spawn path through `runtime::spawn_piped` with a real image, real Codex or Claude binaries, real accounts and ARM64 behavior are untested. Deleting the Kotlin chat, `:agent`, the JAR and the JRE payload remains gated on that device acceptance.
