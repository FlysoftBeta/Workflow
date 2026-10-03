# Editor repair — Rust workspace rewrite

The Sora controller remains a disposable client projection. It reads the buffer from `WorkspaceStore` and submits `PanelView` through the existing layout command; the Rust Engine owns all durable reading position and draft state.

## Changes

- Subscribe to Sora scroll events as well as selection changes. Wait for a fling to finish before the debounced view update; flush the current view on focus loss, composition removal and controller disposal.
- Restore the saved logical line and wrapped column only after Sora has measured and completed its asynchronous layout. Preserve the partial row and horizontal offset; clamp stale positions to the current buffer and viewport. A later explicit navigation request takes precedence over the saved reading position, independently of the caret.
- Keep the reading position when a height reduction occurs with the caret off screen. The Sora adapter still reveals a previously visible caret on shrink, preserving normal IME editing behavior. This prevents delayed IME or workbench sizing from snapping a scrolled reader back to the old caret.
- Apply the current shell theme immediately before constructing the first editor scheme, after grammar/theme loading. This removes the first-open light-shell/dark-editor race caused by the dark theme being registered last. Existing editors still follow theme changes.
- Rewrite the Markdown strikethrough boundary assertion into a fixed-length lookbehind and backreference lookahead accepted by Sora's Joni engine. Keep all delimiter/content captures and the underscore/word boundary. The vendoring script applies this patch reproducibly; the manifest records both upstream and packaged hashes.

## Verification

Evidence is saved under `artifacts/editor-repair/`. All builds and APK snapshot copies use `artifacts/.gradle.lock`; device runs use only `tools/with-emulator.sh` with port 5892 and its 1536 MiB, disk-backed temporary storage defaults. No daily device was used.

- Host regression suite: all 3 tests passed (`grammar-tests.xml` / `build-v3.log`): all 169 static Markdown expressions compile in Sora's actual Joni implementation; strikethrough captures and boundary behavior remain correct; the packaged asset matches its patched manifest.
- Exact vendoring roundtrip: reversing the one local assertion patch reproduces the pinned upstream SHA-256; running the vendoring patch reproduces the packaged asset byte for byte.
- Initial API 28 run stopped before opening an editor because the client rejected Engine `updateConfig` conflict values. This was reported to and repaired by the client-boundary owner; the failure remains in `run-v1/editor.log`.
- A second build encountered independently owned connection/settings compile errors; the owning agents repaired these. `build-v2.log` preserves the diagnostics.

- Final Debug app and instrumentation APK snapshot: `apk-v9/`; build passed in `build-v9.log`.
- Final real Engine / native Sora API 28 x86_64 run: **1 integrated test passed**, 12.674 seconds (`run-v9/editor.log`; wrapper exited 0). It checks initial light color, a scroll-only read at logical line 20 / wrapped column 344 while the caret stays at 0:0, Engine PanelView persistence, session/controller recreation, retained dark theme, horizontal offset 240 restoration, immediate focus-loss flush, explicit navigation, and both offscreen-caret and visible-caret resize behavior.
- The final runner rejects Joni/TMException diagnostics; no Markdown regex failure was present. Initial light, restored light and retained dark screenshots are in `run-v9/screenshots/editor-qa/`; the restored screenshots capture the intentionally reduced editor viewport while the IME is visible. Colors and the restored logical line were visually checked.

The three requested defects are resolved. Acceptance here covers the API 28 x86_64 emulator with the packaged Rust Engine; it does not claim physical ARM64 device testing. No connection/server/workbench files or durable client workspace storage were added or changed by this editor workstream.
