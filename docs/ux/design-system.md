# Design system and shared interaction

Workflow uses one layer of controls around each region. The region's tab row, or a header of the same height, is its only toolbar. There is no app bar above it, NavigationBar or navigation rail beside it, or repeated Panel title, breadcrumb, and status bars within it. Global controls occupy the existing corner headers described in [Workbench interactions](workbench.md).

The interface avoids slogans, welcome paragraphs, oversized Continue working cards, and unnecessary helper text. An exceptional state appears where the problem occurs and includes a useful action. Long-pressing an icon button shows a plain tooltip with the same wording as its accessible description. Package names, engine names, process identifiers, ports, mappings, panel counts, and protocol fields belong in requested details rather than routine screens. Supported requests use structured controls; raw JSON is reserved for unrecognized content and explicit details.

Touch and the soft keyboard are the baseline. Essential actions must not depend on hover, and every drag action has a menu and accessibility equivalent. Hardware keyboards are optional. A model is selected through its anchored slider, not a bottom sheet.

## Density and geometry

Settings → Appearance offers Compact, the default, and Standard density. Components take their dimensions from shared design tokens. Component dimensions elsewhere in this directory describe the compact baseline unless a fixed measurement or standard value is stated.

| Token | Compact | Standard | Purpose |
| --- | --- | --- | --- |
| `bar` | 32dp | 44dp | Tab rows, region headers, shared find bars |
| `iconBtn`, visual/touch | 28/32dp | 40/48dp | Toolbar icon buttons |
| `icon` | 20dp; 18dp in menus and trees | 24dp | Interface icons |
| `treeRow` | 32dp | 40dp | File tree and conversation list |
| `listRow`, one/two lines | 36/48dp | 48/64dp | Session view and app picker; Settings rows stay 48dp |
| `menuItem` | 36dp | 44dp | Menu rows |
| `extraKeys`, row/key | 40/34dp | 48/40dp | Special-key strip |
| `gap` / `padH` / `indent` | 4/12/12dp | 6/16/16dp | Region spacing, horizontal content padding, and tree indentation |

Compact toolbar buttons occupy separate 32×32dp touch cells without overlapping targets. A full-width row is clickable across its entire width. Standalone controls have touch targets of at least 40dp; Launcher and floating controls use at least 48dp. A smaller visual glyph or the composer's 36dp send circle does not reduce that target.

The established viewport references are 1177×approximately 675dp landscape and 736×approximately 1115dp portrait at 261 dpi, and 960×approximately 540dp landscape and 600×approximately 900dp portrait at native 320 dpi. Below 600dp, Workbench presents one focused Stack. Landscape with the keyboard open may leave only about 350dp of height, so keyboard focus must redistribute existing space rather than add more chrome.

## Type and shape

The interface uses the system typeface, including Roboto and Noto Sans CJK as appropriate. Editor and terminal share a bundled monospaced face, with system fallback for CJK. Pinching changes the editor or terminal font from 10sp through 20sp and saves that preference.

| Style | Size / line height, weight | Use |
| --- | --- | --- |
| `titleMd` / `titleSm` | 16/22sp and 14/20sp, 500 | Dialogs and sheets; headers and Session names |
| `body` / `label` | 14/20sp and 13/18sp; active label 500 | Lists, Settings, menus; tabs, tree rows, activity |
| `caption` / `micro` | 12/16sp and 11/14sp | Time and secondary text; ticks and badges |
| `chat` | 15/24sp; h1 20/28, h2 18/26, h3 16/24 at 600 | Message content |
| `mono` | 13/19sp; code blocks 13/20sp | Editor, terminal, and code |

| Shape | Radius | Use |
| --- | --- | --- |
| `xs` | 4dp | Special keys and inline code |
| `sm` | 8dp | Tab top corners, menus, code blocks, chips, fields, tree selection |
| `md` | 12dp | Region, approval, and file-change cards |
| `lg` | 20dp | User bubbles, dialogs, and sheets |
| `xl` | 24dp | Composer, model popover, and expanded floating panel |
| `full` | Circle or capsule | Floating button, send button, segmented controls, badges |

