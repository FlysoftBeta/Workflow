# Chat parity corpus

`reducer.json` contains 26 Kotlin `AgentReducerTest` input/output cases, serialized by the production `ChatWire` codec. It was exported from the retained oracle by the source-bound `agent` run `20261003T130550Z-74022ed5` with `WORKFLOW_RUST_PARITY_DIR=engine/chat/build/rust-parity` (absolute path supplied at execution). Outputs were copied from ignored build data; Rust did not generate expected states. This covers the existing fold-based reducer tests, not full adapter replay or device acceptance. The command-output and unknown-history bounds have separate Rust tests because their Kotlin tests call the reducer outside the fold helper.

Regenerate by setting `WORKFLOW_RUST_PARITY_DIR` to an empty ignored directory and running `tools/workflow check agent`, then collect the emitted JSON objects in filename order. Normal tests never rewrite this corpus. The Kotlin source and adapters remain until the complete parity and guest/device gates are met.
