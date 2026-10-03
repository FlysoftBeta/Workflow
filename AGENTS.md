# Repository working agreement

## Read the right contract

The [product documents](docs/product/README.md) define supported behavior. [UX design](docs/ux/README.md) defines interaction and visual rules, while [implementation](docs/implementation/README.md) describes modules, mechanisms and ownership. Read [the protocol](docs/implementation/protocol.md) before changing a cross-component contract. The subsystem references cover the workspace, Server, container runtime, environment, chat adapters and local proxy.

Use [development guidance](docs/development/README.md) for proposals, coordination and verification. [Status](docs/status.md) records actual completion and limits. Update maintained documentation in the same change as behavior, using fluent English prose. Keep change-specific evidence in `docs/report/`; preserve historical originals and retired source under `docs/archive/`. Archived text retains its original language and is not current implementation guidance. The original brief and the relocation map are indexed in [repository layout](docs/development/repository-layout.md); its images are design inputs, not app assets.

## Preserve ownership

The Rust Workspace Engine owns sessions, panel layout, files, drafts, the complete environment and service state. Android is a shell that may durably cache only connection profiles and workspace-delivered local configuration. Workspace configuration and private state live at `<workspace-root>/.workspace/`, hidden from ordinary exploration; private state is masked inside the guest while explicitly editable configuration remains accessible. UI state is a discardable projection, and file IO never belongs on the main thread.

Session/layout state is separate from Working Resources. Closing a panel or switching a paradigm or session does not discard drafts. Preserve the tested archive-protection policy. Production Rust lives in `engine/server`, `engine/runtime`, and `engine/loader`; archived C code and Kotlin reference fixtures must not become production dependencies. `core`, `agent`, and `proxy` remain pure JVM modules, `agent` and `proxy` do not depend on each other, and feature packages do not import other feature packages.

Both ABIs ship customized images. Environment means the complete runtime, including its lifecycle and tools. Codex, Claude Code and terminal execution require that environment. Chat uses native Compose for streaming Markdown, formulae, tables and code; only the offline xterm adapter remains in WebView. Version remains 1.0.0, with no legacy migration, cross-version protocol adapter or host execution fallback. Remote/SSH bootstrap and transport have abstractions only; do not implement a remote connection without a new explicit request.

## Treat capabilities and data honestly

Never auto-approve an agent request. Preserve unknown protocol methods, notifications, fields and server request IDs, and always send Codex's explicit user-reviews-approvals setting. A switch, installed package or `su` binary does not prove that an operation succeeded. Capability UI and Engine milestones require actual checks and evidence on the documented matrix. Keep secrets out of logs, source, fixtures and backups.

Android 9 is a real target, with minSdk 28 and offline terminal assets. Use the existing Sora and xterm adapters. Pin dependency versions and prebuilt SHA-256 manifests, retain upstream licenses, and use stable releases unless a prerelease is required for Material 3 Expressive or Sora. Large generated output belongs in ignored artifacts, caches or build directories.

## Coordinate work and verify it

Use [the multi-agent workflow](docs/development/multi-agent.md). New subagents start with `fork_turns="none"`, an explicit owned path set, an independent checkout and an agreed delivery. Prefer one complete handoff to repeated progress conversations. After a usage-limit stop, resume contributors serially. Scope changes go through the coordinator before editing shared files.

`tools/workflow check` records independent source-bound results. Heavy Cargo and Gradle work shares the primary checkout's `artifacts/.gradle.lock`; Cargo uses at most two jobs. `tools/with-build-lock.sh` is the entry point for additional heavy commands. Build scripts already holding that lease pass `WORKFLOW_BUILD_LOCK_HELD=1` to nested builders. Linked task checkouts use the same lock identity rather than creating competing per-checkout locks. Lightweight native, web and image checks may run independently.

All AVD use goes through `tools/with-emulator.sh`, directly or through `tools/workflow device`. It owns one global device lease, uses 1536 MiB and disk-backed `ANDROID_TMP`, and cleans up only its own emulator. Never edit a running or queued script. Follow [testing guidance](docs/development/testing.md): compilation is not device acceptance, and a successful command against changed source is not a valid handoff.

The attached tablet is the user's daily device, with root and ClashMetaForAndroid TUN. Do not touch other apps, routes, DNS or system settings; do not uninstall the app or enter the lock-screen PIN. Destructive and root tests use isolated AVDs. Git integration preserves conflicts for deliberate resolution, and task archival retains committed work and evidence. Inspect the staged source before committing; local credentials and generated artifacts stay untracked.
