# FileWork

`engine/filework` is package `workflow-filework`. It owns ordinary workspace-file IO, paths, file versions, drafts, imports, diff and archive save preflight. It depends on Environment's typed store and staging APIs. It does not decide panel layout, accept a client-computed workspace snapshot, or launch guest processes. [Server](server.md) composes its effects with [Workspace](workspace.md) in one committed transaction.

## Working Resources

Working Resources hold unsaved content independently of session arrangements. There is one file draft per normalized path, shared by all sessions. It stores text, edit time, revision and its base `DiskVersion(exists,size,mtime,sha256)`. A file without a draft is clean. With a draft it is dirty when disk matches the base, conflicted when disk changed, and deleted when the disk file disappeared. These conditions are derived rather than separately mutable flags.

A conversation composer is keyed by conversation ID and stores text and pending workspace-path attachments. Content changes advance its revision; an unchanged save retains it. Submission acknowledgement clears only a matching owner, revision, text and attachments. An empty tombstone retains monotonic revisions after clearing, preventing a late acknowledgement from erasing newer input. Chat owns submission and deduplication; Server routes its exact acknowledgement to FileWork.

The Engine periodically refreshes tracked disk files. Clean files adopt external changes, while dirty files retain their draft and report conflict. Keeping a draft updates its base version; using disk discards the draft; comparison opens a Diff panel. Android never runs a competing file watcher or durable draft store.

## Paths and explicit configuration

Ordinary paths must be workspace-relative, cannot traverse parents, and cannot cross a symlink, even one pointing back into the workspace. The explorer always hides `.workspace`. Explicit configuration actions can access `.workspace/config.json`, `.workspace/env.json`, canonical `.workspace/proxy/` files and safe files for other services. Engine-private state remains inaccessible through file APIs and is masked inside the guest.

Environment centralizes private path construction and classifies editable configuration. FileWork uses the owning configuration validator when saving those files. A settings save and editor save refer to the same original file, so disk versions and document sidecars detect conflicts instead of creating independent copies. The proxy root is `.workspace/proxy/`; version 1.0.0 does not migrate the earlier service path.

Only FileWork checks existence and type of ordinary files for Engine-mediated terminal navigation. Terminal supplies generation and cwd context, while Server composes resolution with safe file access. Guest home/system paths and private Engine paths cannot become Workbench file targets.

## Atomic file operations and imports

Saving an existing file preserves its permissions. New files, uploads, moves and trash restores publish through `renameat2(RENAME_NOREPLACE)`, so an external writer creating a destination wins instead of being overwritten. Moving a file reports the final path to Server, which updates session targets, drafts and attachment references. A destination with an unsaved draft blocks the move.

Uploads are private staged resources until commit. Chunks must be contiguous, declared length must match the source, and data is synchronized before publication. The directory/name import form allocates its final filename inside the Engine commit boundary and retries no-replace publication after an external collision. The result contains the actual path; clients do not list names or parse error text to choose a retry. Failure, cancellation or connection shutdown discards incomplete staging.

Android URI/camera staging is a bounded disposable source for this API, not a bypass around it. Its current per-import staging cap is 512 MiB; Engine upload capacity and protocol chunk bounds are separate limits documented in [Server](server.md#contract-export-and-bounds).

## Archive protection and persistence

Session archive policy consults Working Resources before a layout becomes archived. For each dirty resource, the latest live session referring to it is protected from automatic archival. Manual archive requires the explicit `save_all`, `keep_drafts` or `discard` decision when dirty resources exist.

`save_all` validates all file base versions before saving or archiving, refuses on any conflict, and retains unsent conversation input. `keep_drafts` archives only the arrangement. `discard` drops the referenced unsaved resources according to the user's decision. Visibility of a panel does not change these protections.

The combined `state/workspace.json` transaction keeps session state and drafts atomic while their APIs remain separately owned. Unknown formats remain read-only, corrupt originals are retained, and a committed revision is acknowledged only after Environment store publication. There is no legacy migration.

The `rust-server` suite includes FileWork tests for path policy, version conflicts, atomic imports, draft acknowledgements and archive preflight. Android tests must also exercise real Engine imports, reconnect and editor conflicts; reference-store fixtures establish only the behavior they invoke. [Testing](../development/testing.md) and [status](../status.md) define that evidence boundary.