## Color and iconography

The palette uses the app icon's `#286B57` seed. Android 9 has no dynamic color, so light and dark palettes are predetermined. These review values are approximate; the generated theme values are authoritative.

| Role | Light | Dark |
| --- | --- | --- |
| `primary` / `onPrimary` | `#286B57` / `#FFFFFF` | `#90D4BC` / `#00382A` |
| `primaryContainer` / foreground | `#ACF1D8` / `#002117` | `#04513E` / `#ACF1D8` |
| `secondaryContainer` / foreground | `#CEE8DE` / `#0A1F19` | `#354B43` / `#CEE8DE` |
| `tertiaryContainer` / foreground | `#BEE9F9` / `#001F28` | `#214C59` / `#BEE9F9` |
| `surface` / `onSurface` | `#F4FBF8` / `#171D1B` | `#0E1512` / `#DDE4E1` |
| `surfaceContainer`: Low / default / High / Highest | `#EEF5F2` / `#E8EFED` / `#E2EAE7` / `#DDE4E1` | `#171D1B` / `#1B211F` / `#262B29` / `#303634` |
| `onSurfaceVariant` / `outline` / `outlineVariant` | `#3C4A44` / `#6B7A75` / `#BACAC4` | `#BACAC4` / `#84948E` / `#3C4A44` |
| `error` / `errorContainer` | `#BA1A1A` / `#FFDAD6` | `#FFB4AB` / `#93000A` |
| Extended `success` / `warning` | `#216C26` / `#7D5800` | `#8ED888` / `#F7BD55` |

The window frame, status bar, and navigation bar use `surfaceContainer`; region content uses `surface`. Menus, popovers, and the composer use `surfaceContainerHigh`, while code uses `surfaceContainerHighest`. Selected rows use `secondaryContainer`, and focus and active work use `primary`. Success marks added lines and good latency. Warning marks Full access, elevated latency, and conflicts.

Theme can follow the system or use Light or Dark explicitly. Editor and terminal follow it. The terminal has separate light and dark ANSI 16-color sets, with light-theme colors dark enough to meet 4.5:1 contrast. Its background, foreground, cursor, and selection use `surface`, `onSurface`, `primary`, and `primary` at 30% respectively.

Interface icons use Material Symbols Rounded, normally weight 400 and unfilled, with filled active states. File types map approximately twelve extension groups to that same visual family. The Launcher icon has a `#D3EDDC` background and three rounded rectangles in `#286B57`, `#124936`, and `#6B9D86`; its adaptive foreground fits the 108dp safe area. A monochrome variant identifies Home and the floating button. Third-party icons retain their original appearance. Asset provenance, pinned versions, and licenses belong in implementation and dependency guidance.

## Motion and feedback

Motion follows Material Expressive's standard scheme: quick transitions without overshoot, with perceptible animations normally completing within about 300ms. Color, opacity, highlights, and pulsing use the fast effects spec. Menus, tooltips, chips, and row expansion use the fast spatial spec. Region changes, sheets, overlays, and paradigm changes use the default spatial spec. Buttons, toggles, send/stop changes, the floating button's edge snap, drag pickup and return, and slider handles use the corresponding Expressive fast or default spatial behavior.

Streaming text has no animation. Following new content at the bottom jumps directly to the latest content rather than smooth-scrolling. Keyboard-driven layout changes do not animate. A system animation scale of zero applies the final state immediately. Haptics use only the API 28-compatible long-press, clock-tick, and keyboard-tap effects.

Loading indicators wait 300ms before appearing. A longer operation uses a region-level indicator or a 16dp inline indicator. An error is one local row with an error icon, a short reason, and Retry or Details; it is not a Toast. Brief confirmations use a bottom-centered Snackbar, at most 480dp wide and above the keyboard.

