# Workspace domain

`workflow-workspace` owns typed sessions, workbench arrangements, panels, panel views, and layout transactions. It has no filesystem or working-resource state. `WorkspaceState` is embedded in the Server's combined state document; the Server owns persistence, the revision, and the transaction spanning this crate and FileWork.

`layout::apply` is a pure normalized reducer over `Workbench` and `LayoutAction`. `WorkspaceState` provides session activation, naming, pinning, resource touches, explicit archival, retention, path rebasing, and terminal references. The Server supplies FileWork's protected session IDs when running retention. Closing panels or removing a conversation reference never implicitly discards a file draft.

The persisted field names remain unchanged. Every nested persisted object retains unknown extension fields. Only a malformed workbench is reset during session decoding; its session remains present. Use Environment's strict JSON decoder at serialized boundaries so duplicate keys, opaque numeric spellings, and future extension payloads remain distinguishable. Action decoding selects a typed argument structure after reading its discriminator; no untyped layout state is retained.

The unit suite checks the existing Kotlin-generated fixtures, normalization idempotence, unknown extensions, and numeric action decoding. The Server suite can additionally run the larger generated oracle corpus against this reducer.
