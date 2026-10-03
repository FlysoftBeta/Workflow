# Repository layout and retention

The repository root contains build entry points, the working agreement, and the modules. `app/` owns Android integration and features; `core/`, `agent/`, and `proxy/` are pure JVM modules. `engine/server`, `engine/runtime`, and `engine/loader` contain the production Rust components. Image construction, the remaining native adapters, offline terminal assets and dependency manifests live in `image/`, `native/`, `web/`, and `third_party/`. Module READMEs describe their internal boundaries.

Documentation is organized by the question it answers. [Product](../product/README.md) describes supported behavior and terminology. [UX](../ux/README.md) defines interactions and visual rules. [Implementation](../implementation/README.md) explains contracts and mechanisms. This development section describes how to add or change them. [Reports](../report/README.md) record evidence for a particular change, while [archives](../archive/README.md) preserve historical originals. The short old top-level document paths are navigation pointers, so source comments and external references can still find the maintained subject.

## Local material

`artifacts/` is ignored. Keep stable machine inputs and caches under `artifacts/image`, `artifacts/engine`, `artifacts/avd`, and `third_party/.cache`; test image snapshots belong to the task that consumes them. `artifacts/signing` holds local signing material, `artifacts/delivery` holds published local packages, and `artifacts/checkpoints` holds frozen source/delivery snapshots. Do not add any of those generated or private files to Git.

New coordinated work uses `artifacts/workflow/` for task records, worktrees, runs and handoffs. A run is immutable evidence once finished. Its temporary build output can be removed with its worktree, but its log, source fingerprint, copied reports and frozen APK pair remain in the shared run directory. Large image and toolchain caches may be reused; a cache is not acceptance evidence.

The 1.0.0 implementation work directories were relocated to `artifacts/archive/implementation-2026-10-03/`. Original reports and retired source are cataloged under `docs/archive/`. The [relocation manifest](../archive/relocations-2026-10-03.json) maps old paths to their retained locations. Historical prose remains in its original language and is not silently updated to describe today's implementation. The original brief is preserved at `artifacts/archive/implementation-2026-10-03/local-original-brief/initial/prompt.md` when local history is available.

## Retiring material

Before archiving a directory, search active scripts, source and maintained documents for references to it. Move required fixtures into an explicitly named active fixture location first. Preserve historical content and licenses, record the move, and update maintained references. Do not leave many compatibility symlinks at the root or make production builds depend on an archive.

An old report may contain commands that only work in its recorded checkout. Keep those commands as evidence rather than converting the report into a second maintained guide. New instructions belong in this development section; an English archive catalog explains where to find the original. Avoid copying credentials, workspace data or transcript contents into a public source archive.
