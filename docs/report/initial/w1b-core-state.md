# W1b: pure-Kotlin state core (report)

Status: **done** (2026-09-28). The long-term documentation is [docs/workspace.md](../../workspace.md) (Chinese): the model, the invariants, `LayoutOp` semantics, the storage format and the migration. This report records the design decisions, the API for UI engineers, test evidence and known gaps.

## 1. What was delivered

| Package (`:core`, `top.flysoftbeta.workflow.core.*`) | Files | Content |
| --- | --- | --- |
| `io` | `WorkspacePaths`, `FileSystem`, `JvmFileSystem`, `MemoryFileSystem`, `FileWatcher` (+`ManualFileWatcher`) | Workspace-relative paths, reserved paths, atomic IO port. JVM implementation using only API 26+ APIs, with lazy root, canonical confinement and staging in `.workflow/tmp`. In-memory implementation with fault injection. |
| `layout` | `Model.kt`, `LayoutOp.kt`, `LayoutReducer.kt`, `LayoutInvariants.kt` | `Panel`/`PanelTarget`/`PanelView`, `Stack`, `SplitNode`, `Workbench` (Files/Chat/Solo arrangements), 20 ops, normalization, the invariant checker. |
| `session` | `Session.kt`, `SessionPolicy.kt` | Session (kind derived from name), `UsageStats`, persistent ranking and pinning, timeline buckets, protection and auto-archive, search. |
| `resource` | `Resources.kt`, `ComposerDraft.kt` | `ResourceRef`, `DiskVersion`, `FileDraft`, `FileStatus`, the `Drafts` rules, and `ComposerDraft` (0.2.1 revision semantics). |
| `config` | `AppConfig.kt`, `ConfigCodec.kt`, `Launcher.kt` | config.json v2 typed model, strict key-path errors, unknown-key preservation, legacy v1 upgrade, launcher and overlay registry. |
| `store` | `WorkspaceStore.kt`, `WorkspaceState.kt`, `StateCodec.kt`, `LegacyMigration.kt` | Single-writer actor, `StateFlow`, debounced per-file persistence, recovery, 0.2.1 migration. |

`:app` has two new packages. No UI screen was touched, and the old `WorkspaceRepository` still drives the current UI.

- `platform.workspace`:
  - `AndroidFileSystem`: a `JvmFileSystem` with `Os.fsync` on directories.
  - `AndroidFileWatcher`: `FileObserver` per directory.
- `app.AppGraph`:
  - `workspaceStore(context)`, which is lazy and process-scoped, running on `Dispatchers.IO.limitedParallelism(1)`;
  - `processScope`;
  - `workspaceRoot(context)` and `zone()`.

Nothing instantiates the store yet, so the 0.2.1 `.state` on the tablet is not touched until the UI rewrite switches over.

Other edits:

- `docs/architecture.md` §3 links to workspace.md.
- `core/build.gradle.kts` gained a `kotlinx-coroutines-test` test dependency.
- `Json` gained pretty printing and an `Omit` marker, additively; W3 uses it unchanged.

## 2. Design decisions (the ones that shape the UI)

1. **One `Workbench` per session holds all paradigms.**
   - Panels and stacks are shared.
   - Files, Chat and Solo each keep their own region sizes, collapsed flags and focus.

   As a result, `SwitchParadigm(CHAT)` followed by `SwitchParadigm(FILES)` returns the identical Files arrangement (property-tested). Chat's side region shows the *same* editor stacks, not copies (ui.md §3.4), plus a switcher that also accepts the terminal stack.
2. **Bottom (terminals) and aux (conversations) are fixed single stacks.**
   - They cannot be split, and an edge drop on them counts as a centre drop.
   - The editor area is an n-ary split tree; nested splits on the same axis flatten, so "split right" twice gives three columns.
   - The editor area's outer edges are drop targets too.
3. **Promotion remembers where the panel came from.** Promoting a conversation that lives in an editor stack moves it to aux and records its origin. The origin stack is kept even if it becomes empty. `ReturnToFiles` puts the panel back and restores the previous active aux panel, so stacks, editor tree and the Files arrangement come back exactly (property-tested).
4. **Resource identity.**
   - File resources are keyed by normalized path, and a file and an image preview of one path share one key.
   - A session opens a key at most once, and re-opening focuses the existing panel.
   - `LayoutOp.showConversation(id)` replaces the aux conversation in place. This serves the switcher and the chat rail; it pulls in a conversation that is already open elsewhere.
