# Workspace Server

The `workflow-server` package builds `workflow-engine`, the unchanged executable used by Android's local bootstrapper. It serializes cross-domain transactions, serves authoritative snapshots and revisions, and keeps slow environment work outside the workspace lock. Domain crates own behavior; Environment owns atomic private persistence and guest processes.

`src/main.rs` composes the domains. `state.rs` combines typed Workspace and FileWork state into the existing atomic workspace document; `local_services.rs` manages typed executor epochs and operation receipts. `transport.rs` owns bounded stdio framing, connection lifetime and request scheduling. `protocol.rs` defines typed envelopes, parameters/results, exact IDs and errors. `contracts/` declares the method catalog and generates the contract/schema/goldens. `chat.rs` is the temporary typed private bridge to the unchanged Kotlin guest service; its body payloads are explicitly opaque until round 2.

The moved domain tests retain the checked-in Kotlin layout oracle and the archive/draft policies. Server `tests/jsonl.rs` runs the public protocol against a real Server process, including watches, binary uploads, document revisions, service reports and restart recovery. The optional fixture/real-environment workloads in `tools/` have separate acceptance scopes and remain distinct from device proof.

From the repository root:

```sh
tools/workflow check rust-server --task engine-reorg-r1
tools/with-build-lock.sh cargo run --manifest-path engine/Cargo.toml --locked -p workflow-server --bin workflow-engine -j 2 -- export-contract --out engine/protocol
```

Regeneration is explicit: the Rust test fails if any checked-in generated contract file differs. Round 2 will check Kotlin bindings against these fixtures and schemas. The [protocol guide](../../docs/engine/protocol.md), [ownership guide](../../docs/engine/server.md) and [testing guide](../../docs/development/testing.md) describe the supported contract and acceptance limits.
