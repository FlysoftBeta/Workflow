# Chat domain

`workflow-chat` owns agent defaults and the Rust port of the chat model, reducer, bounded journal, conversation index, composer-revision send ledger and optional-tool demand policy. Its interfaces require Environment-managed execution and typed storage; it has no direct filesystem, subprocess or libc access. The Server uses its defaults today while retaining the supervised Kotlin adapters for production chat.

The phase-one models preserve the current Android `ChatWire` representation: `_type` variants, uppercase enum names, explicit nullable defaults and alternating key/value arrays for structured map keys. Twenty-six frozen Kotlin reducer cases verify reduction and codec behavior. Rust service tests cover concurrent submission/reopen, ambiguous outcomes, exact composer acknowledgement, index serialization, process epochs, environment epoch retirement, journal retention and install demand.

This is an incomplete production cutover. Vendor adapter replay, complete wire-variant parity, runtime composition, guest execution and isolated device gates remain required before removing the Kotlin service or JRE. Neither model parity nor a compiled Rust dependency establishes those gates. See the [Chat guide](../../docs/engine/chat.md) and [corpus provenance](tests/golden/README.md).

Run `tools/workflow check rust-server` in the integrated checkout. The original branch catalog exercises public Chat tests through the Server integration harness; the integrated suite also selects `workflow-chat` directly. The retained Kotlin oracle uses `tools/workflow check agent`.
