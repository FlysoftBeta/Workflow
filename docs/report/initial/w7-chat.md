# W7: chat feature (report)

Role: W7 — the complete chat feature: transcript renderer (web/), app-side agent host (`platform.agent`), Compose chat UI (`feature.chat`). Behaviour follows [product.md](../../product.md) §8, visuals [ui.md](../../ui.md) §3.4 / §4.5–§4.8, the agent layer [agents.md](../../agents.md), the panel contract [w5b-shell.md](w5b-shell.md) §2.

Status: **done** (2026-09-29), with the limits in §6. Real Codex turns on the x86_64 emulator are blocked by the app sandbox (§3); all UI flows were verified with the debug demo server that speaks the real Codex wire protocol.

## 1. Transcript page protocol (v1)

One WebView per conversation panel loads `https://appassets.androidplatform.net/assets/web/chat/chat.html` (offline assets, strict CSP, no network). The page never parses the whole transcript per delta: every message keeps its own incremental Markdown state.

### 1.1 App → page

The app calls `WorkflowChat.receive(json)` with one batch: `{"v":1,"ops":[op, …]}`. Unknown ops are ignored; a batch with another `v` is ignored and reported once as `{"type":"error"}`.

| op | Fields | Meaning |
| --- | --- | --- |
| `reset` | `conversation`, `turns: [Turn]`, `hasEarlier`, `paths` (prefixes that denote the workspace root, e.g. `/workspace/`) | Replace everything (load, conversation switch, history hydration). Scrolls to the bottom and follows. |
| `theme` | `vars` (CSS custom properties, e.g. `--wf-primary`), `dark`, `fontScale` | Applies colours/sizes; may arrive before or after `reset`. |
| `append` | `turns: [Turn]` | New turns at the end (a turn already present is replaced). |
| `prepend` | `turns: [Turn]`, `hasEarlier` | Older history above; the visible content keeps its position. |
| `turn` | `turn: Turn` without `items` | Upsert the turn shell (status, timing, user message, changes card, flags). |
| `item` | `turn`, `item: Item`, `after` (item id or null = first) | Upsert one item; new items are inserted after `after`. |
| `text` | `turn`, `id`, `append` | Append streamed text to a `message` (or `reasoning` summary / `command` output) item. |
| `remove` | `turn`, `id` (item) or no `id` (whole turn) | Remove. |
| `finalize` | `turn`, `status`, `durationMs`, `final` (item id or null), `error` | Turn ended: the final message leaves the process area (one full re-render of that message), the process collapses to one summary line, the action row appears. |
| `collapse` | `turn`, `expanded` | Expand/collapse a finished turn's process from the app. |
| `scroll` | `to: "bottom"` or `follow: bool` | Jump to bottom (↓ button) / set follow policy. |

**Turn**

```
{ id, status: "running"|"completed"|"interrupted"|"failed",
  startedAt: epochMs|null, durationMs: number|null, timeLabel: "星期日 6:48"|null,   // separator above the turn (gap ≥ 30 min)
  user: { text, attachments: [{ name, image: bool, src: url|null }] } | null,
  items: [Item], final: itemId|null, error: { message }|null,
  changes: { files: [{ path, dir, name, added, removed, diff: string|null }] } | null,
  canFork: bool, canRetry: bool, expanded: bool|null }
```

**Item** (`id`, `kind`, `status: "running"|"done"|"failed"|"declined"|"incomplete"` plus kind fields)

| kind | Fields | Row |
| --- | --- | --- |
| `message` | `phase: "commentary"|"final"`, `text` (Markdown) | Plain assistant Markdown, no bubble |
| `reasoning` | `text` (summary Markdown), `seconds` | "思考中…" (breathing) / "已思考 12s ›" |
| `command` | `category: "run"|"read"|"list"|"search"`, `title`, `command`, `cwd`, `exitCode`, `output`, `truncated` | "已运行 git status" ; detail: mono block, ✓/✕, first 12 lines + "显示全部" |
| `files` | `title`, `files: [{path, dir, name, added, removed, diff}]` | "已编辑 app/…/X.kt" ; detail: file list with +n −n |
| `search` | `title`, `query`, `url` | "已搜索 …" |
| `tool` | `title`, `server`, `args`, `result` (strings) | "已调用 server.tool" ; detail: args (collapsed) + result |
| `agent` | `title`, `text` | sub-agent row |
| `plan` | `steps: [{text, status}]`, `text` | "计划 2/5 ›" ; detail: checklist |
| `image` | `src`, `caption` | inline image |
| `notice` | `level: "info"|"warning"|"error"`, `text` | centred caption between hairlines |
| `record` | `text` | resolved request, one line ("已批准 · 运行 git status") |
| `marker` | `text` | compaction / interruption marker, caption |
| `unknown` | `title`, `detail` (pretty JSON) | "未识别的活动 ›" ; detail: verbatim JSON (the only place JSON appears) |

