# W5b: app shell and Workbench (report)

Role: W5b. It covers the new app shell (single activity, Launcher and Workbench spaces), the Workbench renderer, sessions, the Launcher and the switch-over from the 0.2.1 UI. Behaviour follows [product.md](../../product.md), visuals [ui.md](../../ui.md), state [workspace.md](../../workspace.md).

Status: **done** (2026-09-28). The 0.2.1 UI is deleted; the app runs on the new shell with placeholder panels. Feature workstreams plug in through §2. Verification: §5; gaps: §7.

## 1. Packages

| Package (`:app`) | Content |
| --- | --- |
| `app.panel` | The panel contract (§2): `PanelContract.kt`, `WorkbenchCommands.kt`, `RegionSlots.kt`, `PanelRegistry.kt`, `PanelWiring.kt` |
| `app.panel.placeholder` | Stand-in providers for kinds whose feature has not landed |
| `app` | `AppGraph` (`workspaceStore`, `panelRegistry`), root composition, navigation |
| `feature.workbench` | Workbench renderer: regions, split trees, stacks, menus, DnD |
| `feature.sessions` | Session view, rename and archive dialogs |
| `feature.launcher` | Apps grid, add-apps sheet, app launching |
| `ui.sora` | `SoraGrammars`: the TextMate grammar/theme loader kept from the 0.2.1 sora adapter, for the editor workstream |

Files: `app/{Shell, ShellViewModel, WorkflowApp, ShellDialogs, SessionLabels}.kt`, `MainActivity.kt` (kept at the package root so an existing default-launcher choice survives), `feature/workbench/{WorkbenchRuntime, WorkbenchEnv, WorkbenchScreen, FilesLayout, ChatLayout, StackView, SeamLayouts, Overlays, CornerClusters, WorkbenchMenus}.kt`, `feature/sessions/{SessionsSheet, SessionDialogs}.kt`, `feature/launcher/{LauncherScreen, AddAppsSheet, AppCatalog}.kt`.

## 2. Panel contract (for feature workstreams)

Code: `app/src/main/java/top/flysoftbeta/workflow/app/panel/`. All UI pieces come from `ui.design` (W5a).

### 2.1 Division of labour

- **The shell owns** layout and chrome: tab rows, header rows, the corner clusters, the More menu's ② layout and ③ session groups, region docking/overlay/resize/collapse, the split tree, drag and drop between stacks (tab reorder, move to a stack's centre or edge), the IME rule (focused stack fills its column), Back, hardware shortcuts, dialogs and snackbars.
- **A feature owns** one kind of panel: its tab label, surfaced actions, the More ① resource group, the content, what it accepts when something is dropped on it, and whether it may close now. Features never reference each other (architecture.md §1); cross-feature actions go through `WorkbenchCommands`.
- **No panel has its own title bar or toolbar.** The stack's tab row (or the single-title header of aux / Chat main / Solo) is the only chrome.

### 2.2 Registration

`PanelWiring.create(context)` builds the process-wide `PanelRegistry` (reached via `AppGraph.panelRegistry(context)`). Each feature replaces its placeholder line:

```kotlin
PanelKind.FILE / IMAGE / DIFF -> feature.editor      PanelKind.TERMINAL -> feature.terminal
PanelKind.CONVERSATION        -> feature.chat        PanelKind.PROXY    -> feature.proxy
PanelKind.SETTINGS            -> feature.settings
explorer = feature.files (ExplorerProvider)          rail = feature.chat (RailProvider)
```

### 2.3 `PanelProvider` (one per kind, process-wide)

```kotlin
interface PanelProvider {
    fun create(panel: Panel, context: PanelContext): PanelController   // main thread, must not block
    suspend fun newTarget(request: NewPanelRequest): PanelTarget? = null // 新建终端 / 新对话: owner-assigned id
    fun resourceTitle(ref: ResourceRef): String? = null                 // conversation titles (session view, search)
    val attention: StateFlow<Boolean>? get() = null                      // pending approval → Workbench tile / ◨ dot
}
```

### 2.4 `PanelController` (one per open panel)

Lifecycle: created when the panel appears in the active session; disposed when it closes, when its target changes (`Retarget`, `RenamePath` → new controller), or when the session is left. It survives moves between stacks, splits, paradigm switches and region collapse. Its `Content` is a movable composition, so it moves with the panel instead of being recreated. `Content` leaves composition when the tab is inactive or its region collapses: keep WebView / CodeEditor instances and undo history in the controller and re-attach them. Properties are read during composition, so back them with snapshot state. Controllers are activity-retained (they survive rotation and the trip to the Launcher), so create retained views with `PanelContext.appContext` (or a `MutableContextWrapper`), never with the Activity.

