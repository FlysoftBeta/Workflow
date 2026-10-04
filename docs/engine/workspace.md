# Workspace

The workspace model separates the arrangement of work from its unsaved content. A Session owns a Workbench layout, while Working Resources hold file and conversation drafts shared across sessions. Closing a tab, changing a layout paradigm, or switching sessions therefore cannot discard the user's work. `engine/workspace` is package `workflow-workspace`; it owns sessions and layouts and depends only on Environment. Server composes and persists commands. `:app:client` supplies Kotlin protocol models and a pure reference layout oracle; historical writers remain test fixtures.

User-facing behavior is defined in the [product specification](../product/README.md), and visual dimensions in the [UX specification](../ux/README.md). This document specifies the state and reducer semantics that support them. Wire encodings and command results are in the [protocol](protocol.md).

## Paths and ownership

Cross-module paths are relative to the workspace root, separated with `/`, and contain no leading slash or `.` or `..` components. The empty string denotes the root. Absolute paths, parent traversal, and symlink traversal are rejected. User files occupy the workspace root; configuration and private state occupy its `.workspace/` directory, which the explorer shows as a protected folder.

Panel targets and explorer state accept user paths plus the `.workspace` entries that Environment's allowlist classifies as visible (editable configuration such as `config.json`, `env.json`, proxy and service files and agent configuration, and the read-only `agents/tools/`). They cannot open, save, or attach private state. The guest also hides state, environment internals, documents, upload staging, corrupt originals, trash, and the Engine lock. `WorkspaceStore` is the connection's command-and-snapshot interface; Android does not keep a second durable copy of sessions, drafts, or layouts.

## Sessions and discovery

A Session contains its ID, optional name, creation and last-use times, usage statistics, archival time, and Workbench. A named session is persistent; an unnamed session is temporary. Naming a temporary session makes it persistent, and an empty name is rejected. A session refers to resources but does not contain their unsaved contents.

Usage is counted in interaction episodes, with a new episode on activation or interaction after at least thirty minutes. A second count decays with a seven-day half-life, preventing frequent rapid switching from dominating the ranking. Persistent sessions rank by `0.65 × recency + 0.35 × frequency`. Recency is one for the first hour, then decreases logarithmically to zero at seven days. Frequency is `ln(1 + decayed uses)`, normalized against the largest value among the ranked sessions. Last-use time and then ID break ties deterministically. The protocol and model also retain a `pinned` group whose explicit order precedes the automatic ranking. This is a retained model capability; the current product specification exposes automatic sorting rather than a session-drag interaction.

Temporary sessions form a timeline by the calendar date of last use in the supplied timezone: today, yesterday, two to six days ago, and earlier. Each group has `fade` and `maxResources` values so the view can gradually reduce emphasis and detail. Search splits the query on whitespace and requires every token to match a name or resource, including file path/name or a supplied conversation title. Name matches rank first, followed by live sessions and then recent use.

## Working Resources are separate

[FileWork](filework.md) owns file drafts and conversation composers. Workspace holds only their references in session arrangements. Closing panels, switching paradigms and archiving with `keep_drafts` cannot discard those resources. Server publishes both domains in one workspace-state transaction, preserving archive atomicity without combining their APIs.

## Workbench structure

Each Session has one Workbench that preserves separate arrangements for Files, Chat, and Solo. Panels and stacks are shared across these paradigms. A Panel has a stable ID, target, and view state; targets include files, images, diffs, terminals, conversations, proxy overview/logs/connections, and settings. View state includes scroll anchor and offset, cursor, and extras such as image zoom or settings category. Targets have identity keys; file and image views share `file:<path>`.

A Stack is an ordered panel list with one active panel when nonempty. Editor stacks form an n-ary split tree with directions and normalized weights. `bottom`, normally used for terminals, and `aux`, normally used for conversations, are permanent single stacks outside the editor tree: they cannot be split or deleted. The explorer is a fixed region rather than a panel.

Files preserves explorer, auxiliary, and bottom-region sizes and collapse state, focus, and the maximized stack. Side regions are measured in dp; bottom height is a fraction of its column, with current window limits applied during rendering. Chat preserves its conversation-list rail, side region and tree column, selected side stack, focus, and any promoted conversation's origin. Solo records the displayed panel and the paradigm to return to.

The Workbench also retains `lastEditorStack`, explorer expansion and selection, and panel MRU order. The last focused editor stack is the default file destination and initial Chat side stack. Changing paradigms changes the arrangement without creating another set of panels or resources.

## Layout invariants

Every panel belongs to exactly one stack, and a target key appears at most once per session. The editor tree contains at least one stack and each editor stack occurs exactly once. `bottom` and `aux` always exist and never enter that tree. Nonempty stacks have an active panel; empty ones do not. Empty editor stacks survive only when they are the sole editor stack or the recorded source of a promoted conversation.