Consecutive `command`/`files`/`search`/`tool`/`agent` rows are merged into one summary row ("已读取 3 个文件，运行了 2 条命令"), expandable to the individual rows.

### 1.2 Page → app

The page has exactly one bridge method, `WorkflowBridge.post(json)`, with these message types (anything else is dropped by the app; every field is length-limited and validated; the app also checks the WebView is still on the asset origin):

| type | Fields | App action |
| --- | --- | --- |
| `ready` | `v` | Send `theme` + `reset`. |
| `openUrl` | `url` | Only `http`, `https`, `mailto`; opened in the system browser. |
| `openPath` | `path`, `line`, `col` | Resolved against the workspace (prefixes, `:line:col`, `#L12`); `.workflow/` refused; opened via `WorkbenchCommands.openFile`. |
| `copy` | `text` or `turn` (+ `what: "final"|"markdown"|"user"`) | Clipboard; the app copies turn text from its own model. |
| `toggle` | `turn`, `expanded` | Remembered for re-renders. |
| `earlier` | — | Load an older history page. |
| `layout` | `atBottom`, `height` | Drives the native ↓ button above the composer. |
| `action` | `turn`, `name: "fork"|"retry"` | Validated against the turn's `canFork`/`canRetry`. (Beyond the brief's list: needed for the action row and the failed-turn [重试]; content cannot trigger it.) |

Content (assistant Markdown, tool output, file names) is sanitized by DOMPurify and rendered without scripts (CSP `script-src 'self'`); links and buttons are handled by page code that only ever calls `post` with the types above.

## 2. What exists (code)

| Area | Files |
| --- | --- |
| Renderer (web/) | `web/chat/src/*.js` (≈1.4k lines: protocol dispatcher, per-message incremental Markdown + math, turns/items, links, scroll, icons, strings), `web/chat/chat.{html,css}`, `web/build-chat.mjs` (esbuild bundle → `chrome66` IIFE; marked/DOMPurify/KaTeX/core-js stay the vendored globals), `web/chat.test.mjs`; output `app/src/main/assets/web/chat/`. `npm run build` = vendor + chat; a test fails when the committed bundle is stale. Written by a helper against §1. |
| Agent host (`platform.agent`) | `AgentHub` (process-scoped, in `AppGraph.agentHub`), `BackendSetup` (pluggable launcher per backend, `install(kind, setup)`), `AgentSetups` (native Codex), `NativeCodexLauncher` (nativeLibraryDir binary + `CodexNetworkBridge` + service lease), `ConversationIndexFile` (+ `ConversationIndexCodec`), `AgentPaths` (workspace ↔ agent paths, link parsing). Debug only: `DemoCodexServer`/`DebugAgentHooks` (src/debug), release stub (src/release). |
| Chat UI (`feature.chat`) | `ChatPanelProvider` (conversation panels + rail), `ConversationController`, `ConversationContent`, `Composer`, `ComposerModel`, `RequestCards`, `AccountPanes`, `ChatRail`, `WorkspaceFilePicker`, `Attachments`, `ChatText`; `transcript/` `TranscriptModel` (projector), `TranscriptDiff`, `TranscriptHost` (the WebView). |

Small changes outside my area: `PanelWiring` (conversation + rail → `ChatPanelProvider`), `AppGraph.agentHub`, `ShellViewModel.flush` also flushes the index, `CodexNetworkBridge.start(…, caFile)` (optional parameter, CA under `.workflow/agents/codex/certs`), glyphs `hexagon`/`asterisk`/`key` added to `tools/design/generate-symbols.py` (regenerated). Attachments use W6's shared `platform/importer/ImportService` (`pick` for camera/gallery/files, `importUris` for drops, owner `conversation:<id>` for results that arrive after process death) into `.attachments/<date>/`. Deleted: legacy `platform/process/LocalCodexSession.kt` and its `LocalCodexTransportTest` (the hub replaces them). Kept the conversation/rail placeholders: W6's `TerminalFeatureTest` uses them as stand-ins.

