# W5a design system (report)

Date: 2026-09-28. Role: W5a. It covers the Compose design system and the model-independent components under `top.flysoftbeta.workflow.ui.design` (plus `ui.design.dnd`) in `:app`.

**Status: done.** `flock artifacts/.gradle.lock ./gradlew :app:assembleDebug :app:assembleDebugAndroidTest :app:testDebugUnitTest` is green. 22 new JVM tests pass. The androidTest screenshot test passes on the tablet AVD. Old screens (`ui/*`, `ui/theme`) were not edited and still compile.

## 1. Files

| Area | Path |
| --- | --- |
| Theme and tokens | `app/src/main/java/top/flysoftbeta/workflow/ui/design/theme/`: `Tokens.kt`, `Type.kt`, `Colors.kt`, `GeneratedColors.kt` (generated), `WorkflowTheme.kt`, `SystemBars.kt` |
| Icons | `ui/design/icons/Symbols.kt` (generated), `FileTypeIcons.kt`; `res/drawable/sym_*.xml` (123 vectors) |
| Components | `ui/design/`: `IconButtons.kt`, `CompositeMenu.kt`, `ListMenu.kt`, `StackTabBar.kt`, `SplitPane.kt`, `Seams.kt`, `DockingGeometry.kt`, `DockingLayout.kt`, `ExtraKeys.kt`, `Timeline.kt`, `AppGrid.kt`, `ModelSliderGeometry.kt`, `ModelEffortSlider.kt`, `Attachments.kt`, `States.kt` |
| DnD | `ui/design/dnd/`: `DndGeometry.kt` (pure), `DragPayload.kt`, `DragDropState.kt`, `DragDropModifiers.kt`, `DragDropHost.kt` |
| Gallery (debug only) | `app/src/debug/AndroidManifest.xml`, `app/src/debug/java/.../ui/design/gallery/*` |
| Tests | `app/src/test/.../ui/design/dnd/DndGeometryTest.kt` (12), `app/src/test/.../ui/design/DesignLogicTest.kt` (10: slider, docking, extra keys, timeline), `app/src/androidTest/.../ui/design/DesignGalleryScreenshotTest.kt` |
| Generators | `tools/design/generate-colors.mjs` (+ `esm-extension-hook.mjs`, `package.json`, lock), `tools/design/generate-symbols.py` |
| Notices | `third_party/material-symbols/{manifest.json,LICENSE}`, `third_party/jetbrains-mono/{manifest.json,LICENSE}`. Both are shipped as `assets/notices/*` via `prebuiltNotices`; this is a 2-line change in `app/build.gradle.kts`. |
| Launcher icon | `res/drawable/ic_launcher_{foreground,monochrome,background}.xml`, `res/mipmap-anydpi/ic_launcher{,_round}.xml`, `res/mipmap-*/ic_launcher{,_round}.png` (the webp files were replaced), `res/drawable/ic_workflow_glyph.xml` |
| Font | `app/src/main/assets/fonts/JetBrainsMonoNL-{Regular,Bold}.ttf` (2.304, OFL-1.1, no-ligature cut) |

## 2. Theme and tokens

- **Theme entry point.** `WorkflowDesignTheme(themeMode: ThemeMode = System, density: UiDensity = Compact, content)` wraps `MaterialExpressiveTheme` with:
  - `motionScheme = MotionScheme.standard()`;
  - the generated color scheme;
  - M3 `Shapes` mapped onto xs/sm/md/lg/xl;
  - a typography mapped onto the ui.md scale, so stock M3 buttons, dialogs and fields come out compact.

  It also provides `LocalWorkflowDimens`, `LocalWorkflowTextStyles`, `LocalExtendedColors` and `LocalIsDarkTheme`. `LocalMinimumInteractiveComponentSize` is set to the touch token. Both parameters can change at runtime.
