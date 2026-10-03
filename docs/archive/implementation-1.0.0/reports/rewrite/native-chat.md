# Native Compose chat (1.0.0 rewrite)

## Scope and integration

`ConversationContent` now renders `NativeTranscript` directly. The former `TranscriptHost`, WebView bridge, JavaScript update protocol and `TranscriptDiff` were removed. Existing native composer, request cards, permission checks, queue/steer controls and settled-request projection remain in the production route. Terminal xterm code is unchanged.

The shell creates `ChatPanelProvider(services: ChatFeatureServices)`. Services supply the process-scoped hub/importer/scope, a typed environment state, retry/install actions and bounded suspend `readResource(path,maxBytes)`. The chat feature no longer references AppGraph or client workspace files. Composer thumbnails and message attachments read through this port. The root integration adds `apply(from = "native-chat.gradle.kts")` to `app/build.gradle.kts`.

## Rendering

- CommonMark 0.30.0 produces a native immutable block tree with GFM tables/strikethrough. Compose `Text`, native selection, `LazyColumn`, table rows, quote/list markers and code surfaces render it. Raw HTML is literal text; link activation accepts http/https/mailto or workspace paths.
- `$…$`, `$$…$$`, `\(…\)`, `\[…\]` are recognized before Markdown's escape parser; code spans/fences preserve their contents. Incomplete expressions remain text during streaming. Inline formulae occupy native text placeholders; display formulae, tables and code scroll horizontally. Code copy preserves full original content across virtualized chunks.
- JLaTeXMath Android 0.2.0 draws native Canvas formulae. A bounded allowlist rejects filesystem/network primitives, macro definitions/dynamic commands, unsupported environments, oversized input, excessive nesting and dimension primitives before reaching that parser. Unknown syntax visibly falls back to its TeX source. Construction runs on `Dispatchers.Default`, serialized because upstream uses shared symbol tables; drawing stays on the UI Canvas.
- Changed message projection/Markdown parsing runs off-main, conflates producer bursts at 64 ms, and caches unchanged messages (bounded by source characters). Long prose and code become at most 4,000-character blocks, each with a stable turn/item/block key; only visible rows compose.
- Tool/process expansion, readable arguments/results, bounded command output, file change cards, final copy/fork/Markdown actions and approval history remain native. Approval actions still originate exclusively in existing native request cards.
- Viewport state uses actual `LazyListState` item keys and pixel offset, plus follow-bottom state. It is submitted through `PanelContext.updateView` after scrolling settles and therefore belongs to Workspace Engine. Recreated controllers restore it and request earlier history when needed. New content does not move a reader who scrolled upward; the existing jump button resumes following.

## Dependencies and evidence

Exact coordinates and SHA-256 values, upstream license texts and the math Classpath Exception are bundled in `app/src/main/assets/notices/native-chat-*`. The AAR also bundles its original font license files. Upstream references: [CommonMark](https://github.com/commonmark/commonmark-java), [JLaTeXMath Android](https://github.com/noties/jlatexmath-android/tree/android). Both native math candidates inspected ([KotlinTeX](https://github.com/darriousliu/KotlinTeX), JLaTeXMath Android) are archived; the selected library has no JNI ABI/page-size dependency and a small audited input surface. This is not a complete TeX engine: disallowed/unsupported commands remain visible source.

## Verified results

- Production `:app:testEmulatorDebugUnitTest --tests 'top.flysoftbeta.workflow.feature.chat.*'`: **17/17 passed**, `artifacts/native-chat/app-unit.log` (ChatText 5, native parser 5, transcript projection 7). A later repeat was blocked by in-flight `platform.agent.AgentHub.kt:185` Long/Int integration changes, not a chat compiler error; root owns that final full-app integration check.
- Latest parser/security tests: **5/5 passed**, `artifacts/native-chat/parser-test-final.log`.
- API 28 renderer instrumentation: **3/3 passed**, `artifacts/native-chat/api28-v6.log`, on wrapper-managed `workflow-tablet-api28`, port 5820, 1536 MiB, disk ANDROID_TMP. Immutable APKs are `artifacts/native-chat/apk-v6/{app,test}.apk`; the independent renderer QA APK uses exact production source snapshots (hashes in `artifacts/native-chat/qa/source-sha256.json`) and real project core/agent jars, without a backend demo or replacement renderer.
- Device checks cover native inline/display formula Canvas drawing, Markdown styles, GFM alignment, code copy, light/wide and dark/360dp surfaces, rejected math primitives, an 81-turn transcript, following bottom, upward-scroll anchoring while a new streaming reply arrives, re-created viewport state with exact key **and pixel offset**, and jump-to-bottom. Screenshots: `artifacts/native-chat/screenshots-v6/native-chat/{light-wide,dark-narrow,long-restored}.png`; visually inspected.
- API 28 exposed ICU's stricter closing-brace regex syntax (fixed) and the distinction between padding-visible items and `firstVisibleItemIndex` (fixed; the persisted key now corresponds to the persisted offset).
- No `WebView`, `AndroidView`, `AppGraph`, direct workspace `File` read, `TranscriptHost`, `TranscriptDiff`, page URL or JavaScript bridge remains in chat Kotlin source.

Scope of acceptance: this proves the production renderer and re-created viewport restore, not an authenticated Codex/Claude round trip or a force-stop/restart of the integrated Workspace Engine app. Root integration supplies the Engine-backed services and runs the complete app/device acceptance. Existing native approval-card protocol checks are preserved; no automatic approval was added.

Integration follow-up: the four exact native chat dependencies are now declared directly in `app/build.gradle.kts`; the temporary applied Kotlin script was removed because AGP lint crashed while analyzing its script symbol. Runtime dependencies and versions are unchanged.
