# W6: Files — explorer, editor, image preview, terminal, importer (report)

Role: W6. Status: **done** (2026-09-29), with the gaps in §8. Built against the panel contract in [w5b-shell.md](w5b-shell.md) §2.

## 1. Verification

| What | Result |
| --- | --- |
| `:core:test` (new: `FileNamesTest`, `TrashTest`, `TerminalLinksTest`, `TerminalKeysTest` incl. scrollback, `OscScannerTest`, `LineDiffTest`) | green |
| `:app:testDebugUnitTest` (new: `ExplorerModelTest`) | green |
| `web`: `node --test *.test.mjs` (new `terminal-links.test.mjs`; `terminal-input.test.mjs` unchanged) | 36/36 |
| Instrumentation on `workflow-tablet-api28` (own instance, port 5810, `-read-only`, killed afterwards) | `AndroidShellBackendTest` 2 · `ImportServiceTest` 1 · `FilesFeatureTest` 5 · `TerminalFeatureTest` 4 · `ShellNavigationTest` 6 (regression) = **OK (18)** |
| `:app:lintDebug` | fails on one error in W7's `feature/chat/AccountPanes.kt`; nothing from W6 except one false positive (`MissingOnRenderProcessGone`, it is implemented) |
| Tablet | not used (brief: emulator only) |

Instrumentation tests never touch the app's real `.workspace`: explorer/editor tests use an in-memory store; terminal and PTY tests use a throwaway workspace under the cache dir.

What the instrumentation tests prove:
- **Explorer**: nested tree (children indented, expansion persisted in the session), `.workflow` never listed, dot files only with 显示隐藏文件; inline create with a refused invalid name ("不能包含 /"), rename (open editor follows), delete → `.workflow/trash` → Snackbar 撤销 restores; long-press drag of a file onto a folder → "移动到「dest」？" → moved.
- **Editor**: typing creates a store draft (dirty), disk untouched; external change → conflict bar, save refused, disk untouched; 对比 opens the diff panel; 保留我的 + save writes the draft; a clean buffer reloads silently; `openFile(path, cursor)` moves the cursor.
- **Terminal** (real JNI PTY): `./sub/x.txt:3:4` opens at line/col; after `cd sub` the relative `x.txt:2` resolves via `/proc/<pid>/cwd`; a missing path shows "找不到 …"; dropped files paste `'sub/a b.txt' sub/x.txt` (quoted, relative to cwd); extra keys: ^C interrupts `sleep`, latched Ctrl + typed `c` interrupts, ↑ recalls history; OSC title names the tab; leaving and re-entering the session gives a new controller attached to the same running process with its scrollback.
- **PTY backend**: tty, cwd, same uid, resize (`stty size`), Ctrl-C, split UTF-8, `/proc` cwd after `cd`, exit code, ordered rapid input, terminate reaps.
- **Importer**: FileProvider content URI imported twice + once more → `name.txt`, `name (1).txt`, `name (2).txt`; staging cleaned; unreadable URI → `Failed`.

Manual review on the emulator (screenshots `artifacts/ui/files/*.png`, dark and light, landscape 1177dp and portrait 736dp): tree with guides and draft dots (`04`, `07`, `20`); real `adb draganddrop` of a file onto a folder, rejected hover over its own folder, confirm dialog (`05`–`07`); JSON keys vs values distinct (`08`); terminal with JetBrains Mono, dashed links only for existing paths and URLs (`12`); tapping `docs/sub/data.json:3:5` opened the file at line 3 (`13`); IME: terminal fills its column, full extra-keys row above the IME (`12`, `33`); editor extra-keys row (Tab, arrows, brackets) (`13`); long-press selection with 复制 · 粘贴 · 全选 (`18`); row menu (`17`); ＋ menu (`22`); camera → `IMG_…jpg` in the selected folder (`27`–`29`); image preview (`30`); find bar (`31`); ended terminal after process death with 重启 / 关闭 (`14`).

Compactness against ui.md: one 36dp header per region (explorer header, tab rows); 32dp tree rows with 12dp indent; no panel title bars, status lines or helper text; extra-keys row 40dp only while the IME is up (or pinned).

## 2. `:core` additions

