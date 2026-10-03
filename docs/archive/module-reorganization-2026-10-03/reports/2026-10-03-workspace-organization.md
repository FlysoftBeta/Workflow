# Workspace organization and collaboration workflow

This change separates maintained source and guidance from the implementation history of Workflow 1.0.0. Product behavior, the protocol version, and the two supported APK architectures are unchanged. Historical originals retain their original language and bytes; maintained documentation is now English.

## Retained history and a readable source tree

The [relocation manifest](../archive/relocations-2026-10-03.json) records 129 moves covering 113,779 regular files and 21,390,483,371 logical bytes. Historical reports now live under `docs/archive/implementation-1.0.0/reports/`, retired diagnostic probes under `docs/archive/experiments/`, and local implementation output under ignored `artifacts/archive/implementation-2026-10-03/`. The original brief and old prototype work directory are preserved there. Active images, runtime fixtures, AVD storage, delivery packages, checkpoints and signing material remain separate from that history.

Moves preserved inode, size, mode and modification metadata; no historical text was rewritten. The one active ARM64 fixture in the former image-research directory moved to `artifacts/engine/fixtures/debian-13-slim-arm64`, and its device harness reference was updated. Maintained source comments and documentation now point to the appropriate current contract or explicit archive.

Module READMEs explain source, tests, generated inputs and platform boundaries. Unused filesystem/trash writers moved from production `core` into its existing reference test fixtures, while production retains the protocol-facing models and hashing helpers. Proxy acceptance now injects a scoped panel provider instead of changing a process-global production service override. Rust Server framing, strict JSON, binary conversion and errors moved into `protocol.rs`; their external behavior is unchanged.

The documentation hierarchy now distinguishes [product](../product/README.md), [UX](../ux/README.md), [implementation](../implementation/README.md), and [development](../development/README.md). Development guidance includes proposals, task packets, review handoffs, resource scheduling and integration recovery. Old top-level subject paths are short English navigation pointers. `.gitattributes` preserves upstream licenses, frozen archives and byte-exact golden specimens rather than reformatting them for a whitespace check.

## Executable coordination

`tools/workflow` creates isolated Git worktrees and rejects overlapping source ownership. It records checks against source fingerprints, copies APK pairs under the build lease, validates their digests before device use, and records the actual instrumentation result rather than trusting adb's exit code. Image inputs are independent snapshots; machine-local configuration and verified prebuilt caches may be linked. Heavy builds and emulators use separate locks owned by the primary checkout across every linked worktree.

A handoff requires a scoped, committed, clean checkout with successful current checks. Review revisions preserve prior handoffs and Git references. Cancellation releases ownership without discarding dirty source. Integration leaves conflicts for explicit resolution or abort, and archival retains branches, summaries and evidence. The tooling does not launch models or automatically publish code. Contributors still receive bounded packets and new subagents start without a conversation fork.

The emulator wrapper now detects occupied console/adb ports, including offline devices, before launch. It owns an emulator process group, closes the device-lock descriptor in children and cleans up that group when the command ends. The driver is copied into each device run before it queues, so a later source edit cannot change the queued script.

## Verification performed

Module verification passed **225 core tests**, **21 Rust Server tests**, Server black-box JSONL/lifecycle replay, and production plus instrumentation Kotlin compilation. The production core JAR was inspected to confirm that the reference filesystem, watcher and store writers are absent. These results are recorded under `artifacts/organization/module-validation/`.

The coordination tests passed **17/17** using real temporary Git repositories and worktrees. They exercise overlapping and invalid scopes, cross-worktree lock identity, serialized heavy commands, source mutation invalidating a successful command, cancellation and child cleanup, immutable APK pairs and tamper rejection, independent image inputs, handoff validation, conflict recovery, review revisions, retained cancellation, and the one-emulator wrapper with SDK-free process fixtures. The fixture checks do not claim an Android device result. Output is retained in `artifacts/organization/infra-tests-final.log`.

The real runner then built and froze the app/test pair in `artifacts/workflow/runs/coordinator/20261003T061922Z-bed70589/`. Its shared-emulator device run, `20261003T062243Z-d1b4d92b`, executed **4/4 ProxyPanelEmulatorTest cases**, zero skips, on API 28 x86_64 with explicit emulator root. It exercised the scoped provider, real Rust workspace connection and actual proxy UI. Instrumentation took 47.296 seconds. Both runs passed their source/artifact checks, and the wrapper terminated its own emulator. No daily-device operation was performed.

Maintained English documentation and local links pass `tools/check-docs.py`; shell syntax and Python compilation checks pass. The staged source passes `git diff --cached --check`. Source review excludes APKs, signing keys, local history, IDE state and generated build directories from Git. This is organization and targeted regression acceptance, not a repeat of every historical release or physical-device matrix entry.

## Git history

The repository had no commits. To make this change reviewable, the verified 1.0.0 source archive was imported as baseline commit `ca9ac90caf597dbc96a64a745a9fc4fee3762dd2`, preserving its 1,443 source entries and archive digest `25f649201309caba9a9807206d73ef6e5cf29a620d07eb53cde06b1981f7989d`. The organization change is committed separately on top of that baseline. Local archives and signed delivery files remain ignored and are not part of either source commit.
