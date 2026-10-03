# Historical archive

This directory preserves original implementation history, retired source, reports, and licenses. It is not a source of current product behavior. Start at the [repository README](../../README.md) for maintained documentation.

Historical originals retain their original language, bytes, commands, links, and evidence. Relative paths and references inside them describe their original locations; they are not maintained instructions. Use the [relocation manifest](relocations-2026-10-03.json) to find moved material. To reproduce a historical experiment, restore its recorded layout in an isolated checkout; do not add archived implementations to production builds.

| Archive | Original location and purpose |
| --- | --- |
| [Initial design documents](initial/) | Existing frozen documentation from before the Rust Workspace Engine rewrite. |
| [Native C engine](native-engine/ARCHIVE.md) | Existing frozen C runtime, loader, syscall generator, harness, and upstream licenses. Production code is in `engine/runtime` and `engine/loader`. |
| [Web chat](web-chat/README.md) | Existing frozen WebView chat and Markdown sources, generated assets, lockfile, and upstream licenses. Current chat uses native Compose. |
| [Legacy proxy examples](legacy-runtime/proxy/) | Existing retired proxy configuration examples with no production callers. |
| [Android file watcher](android-workspace/AndroidFileWatcher.kt) | Existing retired client-side watcher. Android reference-store test adapters remain under `app/src/androidTest`. |
| [Implementation 1.0.0 reports](implementation-1.0.0/README.md) | Original `docs/report/initial/` and `docs/report/rewrite/`, plus their former catalog and the former archive catalog. |
| [Retired experiments](experiments/README.md) | Original diagnostic source retired from `tools/`; kept with its original documentation. |

New verification records belong in [docs/report](../report/README.md). Historical reports only substantiate the source and test matrix recorded at their own dates.

Local build output, research downloads, temporary workspaces, old prototype artifacts, and the original brief are kept separately in ignored `artifacts/archive/implementation-2026-10-03/`. Its `README.md` and `relocations.json` index those opaque local moves. They are not public release artifacts. No local history or credentials are copied into this source archive.

The initial delivery checkpoint remains at `artifacts/checkpoints/initial-1.0.0/`; signed delivery and signing material remain under their existing `artifacts/delivery/` and `artifacts/signing/` roots. Active runtime and image inputs, emulator storage, workflow state, and build/device locks remain under the retained roots listed in the manifest. The active ARM64 device-test fixture moved from `artifacts/image-research/debian-13-slim-arm64` to `artifacts/engine/fixtures/debian-13-slim-arm64`.