## 3. Key findings

- **Native Codex cannot run on the x86_64 API 28 emulator inside the app sandbox.** `strace` of the app shows the child killed with `SIGSYS` (`SYS_SECCOMP`, `__NR_readlink`): Android's app seccomp filter traps legacy x86_64 syscalls that the musl binary uses; exit status 159 (128+31). The same binary works under `run-as` (no app filter). arm64 has no legacy syscalls, so the tablet path (0.2.1 ran Codex natively there) is unaffected. The hub now marks a backend whose process dies with SIGSYS before its handshake as unavailable (UI: "Codex 需要工作环境"), which is exactly the engine's job (docs/engine.md §6.3 rewrites these calls). Real Codex on the emulator therefore needs W10's engine launcher; UI flows were verified with the debug demo server.

- **Page ↔ app deviations from §1** (renderer helper): wrong `v` is only `console.warn`ed (no `error` post type exists); an item `title` already carries its verb; raw HTML in Markdown is shown as text; `earlier` fires at most once per `prepend`/`reset`; a `turn` op that leaves `running` is treated as `finalize`. The app sends `finalize` before the shell `turn` op so the page never sees a final status ahead of the last item update.
- **Security checks on the real WebView (Chromium 66)**: `<script>`, `<img onerror>`, `javascript:` links and inline HTML never reach the bridge; `WorkflowBridge` exposes only `post`; navigation is always refused; only packaged assets and workspace images (`/workspace/<path>`, never `.workflow/`) are served; content cannot trigger anything but the §1.2 allow-list, which the app re-validates (`PageMessage.parse`, URL schemes http/https/mailto only).
- **WebView focus**: the transcript WebView must not be focusable — otherwise a tap on it stole the IME connection from the Compose composer (typing went nowhere). Fixed and re-verified.

## 4. Behaviour delivered (vs. brief)

| Brief item | State |
| --- | --- |
| One WebView per conversation, offline, strict CSP, incremental streaming, KaTeX (4 delimiters, split chunks), code + copy, tables, images, DOMPurify, versioned protocol, narrow callbacks | Done; verified in jsdom (36 web tests) and on the emulator's WebView 66 (`TranscriptWebViewTest`) and by eye (`09`–`11`, `22`–`23`, `60`). |
| In-progress vs final layout (compact rows, grouping, breathing "思考中…", caret; final: "用时 X ›", changes card, copy/fork/⋯) | Done (`22`, `31`–`32`, `37`, `64` "已停止 · 10s"). |
| `AgentHub` in AppGraph, never auto-approves, `approvalsReviewer=user` (by `CodexParams`; a server echo of anything else blocks sending with an inline notice) | Done. Unknown requests use `ASK_USER`: a readable "未识别的请求" card with [详情] (verbatim JSON only there) and the protocol's reject. |
| Codex from nativeLibraryDir + network bridge, `CODEX_HOME=.workspace/.workflow/agents/codex` | Done; works on arm64 (as 0.2.1 did), SIGSYS on x86_64 (§3) → shown as "Codex 需要工作环境". |
| Claude Code "needs environment", pluggable launcher | `AgentHub.install(BackendKind.CLAUDE, BackendSetup(...))` is the hook for W10; until then the picker shows "Claude Code 需要工作环境" + [改用 Codex]. |
| ConversationIndex under `.workflow/state/` (atomic, off main) | `conversations.json`, format 1, temp in `.workflow/tmp` + fsync + rename, 400ms debounce, flushed on `onStop`, newer formats read-only, corrupt → `state/corrupt/`. Survived force-stops on the emulator. |
| Panel: title/backend tab, surfaced 新对话, More ① (重命名 · 分叉 · 复制为 Markdown · 归档/取消归档), title switcher (新对话 · recent 8 · 全部对话…) | Done (`40`, `41`). Backend glyphs: hexagon (Codex), asterisk (Claude). |
| Composer: multiline, hardware Enter sends / soft Enter newline, "+" anchored menu (拍照 · 相册 · 设备文件 ┆ 工作区文件…), attachment strip, tree/external drops, draft as Working Resource | Done (`44`, `46`–`49`). Draft = store composer (revision-checked), survives panel close/session switch. |
| One model+effort slider popover (≤4 models, › full list) | Done with W5a's `ModelEffortPopover` (`06`–`07`). |
| Send/stop, steer (↑ while running), queue (long-press ↑ → 排队到下一轮), queued chips with × | Done (`24`–`28`, `61`–`63`). |
| Approval / question / elicitation / plan / permissions / tool cards pinned above composer, only offered decisions, no preselection, Enter never approves, resolved → one transcript line | Done (`30`–`36`); elicitation form types string/number/integer/boolean/enum, others as JSON text; URL mode with [已完成]/[拒绝]/[取消]. |
| Rail: groups, search, new, rename, archive (+ undo), archived list with 恢复, backend badge, running/pending dots | Done (`42`–`43`). |
| Account/login inline: device code, browser, API key; Claude login state | Codex flows done and exercised with the demo server (`50`–`52`); the real device-code screen needs a runnable Codex (tablet/engine). Claude: API key / setup token / terminal-command display, reachable once a Claude launcher exists. |
| No raw JSON in product UI | Tool args/results and permission sets use a readable key: value rendering (`ReadableJson.format`); verbatim JSON only behind "未识别的活动/请求 → 详情". |