- **Accessors.** `WorkflowTheme.dimens`, `.text`, `.colors`, `.extendedColors`, `.isDark`, `.motion`.
- **Dimensions.** `WorkflowDimens.Compact` and `WorkflowDimens.Standard` hold every §1.1 token:
  - bar, iconButton/iconButtonTouch, icon/iconSmall, treeRow, listRow/listRowTwoLine, settingsRow, menuItem, extraKeysRow/extraKey, gap, padH, indent;
  - density-independent sizes: tab 72–220, dirty dot 8, controlMin 40, launcherTouchMin 48, divider 20/32/4, groupHeader 28.

  Rows use `heightIn(min = token)` so font scale 1.3 does not clip.
- **Radii.** `WorkflowRadii` / `WorkflowShapes`: xs 4, sm 8, md 12, lg 20, xl 24, full, and `tab` (top corners only).
- **Colors.** `tools/design/generate-colors.mjs` runs `@material/material-color-utilities` 0.4.0 `SchemeTonalSpot` with seed #286B57 and contrast 0, and writes every M3 role for light and dark to `GeneratedColors.kt`.
  - Success and warning come from the raw tonal palettes of #2E7D32 and #7D5800. They are deliberately not harmonized, so they stay distinct from the teal primary.
  - Usage helpers: `ColorScheme.frame` / `popover` / `codeBlock` / `selectedRow`.
- **Type.** `WorkflowTextStyles` provides titleMd, titleSm, body, label, labelActive, caption, micro, chat, chatH1–3, mono and monoBlock. Mono uses the bundled JetBrains Mono NL (`MonoFontAssets.REGULAR` / `BOLD`, the same asset paths the sora and xterm adapters should load).
- **System bars.** `WorkflowSystemBarsEffect()` enables edge-to-edge with frame-colored status and navigation bars; the icons follow the theme.
- **Icons.** `tools/design/generate-symbols.py` fetches Material Symbols Rounded (opsz 20, wght 400; fill 0, plus fill 1 for 21 active-state glyphs) from google/material-design-icons at pinned commit `bd8cb85b…`. It records a per-file SHA-256 in the manifest and fails if a cached file differs. It emits `Sym.<Name>` / `Sym.<Name>Fill` (`@DrawableRes`) and `Sym.all`. `SymbolIcon(icon, contentDescription, size, tint)` draws them. `FileTypeIcons.forName(name, isDirectory, expanded)` maps about 14 kinds.

## 3. Components (API)

