# Workspace Server

`workflow-engine` serves the workspace JSONL protocol and is the only production writer of
workspace state. It serializes state commits, returns authoritative revisions and snapshots, and
keeps slow environment/process operations outside the state lock. Persistent data lives beneath
the selected workspace's `.workspace/` directory.

`src/main.rs` owns CLI setup, connection lifetime and method dispatch. `src/protocol.rs` contains
strict JSON parsing, bounded line framing, opaque request IDs, binary chunk encoding and protocol
errors. Keeping that wire layer separate makes its limits independent of workspace storage.
`workspace.rs` handles commands and state; `layout.rs` reduces layouts; `storage.rs` enforces paths
and atomic publication; `environment.rs` reconciles generations; `home_stage.rs` merges successful
post-script changes; and `process.rs` supervises environment processes and bounded output streams.

Unit tests live beside protocol helpers and in `src/tests.rs`. `fixtures/layout.jsonl` is the
checked-in layout oracle. `tools/layout-oracle.sh` compares Rust behavior with the Kotlin layout
model; `tools/test-protocol.py` exercises a real Server process with fixture runtime boundaries;
and `tools/test-environment.py` tests real-image lifecycle behavior. These have different scopes
and should be reported separately.

Run the Server unit suite from the repository root:

```sh
flock artifacts/.gradle.lock cargo test --manifest-path engine/Cargo.toml -p workflow-engine -j 2
```

The [protocol guide](../../docs/implementation/protocol.md) is the cross-component contract, and
the [workspace owner guide](../../docs/implementation/workspace-engine.md) describes persistence
and lifecycle rules. The [testing guide](../../docs/development/testing.md) links the full
protocol, image and device acceptance commands. Generated binaries belong in `engine/target/`
or ignored `artifacts/`, never beside source or fixtures.