5. **Conflict state is derived, not stored.**
   - Each draft keeps the base version the user saw on the first keystroke: `DiskVersion(exists, size, mtime, sha256)`.
   - The status is CLEAN, DIRTY, CONFLICT or DELETED, computed from `drafts[path]` and `disk[path]`.
   - Keep mine rebases the draft. Take disk drops it. Compare opens `PanelTarget.Diff`.
   - Editing back to the base content, or an external write equal to the draft, removes the draft.
6. **Usage counts episodes.** An activation, or an interaction after 30 minutes idle, counts as one use, with a 7-day decay. The ranking score is 0.65 × capped log recency (1 up to 1 hour, 0 at 7 days) + 0.35 × normalized ln frequency. Ties break deterministically. Dragged persistent sessions go into a `pinned` group above the ranking, which resolves ui.md §8-1.
7. **Typing does not rewrite the manifest.** It writes one `drafts/<hash>.json` after 400 ms of quiet, or at most 2 s later during continuous typing. Layout changes coalesce into one `sessions.json` write; a `.bak` of the previous known-good content is kept.
8. **config.json.**
   - An invalid edit keeps the last good config (a persisted copy) and exposes `configProblem` with key paths.
   - Settings writes are then `Blocked`; the user's file is never overwritten.
   - The legacy 0.2.1 file is upgraded once. Its secret `daemonToken` is dropped, and the original is backed up under `.workflow/state/legacy/`.
9. **0.2.1 migration reads without modifying.**
   - Drafts and composers are imported exactly.
   - Sessions are imported best effort, with layouts rebuilt from the panel lists.
   - External apps are added to the launcher.
   - Conversations are listed for W4.
   - `retireLegacyState()` later moves `.state` under `.workflow/state/legacy/0.2.1`. That call is explicit, for the switchover workstream.

## 3. API for UI engineers

```kotlin
val store = AppGraph.workspaceStore(context)       // starts loading; never blocks the caller
store.state: StateFlow<WorkspaceState>              // the only snapshot; map { … }.distinctUntilChanged()
store.awaitReady(); store.flush() /* onStop */; store.runMaintenance() /* onResume, hourly */

// Sessions
store.enterWorkbench(): SessionId                   // "Workbench" tile: active, else latest, else new
store.createSession(name = null); store.openInSeparateSession(PanelTarget.Proxy())
store.activateSession(id) /* also restores */; store.renameSession(id, name)
store.archiveSession(id, decision: ArchiveDecision? = null): ArchiveOutcome
    // Archived | NeedsDecision(resources) | SaveConflict(paths) | Invalid | Failed | NotFound
store.pinSession(id, index); store.unpinSession(id)
SessionPolicy.persistentRanking(sessions, pinned, now) / timeline(sessions, now, zone) / search(sessions, q, titleOf)
state.activeSession, liveSessions, archivedSessions, session.workbench.primaryResources

// Layout (drag-and-drop commits exactly one op on release)
store.layout(op) /* active session */; store.layout(sessionId, op); store.applyLayout(sessionId, op): Workbench?
LayoutOp.Open(target, Placement.Auto | InStack(..) | SplitEdge(stack, edge), focus)
LayoutOp.Focus/FocusStack/Close/Move(id, DropTarget.Center|Tab|Edge|EditorEdge)/SplitStack/ResizeSplit/ResetSplit/
  ResizeRegion/SetRegionCollapsed/ToggleRegion/SetMaximized/SwitchParadigm/PromoteConversation/ReturnToFiles/
  EnterSolo/SetChatSideStack/Retarget/UpdateView/UpdateExplorer/RenamePath; LayoutOp.showConversation(id)
workbench.paradigm, focusedStack, focusedPanel, editor (SplitNode tree), stacks, panelsIn(id), activePanel(id),
  files/chat/solo arrangements, regionState(region), canSplit(id), panelsAfter(id)/otherPanels(id), chat.sideStack
PanelTarget.forFile(path)                           // Image for image types, else File

// Files and drafts
store.openFile(path): FileSnapshot                  // text, shownVersion, status, binary, tooLarge
store.editFile(path, text, shownVersion)            // fire-and-forget, debounced
store.saveFile(path, text?): SaveResult             // Saved(version) | Unchanged | Conflict | Invalid | Failed
store.resolveConflict(path, KEEP_MINE|TAKE_DISK); store.discardDraft(path)
state.fileStatus(path); state.disk[path]            // a clean editor reloads when disk[path] changes
store.listDirectory(path, showHidden); store.directoryChanges(path): Flow<Unit>
store.createFile / importFile(path) { out -> … } / createDirectory / movePath / deletePath: FileOpResult

// Composer (per conversation id)
state.composer(id); store.editComposer(draft, expectedRevision); store.acknowledgeComposer(submitted); store.discardComposer(id)

// Config and launcher
state.config (appearance, agent, overlay, launcher, terminal), state.configProblem
store.updateConfig { it.copy(launcher = Launcher.add(it.launcher, LauncherEntry.App(AppRef(pkg)))) }: ConfigUpdate
Launcher.move/remove/overlayApps

// Other
state.referencedTerminals (runtime cleanup), state.notices (+dismissNotice), state.writeError
store.legacyConversations() (W4 import), store.retireLegacyState() (after the switchover)
StoreOptions(validators = mapOf("container.json" to { text -> error? })) // W3 hook
```

