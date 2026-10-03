# Device UX audit: Workflow 0.2.1 (real tablet)

Auditor role: hands-on UX/UI audit against `.prompt_tmp/initial/prompt.md` (sections 功能 / 工作区 UI/UX / 其他重点 / 悬浮窗 / Launcher) and its reference PNGs. No Gradle and no model turns were run.

Severity scale: **P0** blocks a core flow of the brief or risks data. **P1** is a major deviation from the brief that affects daily use. **P2** is noticeable friction or clutter. **P3** is polish.

Evidence tags: **[dev]** was observed on the tablet (screenshot or uiautomator dump under `artifacts/qa/device/`). **[code]** was read from source, because the tablet locked partway through (see Environment).

## 1. Environment

| Item | Value |
|---|---|
| Device | iFLYTEK CB_C6_STU (`Q102111212246570`), Android 9 / API 28, arm64 |
| Installed build | `top.flysoftbeta.workflow` 0.2.1 (versionCode 3), lastUpdate 2026-09-26 17:00. This is the build under test and the 0.2.1 delivery APK, so no reinstall was needed. |
| Display | Physical 1200×1920 at 320 dpi. The override density is **261**. The device was in landscape (user_rotation=1). |
| Effective dp @261, landscape | Full screen 1920×1200 px ≈ **1177 × 736 dp**. App area 1920×1141 px ≈ **1177 × 699 dp**. Smallest width (from `rng`, 1102 px) ≈ **676 dp**. |
| Effective dp @261, portrait | ≈ 736 × 1177 dp (app area ≈ 736 × ~1140 dp) |
| Effective dp @320 (native), landscape | ≈ 960 × 600 dp. Smallest width ≈ 551 dp, which matches the brief's "最小宽度 564dp" setup. |
| Default HOME | None set (the resolver shows a chooser). Launcher3 is the fallback. I did not change it. |
| Codex account | Logged out (“未登录”). No conversation with content exists (`.state/conversations/*.json` ≤ 106 B). |
| Overlay | Service not running. SYSTEM_ALERT_WINDOW and WRITE_SETTINGS are granted. |

**Coverage limit:** at about 20:52 the tablet screen went to sleep. It is now on the keyguard (`isStatusBarKeyguard=true`, `mWakefulness=Asleep`). Following the common brief, I did not wake it or enter a PIN, and all tablet interaction stopped. After that, only read-only `dumpsys`/`run-as ls/cat` were used.
- **Covered on device [dev]:** Launcher/Apps, the add-app sheet, the app context menu, "在新会话中打开", Back behaviour, a first-state Files paradigm (tree, tabs, editor, terminal, key bar, side chat), Proxy (unconfigured), and the Codex account page.
- **Covered from code only [code]:** Chat paradigm switch, composer "+" sheet, model picker, markdown/streaming, the Sessions screen and archive dialog, Settings/Permissions, and the floating overlay.
- **Not covered:** portrait, IME behaviour, touch activation of terminal links, DnD by touch, and jank measurement. The main log buffer only holds a few seconds, so no Choreographer data survived.

**State I changed on the device (the user may want to revert it):**
1. Added **Brave** to Apps. Long-press it and choose "从应用中移除".
2. The "在新会话中打开" test on 代理 created a **persistent session named “代理”**.

**Crashes/ANRs:** the crash buffer has no entries during the audit window (20:34–20:52). The only entries are from 12:50 and come from instrumentation threads (`LocalCodexExecutableTest`), not from UI flows. No ANR dialogs appeared.

**Emulator check (for later acceptance):**
- `workflow-api28` (ANDROID_AVD_HOME=artifacts/avd) **boots headless** with `-no-window -no-audio -no-snapshot-save`. Boot completed in about 13 s on KVM with swiftshader. I killed it with `emu kill` afterwards and installed nothing. The log is at `artifacts/qa/emulator/emu-api28.log`.
- Caveats for acceptance:
  - It is an **x86_64** image (`abilist x86_64,x86`) with no ARM translation, and the 0.2.1 APK ships `libcodex.so` and `libmihomo.so` **only for arm64-v8a**. Chat and proxy therefore cannot run on it; only the UI shell and PTY can.
  - Its profile is a **phone** (1080×1920 @420 dpi ≈ 411×731 dp), not the 1920×1200 tablet.
  - It still carries Workflow **0.1.0**.
  - Tablet acceptance needs a new tablet-profile AVD (1920×1200 at 320 and at 261 dpi).
  - `workflow-api29` and `workflow-api37-ps16k` also exist; I did not check them.

