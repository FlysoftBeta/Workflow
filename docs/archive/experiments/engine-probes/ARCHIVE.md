# Archived host engine probes

Archived on 2026-10-03 from `tools/engine-probes/`. The original `README.md`, `run.py`, and `ptrace_probe.c` remain byte-for-byte unchanged. No active build or test caller referenced this experiment at relocation.

The original runner assumes its old directory depth and writes to `artifacts/engine-probes/`. Its original README also refers to a retired design-document path. To reproduce this historical experiment, restore that original layout in an isolated checkout. Do not treat its host-only results as Android runtime acceptance.

Generated Python cache files were preserved separately in ignored `artifacts/archive/implementation-2026-10-03/tools-engine-probes-cache/`. Existing probe outputs moved from `artifacts/engine-probes/` to ignored `artifacts/archive/implementation-2026-10-03/engine-probes/`.