- `core.io.FileNames`: name validation (short user-facing reasons), sanitizing untrusted display names, `name (n).ext` collision variants (keeps `.tar.gz`), 255-byte truncation without splitting surrogates.
- `core.store.Trash` + `WorkspaceStore.trashPath / restoreFromTrash / purgeTrash / copyPath`. Entries live in `.workflow/trash/<time>-<id>/{meta.json,<name>}`; restore picks a free name if the path was taken meanwhile; `runMaintenance()` purges entries ≥ 7 days old (`TrashPolicy`). Trashing closes clean panels and keeps drafts (status DELETED), like `deletePath`.
- `core.terminal.TerminalBackend` port: `TerminalSpec(directory, rows, columns, argv?, env)`, `TerminalProcess(output: Flow<ByteArray>, write, resize, awaitExit, terminate, currentDirectory)`, `ShellPaths(workspaceRoot, home)` (shell paths ↔ workspace paths).
- `TerminalLinks` (parse `path[:line[:col]]` / `(line,col)` / `file://`, candidates relative to the shell cwd, `~`, git `a/`/`b/`; never app-internal paths), `ShellQuote` (POSIX quoting; pasted paths relative to the cwd when below it), `TerminalKeys` (xterm encoding incl. DECCKM variants, modifier parameters, F-keys, Ctrl/Alt on typed text), `TerminalScrollback` (bounded retained output, trimmed at line boundaries), `OscScanner` (OSC 0/2 titles, OSC 7 cwd, split across chunks), `LineDiff` (Myers, for the diff panel).

## 3. Platform

- **`platform/importer/ImportService`** (shared with chat attachments): `pick(ImportKind.CAMERA|GALLERY|FILES, targetDir, owner)` and `importUris(uris, targetDir)`, both returning `ImportResult.Imported(paths, failures) | Cancelled | Failed`; `orphaned` flow for results that arrive after process death (routed by `owner`, e.g. `"conversation:<id>"`). Launchers are registered in `MainActivity.onCreate` (`ImportService.get(this).attach(this)`); the pending pick is kept in the activity's saved state. Content is staged in `cacheDir/import-staging` off the main thread (a slow provider never holds the store's actor), then committed with `WorkspaceStore.importFile` under the first free name; camera shots go to `cacheDir/captures` (existing FileProvider path) and are named `IMG_yyyyMMdd_HHmmss.jpg`.
- **`platform/pty`**: `AndroidShellBackend(context, root)` implements the port: `/system/bin/sh -i` on the JNI PTY under the app uid, cwd and HOME = workspace, shared history in `.workflow/terminal/`, one `LocalRuntimeService` lease per process. `PtyTerminalProcess` frees the native handle only after exit + drain. The old `LocalRuntime.createTerminal` / `LocalTerminalSession` (which wrote `.state/runtime`) are gone; `LocalRuntime` remains only as the workspace locator `ProxyService` uses.
- **`platform/workspace`**: `AndroidFileWatcher` now shares one `FileObserver` per directory (on API 28 a second observer of the same directory silently replaced the first, and stopping either removed the kernel watch for both — the explorer and the store watch the same folders). `WorkspaceIntents` (open with / share / copy path), `WorkspaceLocation`.

## 4. Features

- **Explorer** (`feature.files`): lazy tree (root + expanded folders, each watched and reloaded off the main thread), expansion/selection in the session's `ExplorerView`, follows the active editor (expands ancestors, selects, scrolls), `.workflow` never listed, dot files behind 显示隐藏文件. Inline create/rename (28dp field, Enter/✓, Back cancels, red border + reason), delete → trash + Snackbar 撤销 (no dialog, ui.md §8-5), 复制 (duplicate as `name (1)`), 移动到… (folder picker), DnD rows onto folders (primaryContainer highlight, 600ms hover expands, "移动到「y」？"), drops from other apps import into the folder, ＋ (新建文件 · 新建文件夹 ┆ 拍照 · 相册 · 设备文件), More ① (筛选… · 全部折叠 · 刷新 · ☐显示隐藏文件 · 在终端中打开), long-press menu per ui.md §4.2, empty state "空文件夹" [新建] [上传]. Rows are `FilesDragPayload` sources (editor stacks open them, the terminal pastes, the composer attaches).
- **Editor** (`feature.editor`): sora with the pinned TextMate grammars, JetBrains Mono, design-system chrome colors; the TextMate themes were rewritten (JSON keys and values now differ; Light+/Dark+-like palette). Bound to the Working Resource: debounced `editFile` (150ms), `saveFile` never overwrites (edits during a save are replayed on the new base), clean buffers follow disk silently, conflict bar 保留我的 / 使用磁盘版本 / 对比, deleted-on-disk bar. Surfaced 保存 · 撤销 · 重做 · 查找 · 附加到对话; find/replace bar (Aa, .*, n/m); More ① (复制路径 · 复制相对路径 · 在文件树中显示 · 附加到对话 · 在终端中打开所在目录 · 用其他应用打开 · ☐自动换行 · 放弃更改); editor extra-keys row while focused with the IME (hidden while typing in the find bar); `navigate(cursor)` queued until loaded; cursor and wrap restored from `PanelView`; pinch zoom writes `appearance.monoFontSize`. Binary / too large → "无法作为文本打开" [用其他应用打开]. **Image** preview: fit, pinch/pan, double tap 1:1, reload on disk change. **Diff** panel: read-only, side by side ≥ 720dp.
- **Terminal** (`feature.terminal`): `TerminalHost` (process-wide) owns sessions, scrollback, OSC titles and cwd; panels reattach and replay. Tabs "终端 n" / program title / renamed title; ＋ 新建终端; More ① 清屏 · 重命名 · 重启 · ☐常驻特殊键行 · 结束. Full extra-keys row (Esc Tab Ctrl Alt │ arrows │ Home End PgUp PgDn │ `| ~ / - _` │ Del ^C ^D ^Z │ F1–F12; latching Ctrl/Alt also apply to IME input; arrows repeat), ended state (dimmed, 已结束 [重启] [关闭]; an id whose process died with the app shows the same), drop → shell-quoted paths, long-press selection with 复制 · 粘贴 · 全选 (native WebView action mode suppressed), design-system ANSI palettes (light set ≥ 4.7:1). Terminals no live session references are ended after 15s.
- **Terminal page** (`assets/web/terminal.{html,js,css}`, `terminal-links.js`, `dom-polyfills.js`): JetBrains Mono via `@font-face` (CSP `font-src 'self'`), `android-input.js` untouched, touch links (dashed underline always, solid while pressed; URLs → browser; paths validated by the app before they are underlined), wrapped lines and CJK cells mapped correctly. **Found and fixed:** the pinned xterm 6.0.0 calls `Element.replaceChildren`, which the Android 9 system WebView (66) lacks — the terminal rendered nothing; `dom-polyfills.js` adds it. The web-links addon is no longer loaded (still vendored).

