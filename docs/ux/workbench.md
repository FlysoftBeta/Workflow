# Workbench layout and interaction

Workbench preserves the user's place while changing how much room each activity receives. The [product model](../product/README.md) defines Sessions, Stacks, Panels, and shared drafts; the [design system](design-system.md) supplies density and visual tokens.

## Shared frame and corner controls

The application draws edge to edge while respecting safe drawing bounds, display cutouts, and side navigation bars. The status bar remains visible. Status and three-button navigation bars use the frame color, with icon brightness following the theme. The workspace resizes for the soft keyboard without animating that resize on Android 9.

Workbench has no separate top bar. Its global controls are inserted into the header already occupying each window corner. If a sidebar collapses, the top-left controls move to the newly exposed corner header. Solo retains Home, the Session chip, and Settings; when the Solo panel is Settings, the redundant Settings control is hidden. Launcher shows only the Session chip at the top right because Settings already has an app tile.

| Position | Controls and behavior |
| --- | --- |
| Top left, at the start of the header | Home returns to Launcher without changing work. The sidebar toggle opens the Files explorer or Chat conversation list. The 64–120dp Session chip shows the named Session or a temporary Session's start time, such as Sun 14:32; tap opens the Session view and long press names it. |
| Top right, after a 1dp separator | The auxiliary toggle opens the Files conversation panel or Chat file sidebar; a tertiary dot signals a pending decision. Settings opens or focuses its panel in the current Session. |

Every Stack row contains horizontally scrolling tabs, an overflow count, the active Panel's promoted actions, and More. If less than 120dp remains for tabs, promoted actions move into the top of More from lowest to highest priority. The explorer, conversation list, and conversation column use an equivalent-height header without tabs. Find and conflict bars are temporary rows at the top of content, not permanent chrome.

More presents resource actions, then Panel/layout actions, then Session actions. Each menu row has an 18dp icon, related groups have separators, destructive actions appear last in their group in error color, and submenus open to the side. Shortcuts align at the right only when a hardware keyboard is attached. Long-pressing a tab or tree item without dragging opens the same applicable resource and layout actions.

| Panel | Promoted actions, in descending priority | Resource actions in More |
| --- | --- | --- |
| Text editor | Save when a draft exists, Undo, Redo, Find, Attach to conversation | Copy path, Copy relative path, Reveal in tree, Attach to conversation, Open containing folder in terminal, Open with another app, Wrap lines, Discard changes with confirmation |
| Image | Fit/original size | Copy path, Reveal in tree, Attach to conversation, Open with another app, Share |
| Terminal | New terminal; the bottom Stack also has Collapse/Expand | Clear, Rename, Restart, Always show special keys, End |
| Proxy, logs, connections | Test latency | Edit configuration, Logs, Connections |
| Settings | None | Edit workspace configuration, Edit environment configuration |
| File explorer | Add: New file, New folder, then Camera, Photos, Device files into the selected directory | Filter, Collapse all, Refresh, Show hidden files, Open in terminal |
| Conversation | Make main content when in Files | Rename, Fork if supported, Copy as Markdown, Archive |

The shared Stack layout group contains Split right, Split down, Move to a Stack or new Stack, Maximize/Restore, Close others, Close to the right, and Close saved. The explorer offers Collapse sidebar. A Files conversation offers Make main content and Collapse panel. The Chat main column offers Return to Files and a File sidebar toggle. Every More menu ends with New Session, Name Session, Switch to Chat/Files, and All Sessions.

## Available space and layout preservation

Each region is a `surface` card with `md` corners, separated from other regions and the window by the `gap` token, 4dp in Compact. Tab rows lie on the frame. The active tab connects visually to its content. The focused Stack adds a 2dp primary line to the active tab's top edge; there is no additional focus border.

Docking is based on available space. The left sidebar docks if the center can still be at least 360dp wide; otherwise it becomes a drawer. After accounting for that sidebar, the auxiliary area docks only if the center can still retain 360dp. Otherwise it slides in from the right as an overlay, width `min(400dp, window width − 48dp)`, elevation 3. Tapping outside or Back closes that overlay while preserving its state.

| Reference width | Files arrangement | Chat arrangement |
| --- | --- | --- |
| 1177dp landscape | 264dp explorer, about 537dp center, 360dp conversation; all docked | 264dp list, about 497dp conversation, 400dp files; all docked |
| 960dp landscape | 264dp explorer and about 684dp center; conversation overlays, or docks if the explorer closes | 264dp list and about 684dp center; files overlay |
| 736dp portrait | 264dp explorer and about 460dp center; conversation overlays | 264dp list and about 460dp center; files overlay |

