# Offline terminal adapter

`src/` is the maintained source for the terminal page, touch controls, link scanner and Android input adapter. The page uses the existing pinned xterm.js renderer and fit addon. Android supplies navigation, the clipboard and the selection chrome; workspace file candidates are resolved by the Engine. Chat uses native Compose/CommonMark/JLaTeXMath and is not part of this package.

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

The bridge remains `window.Workflow`: `ready()`, `input(text)`, `key(text)`, `resize(columns, rows)`, `openUrl(url)`, `openPath(candidate)`, `checkLinks(id, jsonCandidates)`, `selection(text, x, y)` and `selectionState(json)`. Missing or detached methods do not break rendering or input. Only HTTP(S) candidates open through `openUrl`. File URI, Unicode and diagnostic line/column text remain raw candidates until Engine validation; the web adapter performs no filesystem resolution.

Android answers `checkLinks` through `WorkflowTerminal.linksChecked(id, booleanArray)`, in the original candidate order. Batches contain at most 128 distinct candidates. Android must resolve against the current terminal identity and generation, and reject results when that identity changes. Call `WorkflowTerminal.invalidateLinks()` whenever cwd or generation changes. It retires all pending request IDs and cached results, so an old callback cannot re-enable a path.

The page owns every touch gesture (`touch-action: none`), and the Android host keeps ancestors from intercepting it. A press on a link is identified by its text and first cell, followed by a public row marker. Release activates it when the finger stayed within the tap slop and the link is still visible, even if streaming output moved the viewport. Scrolling, cancellation, a cwd/generation change, a column reflow or the link leaving the screen cancel activation. Touch-generated mouse clicks cannot activate a second time. A URL that fills its row to the last column continues with the next row's leading delimiter-free run, starting in column 0, even without a soft wrap; the continuation is not scanned again as a path. OSC 8 hyperlinks come from xterm's internal link provider, which also enforces the HTTP(S) scheme filter; their cells take precedence over plain-text candidates, and xterm's own `confirm()`/`window.open` activation is disabled. A provider shape change disables only OSC 8 targets.

A vertical swipe past the slop scrolls 1:1 with the finger and suppresses compatibility mouse events, so it never focuses xterm. On release, the velocity of the last 100 ms starts an exponentially decaying fling; any new touch, input, Extra Keys, reset or the scrollback end stops it, and the touch that stops it does nothing else. In the alternate screen, or under VT200/drag/any mouse tracking, rows become line-mode wheel events on xterm's screen, which xterm reports to the program or turns into cursor keys.

The scrollbar has a 44px touch lane and a thumb at least 48px tall. Pointer capture and document-level fallbacks retain dragging outside the lane; only the capturing control losing capture, `pointercancel` or release ends it. A drag freezes its row-to-track mapping so streamed output cannot shift the target under the finger. A resize remaps the track instead of ending the drag. xterm owns the buffer and normal streaming viewport anchor. Arrow keys, Page Up/Down and Home/End work on the labelled scrollbar.

Long press selects a word using xterm cells, including wide, surrogate and combining characters across wrapped rows, at whatever text is under the finger when it fires. `contextmenu` is always cancelled before xterm sees it: xterm's right-click handler would focus its hidden textarea, open the soft keyboard and lose the selection in the resize. Copy text always comes from xterm's selection API. A public xterm row marker retires the selection when its oldest selected row expires, avoiding a retained selection pointing at newer text.

When the host provides `selectionState`, it draws the selection chrome natively and the page's two handle buttons remain only as keyboard and accessibility targets; otherwise they are drawn in CSS. `selectionState(json)` carries `{active:false}` or `{active, toolbar, start, end, rect, line, width}`: endpoints are the bottom-left of their row in CSS pixels with a `visible` flag, `rect` is the visible selected area (null when offscreen) and `width` is the layout viewport width for scaling. It is sent only when it changes. The host forwards a native handle drag through `WorkflowTerminal.dragHandle(edge, phase, x, y)`, with `phase` `start`, `move`, `end` or `cancel` and the handle's hotspot in CSS pixels. Arrow keys adjust a focused handle button, and Escape clears selection. Edge dragging scrolls the viewport.

`selection(text, x, y)` supplies the settled copy text and toolbar anchor in CSS pixels. A handle drag or an extending long press begins with `selection('', 0, 0)` to hide the toolbar while keeping the xterm selection, then reports the final text on release, including after a cancelled drag. Android must not clear the terminal selection merely because this callback hides the toolbar. Copy/Paste actions can call `clearSelection()` explicitly. `selectAll()` selects the current buffer extent; it does not continuously extend to new rows.

`WorkflowTerminal.reset(text)` starts a new Engine generation or retained output window. It cancels gestures, flings, markers, cached checks and selection, and resets the viewport. Writes and resets are serialized so a pending old write cannot appear after reset. Navigation/selection are suspended while that reset drains. Only a column change reflows the buffer. A rows-only resize, such as the soft keyboard or extra-keys row appearing, keeps gestures, the pending tap and the selection, which is restored after xterm clears it. A column change or alternate-buffer switch clears selection and the gestures whose geometry is no longer valid. No second scrollback database or PTY backend is introduced.

## Verification limits

The Node suites exercise the Android IME adapter, candidate parsing, offline packaging and hashes, and complete adapter gestures with the actual xterm parser/buffer/selection. They cover swipe, fling and alternate-screen wheel input, link taps while streaming, hard-wrapped and OSC 8 links, stale validation, disabled bridges, `contextmenu` suppression, native chrome geometry and handle drags, two handles and copy, wide/combining text, touch and pointer cancellation, keyboard controls, streaming/trimmed viewport anchors, rows-only and reflowing resizes, and reset races. Only browser layout/font measurement, canvas painting and fit dimensions are substituted in JSDOM.

These host checks do not establish Android WebView rendering, actual touch hit accuracy, native browser-intent handlers, clipboard integration, rotation, TalkBack or physical-device acceptance. Run the integrated `OfflineRendererTest` and `TerminalTouchAcceptanceTest` through the repository's isolated device workflow after packaging the Android assets. The user's daily tablet is not a test target.