| Component | Signature (abridged) | Notes |
| --- | --- | --- |
| Icon button | `WfIconButton(icon, contentDescription, onClick, enabled, checked: Boolean?, tint, badge: Color?, iconSize, onLongClick)` | Cell = touch token; ripple/indicator = visual token. Long press shows a PlainTooltip. A badge is a 6dp dot. |
| Composite menu | `CompositeMenu(expanded, onDismissRequest, groups: List<MenuGroup>, anchorPosition, offset, style = Segmented\|Dividers, showShortcuts = hardwareKeyboardAttached())`; `MenuEntry.Action/Toggle/Submenu`; `MoreMenuButton(groups)` | Expressive `DropdownMenuPopup` + `DropdownMenuGroup` segments, or a divider fallback. Own rows at `menuItem` height with an 18dp icon. Destructive items are error-colored. Submenus open to the side (`MenuAnchorPosition.End`). |
| List popup | `ItemListMenu(expanded, onDismissRequest, items: List<ListMenuItem>, selectedKey, onSelect)`, `SearchField(value, onValueChange)` | A search field appears above 8 items. Used for the tab overflow `⌄n` and the `[1/3 ▾]` stack switcher. |
| Tab bar | `StackTabBar(tabs: List<TabItem>, activeKey, onSelect, onClose, focused, actions: List<ToolAction>, moreGroups, tabMenuGroups, leading, trailing, dragDrop: StackTabDragDrop?, pulse: TabPulse?)` | Horizontal scroll with 16dp edge fades; the active tab auto-scrolls into view. `⌄n` counts hidden tabs. Surfaced actions collapse into the top of More when the tab area would drop below 120dp. The trailing slot shows a dirty dot or ×. The focused Stack gets a 2dp primary indicator. Tab role and "未保存" semantics. |
| Header, session chip | `RegionHeader(leading, title, actions, moreGroups, trailing)`, `SessionChip(label, onClick, onLongClick)` | Header rows are the same height as tab rows. The trailing cluster has a 1dp separator. |
| Split pane | `SplitPane(state: SplitPaneState, orientation, first, second, firstMin, secondMin, firstMax, secondMax, collapsible: Set<SplitSide>, collapsedSize, gap)`; `rememberSplitPaneState(fraction, defaultFraction, collapsed)` | Initial-pass seam gestures on a 20dp band only start after slop along the axis, so other touches reach the content. Pressing shows a 4×32 pill. Double tap restores the default. Dragging below 50% of min snaps collapsed with a CLOCK_TICK. `state.isDragging` lets WebView and sora debounce. A11y: setProgress plus custom actions 增大 / 减小 / 收起. |
| Docking | `DockingLayout(state: DockingState, side, center, aux, spec: DockingSpec)`; pure `resolveDocking(windowWidth, sideOpen, auxOpen, sideWidth, auxWidth, spec)` | Space-based docking (§3.2). Overlays use a drawer or a right sheet with scrim; Back and outside tap close them, and the content is kept via `movableContentOf`. Seams are draggable with collapse and double-tap. `RegionCard` is the surface/md card. |
| Extra keys | `ExtraKeysRow(state: ExtraKeysState, onKey: (ExtraKeyEvent) -> Unit, groups = ExtraKeyLayouts.Terminal\|Editor)`; `ExtraKeysState.consume()` for IME keys | Ctrl/Alt: tap = one-shot, double tap = locked (primary + underline), tap again = off. Repeat keys fire after 400ms, then every 50ms. KEYBOARD_TAP haptic. Keys never take focus. |
| Timeline | `TimelineAge.of(date, today)`, `TimelineEntry(time, resources, age, onClick, current, isFirst, isLast, onLongClick)`, `GroupHeader(label)`, `HairlineCaption(text)` | Opacity per age: 1 / .85 / .72 / .62, with the weight and the number of listed resources stepping down per age. |
| App grid | `AppGridCell(label, onClick, badge, icon)`, `BuiltInAppIcon(glyph)`, `AddAppCell(label, onClick)`, `ReorderableAppGrid(items, key, onMove, onLongPress, trailing, cell)` | 104dp columns (max 10), centered. A 300ms lift scales to 1.08 with a shadow; other tiles spring aside. Release in place calls `onLongPress` (menu); a move commits once. |
| Model slider | `ModelEffortSlider(models, selection, onSelectionChange)`, `ModelEffortPopover(expanded, onDismissRequest, models, selection, onSelectionChange, onOpenModelList, speedActive: Boolean?, onSpeedToggle)`, `ModelEffortPanel(...)`, `ModelChip(label, onClick)` | Segments separated by 6dp gaps, one detent per effort, and a 4×44 bar handle. CLOCK_TICK on each detent; the selection applies immediately. Labels collapse to the selected model when they don't fit. The popover is 320dp wide, xl, Highest, elevation 3. |
| Attachments | `AttachmentStrip(items: List<AttachmentItem>, onRemove, onRetry, onOpen)`; `AttachmentState.Ready/Uploading(progress)/Failed` | 56dp thumbnails, 40dp chips (name max 160dp), 20dp × badge, progress ring, error outline + ↻. |
| States | `EmptyState(actions)`, `InlineError(message, actions)`, `NoticeBar(message, tone, actions)`, `DelayedVisibility`, `RegionLoading`, `InlineLoading`, `WfSnackbarHost(state)`, `SnackbarHostState.showUndo(message)`, `StatusDot` | Loading indicators appear only after 300ms. The snackbar is at most 480dp wide and sits above the IME. |

## 4. Drag and drop (`ui.design.dnd`)

- **Host.** `DragDropHost(state: DragDropState, acceptExternal = { mimes -> … }) { … }` draws the shadow chip and the target feedback over its content, and Back cancels a drag.
  - Drops from other apps use Compose `dragAndDropTarget`: the MIME types are read on start, and the URIs on drop after `requestDragAndDropPermissions`. They arrive as `ExternalDragPayload` at the same targets.
  - Hovering on a target for 600ms fires its `onHoverActivate`.