## 2. Findings by area

### A. Global shell and navigation

| ID | Sev | What I saw | Evidence | What the brief expects |
|---|---|---|---|---|
| A1 | P1 | Every screen, the Workbench included, has a permanent **labelled NavigationRail** of 131 px ≈ 80 dp (应用 / 会话 / 工作台, plus 设置 at the bottom). Below 840 dp (for example portrait) Apps and Sessions switch to a bottom **NavigationBar** instead (`WorkflowApp.kt`). | [dev] `01-after-back-from-codex.png`, `02-launcher-apps.png`; [code] `ui/WorkflowApp.kt` | “会话管理与设置作为小按钮放在边角位置。不要用 NavigationBar，非常占用空间！” |
| A2 | P1 | Breakpoints and rail interact badly. The Workbench shows the file tree only when its own width is ≥ 900 dp. At 261 dpi: 1177 − 80 = 1097 dp, so the tree shows. At the tablet's native 320 dpi (the brief's 564 dp configuration): 960 − 80 = **880 dp, so the file tree disappears** and is reachable only through the folder button's bottom sheet. | [code] `WorkbenchScreen.kt` `wide = maxWidth >= 900.dp`; `WorkflowApp.kt` `wide >= 840.dp`. Computed; density was not changed on the device. | Files paradigm = tree + tabs + terminal on the user's real configuration |
| A3 | P2 | Redundant entry points. The Launcher shows Settings three times (top-right gear, rail gear, “设置” tile) plus Android's own “设置” tile. The Workbench has a “回到应用” home icon **and** the rail's 应用 **and** 工作台 items. | [dev] `02-launcher-apps.png`, `01-…png` | Compact and efficient, with no duplicated chrome |
| A4 | P1 | Home/Back does not behave like a launcher: <ul><li>Back on the Workflow Apps screen **finishes the activity**. The task is destroyed and the user lands in Launcher3.</li><li>Back from a built-in app page opened from the Launcher (Proxy) goes to the **Workbench**, not back to the Launcher.</li><li>Workbench Back goes to Apps.</li><li>The system Home key leaves Workflow unless Workflow is the default home (Settings only links to the system “默认桌面” page).</li></ul> | [dev] `05-back-from-proxy.png`, `06-back-from-workbench.png`, `07-back-from-launcher.png` (focus = launcher3) | “Workbench 与 Launcher … 交互逻辑（Home、Back）应该类似多用户的场景”. Launcher Back should be a no-op, and Home from the Workbench should return to the Workflow launcher. |

### B. Launcher and Apps

| ID | Sev | What I saw | Evidence | What the brief expects |
|---|---|---|---|---|
| B1 | P2 | The top ~45% of the Launcher is decoration: a headline slogan “让思路，自由流动。”, the subtitle “本机工作区”, and a full-width 133 px “继续工作 · 临时会话” card. The app grid starts at y≈640 of 1141. | [dev] `02-launcher-apps.png` | Compact; “非必要不要增加 helper text” |
| B2 | P1 | **“在新会话中打开” is not real.** For 代理 it (a) creates a **persistent** session auto-named “代理” (so the user never named it), (b) appends a PROXY panel that the Workbench **never renders**, and (c) shows Proxy as a separate full-screen page. Back then lands in an empty Workbench for session “代理”. A plain “打开” also just navigates to the full-screen page, so nothing opens *in* the current session. | [dev] `04b-proxy-in-new-session.png`, `05-back-from-proxy.png`; [code] `WorkflowController.openApp`; state.json shows session “代理” PERSISTENT with panels `[PROXY]` | “代理/设置会在当前 Session 中打开，右键 Context Menu 可以选择在单独的会话中打开.” Persistent sessions must be named by the user. |
| B3 | P2 | The add-app sheet works: search, “+” changes to ✓ “已添加”, and third-party apps are hidden until added. However, each 72 dp row shows the **package name** (`com.brave.browser`) as secondary text, which is tech leakage, and the list is not dense. | [dev] `09-add-apps-dialog.png`, `10-add-app-after-tap.png` | Third-party apps configurable, low tech detail |
| B4 | P3 | In the context menu only “打开” has an icon; “在新会话中打开” and “从快捷切换中移除” do not. “快捷切换” (the overlay quick-apps list) is not explained anywhere near. Two tiles are both labelled “设置”. | [dev] `03-launcher-longpress-proxy.png` | “认真设计菜单” |