| Member | Meaning |
| --- | --- |
| `tab: PanelTab(title, icon, dirty, caption)` | Tab label. `dirty` shows the dot instead of ×. For files leave `caption` null; the shell adds the parent directory when two tabs of a stack share a title. |
| `actions: List<ToolAction>` | Surfaced buttons, highest priority first (ui.md §2.2 table). The tail collapses into More when narrow. `ToolAction.menu` makes a button open an anchored menu (explorer `＋`). Standard keys in `PanelActionKeys` (`save`, `undo`, `redo`, `find`, …) receive Ctrl+S / Ctrl+Z / Ctrl+Shift+Z / Ctrl+F. |
| `resourceMenu: List<MenuEntry>` | More group ①. The shell appends ② and ③. Long-pressing the tab shows ① + ②. Destructive entries last. |
| `switcher: List<MenuGroup>?` | Under a single-title header (aux, Chat main, Solo) the title becomes this dropdown (conversation switcher: 新对话 · 最近 8 个 · 全部对话…). |
| `needsAttention` | A request waits for the user. |
| `@Composable Content(frame: PanelFrame, modifier)` | `PanelFrame(focused, placement: EDITOR/BOTTOM/AUX/CHAT_MAIN/SOLO, resizing, imeVisible)`. While `resizing` is true, WebView/sora content keeps its size and relayouts ~100ms after. |
| `dropAffordance(payload): DropAffordance?` / `onDrop(payload)` | Non-panel payloads (FilesDragPayload, ExternalDragPayload). `DropAffordance(AreaStyle.Outline, "添加为附件")` for the composer, `(AreaStyle.Scrim, "粘贴路径")` for the terminal. `onDrop` is the single commit. Panel (tab) payloads are handled by the shell. |
| `suspend prepareClose(): Boolean` | Before closing; may ask via `commands.decide(...)`. Closing never touches drafts. |
| `onFocusChanged(focused)`, `requestInputFocus()` | Focus hooks: the panel became the session's focused panel; the shell wants keyboard focus in the content (tab tapped, terminal created). |
| `navigate(cursor: TextCursor)` | Move the cursor / scroll (`path:line:col` links via `commands.openFile`). The shell has already stored the cursor in the panel's `PanelView`. |
| `dispose()` | Release resources. |

### 2.5 `PanelContext` (given to each controller)

`appContext`, `store: WorkspaceStore`, `sessionId`, `panelId`, `target`, `initialView: PanelView` (restore once), `scope` (main dispatcher, cancelled on dispose), `commands: WorkbenchCommands`, `layout(op)` (on this session), `updateView(view)` (reading position; call when scrolling settles).

### 2.6 `WorkbenchCommands` (cross-feature, implemented by the shell)

| Command | Behaviour |
| --- | --- |
| `open(target, placement = Auto)` | Open or focus (resources open once per session). |
| `openFile(path, cursor?)` | Terminal `path:line:col` links and chat file links. In Chat it opens in the side region and expands it. |
| `attachToConversation(paths)` | Delivers a `FilesDragPayload` to the conversation in view (Files aux / Chat main) via its `onDrop`; opens a new conversation in aux when none. |
| `pasteIntoTerminal(paths)` | Delivers a `FilesDragPayload` to the focused (else most recent) terminal via its `onDrop`; opens one when none. The terminal escapes them. |
| `newTerminal(directory?)`, `newConversation()` | Uses the kind's `PanelProvider.newTarget`, then opens it. |
| `revealInExplorer(path)` | Makes the explorer visible and calls `ExplorerController.reveal(path)`. |
| `snackbar(message, actionLabel?, onAction?)` | Bottom-centre snackbar, max 480dp, above the IME. |
| `suspend decide(DecisionRequest): String?` | Shell dialog: title, message, affected items (≤ 5 + "等 n 个"), options (`Filled/Tonal/Text/Destructive`) right-aligned after a left cancel, nothing preselected, outside tap does not dismiss. Returns the option key or null. |

### 2.7 Region slots (not panels)