## 5. Verification

| Check | Result |
| --- | --- |
| `cd web && npm test` | 36 pass (16 chat: chunked streaming = one-shot DOM, frozen blocks not re-rendered, 4 math delimiters, code copy, tables, XSS, links/paths, images, finalize, grouping, follow/prepend, allow-listed posts, stale build) |
| `:agent:test` | 46 pass (unchanged module) |
| `:app:testDebugUnitTest` | 45 pass (chat: `TranscriptTest` 8, `ChatTextTest` 5, `AgentPathsTest` 4; rest W5a/W6) |
| `:app:lintDebug` | 0 errors (the `StateFlowValueCalledInComposition` error in `AccountPanes` fixed) |
| `TranscriptWebViewTest` (real WebView 66, emulator) | OK ×4 runs |
| `NativeCodexHostTest` (real packaged Codex in the app sandbox) | OK: on x86_64 asserts the SIGSYS exit (159); on arm64 it asserts handshake + account read |
| Emulator `workflow-tablet-api28` (`-read-only`, port 5820, killed afterwards) with the demo backend | Screenshots `artifacts/ui/chat/00`–`71`: new chat, model popover, streaming math/code/table, stop, steer, queue, command approval, question, file approval + changes card, copy snackbar, open file in side stack, More/switcher/rail menus, archive + undo, + menu, document picker, workspace picker, attachment strip, drag from tree, login device code → logged in, Files aux, portrait. |

**Demo backend** (debug builds only): `adb shell run-as top.flysoftbeta.workflow touch files/debug-agent-demo` (content `logged-out` starts logged out). Scripts by message text: "审批/approve", "文件/edit", "问题/question", "长/long", else Markdown+math+code+table.

**Critical review against the references / ui.md**
- Matches `chatgpt_app_ui*.png`: right bubbles, plain assistant Markdown, compact 16px rows, "用时 X ›" collapse with the final answer and action row, file-change card like `chat_paradigm.png`, popover slider like `picker.png` (not a sheet).
- Weak spots: the user bubble colour is `primaryContainer` (dark teal) rather than the reference blue (by design tokens); the thinking row shows "已思考" without seconds (no per-item timing from the backend); streaming list items briefly show the raw `$…` tail until the formula closes (by design, never frozen half-open); `1/2 ▾` Chat side header is crowded at 400dp (W5b's known gap).

## 6. Limits and follow-ups

- **Real Codex on emulator**: needs W10's engine launcher (SIGSYS, §3). Not verified on the arm64 tablet in this run (absent). No real model turn was taken; no host credentials were copied.
- **Claude Code**: UI and host path complete but untested end-to-end (no guest launcher). `LoginFlow.Terminal` shows the command; opening it in a Workflow terminal needs a `WorkbenchCommands` hook (W6/W10).
- **Legacy 0.2.1 conversations** (`store.legacyConversations()`) are not imported into the index.
- `TextCursor` from chat links is 1-based line/column (same numbers as `path:line:col`); the editor must agree.
- Index is written by the hub, not by `WorkspaceStore` (architecture says the store is the only writer of `.workflow/state`; it has no API for it). Suggest folding it into the store later.
- Report-only `:agent` notes: `thread/queue/changed` is still not mapped (queued chips rely on local turns); a steer message becomes a "引导：…" record line in the running turn.
