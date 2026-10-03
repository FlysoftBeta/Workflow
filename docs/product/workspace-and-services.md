# Workspace tools and services

The [product model](README.md) explains Sessions, shared drafts, and the Files and Chat paradigms. This document describes what users can do inside that model. Exact sizing, controls, and visual states are specified in the [UX documentation](../ux/README.md).

## Files and terminals

The file explorer is a tree with creation, rename, move, copy, copy-path, and upload actions. Upload accepts camera, photo-library, and device-file input and writes into the selected directory. Workspace allocates a free name at commit, preserving existing files even when another writer races the import. Android imports are limited to 512 MiB per file; switching connections while a picker is open requires choosing the file again. The workspace's `.workspace/` folder always appears at the top of the tree as a protected folder, even when Show hidden files is off. It contains workspace configuration (`config.json`, `env.json`, proxy and service files), the Codex and Claude Code configuration in `agents/codex/` and `agents/claude/`, and the installed agent tools in `agents/tools/`. Configuration opens and saves like any file; the tools are read-only. Private Engine state and agent credentials never appear, and the folder's contents cannot be renamed, moved, copied, deleted or uploaded into. Settings keep their explicit configuration actions for the same files.

Deleting a file moves it into the workspace trash and offers Undo in a Snackbar, without a confirmation dialog. Trash is cleaned after seven days. Moving a file by dropping it onto a folder does require confirmation. Internal atomic-write and backup files must not leak into the user's tree.

The text editor provides syntax highlighting, Undo, Redo, Save, and Find through the Stack toolbar. Code files default to no wrapping; Markdown and plain text default to wrapping. Images open directly in a zoomable preview. Unsupported binary or oversized text files offer Open with another app. Discard changes is an explicit, confirmed action. When the soft keyboard is visible, the editor shares the terminal's special-key component, showing Tab, arrows, and common brackets.

Terminals run inside the workspace environment, initially as user `work` in the workspace root. Multiple terminals appear as tabs in one Stack. A URL opens in the system browser; a local path, including `path:line:column`, opens and positions the editor. Relative paths use the terminal's current directory, and `~` refers to the environment's home directory. Dropping a file pastes its escaped path.

The terminal's special-key row provides Esc, Tab, Ctrl, Alt, arrows, Home, End, Page Up, Page Down, function keys, and common shell characters. Ctrl and Alt can apply once or stay locked. Selection offers Copy, Paste, and Select all. A finished process leaves its retained output visible and offers Restart or Close. Recreating or switching a panel does not restart the terminal. Terminal names, cwd and restart state belong to the Workspace; a terminal no longer referenced by a live Session is cleaned up after a fifteen-second grace period.

## Conversations and coding agents

Conversations follow a chat-centered interaction model and connect to Codex App-Server and Claude Code. Users can search, rename, and archive conversations. A new conversation offers a backend choice and remembers the previous choice. The relevant backend must be logged in and have an available model before sending is enabled. Backend capability coverage and protocol handling are defined in the [Chat reference](../engine/chat.md); controls that depend on a capability appear only when the current backend declares support.

During a turn, assistant output streams as Markdown without a bubble or a per-character animation. Reasoning summaries and tool activity appear in compact expandable rows. Users can stop a running turn, continue typing to steer it, or queue a message for the next turn. After completion, the final response stays fully visible while the process detail collapses into one summary row. Copy and other supported actions appear below the response.

Messages support selectable text, tables, inline and block mathematics, and copyable code. Wide blocks scroll within their own bounds. Incomplete or unsupported formula syntax remains readable text. Reading position and whether the view follows the bottom survive panel switches and process recreation. Local file links open the focused editor in Files or the file sidebar in Chat, expanding that sidebar if necessary.

A segmented slider combines model and reasoning-effort selection: each model has a segment, and its supported reasoning levels occupy positions within that segment. Changes apply to the next turn without another confirmation. The full model list remains available from the slider. Speed controls appear only when supported by the backend.