| Situation | Presentation |
| --- | --- |
| Editor area has no tabs | Centered text actions: Open file, New file, New terminal; Open file focuses the tree |
| Folder is empty | Empty folder with New and Upload |
| New conversation | Backend choice and composer, with the last backend remembered; no greeting or suggestion cards |
| Backend is logged out | Centered Log in action naming the backend; Send disabled |
| Environment is preparing | A wavy linear progress indicator and Preparing environment n% at the terminal top; a small loading indicator in the send position |
| Environment status cannot be read | A neutral Checking environment line at the terminal top; the last known readiness still applies |
| Conversation opens before its entry exists | The environment's progress or failure with Retry environment, or an error row with Retry when the open failed |
| Verified replacement awaits activation | Environment configuration changed with Restart environment at the terminal top; Settings gains a dot |
| Copy or archive completes | A concise Snackbar such as Copied or Archived · Undo |

## Readiness and failures

Every capability depends on a prefix of one ladder: the connection, then the environment, then the capability itself, such as a signed-in backend or a running terminal. A surface presents the lowest condition that is not yet satisfied, so the same condition looks the same wherever it appears. A condition that resolves by itself is shown as waiting, with its step and measured progress, and actions that need it wait visibly instead of failing. A condition that needs a decision is shown as blocked: one line states the reason, the single action that unblocks it sits beside it, and dependent controls are disabled. A ready condition adds nothing.

A failed action is classified before it is shown. A lost connection is not reported by the action, because the connection card already explains it. A slow Engine or a preparing environment is transient and keeps the waiting presentation. A blocked capability names its reason. A refusal from Engine appears as one local error row with a short summary chosen by Engine's error kind, and the raw message is available only under Details. A Snackbar reports a failure only when no panel hosts the action, such as creating a panel from a shell command.

The first connection uses the full-screen connection screen, which names the step being performed: starting the workspace Engine, loading the workspace, or synchronizing configuration. A connection lost after it was online keeps the Workbench visible but inert under a 32% scrim. A centered card reads Reconnecting to the workspace with the cause, the attempt number and a countdown to the next attempt, or the step of the attempt in progress, together with Retry now and, when Engine left diagnostic output, Details. After the last automatic attempt, or a failure that retrying cannot fix, the card reads Workspace offline with the reason. The card is a polite live region, the inert Workbench is hidden from accessibility focus, and focus moves to the card's primary action.

## Accessibility and keyboard input

Text contrast is at least 4.5:1; icons and boundaries reach 3:1. Even the oldest timeline group must retain 3:1 contrast, so its opacity does not fall below 0.62. Disabled presentation follows the Material 3 38% convention. Font scaling works through 1.3×: rows grow to the greater of their token height or text height plus padding, and text is not clipped. Tab labels use end ellipsis only.

Tabs expose the tab role and announce selected and unsaved states. Splitters are adjustable controls with Increase, Decrease, and Collapse actions. Each drag capability has a custom accessibility action. Completion is announced once per turn, and an approval card appears with a polite announcement. Focus runs from the top-left corner controls through regions in reading order to the top-right controls. RTL mirrors the overall interface while editor and terminal remain LTR.

| Hardware shortcut | Action |
| --- | --- |
| Ctrl+S / Ctrl+Z / Ctrl+Shift+Z | Save / Undo / Redo |
| Ctrl+F / Ctrl+W / Ctrl+Tab | Find / Close tab / Switch tab |
| Ctrl+B / Ctrl+J / Ctrl+Alt+B | Toggle sidebar / bottom Stack / auxiliary area |
| Ctrl+N / Ctrl+Shift+N | New file / New conversation |

Composer Enter behavior and the prohibition on Enter approving requests are specified in [Conversations](conversations.md). Keyboard availability only adds shortcuts; it never removes touch actions.

Elevated menus reserve 12dp of transparent space around their surfaces inside popup and scroll bounds. This keeps shadows from ending at a hard viewport edge; the same allowance applies to list menus, nested menus, and model popovers.