- **Explorer** (`ExplorerProvider.create(ExplorerContext): ExplorerController`), one per session. The shell renders the header row `[⌂][◧][chip]─[actions][⋯]` and the card; the feature renders `Content(variant: SIDEBAR | TREE_COLUMN, modifier)` (Files side region; Chat side region's 200dp tree column). `ExplorerContext` adds `view: StateFlow<ExplorerView>` + `updateView(view)` (expanded directories and selection, persisted per session) and `activeFile: StateFlow<String?>` (follow the editor). Controller: `actions` (the `＋` menu via `ToolAction.menu`), `resourceMenu` (筛选… · 全部折叠 · 刷新 · ☐显示隐藏文件 · 在终端中打开), `reveal(path)`, `beginCreate(directory)` (empty-editor "新建文件"). Tree rows are drag sources: `Modifier.dragSource(LocalDragDropState.current!!, key) { FilesDragPayload(paths) }`; folders are `dropTarget(… Area(Self) …)` targets.
- **Conversation rail** (`RailProvider.create(RailContext): RailController`), Chat paradigm only. `RailContext.activeConversation` drives the selected pill; selecting calls `context.layout(LayoutOp.showConversation(id))`; ✎ calls `commands.newConversation()`. Controller: `actions` (⌕, ✎), `resourceMenu`, `Content(modifier)`.

### 2.8 Placeholders

`app.panel.placeholder`: `PlaceholderPanelProvider` (file: read-only preview; others: centred title), `PlaceholderTerminalProvider` / `PlaceholderConversationProvider` (locally generated ids; accept file drops so routing can be exercised), `PlaceholderExplorerProvider` (plain expandable tree, tap opens, long-press drags), `PlaceholderRailProvider` (conversations open in the session). Delete each when its feature lands.

### 2.9 Design-system changes made for the contract

- `ToolAction` gained `menu: List<MenuGroup>?` (anchored menu button; collapses into a submenu). `ToolActionButton(action)` renders one.
- `DragDropState`: among overlapping targets, the best target that **accepts** the payload now wins; the rejection feedback appears only when none accepts. This lets a terminal's "粘贴路径" area sit over the stack's placement zones.

## 3. Navigation (product.md §2)

- **Single activity**, `singleTask`, optional HOME (`MAIN`+`HOME`+`DEFAULT` filter, never forced). It handles rotation, density, font scale and ui mode itself (`configChanges`), so panels and WebViews are not recreated.
- **Spaces.** `Shell.space` is LAUNCHER or WORKBENCH; it is activity-retained (`ShellViewModel`) and saved in the instance state. A cold start shows the Launcher.
- **Home.** A `HOME` intent (system Home while Workflow is the default launcher) and ⌂ both go to the Launcher; the Workbench, its session and every controller stay as they are.
- **Back** order: menus/popups and dialogs (own windows) → the session sheet → an active drag (cancel) → overlay regions → Launcher. Back never closes tabs or switches paradigm. In the Launcher Back is consumed and does nothing.
- **Explicit entry points** for other components (the floating overlay): `MainActivity.EXTRA_DESTINATION` = `LAUNCHER | WORKBENCH | PROXY | SETTINGS`.
- **Lifecycle.** `onStop` → `store.flush()`; `onStart` → `runMaintenance()` now and hourly while in the foreground. After the store is ready, `retireLegacyState()` runs once (idempotent); when it retired a 0.2.1 `.state`, the stray 0.2.1 `config.json.bak` in the workspace root is removed too (its original is kept in `.workflow/state/legacy/config-v1.json`).

## 4. What was built

### 4.1 Workbench renderer (ui.md §2–§3)

- **State.** Everything is read from `WorkspaceStore.state`; every change is a store command or one `LayoutOp`. No second snapshot. Transient state only: overlay-open flags, in-flight seam sizes, the drag.
- **Controllers.** `WorkbenchRuntime` (activity-retained) holds one `SessionRuntime` for the active session: panel controllers keyed by panel id + target key, the explorer and rail controllers, the `WorkbenchCommands` implementation. Switching sessions disposes the previous runtime; going to the Launcher does not. Each panel's `Content` is a `movableContentOf`, so moves between stacks, Files ↔ Chat and promotion keep the composition.
- **Regions.** `RegionRow` (side ｜ centre ｜ aux), `CenterColumn` (editor tree over the bottom stack) and `WeightedSplit` (n-ary split tree) all use W5a's seam gestures: 20dp touch band, pill while pressed, live resize, one commit on release (`ResizeRegion` / `ResizeSplit`), collapse below 50% of the minimum with CLOCK_TICK (`SetRegionCollapsed`), double tap resets. Docking uses W5a's `resolveDocking` per paradigm; a region that cannot dock becomes an overlay (drawer / right sheet with scrim, Back and outside tap close it). Overlay state is not persisted; ◧/◨ toggle the persisted collapse when the region can dock, else the overlay.
- **Files.** Explorer ｜ editor split tree over the terminal stack ｜ conversation region. Maximized stack fills the Workbench with ⤡. Below 600dp: only the focused editor stack with `[1/3 ▾]`.
- **Chat.** Conversation list ｜ conversation column (the aux stack) ｜ the side region showing `chat.sideStack` itself with `[1/3 ▾]`, 🗀 (tree column, resizable) and ⤢ (回到 Files).
- **Solo.** `[⌂][chip] 面板名 … ⋯ │⚙` over one panel, content max 840dp; ⚙ hidden for Settings. Opening anything else turns it into Files (reducer).
- **Corner clusters.** `[⌂][◧][chip]` goes into the header that owns the window's top-left corner (explorer header, else the top-left stack); `│[◨][⚙]` into the top-right one. ◨ carries the attention dot. While dragging, resting 600ms on a collapsed region's toggle opens it.
- **Headers.** `StackTabBar` for editor and bottom stacks and for aux with several panels; a single-title header (`TitleHeader`, with the panel's switcher) for aux / Chat main / Solo. Surfaced actions = shell actions (⤢ 设为主要内容, ⌄/⌃, 新建终端, 🗀, ⤢) + the active panel's. More = ① panel resource group + ② layout group (向右/向下拆分 · 移动到 ▸ · 最大化/还原 · 关闭其他/右侧/已保存 · region items) + ③ session group (新建 Session · 命名 Session… · 切换到 Chat/Files · 所有会话…). Long-pressing a tab shows ① + ②. Equal file names get the parent directory as caption.
- **DnD.** Tab reorder (tab-row sort target, `Move(Tab)`), move to a stack's centre or edge (`Move(Center|Edge)`, zones on the content card; bottom/aux accept centre only), file payloads dropped on an editor stack open there or in a split, non-panel payloads routed to the active panel's `dropAffordance`/`onDrop` (priority target over the zones). Back cancels a drag. One op per release. W5a's `DragDropState` now prefers an accepting target among overlapping ones.
- **IME.** When the keyboard is visible the focused stack keeps its column: siblings in a column split and the bottom stack (or the editor area when a terminal is focused) shrink to the 36dp tab row.
- **Focus.** A touch anywhere in a stack focuses it (`FocusStack`, Initial pass, not consumed). Controllers get `onFocusChanged`; tab taps call `requestInputFocus`.
- **Hardware keys.** Ctrl+S/Z/Shift+Z/F → the focused panel's action with that key; Ctrl+W close; Ctrl+Tab next tab; Ctrl+B / Ctrl+Alt+B / Ctrl+J regions; Ctrl+N new file; Ctrl+Shift+N new conversation.
- **Empty states** (ui.md §6): editor "打开文件 · 新建文件 · 新建终端", bottom "新建终端", aux "新对话".

### 4.2 Sessions (product.md §3, ui.md §4.9)

- Session view: 400dp modal side sheet from the left (full screen < 600dp), 44dp header with search + ＋. Persistent sessions ranked by `SessionPolicy.persistentRanking` in two-line rows (name, main resources, relative time, current dot). Temporary sessions as the timeline (今天 / 昨天 / 本周 / 更早, fading via `TimelineEntry`, start time, no "临时会话", no panel count). "已归档 (n) ›" opens the archived list with [恢复]. Search uses `SessionPolicy.search` with resource titles from the registry.
- Tap switches; long press → 命名/重命名 · 归档…. The chip opens the sheet, long press names the session. Naming converts a temporary session.
- Manual archive: no unsaved content → archived + snackbar "已归档 · 撤销"; otherwise the decision dialog (归档「名称」, ≤ 5 resources, [取消] ┆ [丢弃] [保留草稿] [全部保存], nothing preselected, outside tap ignored). Save conflicts / invalid files keep the dialog open with the reason. Archiving the active session moves to the most recent live one (or a new one).

### 4.3 Launcher (ui.md §3.1)

- Apps grid (W5a `ReorderableAppGrid`), session chip top-right, `surface` background, nothing else. Built-ins 工作台 / 代理 / 设置 (renamed "Workflow 设置" when a third-party app has the same label), third-party apps from `config.launcher`, the "添加" tile. Workbench tile badge from `PanelRegistry.attention`.
- Tap: Workbench → last active session; 代理 / 设置 → a panel in the current session (focused if open); third-party → launched as its own task.
- Long press (release in place) → 打开 · 在单独的会话中打开 (代理/设置, Solo) or 新建 Session (工作台) ┆ 应用信息 · 从 Apps 移除 (third-party only; built-ins are not removable). Drag reorders; one `updateConfig` on release.
- Add apps: 560dp × ≤80% dialog (full screen < 600dp), 40dp search, 44dp rows, checkbox applies immediately, "完成"; no package names, the application label only when two activity labels collide. `AppCatalog` loads labels and icons off the main thread and caches them.

### 4.4 Switch-over

- `MainActivity` → `ShellViewModel` → `WorkflowApp`; the store is the only state. Migration from 0.2.1 runs on first start; nothing writes `.state` any more (the overlay's position file moved to `.workflow/overlay/position.json`, its quick-app list now reads `Launcher.overlayApps(config)`).
- **Deleted:** `ui/{AccountScreen, ChatPanel, Components, FileActions, FileChanges, FilePreview, LauncherScreens, ProxyScreen, RuntimeUi, SettingsScreens, SoraEditor, WorkbenchScreen, WorkflowApp, WorkflowController, WorkspaceDragShadow}.kt`, `ui/theme/*`, the androidTest `RuntimeConversationTest` (it drove the old controller), the `material-icons-extended` dependency and catalog entry, the template colors. Copies of the deleted sources are in `artifacts/legacy-0.2/old-ui-w5b/` for reference.
- **`:core`:** `core.workspace` (the 0.2.1 `WorkspaceRepository`, codec, models, composer) moved from `main` to `test` sources, where it stays only as the genuine fixture generator of `LegacyMigrationTest`; its four old test classes were removed (the archive-protection and composer rules are covered by W1b's `ArchiveAndComposerTest`). `LocalRuntimeInstrumentedTest` now uses plain file IO.
- **Kept platform code:** `platform/{agent-related process + network, proxy, pty, service, workspace, capabilities, overlay}` are untouched apart from the overlay rewiring above. `ui/web/ResourceWebViews.kt` (xterm / Markdown WebView adapter) is kept as is.
- **Build:** Compose stability config (`app/compose-stability.conf`: `top.flysoftbeta.workflow.core.**`), frame-colored XML theme (`values`/`values-night`) so there is no white flash before the first frame.

### 4.5 Design-system changes (small, in `ui.design`)

- `ToolAction.menu` + `ToolActionButton` + `ToolAction.asMenuEntry()` (§2.9).
- `DragDropState`: accepting targets win over non-accepting overlapping ones (§2.9).
- `ReorderableAppGrid`: the lifted tile's pointer delta is accumulated per event; it was measured in the tile's moving frame, so drops landed about half-way (a tile dragged four slots moved two).
- `WorkflowSystemBarsEffect` re-applies bar colors on configuration changes (the activity now handles rotation itself).
- Glyphs added through the generator: `space_dashboard` (工作台 tile), `chat` (conversation), `do_not_disturb_on` (从 Apps 移除).

## 5. Verification

| What | Result |
| --- | --- |
| `:core:test :app:testDebugUnitTest` | 205 tests, 0 failures |
| `:app:lintDebug` | passes (0 errors; the two Compose lint errors found on the way are fixed) |
| `ShellNavigationTest` (androidTest, in-memory store, never touches the real `.workspace`) on `workflow-tablet-api28` | OK (6): Back on Launcher does nothing; Back in Workbench → Launcher with session and layout unchanged, Workbench tile returns to it; Back closes the session sheet first; Home → Launcher and 代理 opens in the current session; tab dragged to a stack's right edge splits in one commit (invariants hold); tab dragged within its row reorders |
| Emulator, real touch (`adb input` tap / swipe / draganddrop), `artifacts/ui/shell/*.png` (50 screenshots) | see below |

Exercised on the tablet AVD (1920×1200, 261 dpi; also 320 dpi and portrait), with placeholder panels:

- Launcher → 工作台 creates/enters a session; Home / Back → Launcher; Back on Launcher no-op; 代理 tile opens a panel in the current session; long press → 在单独的会话中打开 gives Solo; ⚙ from Solo turns it into Files with both tabs; a `HOME` intent from the Workbench lands on the Launcher (`01`–`10`, `33`–`35`).
- Files: open file from the explorer, new terminal (bottom expands), new conversation in aux; drag terminal tab to the editor's right edge (split right), editor tab to the terminal stack's bottom edge (split down, feedback captured mid-drag in `06`); tab reorder with the insertion line (`38`–`39`); seam drags of the explorer and the bottom stack; state survives `force-stop` (`37`).
- More menu with ②/③ groups (`11`); 切换到 Chat, ⤢ 回到 Files, ⤢ 设为主要内容 (promotion) and back — Files layout restored exactly (`12`–`14`).
- Sessions: sheet, new session, timeline, rename (temporary → persistent), archive without drafts + undo snackbar (`15`–`22`); archive with a migrated draft shows the decision dialog, 保留草稿 keeps the draft (`45`–`46`).
- Portrait 736dp: explorer docked, conversation as a right overlay, Back closes the overlay first (`23`–`25`); 320 dpi landscape (960dp): aux overlay, portrait 600dp: explorer not dockable, clusters move to the stack header (`26`, `41`–`42`).
- Launcher: add apps sheet, Calculator/Clock added, drag reorder commits once (after the grid fix) (`28`–`31`, `40`).
- 0.2.1 migration on the device: a genuine 0.2.1 `.workspace` (written by the old repository) → sessions, the unsaved draft (dirty dot), dark theme, the added Clock app all present; `.state` moved to `.workflow/state/legacy/0.2.1`; the stray `config.json.bak` removed (`43`–`44`, `47`–`50`, dark theme).

### Critical review against ui.md / ref.png

- One 36dp header per region, no app bar / rail / status line; the corner clusters sit inside the corner headers. At 261 dpi landscape the editor card gets ≈ 580dp of the ≈ 675dp window height with the terminal collapsed to its tab row (ref.png loses ≈ 140dp to a 56dp bar, a 44dp panel header and a tab row). The Launcher has no title, slogan or "继续工作" card.
- Docking matches the §3.2 table at 1177 / 960 / 736dp.
- Weak spots: the Chat side region (400dp) is crowded — switcher, tabs, 🗀, ⤢, ⋯, ◨, ⚙ leave ~130dp for tabs; the tree column takes half of it by default. The session row menu anchors at the row end. With a hardware keyboard, closing the rename dialog with Enter moves focus to ⌂ and shows its tooltip.

## 6. Notes for the feature workstreams

- Replace your placeholder in `PanelWiring` (one line) and delete it from `app.panel.placeholder` when unused.
- Keep WebView / CodeEditor instances in the controller and create them with `PanelContext.appContext`; `Content` is composed only while the tab is active and visible.
- Use `PanelActionKeys` for save/undo/redo/find so hardware shortcuts work; use `ToolAction.menu` for anchored menus.
- The explorer must render `ExplorerVariant.TREE_COLUMN` too (Chat); tree rows should be `dragSource`s of `FilesDragPayload`.
- The conversation provider's `switcher` becomes the aux / Chat title dropdown; `attention` drives the badges.
- Terminal ids and conversation ids come from `newTarget`; the placeholders generate UUIDs, so sessions created now may reference ids the real providers do not know — treat an unknown conversation id as a new conversation and an unknown terminal id as ended.
- Old sources for reference: `artifacts/legacy-0.2/old-ui-w5b/` (RuntimeUi: PTY/Codex wiring; ChatPanel; SoraEditor).

## 7. Known gaps and follow-ups

- **IME rule not device-verified.** It is implemented from `WindowInsets.ime`, but no placeholder panel takes text input; verify once the editor/terminal land.
- **Narrow (< 600dp) mode** is minimal (focused stack + `[1/3 ▾]`, modal drawer) and only reachable in split screen; not exercised.
- **Editor-area outer edges** (`DropTarget.EditorEdge`) have no drop zone; "移动到 ▸ 新 Stack" covers it.
- **TalkBack custom actions for drag and drop** (ui.md §7) are not added; every drag has a menu equivalent.
- **Other 0.2.1 `*.bak` files** in the workspace (the tablet has `qa.md.bak`, `qa-second.md.bak`) are not removed — only `config.json.bak`, whose provenance is certain. W1b's migration could list and retire them.
- **Archive undo** re-activates the session (`restoreSession` = activate); a restore-without-activate store command would be more precise.
- **Launcher** re-queries installed apps only when the add sheet opens; uninstalled entries show "未安装" until removed.
- `Launcher.remove` in `:core` accepts proxy/settings; the UI never offers it (ui.md §8-3).
- Not run: the full `:app:connectedDebugAndroidTest` (other suites need the arm64 tablet), no tablet run (per the planner the tablet was absent). The emulator was started on port 5800 and killed afterwards.