Each split has at least two children and cannot directly contain another split with the same direction. Its finite weights sum to one; normalization enforces a minimum of 0.05, or `1/n` when the number of children makes that minimum impossible. Focus, Chat side stack, maximization, Solo, and promotion references must all identify valid objects. Files mode has no outstanding `promotedFrom` reference. `Workbench.violations()` and the corresponding Rust tests make these invariants executable.

The reducer is pure and total: invalid or missing references produce an unchanged layout rather than an exception. Results are normalized, and normalization is idempotent. The Kotlin reference preserves the same instance for no-op cases. New layout IDs are deterministic from `nextSeq`; terminal and conversation IDs are supplied by their respective owners. Dragging commits one operation when the user drops, instead of persisting intermediate hover layouts.

## Layout operations

`Open` focuses an existing target instead of duplicating it, or inserts a new target after the active panel in the destination stack. Automatic placement sends terminals to `bottom` and conversations to `aux`. Files, images, diffs, proxy panels, and settings use the focused editor stack; Files falls back to `lastEditorStack` if focus is elsewhere, while Chat uses its side stack. `InStack` selects an explicit destination: `replaceActive` replaces only an active panel of the same type, and `moveExisting` moves an already open target there. `showConversation(id)` combines those options for `aux`. `SplitEdge` opens into a new split beside a specified stack. Opening another target in Solo first returns to Files.

`Focus` and `FocusStack` activate and reveal their target. They expand a collapsed bottom or auxiliary region, cancel maximization when focusing another stack, and select the relevant side stack in Chat. Focusing another panel leaves Solo. `Close` leaves drafts untouched, activates the most recent remaining panel, removes empty editor stacks where permitted, merges redundant splits, and transfers focus by MRU order. Closing every terminal collapses the bottom region and returns focus to the editor.

`Move` accepts `Center`, `Tab`, `Edge`, and `EditorEdge` drop destinations. A `Tab` index is measured in the displayed order, including the dragged tab. An `Edge` drop creates a stack beside its target; if its direction matches the parent split, it joins that split and divides the target's weight. An edge drop on `bottom` or `aux` acts as a center drop. `EditorEdge` creates a row or column at the outside edge of the entire editor region. `SplitStack` moves the active panel into a new split and requires at least two panels in its source.

`ResizeSplit` and `ResetSplit` normalize weights. `ResizeRegion`, `SetRegionCollapsed`, and `ToggleRegion` alter region geometry within its allowed limits. `SetMaximized` also focuses the selected stack. `UpdateView` and `UpdateExplorer` change view state without changing focus or MRU. `RenamePath` updates file targets and explorer paths after a move; if targets collide, the panel whose path changed survives.

`SwitchParadigm` enters Files or Chat. Entering Chat selects `lastEditorStack`, or the most recently used nonempty editor stack if it is empty, and focuses the main column. Returning to Files restores the Files arrangement and focus. `PromoteConversation` first moves a conversation to `aux` when necessary and records its original stack, position, and the previous active auxiliary panel. Its source stack survives while Chat is active, even when empty. `ReturnToFiles` restores that position and auxiliary selection unless the conversation has since been closed or moved elsewhere.

`EnterSolo` displays a single panel and remembers its source paradigm. Focusing another panel returns to that source; a newly created Solo session returns to Files. `SetChatSideStack` accepts only an editor stack or `bottom`. `Retarget` replaces a target in place for actions such as the conversation switcher, but focuses an already open instance when the replacement target exists elsewhere.

## Persistence, configuration, and archival

State changes are serialized and atomically persisted before their revision is acknowledged. Clients replace their projection from the returned snapshot. Watches, process waits, stream reads, and environment builds wait outside the state commit lock. A disconnected client stops mutating the workspace and loads a fresh snapshot on reconnect.

Server composes `.workspace/state/workspace.json` and publishes through Environment's typed store, including a valid backup and recovery originals under `.workspace/corrupt/`. Unknown formats remain read-only; this version does not scan, import or migrate old state. Environment owns revisioned workspace-delivered appearance, overlay and client configuration; Chat owns backend defaults; Terminal owns terminal settings. Their combined `.workspace/config.json` remains version 2, preserves unknown keys and uses `expectedRevision`. `.workspace/env.json` independently declares the complete [environment](environment.md).

Temporary sessions become automatic-archive candidates after one day, persistent sessions after seven days. For every dirty resource, the latest live session referencing it remains protected; older sessions referencing the same resource may archive. Manual archive decisions are explicit. `save_all` checks all file versions first, refuses archival on any conflict, and retains unsent conversation input. `keep_drafts` archives only the arrangement. `discard` drops the referenced unsaved resources according to the user's decision. These protections do not depend on whether the panels are currently visible.

Explorer visibility preferences, reading positions, and cursors are saved with the layout. Dragging an editor tab into chat or a terminal transfers the file reference, whereas dropping on a tab strip moves the panel. Accessible menus expose equivalent actions. File import and attachment reads use Engine ports like other file operations. Capacity, upload, and recovery details are in the [Server document](server.md); recorded acceptance is in [status](../status.md).
