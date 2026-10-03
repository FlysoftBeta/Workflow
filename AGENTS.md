# Repository working agreement

## Sources of truth

- Behaviour: `docs/product.md`. Visuals and layout: `docs/ui.md`. Modules and data ownership: `docs/architecture.md`.
- Cross-component contract: read `docs/protocol.md` before edits. Subsystems: `docs/workspace.md`, `docs/workspace-engine.md`, `docs/container-runtime.md`, `docs/environment.md`, `docs/agents.md`, `docs/proxy.md`.
- Verification: `docs/testing.md`. Actual completion and limits: `docs/status.md`.
- Update the relevant long-lived doc in the same change as behaviour. Progress logs, audits and acceptance evidence belong in `docs/report/`; obsolete designs and frozen source belong in `docs/archive/`.
- The original brief is `.prompt_tmp/initial/prompt.md` when present; its images are design inputs, not app assets. Initial continuation is frozen in `artifacts/checkpoints/initial-1.0.0/`.

## Ownership and boundaries

- Rust Workspace Engine owns sessions, layout, files, drafts, environment and service state. Android is a shell: only connection profiles and workspace-delivered local configuration may be durably cached by the client.
- Workspace configuration and private state live at `<workspace-root>/.workspace/`, hidden from normal exploration and masked inside the environment. UI holds only discardable projections; no file IO on the main thread.
- Session/layout state is separate from Working Resources. Closing panels or switching paradigm/session never discards drafts. Keep the tested archive-protection policy.
- `engine/server`, `engine/runtime`, `engine/loader` are the production Rust implementations. Archived C code and Kotlin test fixtures are not production dependencies.
- `:core`, `:agent`, `:proxy` are pure JVM and cannot reference Android. `:agent` and `:proxy` never depend on each other; `feature.*` packages never import each other.
- Environment means the complete runtime. Both ABIs ship customized images, never vanilla Debian. Chat uses native Compose for streaming Markdown, formulae, tables and code; only xterm remains in WebView.
- Version is 1.0.0: no legacy migration, cross-version protocol adapter or host agent/terminal fallback. Only abstract remote/SSH bootstrap/transport interfaces; do not implement remote connections.

## Safety invariants

- Never auto-approve agent requests. Preserve unknown protocol methods, notifications, fields and server request IDs. Always send Codex's explicit user-reviews-approvals setting.
- Capability UI reflects real checks: a switch, installed package or `su` binary does not prove success. Engine milestones require test evidence on the documented matrix.
- No secrets in logs, source, fixtures or backups.

## Platform and dependencies

- minSdk 28; Android 9 WebView is a real terminal target. Offline assets only, no CDN. Use existing Sora and xterm adapters.
- Exact dependency versions; stable releases unless a prerelease is required (Material 3 Expressive, Sora). Prebuilt binaries are pinned by sha256 in `third_party/*/manifest.json`, fetched into an ignored cache; retain upstream licenses.
- Large generated artifacts belong in ignored `artifacts/`, `third_party/.cache/` or `build/`.

## Working practice

- All heavy Cargo/Gradle builds share `flock artifacts/.gradle.lock`; Cargo jobs <= 2. Build scripts already holding that lock pass `WORKFLOW_BUILD_LOCK_HELD=1` to nested Rust builders. Non-heavy native, web and image checks may run in parallel.
- Follow `docs/testing.md`; compilation is not device acceptance. AVDs only through `tools/with-emulator.sh` (one device lock, 1536 MiB, disk ANDROID_TMP). Never hot-edit running scripts.
- The attached tablet is the user's daily device (root, ClashMetaForAndroid TUN): never touch other apps, routes, DNS or system settings, uninstall the app, or enter the lock-screen PIN. Destructive/root tests use isolated AVDs.
- New subagents MUST use `fork_turns="none"`, bounded explicit file ownership and independent delivery. After usage-limit stops, resume serially.