| Region | Default | Minimum | Maximum |
| --- | --- | --- | --- |
| Sidebar | 264dp | 200dp | 40% of window width |
| Auxiliary area | 360dp conversation or 400dp files | 300dp | 55% of window width |
| Center | Remaining width | 360dp wide | Available space |
| Editor Stack | Available editor region | 240×120dp including tabs | Available space |
| Bottom Stack | 35% of its column height | Header only, 36dp in Compact | Column height minus minimum editor height |

The ordinary 4dp gap also acts as a splitter with a 20dp touch target. Pressing reveals a 4×32dp primary pill. Dragging starts only after movement exceeds touch slop along the splitter's axis; otherwise content receives the gesture. Regions follow the drag. Going below half the minimum size snaps a sidebar to zero or the bottom Stack to its tab row, with a clock-tick haptic. Double tap restores default proportions. Ratios and maximized state belong to the Session; resizing the window reapplies the ratio and clamps it to the allowed range.

A maximized Stack fills Workbench and shows an inline Restore control. When the keyboard appears, the focused Stack temporarily fills its own column and other Stacks in that column reduce to compact 36dp tab rows. Other columns simply become shorter. Hiding the keyboard restores the previous proportions; this temporary focus layout is not persisted. The terminal key strip sits directly above the keyboard.

Back dismisses, in order, menus and tooltips, an active drag, popovers/sheets/dialogs, then overlay side regions. With none left, it returns to Launcher. It never closes tabs or changes paradigm. A Solo opened from Launcher returns there on Back; Back on Launcher does nothing.

## Files, Chat, Solo, and narrow windows

In Files, the bottom terminal belongs only to the center column while the tree retains full height. Editors can split right or down. At the 736dp portrait reference width, the right corner controls sit at the end of the editor tab row and the conversation is an overlay. The conversation panel title doubles as a switcher containing New conversation, the eight most recent conversations, and All conversations.

Chat's left list groups conversations into Today, Yesterday, Last 7 days, and Earlier, with 28dp caption headers. Rows use `treeRow` height and show a 14dp backend glyph, title, and relative time. A pulsing primary dot indicates active work; a tertiary dot indicates a pending decision. The selected conversation uses a secondary-container pill. Search reveals a 32dp filter field under the header, and New conversation sits alongside it. Long press offers Rename and Archive.

The main Chat header contains only its editable title and More. The file sidebar shows the focused Files editor Stack itself. A leading `1/3`-style switcher selects other Stacks. Its tree toggle reveals a 200dp tree on the right; with no file open, the tree occupies the entire sidebar. The terminal is collapsed, and Return to Files restores the existing Files layout. Switching paradigm changes region weights and placement without reconstructing the conversation or reopening files.

Solo fills the window with one Panel. Its header combines the corner controls, a `titleSm` panel name, promoted actions, More, and Settings when relevant. Content is centered with a maximum width of 840dp. Opening a file, for example Edit configuration, changes the Session to Files and keeps the original Panel in the editor Stack.

Below 600dp, the sidebar is a modal drawer with scrim and width `min(320dp, 85% of window width)`. The auxiliary area is a full-width overlay. Only the focused center Stack is shown, selected by the leading Stack switcher; the underlying layout tree remains intact and returns when space grows. At most two promoted actions remain visible. The bottom Stack collapses to its tab row when it lacks focus.

## Tabs, the explorer, and editors

Compact tabs are 36dp high, 72–220dp wide, and padded 10dp horizontally and 4dp vertically. They contain a 16dp type icon, an end-ellipsized `label`, and a 24dp trailing slot. Files with identical names add the parent directory as a caption. The trailing slot is always visible on touch devices: it shows an 8dp draft dot, primary in the focused Stack, or a 16dp close icon. Tapping the dot closes the tab while retaining the shared draft, without confirmation.

Tab rows scroll horizontally with a 16dp fade at each end. The active tab scrolls into view. Overflow shows a count and opens the Stack's tab list; more than eight entries adds search. Tap activates, while a 300ms long press gives pickup haptics; releasing in place opens the menu and moving begins a drag. Opening an already-open resource focuses its tab and pulses its highlight once.

The Compact file tree uses 32dp rows and 12dp indentation per level. A 20dp slot holds a 16dp chevron, followed by a 16dp type icon, label, and a 6dp primary draft dot. Open files use weight 500. Selection is a secondary-container `sm` pill inset 4dp horizontally; 1dp indent guides clarify hierarchy. Rows have no separate More button. The tree expands ancestors and scrolls to the active editor's file.

The internal `.workspace/` directory stays hidden even with Show hidden files enabled. Internal write and backup artifacts such as `*.bak` must never appear in the user tree. New-file and rename fields edit inline at 28dp high. Enter or a checkmark commits; Back cancels. Invalid names show a red border and a short reason. Deletion moves to trash without a confirmation dialog and offers Snackbar Undo; cleanup occurs after seven days. Dropping onto a folder asks to confirm the move.

