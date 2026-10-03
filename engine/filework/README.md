# FileWork domain

`workflow-filework` owns editable workspace file IO and Working Resources: file versions, file drafts, composer drafts, conflicts, diff inputs, imports, uploads, and trash. It depends only on Environment. User-file paths are checked component by component and never follow symlinks. Explicit configuration and service-asset paths are routed through Environment's Store; FileWork does not construct private storage paths.

`FileWorkState` contains `drafts`, `composers`, and observed `disk` versions. The Server embeds it in the same state document as `WorkspaceState`, serializes commands, and persists the combined candidate before acknowledging its revision. A `FileWork` service operates on that candidate state. File operations return typed outcomes; after a successful move or removal the Server rebases or closes affected layout references and refreshes observations.

Archive policy preflights every referenced file before saving any file, retains unsent composer content during `save_all`, and discards working resources only for an explicit `discard` decision. `protected_sessions` selects the newest live reference to each dirty resource. The Server supplies candidates and performs the final session archival. FileWork's save callback validates configuration documents using their owning models.

Uploads use a separate `Uploads` collection and Store-owned staging handles. `begin_upload`, `upload_chunk`, `commit_upload`, and `cancel_upload` enforce declared sizes, sequential offsets, the 64 KiB chunk bound, and no-overwrite publication. Allocated names are chosen at publication time, including compound extensions and concurrent collisions. `read_chunk` returns raw bytes; the Server owns base64 wire encoding. `diff` returns typed disk and draft inputs without changing either version.

Tests cover stale shown revisions, archive preflight, composer acknowledgements, protection selection, drafts following moves, trash restore, executable modes, private paths, symlinks, service assets, and upload collision and offset rules. These host checks do not establish device acceptance.