### C. Workbench: Files paradigm

| ID | Sev | What I saw | Evidence | What the brief expects |
|---|---|---|---|---|
| C1 | P1 | **Chrome stacks up.** From top to bottom: <ul><li>session top bar, 56 dp (home, name, 文件/对话 segmented control, side toggle, ⋮)</li><li>tree header “本机文件”, 44 dp, plus a permanent filter field, ≈ 56 dp</li><li>tab row, 44 dp</li><li>editor</li><li>status line “本机 · container.json · 已保存”</li><li>terminal header “系统终端”, 38 dp</li><li>terminal</li><li>key bar, 44 dp</li></ul>The side chat adds its own 42 dp header (“对话” + expand). The editor gets 521 px ≈ 320 dp of the 699 dp app height, and the terminal ≈ 150 dp. | [dev] `01-after-back-from-codex.png` | “避免重复 Chrome … 能由上一级根据 Focused Panel 提供的功能，就不要在下一级永久占据空间”; compactness of `ref.png` |
| C2 | P1 | The **file browser is a flat single-directory list** (tap a folder to enter it, back arrow to leave), not a tree. Rows are 48 dp (78 px), each with a “…” button. It also **shows internal backup files** (`config.json.bak`, `qa.md.bak`, `qa-second.md.bak`) at the workspace root. | [dev] `01-…png`; `run-as ls` of `.workspace` | VSCode-like explorer tree (`files_paradigm.png`) |
| C3 | P2 | Tab strip: the active tab “container.json” is clipped under the undo button and **its close button is hidden**. There is no overflow indicator or tab list and no file-type icons. | [dev] `01-crop-tabs-editor.png` | Scattered tabs are normal, so they must stay manageable |
| C4 | P2 | `container.json` and `config.json` are stored **minified on one line**, yet the user is expected to hand-edit `container.json`. Soft-wrap breaks inside tokens (“debian:13-/slim”, “ca-/certificates”), and keys and string values share one colour, so the highlighting is weak. | [dev] `01-crop-tabs-editor.png` | Declarative, user-edited `container.json`; the editor needs highlighting |
| C5 | P2 | The status line “本机 · <path> · 已保存/草稿已保留” repeats what the tab's dirty dot and the save button already show, and “本机” is jargon. | [dev] `01-…png` | No needless helper text |
| C6 | P2 | Three controls cover one concept: the 文件/对话 segmented control, the side-panel toggle, and the side header's “作为主要内容” expand icon. The Workbench ⋮ menu mixes 浏览文件 / 打开终端 / 新建文件 / 新建对话 / 会话列表 / **授权与环境**. “新建文件” exists in three places. | [dev] `01-…png`; [code] `WorkbenchScreen.kt` | Well-designed composite menus with separators per category |
| C7 | P1 | **There is no Panel/Stack system.** The layout is fixed: one file-tab stack, an optional bottom terminal, and one side panel. There is no split, no moving panels between stacks, and no drop-zone placement preview. DnD covers only tab reorder (2 dp border on the target tab), tree→chat attachment, and tree→terminal path paste. | [code] `WorkbenchScreen.kt` | “全面支持 Drag and Drop … 移动 Panel … 视觉提示” (`dnd_placement.png`, `dnd_sort.png`) |
| C8 | P3 | The empty editor shows a large icon plus the slogan “在这里展开你的思路”. | [dev] `05-back-from-proxy.png` | Efficiency over decoration |
| ✓ | — | Panel actions sit on the stack row: undo, redo and save icon buttons next to ⋮, with correct disabled states. This matches `stack_button.png`. | [dev] `01-crop-tabs-editor.png` | Keep this |

### D. Terminal

