# Chat domain

`workflow-chat` owns agent defaults and the Engine chat implementation: the `ChatWire`-compatible model and in-place reducer, the bounded journal, the conversation index, the composer-revision send ledger, the optional-tool demand policy, the Codex App-Server and Claude Code adapters over typed JSON-RPC and stream-json/control transports, and the `Chat` service facade behind `chat.command`. Server links it and implements its ports over Environment, FileWork and the Workspace transaction. The crate has no direct filesystem, subprocess or libc access; a source test enforces that.

Parity evidence: 26 frozen Kotlin reducer cases, 18 frozen approval cards, and replays of the recorded Codex and Claude sessions in `tests/fixtures/` through the actual Rust adapters, ported from the Kotlin `agent` tests together with the Codex login races of the [Chat guide](../../docs/engine/chat.md). The `testing` feature supplies the fakes and the scripted vendor server.

The Kotlin service, `:agent`, JAR and JRE remain as the oracle until isolated API 28 device acceptance permits their deletion. See the [Chat guide](../../docs/engine/chat.md) and [corpus provenance](tests/golden/README.md).

Run `cargo test --manifest-path engine/Cargo.toml --locked -p workflow-chat -j 2`, or `tools/workflow check rust-server` in the integrated checkout. The retained Kotlin oracle uses `tools/workflow check agent`.
