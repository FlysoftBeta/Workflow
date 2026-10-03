# Repository layout and retention

The repository root contains build entry points, the working agreement and source modules. `engine/` is one Cargo workspace: `server` owns protocol and composition; `workspace`, `filework`, `terminal` and `chat` own their domains; `environment` supplies typed private storage, configuration/services and the runtime API. The isolation runtime and freestanding loader are nested under `engine/environment/runtime` and `engine/environment/loader`. The Rust Chat port remains partial, so the production Kotlin `:agent` adapters and `:engine-chat` service are explicit temporary exceptions until parity and isolated guest/device acceptance pass.

All client source lives under `app/`. `app/android` (`:app:android`) contains the Android shell, Compose features and platform adapters; `app/client` (`:app:client`) contains pure JVM protocol, connection, projection and synchronization code; `app/proxy` (`:app:proxy`) contains the pure JVM local proxy core. The former `core` and `agent-model` modules are merged into the client module while their Kotlin package identities remain stable. Temporary JVM Engine modules may use these definitions, but Android production does not depend on vendor execution.

App native guardian and instrumentation-only PTY support live in `app/native`, and offline xterm sources/tests in `app/web`. Image construction stays in `image/`; `engine/tools` packages the verified guest payload, including the retained JRE/JAR until Chat cutover. `third_party/` holds exact manifests and licenses. Generated tool archives, binaries and JARs belong to ignored build directories, never checked-in app assets. Module READMEs explain internal boundaries.

Documentation mirrors this ownership. [Product](../product/README.md) defines supported behavior and [UX](../ux/README.md) defines presentation. [Engine](../engine/README.md) has one page per crate plus the protocol and image format; [App](../app/README.md) covers connection, configuration synchronization, Workbench and proxy. This development section describes proposals, builds, coordination and verification. [Status](../status.md) identifies actual completion and limits, [reports](../report/README.md) preserve change-specific evidence, and [archives](../archive/README.md) retain historical originals.

The former `docs/implementation/` tree and top-level navigation stubs are retired. Their [relocation manifest](../archive/module-reorganization-map.json) records maintained destinations and byte-preserved originals. Current inbound links use the new references; historical archived text keeps its recorded paths and meaning. Runtime tree extraction is independently owned by the `runtime-dedup` task and is not part of this round's source relocation.

## Local material

`artifacts/` is ignored. Keep stable machine inputs and caches under `artifacts/image`, `artifacts/engine`, `artifacts/avd`, and `third_party/.cache`; test image snapshots belong to the task that consumes them. `artifacts/signing` holds local signing material, `artifacts/delivery` holds published local packages, and `artifacts/checkpoints` holds frozen source/delivery snapshots. Do not add any of those generated or private files to Git.

New coordinated work uses `artifacts/workflow/` for task records, worktrees, runs and handoffs. A run is immutable evidence once finished. Its temporary build output can be removed with its worktree, but its log, source fingerprint, copied reports and frozen APK pair remain in the shared run directory. Large image and toolchain caches may be reused; a cache is not acceptance evidence.

The 1.0.0 implementation work directories were relocated to `artifacts/archive/implementation-2026-10-03/`. Original reports and retired source are cataloged under `docs/archive/`. The [relocation manifest](../archive/relocations-2026-10-03.json) maps old paths to their retained locations. Historical prose remains in its original language and is not silently updated to describe today's implementation. The original brief is preserved at `artifacts/archive/implementation-2026-10-03/local-original-brief/initial/prompt.md` when local history is available.

## Retiring material

Before archiving a directory, search active scripts, source and maintained documents for references to it. Move required fixtures into an explicitly named active fixture location first. Preserve historical content and licenses, record the move, and update maintained references. Do not leave many compatibility symlinks at the root or make production builds depend on an archive.

An old report may contain commands that only work in its recorded checkout. Keep those commands as evidence rather than converting the report into a second maintained guide. New instructions belong in this development section; an English archive catalog explains where to find the original. Avoid copying credentials, workspace data or transcript contents into a public source archive.
