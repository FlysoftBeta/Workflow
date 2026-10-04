# Verification and implementation reports

Use this directory for new, dated implementation reports, acceptance evidence, audits, and explicit test limitations. Keep current product and development guidance in the maintained documentation linked from the [repository README](../../README.md).

A useful report identifies the source revision, exact commands and environment, observed results, artifact locations, and untested cases. Compilation, a successful package build, and device acceptance are separate results. Do not commit credentials, private workspace data, transcripts, or large generated artifacts.

The [ABI build report](2026-10-03-abi-build-names.md) records architecture naming, package inspection and fresh-checkout verification. The [workspace organization report](2026-10-03-workspace-organization.md) records the documentation, module and coordination-tool changes and their checks. The [Rust chat cutover report](2026-10-04-rust-chat-cutover.md) records the host evidence for the in-process Rust chat and what remains for device acceptance. The [Kotlin chat deletion report](2026-10-04-kotlin-chat-deletion.md) records the removal of the JVM chat path, `:agent` and the JRE before that acceptance. The [Code Mode host and workspace cache report](2026-10-04-code-mode-host-and-workspace-cache.md) records the Codex Code Mode host in the tools payload, the editor highlighting investigation, and the `.workspace` layout with the declaration in `config.json` and rebuildable data in `cache/`. The [terminal readiness report](2026-10-04-terminal-before-environment.md) records opening a terminal panel before the environment is ready, with its host and isolated API 28 evidence.

The original 1.0.0 history is preserved without rewriting in [the implementation archive](../archive/implementation-1.0.0/README.md):

- [Initial implementation and legacy reports](../archive/implementation-1.0.0/reports/initial/), formerly `docs/report/initial/`.
- [Rust rewrite and integration reports](../archive/implementation-1.0.0/reports/rewrite/), formerly `docs/report/rewrite/`.
- [Original report catalog](../archive/implementation-1.0.0/reports/index.original.md).

Those records describe historical source and acceptance runs. Their original language, embedded paths, hashes, and commands are retained as evidence; they do not imply that later source has passed the same checks. The [relocation manifest](../archive/relocations-2026-10-03.json) records the exact moves.