Notes for UI engineers:

- **Order of calls.** `layout(...)` and `editFile(...)` are queued. Read the result from `state`; they are applied in call order.
- **Scroll state.** Push `UpdateView` when scrolling settles, not per frame.
- **Transient UI state stays out of core.** IME focus expansion, narrow-screen overlay open state and the in-flight drag preview are not in the model.
- **Compose stability.** Add `top.flysoftbeta.workflow.core.**` to the Compose compiler `stabilityConfigurationFile`. All core types are immutable data classes.

## 4. Tests

The `:core:test` run on the live tree on 2026-09-28 gave **225 tests, 0 failures**.

| Scope | Tests | Notes |
| --- | --- | --- |
| New W1b tests | **125** | see breakdown below |
| Existing 0.2.1 core tests (`core.workspace`) | 43 | still green, including the archive-protection tests (`ComposerArchiveTest` 12, `WorkspaceRepositoryTest` 19) |
| Terminal stream tests | 17 | still green |
| W3's `core.environment` | 40 | not W1b's |

Breakdown of the 125 new tests:

| Test class | Tests | What it covers |
| --- | --- | --- |
| `LayoutOpTest` | 37 | scenarios for every op |
| `LayoutPropertyTest` | 6 | 400 × 80 random ops keep invariants and idempotent normalization; open always reveals; Files→Chat→Files and promote→return restore; determinism |
| `WorkspaceStoreTest` | 24 | debounce and write counts, drafts, conflicts, config, move/delete, write retry |
| `ArchiveAndComposerTest` | 15 | the 0.2.1 archive-protection and composer rules re-established on the store |
| `SessionPolicyTest` | 11 | |
| `RecoveryTest` | 7 | corrupt sessions/bak/drafts, newer format, temp cleanup, failed load, watchers |
| `FileSystemTest` | 7 | atomic write, simulated crash before rename, symlink escape, move/delete |
| `ConfigCodecTest` | 6 | |
| `DraftsTest` | 5 | |
| `StateCodecTest` | 4 | includes a 300-seed round-trip of random workbenches |
| `LegacyMigrationTest` | 3 | fixtures are generated by the real 0.2.1 `WorkspaceRepository`; `.state` is byte-identical afterwards |

Run conditions:

- **Snapshot run.** During development W3's tests briefly did not compile, so iterations also ran in a temporary snapshot of the tree minus W3's tests (since deleted). The final numbers above are from the live tree.
- **`:app`.** `:app:compileReleaseKotlin` passes with the new adapters. `compileDebugKotlin` currently fails only in another workstream's in-progress `src/debug/.../DesignGalleryActivity.kt` (unresolved `DesignGallery`), not in W1b files.
- **Not tested.** No device run. The Android adapters, `Os.fsync` and `FileObserver`, compile but have not been exercised on the tablet.

## 5. Known gaps and follow-ups

- **Switchover.**
  - The UI rewrite must start using `AppGraph.workspaceStore`, delete `core.workspace` together with its 43 tests, and then call `retireLegacyState()` once.
  - Migration runs only once. Drafts typed in the old UI after the first store start are not re-imported, so the switchover must be atomic: one build replaces the other.
- **Promotion into an emptied stack.** If a promoted conversation's origin stack was removed while in Chat, it stays in aux on return. This only happens after that stack was explicitly closed.
- **Archived sessions are never pruned.** `sessions.json` grows with history. A cap (for example, the newest 500 archived) can be added without a format change.
- **Watch coverage.** A directory that does not exist yet when it is first watched (`FileObserver` limitation) is only re-checked by `runMaintenance()` or the next change to the tracked set.
- **Legacy draft bases.** Draft bases migrated from 0.2.1 carry a text hash only. For non-UTF-8 files, the first comparison may report a conflict. Keep mine resolves it.
- **API level is unchecked for pure modules.** This gap was already noted in the W1a report. W1b code sticks to java.time, java.nio.file and API-26 file APIs, and avoids JDK 21 collection members.
- **`docs/implementation-status.md`** was not updated; the brief said not to trust it. The planner should fold this report in.
