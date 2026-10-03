# Conversation interaction

A conversation remains readable while an agent works and makes requests for user input. Its placement within Files or Chat is defined in [Workbench interactions](workbench.md); the product's [conversation behavior](../product/workspace-and-services.md) defines persistence and capability boundaries.

## Reading messages

The message column is centered with a maximum content width of 720dp. Horizontal margins are 16dp, reduced to 12dp in a 360dp panel. Turns are separated by 24dp and blocks within a turn by 8dp. Messages more than thirty minutes apart receive a centered caption timestamp.

User messages are right-aligned primary-container bubbles with `lg` corners, 12×8dp padding, and a maximum width of 85%. Text longer than ten lines collapses behind Show more. Attachments appear above the bubble as 64dp thumbnails. Long press offers Copy and, when the backend supports it, Fork from here.

Assistant messages have no bubble and use the `chat` Markdown style. Streaming adds content without a typing animation. Code blocks use highest-container fill, `sm` corners, and a 28dp language-and-copy header. Code, tables, and display mathematics scroll horizontally inside their own blocks. Mathematics accepts `$…$`, `$$…$$`, `\(…\)`, and `\[…\]`; incomplete or unsupported syntax stays readable as text. Text selection and code copying work through native interaction. Local-path links open the focused editor Stack in Files or the file sidebar in Chat, expanding it if needed.

Quota, compaction, and error notices appear as centered captions between hairlines; errors include Retry. While the user is at the bottom, new content stays in view. Scrolling upward disables following and reveals a 36dp down button above the composer. If a decision is pending, it changes to a count such as 1 pending decision ↓. After scrolling stops, the exact visible block, offset, and follow-bottom state are retained so panel changes and process recreation restore the same place, loading older history first when necessary.

## A running or completed turn

A 28dp turn header uses `label` and `onSurfaceVariant`. Running turns begin expanded with a 16dp loading indicator and elapsed time. A completed turn collapses its process into Took X ›; a stopped turn becomes Stopped · X ›; a failed turn uses an error icon and Retry. The final reply remains fully visible after completion.

Intermediate messages and activity rows remain chronological. Each activity row is 28dp high with a 16dp icon and one-line summary that expands on tap. Consecutive calls of the same type combine into one row. A reasoning summary shows Thinking… with an opacity pulse while active, then Thought for 12s › and an expandable summary. A plan shows progress such as Plan 2/5 › and expands into its task list.

| Expanded activity | Detail |
| --- | --- |
| Command | A low-container block with a monospaced command, success/failure mark, and the first twelve output lines; Show all reveals the rest |
| File edit | Files and added/removed counts; selecting one opens its diff |
| Search | Query and linked results |
| MCP | Tool name and collapsed arguments |
| Unknown activity | Unrecognized activity › with lossless JSON in its expanded detail |

The file-change card uses `md` corners and an outline-variant border. Its 44dp header offers View changes, plus Undo only when the backend supports it. Each 36dp file row uses a caption directory, body filename, and monospaced success/error-colored `+n −n` counts. More than three files collapses behind an expansion action. Below the response, 32dp action icons offer Copy, supported Fork, and More with Copy as Markdown.

## Approval and question cards

Pending cards sit at the end of the message flow and remain pinned above the composer even when the user scrolls upward. They occupy at most half the column height, scrolling internally beyond that, and stack in arrival order. A decision collapses its card to a transcript record such as Approved · Run git status. A canceled turn marks outstanding requests Expired.

Cards have `md` corners, high-container fill, and a 1dp warning border. An 18dp icon precedes a heading such as Run command, Modify files, Permission, Your answer needed, or Tool request. Commands use monospace and collapse after six lines. The working directory appears only when it differs from the workspace root, followed by the agent's reason. File-change requests show filenames, change counts, and a View action that opens the diff.

Buttons come strictly from the decisions allowed by the request. Typical presentations are a filled Approve, tonal Allow in this conversation, Reject, and an overflow for choices such as Abort turn. The wording uses conversation rather than Session to avoid confusing approval scope with the Workbench Session. No decision is preselected, initial focus is not on Approve, and hardware Enter never approves.

Questions and MCP elicitation show the question, single- or multiple-choice chips, and an Other free-text field. Submit remains disabled until required answers are complete. Forms support string, number, boolean, and enum fields; unsupported structures fall back to a JSON editor. An unrecognized request is labeled as such, retains its complete JSON in Details, and offers the reply or reject actions the protocol permits. It never receives an automatic answer.

## Composing and attaching

The composer is a high-container card with `xl` corners and an 8dp outer margin, above the keyboard. Input uses `chat` type, grows from one to eight lines, then scrolls internally. The lower control row is 40dp high. Soft-keyboard Enter inserts a newline; hardware Enter sends, and Shift+Enter inserts a newline.

Add opens an anchored menu with Camera, Photos, Device files, then Workspace files. The workspace option opens a tree sheet with checkboxes and an Add (n) action. Its camera, photo, and device-file behavior is shared with explorer upload.

The attachment strip appears only when needed and is 56dp high. Images use 56×56dp thumbnails; files use 40dp chips with names at most 160dp wide. A 20dp close control sits at the upper right. Uploading adds a progress ring; failure adds an error border and Retry. The permission chip reports the current access mode and opens its mode menu. Each mode has a separate wrapping description below its title, so the complete permission behavior remains readable on narrow screens. Full access uses warning color.

The send control is a 36dp visual circle within the minimum standalone touch target. It changes shape between Send and Stop using Expressive motion.

| Composer state | Control |
| --- | --- |
| Empty, or backend unavailable because login or a model is missing | Disabled |
| Text ready to send | Primary up arrow |
| Turn running, input empty | Stop |
| Turn running, input present | Separate Stop and up-arrow controls; the arrow steers the current turn, and long press offers Queue for next turn |

Queued messages appear as removable chips above the input. Pending approvals remain visually distinct from the composer's send action.

## Model and reasoning selection

The model chip opens a popover anchored above it, 320dp wide, with `xl` corners, highest-container fill, and elevation 3. Its 36dp header names the current model and reasoning level, includes a full-list chevron, and shows a speed control only when supported. The slider has a 24dp track, a 4×44dp bar handle, and 6dp gaps between model segments. Micro labels identify the segments; if space is insufficient, only the selected model's label remains.

Up to four recent or recommended models appear as segments, with supported reasoning levels occupying detents within each segment. Dragging snaps from level to level with a clock-tick haptic and updates the title and chip immediately. The level labels are Minimal, Low, Medium, High, and Extra high as applicable to the backend. Selection takes effect for the next turn without confirmation. Choosing a model from the full list moves the slider to its position. Tapping outside or Back closes the popover.
