# Workbench UI integration acceptance

## Scope and changes

Owned source: `feature/workbench`, `feature/launcher`, `feature/sessions`, `ui/design`, and UI acceptance tests. No theme palette generation, dependency pins, client persistence, Engine protocol ownership, or daily-device settings changed.

- Added a 12dp outer band around the complete editor split tree. Its preview spans the whole tree and release submits one `Move(EditorEdge)` command. The inner content remains available to individual stack drop zones. Four matching menu destinations make all edges reachable without dragging.
- Split menu actions now submit a single `Move(Edge)` command, avoiding separate asynchronous Focus/Split commands. Tabs expose forward/backward sorting; Launcher tiles expose both menu entries and accessibility actions for reordering. Cancelled Launcher gestures no longer commit a reorder.
- Live region, split and bottom dividers expose bounded resize, collapse/expand and reset accessibility actions through existing layout callbacks. IME-forced geometry does not become persisted resize state.
- Region headers collapse lower-priority actions into More before the title loses its reserved width. Stack headers below 480dp replace the session-name chip with a compact History button, preserving readable file tabs. Session chips and tab-close targets use toolbar touch height; Launcher session targets use 48dp.
- Maximized editor overlays remain mounted. Its explorer/conversation toggles now open overlays instead of changing invisible docking state. Opening a file from a modal explorer dismisses that drawer; terminal creation and file delivery also dismiss it.
- Launcher config-write failures now surface a snackbar. Empty session searches and archived lists expose a clear-search/return action.

## Reproducible checks

All Gradle/APK-copy work used `flock artifacts/.gradle.lock`; each app/test pair was copied inside that same lock. All emulator work used `WORKFLOW_EMULATOR_PORT=5890 tools/with-emulator.sh`, API 28, 1536 MiB, two cores, disk `ANDROID_TMP`. Only this disposable emulator was installed/reset/resized. No real tablet was accessed.

- `:app:compileEmulatorDebugKotlin` and `:app:testEmulatorDebugUnitTest --tests 'top.flysoftbeta.workflow.ui.design.*'`: **24 tests passed**, including the perimeter-vs-child drop/one-commit regression. Evidence: `artifacts/workbench-qa/{compile-ui,build-v2,build-v3}.log` and app JUnit XMLs.
- `WorkbenchEngineAcceptanceTest` launches the actual `MainActivity`, selects its real Embedded connection, and uses `RemoteWorkspaceStore` plus the APK's packaged Rust Engine. It retains the inline-create acceptance route as the default. Diagnostic `workbenchSeedViaRpc=true` prepares input files separately so one upload blocker does not prevent unrelated UI acceptance; these runs do not count as inline-create acceptance.
- `WorkbenchControlsTest` is an isolated actual Compose test of live geometry callbacks and the narrow 1.3× header; it does not initialize a fake workspace or an Engine.

## Findings and current evidence

The initial actual-app run reached connection selection and Launcher, then exposed a client boundary error: `RemoteWorkspaceStore.importFile` calls `files.upload.begin` with only `path`, omitting required `size`. The server rejects it with `WorkspaceRpcException: size must be nonnegative integer`. The same function is used by `createFile`. This was reported to root integration with `run-v1/1080-306.log` and `logcat.txt`; it is outside the owned UI source.

The second run used diagnostic fixture seeding. It verified actual Sora edit/save, a real external file change and explicit keep-mine conflict resolution, archive decision/keep-drafts/restore, Activity recreation with cursor and draft recovery, tab menu reordering, touch tab split and valid server layout, and the maximized explorer overlay. The suite stopped at trash/undo and is not reported as passing. Evidence and visually inspected screenshots: `artifacts/workbench-qa/run-v2/`.

Additional editor issues reported to root integration: the Sora controller subscribes to selection changes but not scroll-only changes, and it never reads `initialView.scrollAnchor`; therefore cursor restoration is demonstrated but scroll-only restoration is not. Initial editor coloring appears dark in a light shell until recreation. Markdown grammar emits Joni variable-length-lookbehind errors on API 28. These are outside the owned workbench files.

## Completed matrix (diagnostic file setup)

The v3 APK run passed all three actual-app flows: **1920×1080 @306dpi**, **1920×1200 @261dpi**, and **800×1280 @320dpi (400dp narrow)**. In each, the test exercised the actual IME visibility inset, editor special-key row, conflict resolution, archive decision/restore, Activity recreation cursor and draft, reorder menu/touch split, maximized explorer, trash/undo, settings/proxy, native chat startup state and file attachment delivery. It used `saveFile` only as an explicitly logged fixture fallback after failed createFile, so inline creation is still excluded. Logs: `run-v3/{1080-306,1200-261,portrait-400}.log`, all **OK (1 test)**.

