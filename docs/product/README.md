# Product behavior

Workflow is a compact Android workspace for editing files and working with coding agents. Its two main ways of working, Files and Chat, put the current task at the center without forcing files and conversations into one tab system. Switching between them preserves the work already in progress.

This directory defines intended user-visible behavior. [Workspace tools and services](workspace-and-services.md) covers files, conversations, the runtime, and device utilities. The [UX specification](../ux/README.md) defines presentation and interaction, while the [Engine](../engine/README.md) and [App](../app/README.md) references define ownership and mechanisms. These are specifications, not claims that every interaction has passed device acceptance; verification and current limits belong in the [development documentation](../development/README.md).

## Entering and leaving a workspace

Users choose and save a workspace connection before entering the Workbench. Version 1.0.0 supports the local connection; it does not offer a working remote or SSH connection. When a connection is lost, workspace changes become unavailable. Reconnection restores the workspace's authoritative state, including drafts already accepted by it.

Launcher and Workbench are separate spaces, comparable to a desktop and the applications running on it. Launcher contains an Apps grid and a few corner controls. Workbench contains the current Session. Returning to Launcher preserves the Workbench exactly as it was.

Workflow can be selected as the system Home application. When it is the default, the system Home action always returns to Workflow Launcher. Otherwise, system Home leaves the application; the in-app Home control still opens Launcher. Back first dismisses temporary UI such as menus, dialogs, dragging, and overlay sidebars, then returns from Workbench to Launcher. It does not close tabs or change the working paradigm. Back does nothing on Launcher.

## Apps and the Launcher

Workbench, Proxy, and Settings are built-in Apps. They can be reordered alongside third-party apps but cannot be removed. Third-party apps appear only after the user selects them from the installed launchable applications through Add app. Removing one from Apps changes only this list.

Opening Workbench restores the most recently used Session. Opening Proxy or Settings normally adds that panel to the current Session and focuses it if it is already open. Their long-press or right-click menu also offers Open in separate Session, which creates an unnamed temporary Session containing only that panel. Workbench's corresponding action creates a new Session. Third-party app menus offer Open, App info, and Remove from Apps. The grid supports drag reordering.

## Sessions and resources

A Session is the context of a continuous period of work, organized by time rather than by project. It records the working paradigm, panels, tab groups, layout, focus, and reading positions. Users can leave and return without saving this UI state explicitly.

A new Session is temporary and does not need a name. Naming it makes it persistent. Temporary Sessions become eligible for automatic archive after one day without use; named Sessions do so after seven days. The Session view searches by both name and resources, shows named Sessions ranked by normalized usage frequency and recency with recency taking priority, and presents temporary Sessions on a timeline. Named Sessions are sorted automatically rather than by dragging. Archived Sessions remain available to inspect and restore.

A Resource is a file or conversation. A Working Resource is its unsaved work, such as file edits, conversation input, or pending attachments. It belongs to the resource, not to a panel or Session. There is one shared draft for each resource across all Sessions. Closing a panel, switching Sessions or paradigms, returning to Launcher, and process death do not implicitly discard it. Conversation input and attachments remain separate for each conversation.

Automatic archive protects the newest unarchived Session that references each dirty resource. Older Sessions referencing the same resource may still archive. This rule applies to both file drafts and unsent conversation input; it is not a blanket exemption for every Session that has ever displayed a draft.

Manual archive needs an explicit decision whenever the Session references unsaved work. Save all saves file drafts after checking all affected files for conflicts, and keeps unsent conversation input. If any file conflicts, the Session does not archive. Keep drafts archives without deleting the shared drafts. Discard removes the affected shared unsaved work, so its effect is visible in other Sessions that reference those resources. A clean Session archives directly and offers Undo. Merely closing a dirty tab is never a discard decision.

An external edit to a clean open file refreshes it silently. An external edit to a file with a draft produces a conflict with Keep mine, Use disk version, and Compare actions; neither version silently overwrites the other.

## Panels, Stacks, and working paradigms

A Panel displays a resource or function: an editor, image, terminal, conversation, file explorer, Proxy, or Settings. A Stack is a group of tabbed Panels sharing a region. A Paradigm determines the roles and arrangement of those regions.

| Paradigm | Main work | Supporting work |
| --- | --- | --- |
| Files | File explorer, splittable editor Stacks, and a bottom terminal Stack | A separate conversation panel on the right |
| Chat | A conversation with a collapsible conversation list on the left | A reduced file workbench on the right, with an explorer and editor Stack; less-used areas such as the terminal collapse |
| Solo | One application panel, used when Proxy or Settings opens in its own Session | None |

The conversation panel in Files can become the main content, taking that same conversation into Chat. Chat offers a direct return to Files. Switching changes the arrangement while preserving the conversation, open files, and previous Files layout. Chat's file sidebar shows the same editor Stack, not a second copy. Opening a file from Solo changes that Session to Files and retains the original application panel as a tab.

A resource opens only once within a Session. Opening it again focuses its existing Panel. Regions can be resized and collapsed; supporting regions become temporary overlays when there is insufficient room. Editor Stacks can split to the right or below, and a Stack can be maximized and restored.

Each region has one toolbar: its tab row or an equivalent header. The active Panel contributes actions such as Save, Undo, and Search to that row. Panels do not add another title bar. More groups actions in the order resource, Panel or layout, then Session. Common paths should be ready to use and compact, with no NavigationBar, redundant toolbars, or explanatory text that the user does not need to make a decision.

## Moving work between places

Drag and drop previews the destination before release and applies the result as one operation. Every drag capability also has a menu or accessibility equivalent.

| Dragged item | Destination | Result |
| --- | --- | --- |
| Tab | Its tab row | Reorder at the indicated insertion point |
| Tab or Panel | Another Stack's center or edge | Move into the Stack or create a split on that edge |
| File-tree item | A folder | Move the file after confirmation |
| File-tree item, editor tab, or external file | Conversation composer | Add an attachment |
| File-tree item or editor tab | Terminal | Paste a shell-escaped path |
| App icon | Apps grid | Reorder the Apps list |

Version 1.0.0 does not automatically import, rewrite, or move legacy Sessions, conversations, drafts, accounts, or configuration. Implementation details and protocol fields stay out of ordinary product flows; diagnostics may expose them when the user asks for details.
