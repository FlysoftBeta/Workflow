# Workbench adapters

`app/android` presents Engine state with native Compose features, the Sora editor and the offline xterm terminal. `app/client` supplies typed bindings and disposable projections. Feature controllers and ViewModels coordinate presentation; sessions, layouts, file drafts, conversation composers and resource cleanup belong to Engine. Features communicate through injected panel/service ports and do not import one another. [UX](../ux/README.md) defines the interactions and visual rules.

## Compose and resources

The client targets API 28 and uses Material 3 Expressive. Shared components and tokens live in `ui.design`; `LocalMinimumInteractiveComponentSize` comes from the touch-target token, independently of visual icon size. Root layouts account for `WindowInsets.safeDrawing`. The activity uses `adjustResize` and IME insets, so input does not depend on API-30-only IME animation APIs.

Theme generation uses `SchemeTonalSpot`, seed `#286B57` and contrast zero. `tools/design/generate-colors.mjs` writes the checked-in colors. Material Symbols Rounded are pinned offline vectors with manifests and licenses, and JetBrains Mono NL is bundled under OFL. Historical source-art paths are not build dependencies. [Dependencies](../development/dependencies.md) identifies asset inputs and regeneration tools.

PackageManager labels and icons are cached on a background dispatcher; Compose does not load them while rendering launcher rows. File IO, syntax assets, thumbnails and expensive parsing also stay off the main thread. App-authored JSON uses two spaces, but formatting preferences must not rewrite opaque vendor data or change wire identity.

## Chat projection and rendering

Android's `AgentHub` is a `chat.snapshot`/`chat.watch` projection and `chat.command` client. Its production models and shared `ChatWire` codec live in `:app:client`. Vendor adapters, process launch, conversation-index writers and tool installers are excluded from Android production. The retained event reducer is a disposable projection mechanism until the coordinated Engine-reduced change protocol is implemented; moving packages does not remove that remaining work.

Ordered events update the shared reducer and metadata replaces the Engine projection. A changed service epoch or journal gap requires a new snapshot; chunked transfers enforce bounds and offset continuity. Sends carry a client operation ID and the exact Engine-saved composer revision. Engine owns deduplication and acknowledgement. Failed input can be restored without replaying an ambiguous submission. Approval cards retain their displayed process epoch so reused vendor IDs cannot authorize a different process.

Account flows, permission defaults and tool readiness follow current Engine metadata. Claude install controls submit `environment.tools.install` with a tool ID and retry intent, never a vendor URL, release pin, guest home or executable path. The [Chat reference](../engine/chat.md) describes protocol fidelity and the pending Rust cutover.

CommonMark parses Markdown into blocks. Compose renders prose, code, tables, activities, approval cards and controls; there is no chat WebView bridge. JLaTeXMath draws formulas through Canvas. Construction runs on a background dispatcher under a mutex because the parser shares symbol tables; input/output limits and a small cache bound work.

`ConversationController` conflates bursts on `Dispatchers.Default`, with a 64 ms pause between projection passes. A per-controller Markdown cache reparses changed messages and bounds retained text. `NativeTranscriptProjector` flattens messages into stable keyed rows for `LazyColumn`, virtualizing a long answer below the turn level.

Panel view state records row anchor, offset and follow mode. The renderer requests earlier history before restoring an unloaded anchor. Link callbacks send safe web URLs to Android and workspace paths to panel navigation. Native selection and copy preserve source content.

## Editor and terminal

Sora remains the editor adapter. Its controller coordinates authoritative draft/file state, cursor, logical reading line, wrapped-column position, partial-row offset and horizontal scroll. Resize and IME changes preserve reading position even when the caret is elsewhere. The split-pane contract recommends settling for 100 ms before expensive Sora/WebView relayout; that guidance is not proof that every resize path has identical debounce behavior.

Only xterm uses WebView. Source and tests live in `app/web`; packaged assets live in `app/android/src/main/assets/web`. They are local and transpiled for Android 9's Chromium 66 with bundled built-in compatibility. `TerminalHost` retains connection-scoped attachments; stable IDs, cwd, generation, restart and reference cleanup belong to [Engine Terminal](../engine/terminal.md).

The client applies metadata and matching bytes together. A generation change or clipped output resets the disposable decoder, viewport anchors and selection. Reading continues after EOF to observe another client's restart or rename. Rename, clear and restart submit Engine operations, and UI reset follows authoritative replies. Cancelling a view never terminates the resource. JNI host-shell support is instrumentation-only.

Terminal URL taps use refreshed buffer-cell hit regions and HTTP(S) intent handling. Workspace file candidates go to Engine resolution with terminal generation, rather than guessed client directory listings. Touch scrolling needs a minimum-size draggable thumb, row-based mapping and viewport preservation during streaming. Selection needs visible independently draggable handles, wrapped/Unicode cell mapping and edge autoscroll. These are the four repair requirements; callback presence or isolated web tests do not establish API 28 touch acceptance.

## Dragging, imports and attachments

Compose drag/drop uses a shared overlay and target geometry. Android external drags enter through `dragAndDropTarget`, with URI permission and import handling at the platform boundary. A layout drop submits one `LayoutOp`; hovering does not generate durable intermediate layouts. Dropping files on chat or terminal content transfers file references. Menus and accessibility actions offer equivalent navigation and region resizing.

Imports and attachments use Engine file-selection and upload ports. Android supplies a length and stream factory, sends bounded chunks and cancels on failure. URI/camera staging is capped at 512 MiB per import and bound to the initiating connection. Engine allocates the collision-free destination and returns its final path; Android neither writes user files directly nor predicts names by listing and retrying locally.

## Verification

`NativeTranscriptTest` covers native conversation rendering; `OfflineRendererTest` exercises the actual Android WebView. Editor and Workbench tests cover reading positions, IME/window changes, layout and dragging. Terminal touch acceptance must exercise link/file navigation, scrollbar and selection together, including streaming and rotation. JVM, Node and managed-process fixtures cannot establish real Android input or guest execution on their own.

Historical [chat](../archive/implementation-1.0.0/reports/rewrite/native-chat.md), [editor](../archive/implementation-1.0.0/reports/rewrite/editor-repair.md), [Workbench](../archive/implementation-1.0.0/reports/rewrite/workbench-qa.md) and [client-boundary](../archive/implementation-1.0.0/reports/rewrite/client-boundary.md) reports retain their source-specific evidence. [Status](../status.md) distinguishes those results from remaining devices, live accounts and the current reorganization's integrated checks.