| ID | Sev | What I saw | Evidence | What the brief expects |
|---|---|---|---|---|
| D1 | P1 | The terminal is **Android `/system/bin/sh`**, labelled “系统终端”. `curl` → “not found”, the prompt is `.workspace $`, and `pwd` shows `/data/data/top.flysoftbeta.workflow/files/.workspace`. The `container.json` in the next tab promises Debian, the `work` user, and curl/git/etc. that don't exist here. | [dev] `01-crop-terminal-keys.png` | Terminal in the declarative Debian workspace (`work` user) |
| D2 | P2 | The special-key bar is reduced to Ctrl (sticky), Esc, Tab, ↑↓←→, `/ − \| ~`. It lacks Alt, Shift, Home/End, PgUp/PgDn, Del, ⌫, Enter, F-keys and one-tap ^C/^D/^Z. The keys are spread across the full width with large gaps. | [dev] `01-crop-terminal-keys.png`; `ref.png` shows a full compact bar | “特殊字符与控制字符键盘” |
| D3 | P2 | Link handling: <ul><li>URLs go through WebLinksAddon.</li><li>File links match only paths starting with `/` or `~/`. Relative paths such as `./settings.gradle.kts` or `qa.md` (the exact `term_link.png` case) are **not linked**.</li><li>`~/x` is resolved as `File(cwd,"~/x")`, which is wrong.</li><li>Link decoration appears only on hover, so on touch nothing is underlined and links cannot be discovered.</li></ul>I could not tap-test this on the device. | [code] `assets/web/terminal.js`, `RuntimeUi.openTerminalFile` | “支持打开链接、打开本地文件” (`term_link.png`) |
| D4 | P3 | The terminal header row (38 dp) exists only for the “系统终端” label, stop and close. These could live on the stack row. | [dev] | Avoid duplicate chrome |
| ✓ | — | Real PTY in xterm, sticky Ctrl, and shell-quoted path paste on drop (`'…' `). | [dev]/[code] | Keep this |

### E. Chat panel, Chat paradigm, composer, picker

| ID | Sev | What I saw | Evidence | What the brief expects |
|---|---|---|---|---|
| E1 | P1 | **There is no conversation management in the chat UI.** There is no conversation list or history, no title bar, and no “新对话” button; that option exists only in the Workbench ⋮ menu. The Chat paradigm is the same `ChatPanel` stretched wide. | [code] `ChatPanel.kt`, `WorkbenchScreen.kt` | “聊天用 ChatGPT App 同款 UI” (`chat_paradigm.png`, `chatgpt_app_ui*.png`) |
| E2 | P1 | The Chat paradigm's side panel is titled “本机文件” but holds the **editor tab stack, not the tree**. The terminal is hidden entirely in the Chat paradigm (`terminal && isFiles`). | [code] `WorkbenchScreen.kt` | A reduced Files paradigm as the side panel (`chat_right_panel.png`: tree plus open file) |
| E3 | P1 | The model+reasoning picker is a **ModalBottomSheet**: a “模型与推理” title, one slider over a flattened (model × effort) list, and a “使用此设置” confirm button. | [code] `ChatPanel.kt` (not reachable on device while logged out) | `picker.png`: a compact popover above the composer, with a model row plus chevron and an effort slider with dots that applies immediately |
| E4 | P2 | Tech leakage in the composer and dialogs: <ul><li>The composer's “…” menu offers “刷新模型” and “**App Server 控制台**” (raw JSON-RPC console).</li><li>Approval dialogs show raw `params.toString(2)` JSON.</li><li>Other server requests require a hand-typed “JSON 回应”.</li></ul> | [dev] `01-crop-side-chat.png` (… button); [code] `ChatPanel.kt` | “使用过程中不要涉及太多技术细节”; Codex-style command/diff cards |
| E5 | P2 | The send button is **enabled (filled)** while the account is logged out and no model is chosen, yet the empty state says “登录后，即可开始对话。” | [dev] `01-crop-side-chat.png` | Consistent state |
| E6 | P2 | Rendering structure: <ul><li>Each assistant message is its **own WebView** (`MarkdownContent` per LazyColumn item), which is heavy on Android 9 / WebView 66 for long threads.</li><li>Tool activity renders as one row per item with a terminal icon; there is no grouping such as “已运行命令 / 已读取文件” and no collapsible “用时 Xm Ys” work log.</li></ul>I could not observe streaming markdown or KaTeX because there is no conversation and no login. | [code] `ChatPanel.kt`, `ui/web/ResourceWebViews.kt` | `chatgpt_app_ui.png` / `_final_response.png` |
| E7 | P3 | “+” opens a bottom sheet with three full-width rows (拍照 / 相册 / 上传文件). The sheet is reused from the file manager, which is good, but it is heavy for three items on a tablet; an anchored menu would suit better. | [code] `UploadSheet` | “加号支持上传照片（拍照、相册）… 上传文件” |
| ✓ | — | Attachment chips and the draft survive panel and session changes (the leftover QA draft “session_A_only” plus two chips was still present). Dropping onto the composer highlights it and changes the hint to “松开以添加附件”. | [dev]/[code] | Keep this |