The tree context menu offers Open and Open in right split; creation and folder upload; Rename, Copy, Move to, and Delete; then path copying, Attach to conversation, Open in terminal, and Open with another app. The explorer's Add action and the composer's attachment picker provide a consistent camera, photo, and device-file flow.

Editors use `mono`, variant-colored line numbers, a low-container current-line highlight, and primary selection at 25%. Code defaults to no wrapping, Markdown and plain text to wrapping. JSON keys and values are visibly distinct. There is no minimap, breadcrumb, or status row. The editor uses the shared special-key strip when the keyboard is visible, with Tab, arrows, and common brackets.

Find uses a 40dp bar with Find, expandable Replace, previous/next, match count, case sensitivity, regular expressions, and Close. A disk conflict uses a 36dp tertiary-container bar with Keep mine, Use disk version, and Compare. Compare opens a read-only diff: side by side at 720dp or wider, vertically inline below that. Images initially fit the panel, support pinch zoom, and switch between fit and 1:1 on double tap. Binary or oversized files show Cannot open as text and Open with another app.

## Terminal interaction

The terminal uses `mono`, 4dp padding, and a thin overlay scrollbar. Its tab follows the terminal's reported title, falling back to Terminal n. The special-key strip appears when the terminal is focused and the keyboard is visible, and can be made permanent. It scrolls horizontally, uses 8dp gaps between groups, and contains keys with high-container fill, `xs` corners, and a minimum width of 40dp.

The key groups are `Esc Tab Ctrl Alt`, arrows, `Home End PgUp PgDn`, common characters `| ~ / - _`, `Del ^C ^D ^Z`, and `F1` through `F12`. A single Ctrl or Alt tap applies once with secondary-container fill; double tap locks it with primary fill and an underline; another tap unlocks. Long-pressing an arrow repeats it.

Visible URLs and file paths have persistent dashed underlines because touch has no hover. Pressing changes the underline to solid, and release opens the target. URLs go to the system browser. `path[:line[:column]]` opens in the focused editor Stack; relative paths use the terminal's reported working directory, and `~` uses the environment home. Long press enters selection with handles and Copy, Paste, and Select all. When a process ends, its output dims and an Ended row offers Restart and Close.

## Session navigation and archive decisions

At 600dp or wider, the Session view is a 400dp modal side sheet sliding from the left with scrim; below that it is full screen. Its 44dp header contains a 36dp search field for names and resources plus New temporary Session. Named entries are 48dp two-line rows with a `titleSm` name, resource captions, relative time, and a primary dot on the current Session. Their automatic ranking is defined by the product's archive and Session policy.

Temporary Sessions form a timeline with a 1dp vertical line and nodes, grouped by Today, Yesterday, This week, and Earlier. Entries show the start time and main resources, without the label Temporary Session or a panel count. Age is conveyed by weight, opacity, and information density rather than platform blur.

| Group | Opacity | Weight | Resources shown |
| --- | --- | --- | --- |
| Today | 1.0 | 500 | Three |
| Yesterday | 0.85 | 400 | Two |
| This week | 0.72 | 400 | One |
| Earlier | 0.62 | 400 | Time and first resource |

Tap switches Sessions; long press offers Name/Rename and Archive. The bottom Archived (n) row opens entries with Restore actions. Unsaved work triggers an Archive “name” dialog listing up to five affected resources and a remainder count. Cancel sits on the left; Discard in error text, Keep drafts as tonal, and Save all as filled actions sit on the right in that order. No action is preselected, and tapping outside does not dismiss it. A clean Session archives immediately with Archived · Undo. The shared-draft and latest-live-reference protections remain those in the [product specification](../product/README.md).

## Drag feedback

Pickup requires a 300ms long press and haptic feedback. The source drops to 40% opacity. A compact ghost chip with icon and name, highest-container fill, `sm` corners, and elevation 3 sits 32dp above the finger.

| Target | Preview |
| --- | --- |
| Stack content | Five zones: center 50% and outer 25% edges. A rectangle shows the resulting region with primary 12% fill, a 2dp primary border, and `md` corners; it springs between zones. |
| Tab order | A 2×28dp primary insertion line; other tabs do not shift |
| Folder | A primary-container row highlight; 600ms dwell expands it, and releasing asks for move confirmation |
| Composer | A 2dp primary border, 8% fill, and Add as attachment |
| Terminal | A translucent Paste path overlay |

Approaching within 32dp of a list or tab-row edge scrolls it automatically. Dwelling on a collapsed-region toggle for 600ms expands it. Invalid targets show a prohibited mark on the ghost. Releasing over an invalid target or pressing Back cancels and springs the ghost back to its origin. External drags receive the same destination feedback as internal ones. Release submits one layout operation, rather than a series of intermediate changes.
