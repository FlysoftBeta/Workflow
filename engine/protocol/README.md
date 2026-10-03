# Generated Workspace contract

`workflow-server` owns these files. Regenerate from the repository root with:

```sh
tools/with-build-lock.sh cargo run --manifest-path engine/Cargo.toml --locked -p workflow-server --bin workflow-engine -j 2 -- export-contract --out engine/protocol
```

`contract.json` contains the method catalog, frame limits and a schema reference for each method's parameters and result. Resolve its `methodSchemas` fragments against `schemaFile` (`schema.json`), whose shared `$defs` include all referenced types. `schema.json` is a JSON Schema 2020-12 document for canonical known requests; response, error and notification definitions are included for consumers selecting the corresponding definition. `golden.json` is a deterministic collection of messages serialized from Rust instances, covering envelopes, IDs, errors, snapshots, commands, files, network and terminal output. There are no credentials, device paths or timestamps from a real workspace in the fixtures.

The `generated_contract_has_not_drifted` Rust test regenerates all three in memory and compares exact bytes. The same suite runs public JSONL black-box tests. The schema is a type/shape contract: filesystem authorization, capability measurements, revision conflicts and bounds depending on current state still require method execution tests. The receiver tolerates some omitted defaults for compatibility; examples use canonical explicit fields.

The round-1 Kotlin Chat service retains ownership of its body codec. Its payload schema is explicitly opaque; the Rust bridge types its envelopes and preserves the body. Custom service measurements and unknown extension members likewise use named opaque types. Known Workspace, FileWork, Terminal, Environment and proxy data are typed.

Round 2 will validate Kotlin requests against this schema, decode and re-encode Rust goldens, check method coverage, and add the Rust Chat model schema after its parity gate. Version remains 1.0.0 and protocol negotiation still requires the exact `workflow.workspace/1` string.