- **Source.** `Modifier.dragSource(state, key, enabled, onLongPressWithoutMove, payload: () -> DragPayload?)`.
  - A 300ms press picks the item up (LONG_PRESS haptic, source at 40% alpha).
  - Release in place calls the menu callback.
  - After pickup the events are consumed in the Initial pass, so clickables and scroll containers don't fire.
  - A tap or scroll that happens before the pickup is left untouched.
- **Target.** `Modifier.dropTarget(state, key, kind, priority, accepts, onHoverActivate, onDrop: (DragPayload, DropResult) -> Unit)`. `onDrop` is the single commit (one `LayoutOp`).
  - Kinds: `Zones(allowEdges)`, `Sort(horizontal, items)`, `Area(Outline|Scrim|Self, label)`.
  - Among overlapping targets, the one with the highest priority wins, then the smallest.
  - A target that rejects the payload makes the chip show ⃠.
- **Other pieces.**
  - `Modifier.dragAutoScroll(state, horizontal, scrollBy)` scrolls within 32dp of the ends.
  - `DropResult(zone, insertionIndex, position)`.
  - Payloads: `PanelDragPayload`, `FilesDragPayload`, `AppDragPayload`, `ExternalDragPayload`.
- **Feedback.**
  - Placement preview: primary 12% fill + 2dp stroke with md corners, spring-animated between zones.
  - Insertion line: 2×28dp primary.
  - Composer: outline + 8% fill + "添加为附件".
  - Terminal: translucent layer + "粘贴路径".
  - Drag shadow: `surfaceContainerHighest`, sm corners, elevation 3, 32dp above the finger.
  - Cancel springs the shadow back to its origin.
- **Pure geometry (`DndGeometry`), unit-tested.**
  - `resolveZone`: centre = inner 50%; edges = outer 25%; corners go to the nearer normalized edge; ties go to horizontal.
  - `zonePreview`, `insertionIndex` (midpoints), `insertionIndicatorPosition`.
  - `isNoOpMove` / `targetIndexAfterRemoval`, `autoScrollSpeed`, `pickTarget`, `gridSlot`, `reorderedIndex`.
- **Tested with real touch on the AVD.** I used `adb shell input draganddrop`. Tab reorder committed `reorder prompt.md → A[2]`; tab → other stack's right edge committed `split … → B Right`; file → composer committed `attach product.md`. Each release committed exactly once (see the `real-dnd-*.png` screenshots).

## 5. Gallery and screenshot path

- **Gallery.** `DesignGalleryActivity` exists only in the debug source set and is not linked from the product UI. Start it with:

  ```
  adb shell am start -n top.flysoftbeta.workflow/.ui.design.gallery.DesignGalleryActivity \
      [--es page chrome|menus|content|layout|tokens] [--es theme light|dark] [--es density compact|standard] \
      [--es dnd zone|sort|area|scrim|rejected]
  ```

  The page switcher is itself a `StackTabBar`; its trailing buttons toggle theme and density at runtime. The `dnd` extra freezes a synthetic feedback state for screenshots.
- **Screenshot test.** `DesignGalleryScreenshotTest` (androidTest, Compose `captureToImage`) renders 4 pages × 2 themes × 2 densities and writes the PNGs to `/sdcard/Android/data/top.flysoftbeta.workflow/files/screenshots/`.
  - Run it without `connectedAndroidTest`, so nothing touches other devices: install both APKs, then `adb -s emulator-5800 shell am instrument -w -e class top.flysoftbeta.workflow.ui.design.DesignGalleryScreenshotTest top.flysoftbeta.workflow.test/androidx.test.runner.AndroidJUnitRunner`.
  - Result: OK (1 test). The 16 PNGs are in `artifacts/ui/androidTest/screenshots/`.
  - Popups (menus) aren't captured this way; those come from `screencap` below.
  - Roborazzi/Robolectric was not added: there are no pinned versions in the catalog, and SDK 37 is not supported there.