**5/5 WorkbenchControlsTest + 3/3 NativeTranscriptTest passed** in the same production app/test APK, `run-v3/controls-native.log`. Controls cover actual bounded resizing and collapse callbacks, IME state, and 320dp/1.3× header overflow. Native fixtures cover formulae, Markdown, tables, code copy, light/dark/narrow surfaces, 81 turns, stream anchoring and exact key/pixel restoration. Native screenshots were visually inspected in `run-v3/screenshots/native-chat/`; no authenticated Codex/Claude model response is claimed.

The file-setup diagnostics identified the second backend blocker: Engine `workspace.command createFile` returns `Permission denied (os error 13)` on the real app UID. `storage::create_atomic` publishes with `fs::hard_link`, which the Android app sandbox denies; atomic replacement in saveFile succeeds. Root integration received the exact result and source line. The v2 trash failure was consequently a missing test input file; v3 validates fixture creation and real trash/undo passes.

UIAutomation's full-display screenshot API crops the wrong framebuffer coordinates after API 28 `wm size` overrides; overridden-size v3 screenshots therefore are not accepted as visual layout evidence. The v4 repeat uses Compose PixelCopy to verify the main-window layouts; all three flows passed again. Main-window screenshots at all three sizes were visually inspected. Dialog capture remains a separate window limitation: the API 28 capture helper can return the underlying activity crop for dialog roots, so dialog images alone are not accepted as layout proof. Semantics assertions and the actual archive action passed. Natural 1920×1200 native samples are unaffected.

The corrected 400dp header was retested in v5, **OK (1 test)**, with visible selected tab and accessible History/stack/menu controls (`run-v5/screenshots/workbench-qa/portrait-400-split.png`). Latest production Kotlin and all design tests compiled/passed in `build-v5.log`; the source includes an internal-metadata exclusion assertion.

Terminal in v5 reached real Engine `Running`, rendered the xterm prompt `work@localhost:/workspace`, and screenshot capture waited for the real page's ready bridge. The first raw-output assertion was timing-sensitive to the asynchronous xterm paste bridge. The corrected v6 assertion checks the actual workspace output file: **OK (1 test)**. `printf '%s'` received the menu-delivered shell-quoted filename containing spaces and wrote exactly `path with spaces.txt` into `qa-terminal-path-output.txt`, read back through Engine `openFile`. Actual xterm screenshot `run-v6/screenshots/workbench-qa/terminal-terminal-path.png` was visually inspected. This uses the real customized environment and real Engine PTY, with no host backend.

Also reported to root: agent startup emits `CODEX_HOME points to /home/work/.codex, but that path does not exist`, which causes the rendered Codex startup failure independently of account authentication.

v3 immutable APK SHA-256: app `f1079bf5fe8882d38ed8085c6994eea8c10c16edcb72e34df576f24a2bb321ad`; test `1a19c6e9b6a857147239cd05dd0710bac00af4ed87d1cee43f20ec24e8a8dc92`.

Latest UI source snapshot v5 APK SHA-256: app `90c5623a1092a021667551e1ff65d8e5fd9c4c5e561a7ba155c59e905eac5521`; test `e87203954a3431c764cc2c5a22e1f4efd637292f59c7d74dfa0f7fb6680b53cb`.

## Remaining integration acceptance

Root integration must repair upload-size and no-overwrite publication, then run `WorkbenchEngineAcceptanceTest` without `workbenchSeedViaRpc`. It also owns environment rebuild/lifecycle, Codex guest home initialization, editor scroll-only restoration/coloring/grammar, final release build, and authenticated model/device acceptance. The bounded UI scope does not claim those are complete. Existing reference-store fixtures are not evidence for the new connection boundary.

Final v6 test APK SHA-256: `8f4bd54232a08a127a4c04f960463f0585431a3d36bef1e11cca66ceae1cee02`; app APK is byte-identical to v5. `build-v6.log` confirms production/test compilation. All wrapper-managed emulators exited after their commands; no background QA process remains.

## Root integration follow-up

The originally reported upload size, Android no-overwrite publication, Codex guest home, initial editor theme, Markdown grammar and scroll-only persistence issues have been repaired. Root also fixed narrow first-use file creation, Ctrl+N revealing the correct file tree, and stale overlay flags after docking. The final default (no fixture seeding) flow passes all three sizes; final evidence and source/build matching are recorded in [integration.md](integration.md).