## 5. Notes for other workstreams

- **W7 (chat):** `feature/chat/Attachments.kt` still has its own `ContentUriImporter`. Please switch to `ImportService.get(context).importUris(uris, dir)` (map `ImportResult.Imported.paths`) and use `ImportService.pick(kind, dir, owner = "conversation:<id>")` for the composer ＋; collect `ImportService.orphaned` for your owners. `ImportService(context) { store }` is available for tests.
- **Engine:** implement `core.terminal.TerminalBackend` for the Debian guest (`ShellPaths("/workspace", "/home/work")`, `currentDirectory()` mapped to the guest) and construct `TerminalHost(backend, WorkspaceLocation.root(ctx))` with it in `TerminalHost.get`. `WorkspaceIntents.absolutePath` ("复制路径") then should use the same backend's `ShellPaths`.

## 6. Changes outside my packages

- `MainActivity`: one line attaching the importer.
- `app.panel.PanelWiring`: editor / terminal / explorer providers; the terminal and explorer placeholders were deleted from `app.panel.placeholder`.
- `platform/workspace/AndroidFileWatcher`: shared observers (API 28 bug, §3).
- `platform/pty/LocalRuntime`: slimmed to the workspace locator.
- `core.store.WorkspaceStore`: trash/copy commands, purge in `runMaintenance()` (additive).

## 7. Files

`core/…/io/FileNames.kt`, `core/…/store/Trash.kt`, `core/…/terminal/{TerminalBackend,TerminalLinks,TerminalKeys,TerminalScrollback,OscScanner}.kt`, `core/…/resource/LineDiff.kt`; `app/…/platform/importer/ImportService.kt`, `app/…/platform/pty/{AndroidShellBackend,PtyTerminalProcess,LocalRuntime}.kt`, `app/…/platform/workspace/{AndroidFileWatcher,WorkspaceIntents,WorkspaceLocation}.kt`; `app/…/feature/files/{FilesExplorer,ExplorerTree,ExplorerModel,MoveDialog}.kt`; `app/…/feature/editor/{EditorPanelProvider,TextEditorController,FindBar,EditorScheme,ImagePanel,DiffPanel}.kt`; `app/…/feature/terminal/{TerminalHost,TerminalPanel,TerminalWebView,TerminalTheme}.kt`; `app/src/main/assets/web/{terminal.html,terminal.js,terminal.css,terminal-links.js,dom-polyfills.js}`, `assets/textmate/theme-{light,dark}.json`; tests as listed in §1.

## 8. Known gaps

- **Tablet not verified** (Android 9 device, real IME incl. Chinese composition on the new page). The input adapter is unchanged and its tests pass, but the page around it changed (fonts, polyfill, touch handlers).
- **Terminal scrollback is not persisted**: after process death a terminal shows 已结束 and restarts empty.
- **Explorer "显示隐藏文件"** is per session in memory (not persisted; `ExplorerView` has no field for it).
- **Terminal pinch zoom** is not implemented (the editor's is); both read `appearance.monoFontSize`.
- **Selection** in the terminal: long press selects a word, dragging extends it; no draggable handles.
- **Link detection** underlines only paths that exist relative to the shell's *current* directory (like VS Code); output printed before a `cd` may lose its underline.
- **Gallery** uses `GET_CONTENT image/*` (no photo picker on API 28). The emulator's camera app loses the capture intent on its very first launch (its onboarding), a second try works.
- **Diff panel** is read-only; no inline merge.
- **More grammars** (shell, YAML, XML, C, …) are not bundled; only the six pinned ones.
- TalkBack custom actions for drag and drop are not added (every drag has a menu equivalent).