Attachments can come from camera, photo library, device files, workspace files, or dragging a file into the composer. The picker and explorer upload use the same interaction. Pending attachments appear above the input, show upload progress or failure, and can be removed. Draft text and attachments belong to the conversation and survive closing its panel. Workspace acknowledges only the exact submitted draft after backend acceptance, preserving newer edits. If submission outcome is uncertain, the app reports it rather than silently sending the same message again.

Codex and Claude Code keep their homes in the workspace at `.workspace/agents/codex/` and `.workspace/agents/claude/`, so chat conversations and terminal sessions share one login and configuration per agent. Inside the environment the agents see these homes at `~/.codex` and `~/.claude`. Credentials stay inside the home and are never shown in the file tree or written to logs. Version 1.0.0 does not read earlier agent homes, so a previous login must be repeated once.

Agent requests for commands, file changes, permissions, user input, and MCP interactions appear as cards awaiting a user decision. The application never approves or answers automatically. It presents only the decisions the request permits, preserves unrecognized requests for inspection, and does not focus or preselect Approve. Pressing hardware Enter must not approve a request. Pending cards remain visible above the composer; after a decision they become a compact transcript record, and cancellation marks them expired.

## The environment and workspace connection

The environment is the complete runtime: tools, packages, variables, mounts, processes, and lifecycle. Environment configuration can select multiple Python and Node versions, additional packages, variables, and post-build scripts. Both supported ABIs ship customized images and the verified mandatory Engine tools payload, so preparing the default environment does not require an online toolchain installation. Optional Claude installation and additional packages can require downloads; their progress and readiness come from Engine checks.

A failed build or verification leaves the previous usable environment intact. A successful replacement waits for an explicit restart when processes are still running; otherwise it can activate directly. Users see actual preparation progress, readiness, restart requirements, or errors. Terminals and coding agents run only inside this environment and do not fall back to host execution.

User files live at the workspace root. Workspace configuration, agent homes and tools, and private state live under `.workspace/`; the workspace service owns their durable state. Inside the environment, `/workspace/.workspace` exposes only workspace configuration; private state and the agents tree are masked there, and agent homes and tools appear at their own guest paths. The Android shell can retain connection profiles and local configuration delivered by the workspace, but does not become an alternative store for Sessions or drafts. Disconnecting disables workspace changes until authoritative state is available again. The embedded connection currently stops its Server and running processes when the transport closes; reconnect restores committed state, not uninterrupted execution. A restart action with no pending environment replacement leaves running resources alone.

## Floating controls

The floating control gives quick access to Apps, brightness, grayscale mode, and screen locking. Its collapsed button can be dragged and snaps halfway into the nearest side of the screen on release. It becomes less opaque when idle. Tapping it opens a compact panel; tapping outside, Back, or Home closes the panel.

The app section lists Apps in their Launcher order, followed by any apps selected exclusively for the floating panel. Launching an app closes the panel. Brightness changes apply while dragging. The grayscale toggle reflects the system's actual state, and Lock screen closes the panel before locking.

A capability is available only when the underlying permission and operation work. An unavailable action is disabled and offers a direct authorization route. A toggle, package, or root binary alone is never evidence of success. The same rule governs all permission and capability displays in Settings.

## Proxy and Settings

Proxy offers start and stop, Rule/Global/Direct modes, proxy groups, node selection, latency tests, and live traffic in one screen. Logs and connections are secondary panels. Configuration is edited directly rather than through an override layer; its files live under `.workspace/proxy/`. Workspace records desired operations before the Android executor performs measured local root/TUN work. Failed or interrupted operations remain distinct from successful execution; an old connection cannot authorize a new executor. The service runs through root with TUN. If another VPN or TUN is already active, Workflow reports the conflict instead of silently taking it over.

Settings contains appearance and density, backend accounts, environment status, floating controls, default-Home status, permissions, and About. It shows only details needed for a user decision. Accounts offer login and logout, with browser and device-code login choices where supported. Environment settings offer Restart environment when a verified replacement is ready. Permission rows report measured state and offer authorization when needed. Version and open-source licenses belong in About.