### F. Session management

| ID | Sev | What I saw | Evidence | What the brief expects |
|---|---|---|---|---|
| F1 | P1 | The temporary “timeline” is a plain list whose rows fade by **alpha (1.0 → 0.4)**. There is no time axis, no day grouping and no blur. Every unnamed temporary session is titled “**临时会话**”, so rows differ only by “N 小时前 · N 个面板”. | [code] `SessionsScreen`, `SessionPolicy.timelineOpacity`; state.json has 3 unnamed temporary sessions | “temporary session 的呈现是一个 timeline … 越久远越模糊”, for fast retrieval |
| F2 | P2 | Rows show “N 个面板”, an implementation detail, and there is no content preview (files or conversation title). | [code] | Low tech detail |
| F3 | P2 | Switching sessions takes rail → 会话 → row. The session name in the Workbench top bar is not tappable, so there is no quick switcher from where the user works. | [dev] `01-…png`; [code] | Sessions as a small corner control; switch back at any time |
| ✓ | — | The archive dialog asks whether to keep all drafts, save files, or discard, and shows counts. This matches “归档前必须决定是否要保留内容”. | [code] | Keep this |

### G. Proxy

| ID | Sev | What I saw | Evidence | What the brief expects |
|---|---|---|---|---|
| G1 | P2 | The layout is a full-width stack of large cards with a lot of empty space. The title is the engine name “Mihomo”. “以 Root 启动” is disabled with no inline reason apart from the config card below. Several helper lines appear (“启动时请求 Root”, “导入 YAML，或从关闭 TUN 的示例开始。”, “启动后可切换模式与节点。”). | [dev] `04b-proxy-in-new-session.png` | CMFA-like, compact, low tech detail |
| G2 | P2 | The mode segmented control (规则/全局/直连) and the proxy-group picker appear or become enabled only after start. Nothing shows the configured groups before starting. The “编辑配置” row, which exists once a config is present, shows the path `.workflow/proxy/config.yaml`. | [dev]/[code] `ProxyScreen.kt` | Rule/global/direct plus proxy-group selection |
| G3 | P1 | Proxy is a full-screen destination and never a panel in a session (see B2). | [dev] | Opens in the current Session |
| ✓ | — | Controls stay disabled until the core is actually running, so the UI does not claim success it hasn't verified. | [dev] | Keep this |

### H. Settings, Codex account, Permissions

| ID | Sev | What I saw | Evidence | What the brief expects |
|---|---|---|---|---|
| H1 | P2 | The Codex account page is full of runtime detail: “本地服务已启动”, 登录 ChatGPT / 使用设备验证码登录 / 刷新账户状态, the footer “登录信息由本机 Codex 管理。检查连接和登录不会发送对话。”, “运行方式 → 高级兼容设置 · 当前使用本机工作区”, and “停止本机 Codex”. | [dev] `00-initial-state.png` | Worry-free, minimal detail |
| H2 | P2 | Settings ends with the tagline “Workflow · 为文件与思路保留空间。”. Row subtitles expose “打开 config.json” and “.workspace”. There are no theme or density options, even though the user may change scaling. | [code] `SettingsScreens.kt` | Compact, no decoration |
| H3 | P2 | “授权与环境” still carries the full **Termux compatibility** UI: install guide with shell commands, runtime-bundle export, pairing-key field. It sits behind “高级”, but it is heavy tech surface for a path AGENTS.md calls optional legacy. | [code] `PermissionsScreen` | Termux is not a product prerequisite |

### I. Floating overlay ([code] only: service not running and the tablet locked before I could enable it)