- **AVD `workflow-tablet-api28`.** Created under `ANDROID_AVD_HOME=artifacts/avd`: API 28 x86_64, 1920×1200 landscape, lcd density 261, 3 GB RAM. It booted headless on port 5800 in about 15s. I killed it with `emu kill` when done; no other emulator was touched.
- **Screenshots (`screencap`) in `artifacts/ui/gallery/`.**
  - `{chrome,menus,content,layout,tokens}-{light,dark}-{compact,standard}-261.png`;
  - `*-320.png` (after `wm density 320`, then reset);
  - `dnd-{zone,sort,area,scrim,rejected}-light-compact-261.png`;
  - `real-dnd-{1-reorder,2-split,3-attach}.png`.

### Critical review against ui.md and ref.png

- **Compactness.** At 261dpi compact, the Files-style Chrome page has exactly one 36dp row per region: `[⌂][◧][chip]…[+][⋯]`, `[tabs…][⌄7][💾][↶][↷][⌕][✦][⋯]`, `[title▾][⤢][⋯]│[◨][⚙]`. There is no app bar, rail or status line. The editor gets about 590dp of the 675dp height and the tree has 32dp rows. That is much denser than ref.png (56dp bar, 44dp panel headers, 48dp list rows).
- **Docking at 320dpi (960dp).** Side region 264 docked, aux as a 400dp overlay with scrim. This is exactly the §3.2 table. The center widths 537 / 497 / 684 / 460dp also match the table; they are checked in `DockingGeometryTest`.
- **Both themes** render with frame-colored system bars. The dark tokens page confirms all 123 glyphs render.
- **Fixes made during review.**
  - The active-tab indicator was a child `fillMaxWidth` box that widened tabs to 220dp; it is now drawn in `drawWithContent`.
  - The DnD area label overlapped the shadow chip; it moved to the top-centre of the area.
  - Two crashes when composing the slider inside the menu popup: `BoxWithConstraints` under an intrinsic measurement, and an unbounded-width `Layout`. Both are fixed.

## 6. Deviations from ui.md, and reasons

1. **Colors differ slightly from the §1.3 review table.** For example, primary is #156B54 instead of #286B57, and onPrimaryContainer is #00513E (tone 30, the current M3 spec) instead of #002117. ui.md says "以生成结果为准".
2. **`material-icons-extended` is still a dependency.** The old screens import `Icons.Outlined.*` (9 files), and I was told not to edit old screens. The new design system uses only `Sym`. Remove the dependency together with the old `ui/*`. Old `ui/theme` is also left as-is; no shim was needed because nothing new depends on it.
3. **Standard density icon buttons inside a 44dp bar** get a 48dp-wide × 44dp-high touch cell. The parent's height constraint wins; Compose cannot extend touch bounds without taking layout space.
4. **Menu rows are custom** (inside the Expressive `DropdownMenuGroup`). M3's expressive `DropdownMenuItem` enforces a 48dp minimum height, which breaks the 36dp `menuItem` token.
5. **Glyph size.** All glyphs use optical size 20, which suits compact 16–20dp icons. At 24dp (standard) they are slightly heavy. A second opsz 24 set would double the resources.
6. **ANSI 16-color terminal palettes are not generated.** They belong with the terminal adapter theme.
7. **Floating overlay, composer and approval cards** are model- or feature-specific and are not in W5a.
8. **Session and tab-overflow search** uses the plain `SearchField` rather than an M3 SearchBar, which is too tall.

## 7. Notes for the UI shell and feature workstreams

- Wrap the activity in `WorkflowDesignTheme(config.theme, config.density)`, call `WorkflowSystemBarsEffect()`, and pad the root for `WindowInsets.safeDrawing`.
- Put one `DragDropHost` at the window root.
- Build the Workbench from `DockingLayout` + `SplitPane` + `StackTabBar` + `RegionCard`. `StackTabDragDrop.onDrop` and each `dropTarget.onDrop` should emit exactly one `LayoutOp`.
- The terminal and editor should share one `ExtraKeysState` and call `consume()` for IME keystrokes.
- sora and xterm should load `assets/fonts/JetBrainsMonoNL-*.ttf` via `MonoFontAssets`.
- Adding a glyph: edit `GLYPHS` in `tools/design/generate-symbols.py` and rerun it. It downloads from the pinned commit and updates the manifest.
- Regenerating colors: `cd tools/design && npm ci && npm run colors`.
