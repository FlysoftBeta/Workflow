# Offline terminal adapter

`src/` is the maintained source for the terminal page, touch controls, link scanner and Android input adapter. The page uses the existing pinned xterm.js renderer and fit addon. Android supplies navigation and clipboard UI; workspace file candidates are resolved by the Engine. Chat uses native Compose/CommonMark/JLaTeXMath and is not part of this package.

`package.json` and `package-lock.json` pin the npm inputs. `vendor.mjs` copies the page sources and vendors xterm, its addons and the core-js compatibility layer, retaining upstream licenses and SHA-256 manifests. Vendor JavaScript is transpiled to Chromium 66. Do not edit generated Android asset copies or vendor bundles.

From this directory:

```sh
npm ci --ignore-scripts
npm test
npm run build
```

`npm test` first packages this checkout into ignored `build/test-assets/`. Tests execute those exact sources with the actual packaged xterm bundle; another checkout's Android assets are never used as the test implementation. `npm run build` writes the Android asset directory. Its default destination is `../android/src/main/assets/web`. An explicit destination is also supported:

```sh
npm run build -- --output-dir ../android/src/main/assets/web
```

The package does not copy fonts: the Android module already owns the offline fonts referenced at `../fonts/` relative to its web assets. Installation caches and test packages remain ignored.

## Interaction and bridge contract

The bridge remains `window.Workflow`: `ready()`, `input(text)`, `key(text)`, `resize(columns, rows)`, `openUrl(url)`, `openPath(candidate)`, `checkLinks(id, jsonCandidates)` and `selection(text, x, y)`. Missing or detached methods do not break rendering or input. Only HTTP(S) candidates open through `openUrl`. File URI, Unicode and diagnostic line/column text remain raw candidates until Engine validation; the web adapter performs no filesystem resolution.

Android answers `checkLinks` through `WorkflowTerminal.linksChecked(id, booleanArray)`, in the original candidate order. Batches contain at most 128 distinct candidates. Android must resolve against the current terminal identity and generation, and reject results when that identity changes. Call `WorkflowTerminal.invalidateLinks()` whenever cwd or generation changes. It retires all pending request IDs and cached results, so an old callback cannot re-enable a path. Link hits are refreshed at both press and release; scrolling, cancellation and a changed target cancel activation. Touch-generated mouse clicks cannot activate a second time.

The scrollbar has a 44px touch lane and a thumb at least 48px tall. Pointer capture and document-level fallbacks retain dragging outside the lane. A drag freezes its row-to-track mapping until release, so streamed output cannot shift the target under the finger. xterm owns the buffer and normal streaming viewport anchor. Arrow keys, Page Up/Down and Home/End work on the labelled scrollbar.

Long press selects a word using xterm cells, including wide, surrogate and combining characters across wrapped rows. Two 44px handles adjust endpoints independently; edge dragging scrolls the viewport. Handle targets separate near corners. Arrow keys adjust a focused handle, and Escape clears selection. Copy text always comes from xterm's selection API. A public xterm row marker retires the selection when its oldest selected row expires, avoiding a retained selection pointing at newer text.

`selection(text, x, y)` supplies the current copy text and toolbar anchor in CSS pixels. A handle drag begins with `selection('', 0, 0)` to hide the toolbar while keeping the xterm selection, then reports the final text on release. Android must not clear the terminal selection merely because this callback hides the toolbar, and its toolbar must allow touches to reach the handles. Copy/Paste actions can call `clearSelection()` explicitly. `selectAll()` selects the current buffer extent; it does not continuously extend to new rows.

`WorkflowTerminal.reset(text)` starts a new Engine generation or retained output window. It cancels gestures, markers, cached checks and selection, and resets the viewport. Writes and resets are serialized so a pending old write cannot appear after reset. Navigation/selection are suspended while that reset drains. A resize or alternate-buffer switch clears selection and active gestures whose geometry is no longer valid, then rebuilds current link geometry. No second scrollback database or PTY backend is introduced.

## Verification limits

The Node suites exercise the Android IME adapter, candidate parsing, offline packaging and hashes, and complete adapter gestures with the actual xterm parser/buffer/selection. They cover wrapped URL activation, stale validation, disabled bridges, two handles and copy, wide/combining text, touch and pointer cancellation, keyboard controls, streaming/trimmed viewport anchors, resize and reset races. Only browser layout/font measurement, canvas painting and fit dimensions are substituted in JSDOM.

These host checks do not establish Android WebView rendering, actual touch hit accuracy, native browser-intent handlers, clipboard integration, rotation, TalkBack or physical-device acceptance. Run the integrated `OfflineRendererTest` and terminal API 28 acceptance through the repository's isolated device workflow after packaging the Android assets. The user's daily tablet is not a test target.