| ID | Sev | What I saw | Evidence | What the brief expects |
|---|---|---|---|---|
| I1 | P2 | It is built from raw Views with **Unicode glyphs as icons**: ◈ bubble, ◐, ▣, ⇄, ⚙, ⌘, ×. It is not Material 3 Expressive and looks unlike the app. The bubble is 44×52 dp. | [code] `platform/WorkflowOverlayService.kt` | “很常用，用心设计一下”; Material 3 Expressive |
| I2 | P2 | The expanded card has: “快速控制” + ×, a quick-apps row, a brightness slider with a % label (or “允许调整亮度 · 仅需修改系统设置权限”), tiles “黑白 · 开/关 · 需要 Root” and “锁定屏幕 · 设备管理员 / Root”, and “管理快捷应用 ›”. The subtitles expose permission mechanisms. There are no recent or running apps and no auto-brightness toggle. | [code] | Quick app switch (plus extra choices from added apps), brightness, grayscale, lock |
| I3 | P2 | Built-in quick apps (代理 / 设置) launch full-screen MainActivity destinations, which inherits B2. | [code] | Apps open in the session |
| ✓ | — | Behaviour matches the brief: drag with touch slop, snap to the nearest edge with a 180 ms animation, persisted position, collapse on outside tap and on Back, hidden on the keyguard, and brightness gated on the real WRITE_SETTINGS capability. | [code] | Keep this, then device-verify |

## 3. Top 10 cross-cutting UX problems

1. **Permanent navigation chrome.** A labelled NavigationRail sits on every screen and a bottom NavigationBar appears on narrow widths. The brief explicitly forbids this. Replace both with small corner buttons for sessions and settings (A1, A3).
2. **Chrome layering in the Workbench.** Top bar → panel header + filter → tab row → status line → terminal header → key bar. Panel actions should move up to the stack row, as the undo/redo/save buttons already do (C1, C5, D4).
3. **Built-in apps are not session panels.** Proxy and Settings are full-screen destinations, “在新会话中打开” creates an auto-named persistent session holding an invisible panel, and Back leads somewhere unexpected (B2, G3, I3, A4).
4. **No real Panel/Stack/Paradigm infrastructure.** The layout is hard-coded with no DnD placement or preview, and the Chat paradigm's side panel is the wrong content (C7, E2).
5. **Files paradigm is weaker than a VSCode explorer.** It has a flat list instead of a tree, internal `.bak` files are visible, tabs are clipped, and the tree vanishes at the native density because of breakpoint math (C2, C3, A2).
6. **Chat is not a ChatGPT/Codex-style UI.** It has no conversation list or title, uses a bottom-sheet picker instead of the popover slider, shows no grouped work log, and renders one WebView per message (E1, E3, E6).
7. **Terminal does not meet the brief.** It is Android `sh` instead of the Debian workspace, the key bar is reduced, and relative-path and touch link opening are weak (D1–D3).
8. **Technical details leak into daily surfaces.** Examples: Mihomo, “本机”, package names, “N 个面板”, “App Server 控制台”, raw-JSON approvals, the Termux guide, config file names in Settings (B3, E4, F2, G1, H1–H3).
9. **Decoration and helper text instead of density.** Slogans (“让思路，自由流动。”, “在这里展开你的思路”, “为文件与思路保留空间。”), 48 dp list rows, 56 dp bars and big cards are far from `ref.png` compactness (B1, C8, G1, H2).
10. **Sessions are hard to tell apart and to reach.** All temporary sessions share the name “临时会话”, the timeline is only an alpha fade, and there is no quick switcher from the Workbench (F1–F3).

## 4. What works and should be kept

- The **stack-row actions pattern**: undo, redo and save icon buttons beside ⋮ on the tab row, with correct enabled/disabled states. This is the right direction for the whole Workbench.
- **Working-resource safety:** drafts and composer attachments survive panel close and session switches, and the archive dialog offers keep/save/discard per draft category.
- **Apps model:** third-party apps are hidden until added, the add sheet has search and a clear added state, and the context menu has separators.
- **Shared import code:** the upload sheet (拍照 / 相册 / 上传文件) is shared between the file browser and the chat composer, as the brief asked.
- **DnD basics already wired:** tree→composer attachment with a highlighted drop target and hint text, tree→terminal shell-quoted path paste, and tab reordering with a target highlight.
- **Real PTY terminal** in offline xterm, with a sticky Ctrl key.
- **Honest capability gating:** the proxy only enables its controls when actually running, overlay brightness checks for WRITE_SETTINGS, and the Codex empty state routes to the account page.
- **Overlay interaction logic:** drag, edge snap, persisted position, outside-tap and Back collapse, hidden on the keyguard. The logic is sound; the visuals need a rework.
- **Local-first sharing:** the editor, terminal and Codex all use the app-owned `.workspace`, with no Termux dependency on the main path.
- **Stability:** no crash or ANR occurred during the audited flows.
