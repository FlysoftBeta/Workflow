# App-wide readiness and connection states

Status: accepted, implemented with the limits below. Owner: coordinator. Updated: 2026-10-04.

## Problem and intended outcome

Workflow depends on a chain of conditions that are frequently not yet true: the embedded Engine must start and hand over the workspace, the environment must be prepared, and a chat backend must start and sign in. Each link reports its state differently, and most user actions assume the whole chain is ready. The result is a fragile experience.

- **Actions crash the app.** About a hundred coroutines are launched from UI callbacks across chat, editor, files, terminal, proxy and settings. Many call Engine without handling failure, so a refused or failed call escapes on the main thread and kills the process. Opening a conversation while the environment was preparing was one instance. Creating, renaming or deleting a file during a disconnect, and creating a terminal after an environment failure, are others.
- **Connection loss is all or nothing.** When the Engine stops, `MainActivity` replaces the entire Workbench with the first-run connection screen. Nothing reconnects automatically, the user loses visual context, and startup shows one indeterminate bar without its phase or diagnostic output.
- **Readiness signals decay or disappear.** `EngineController` stops polling after a single failed `environment.status` call and then reports the environment as unavailable until the next connection. The terminal shows nothing for `NotInstalled` and `Unavailable`. Several panels show a spinner that never resolves.
- **Session accessors throw during transitions.** `AppGraph` and the feature service adapters call `requireSession()` lazily. If a composition or callback runs while the connection is changing, the accessor throws.
- **Errors are presented inconsistently.** Engine messages in English, raw exception text and Chinese summaries are mixed; some failures are silent and others use a Snackbar where the UX rules call for a local error row.

The intended outcome is that no readiness condition can crash the app, that every surface states plainly what it is waiting for or why it is blocked, and that a lost connection recovers by itself whenever it can. Existing guarantees remain: Engine owns all workspace state, workspace changes stay disabled while disconnected, and a reconnect restores committed state rather than promising uninterrupted processes.

## Experience and design

### One readiness ladder

Every capability depends on a prefix of one ladder. A surface derives its state from the lowest unsatisfied rung, so the same condition always looks the same wherever it appears.

| Rung | States | Needed by |
| --- | --- | --- |
| Connection | Connecting (phase), Online, Reconnecting (attempt), Offline (reason) | Everything except the connection screen |
| Environment | Preparing (step, progress), Ready, Applying, Needs restart, Failed, Unknown | Terminals, chat, agent tools |
| Capability | Backend starting, signed out, failed; terminal starting, ended; proxy permission missing | The individual panel |

Each rung resolves to one of three presentations:

- **Waiting** is transient and resolves without the user. It shows what is happening, and progress when it is measured. Actions that need the rung wait for it with a visible pending state instead of failing.
- **Blocked** needs a decision. It states the reason in one line and offers the single action that unblocks it, such as Retry environment, Sign in or Authorize. Dependent controls are disabled.
- **Ready** shows nothing extra.

### Connection

First connection keeps the full-screen connection screen. It gains the phase being performed: starting the workspace Engine, handshake, loading the workspace and synchronizing configuration. After 300 ms the progress indicator appears. On failure it shows a short reason, Retry, and Details containing the Engine's diagnostic tail.

A connection lost after it was online no longer discards the Workbench. The last projection stays visible but inert beneath a scrim. A centered status card reads Reconnecting to the workspace, shows the attempt number and offers Retry now. Drafts, layout and scroll positions remain visible, which makes the loss and its recovery easy to follow. Input, drag and drop and every command are disabled, as the product rule requires. When the new connection is ready, the Workbench is rebuilt from fresh Engine state and the card disappears. A short Snackbar reports that running terminals were stopped if any were open.

Reconnection is automatic for transport loss and Engine exit. Attempts back off at 1, 2, 5, 10 and 30 seconds and run immediately when the app returns to the foreground. After five failed attempts, or a non-retryable failure such as a protocol or version mismatch, the card becomes Offline with the reason, Retry and Details. Explicitly choosing another connection cancels pending attempts.

### Environment

Environment readiness is shown where it matters and nowhere else, following the existing [design system](../../ux/design-system.md) table.

- The terminal top bar shows preparation progress, application of a new configuration, a pending restart and failures. It also covers `NotInstalled`, which reads Preparing environment, and `Unknown`, which reads Checking environment while status cannot be read.
- A conversation shows the same state in its empty pane and a small loading indicator in the send position. A command issued while the environment prepares waits, as `AgentHub` now does.
- A new terminal or conversation opens its panel immediately and shows the waiting state inside it instead of delaying the click.

### Actions and errors

Every UI-launched operation runs through a shared action helper that classifies its failure:

| Class | Examples | Presentation |
| --- | --- | --- |
| Transient | `environment_preparing`, RPC timeout during startup | Wait and retry automatically while the surface shows Waiting |
| Lost | Connection closed, store failed | Silently ended; the connection card already explains it |
| Blocked | Environment failed, signed out, permission missing | Disabled control with the unblocking action |
| Rejected | `conflict`, `exists`, `not_found`, `read_only`, `too_large`, `invalid_params` | One local error row in the panel, with Retry when meaningful |

User-facing text is a short Chinese summary chosen by class and Engine error kind. The raw Engine message is available only under Details. A Snackbar is used only when no panel hosts the action, such as creating a panel from a shell command.

As a last line of defence, every Workbench, panel, rail and session scope carries an exception handler. It logs the stack trace under the `Workflow` tag and reports Something went wrong locally instead of terminating the process. Instrumentation tests fail when this handler runs, so reaching it remains a bug that is visible rather than a crash in daily use.

### Accessibility

The connection card and Blocked rows are polite live regions. The inert Workbench is hidden from accessibility focus while the card is shown, and focus moves to Retry now. Disabled controls keep their labels and expose the blocking reason as a state description.

## Implementation and ownership

The work divides into three tasks that can be implemented serially. The coordinator settles the failure classification and the readiness types first, because the other tasks consume them.

**Shared contracts (coordinator).** `:app:client` gains a pure `Failure` classification from `WorkspaceRpcException` kinds and connection exceptions, and a `Readiness` type with Waiting, Blocked and Ready. These are presentation-neutral and unit tested on the JVM.

**Platform robustness** owns `platform/connection`, `platform/engine`, `app/AppGraph.kt` and `MainActivity.kt`.

- `ConnectionStatus` gains connection phases, `Reconnecting(attempt, cause)` and `Offline(reason, retryable, diagnostic)`. `WorkspaceConnectionManager` owns the backoff policy and the foreground trigger; tokens still prevent stale sessions from authorizing work.
- `MainActivity` keeps the previous `ShellViewModel` composed but inert while reconnecting, instead of disposing it, and swaps to the new session only when it is connected.
- `EngineController` keeps polling through transient failures with backoff and reports `Unknown` while status is unreadable. It stops only when its session retires.
- Feature service adapters receive their session explicitly at bind time. `requireSession()` is no longer called lazily from accessors.

**Action safety** owns the `launchReporting` helpers in `feature/workbench` and `feature/chat`, the scope construction in `WorkbenchRuntime`, and each feature's launch sites. It replaces those helpers with one panel-contract helper, adds the scope handlers, and converts the launch sites in chat, editor, files, terminal, proxy, settings and launcher.

**UI states** owns `platform/connection/ConnectionScreen.kt`, a new reconnect card in `app/`, `feature/terminal/EnvironmentNotice.kt`, the chat panes and `ui/design/States.kt`. It adds a shared `ReadinessPane` and `ReadinessRow` that render Waiting and Blocked consistently, then updates `docs/ux/design-system.md`, `docs/ux/workbench.md`, `docs/app/connection.md` and `docs/app/workbench.md`.

No Engine protocol change is required. Engine already reports error kinds, `environment.status` and transport closure. A future Engine notification for environment changes could replace polling but is outside this proposal.

## Verification and rollout

Host suites cover the classification, the backoff schedule, and the reconnect state machine with a fake bootstrapper. They also cover `EngineController` polling through injected failures and the action helper's behavior for each class. `app-unit`, `client` and `lint` must pass.

Device evidence uses an isolated API 28 AVD through `tools/with-emulator.sh`, never the daily tablet. `ConnectionBoundaryAcceptanceTest` is extended to kill the Engine process and assert that the reconnect card appears, input is inert, the Workbench recovers without user action and drafts are intact. A second case blocks reconnection to reach Offline. The flows for an environment that is preparing and one that has failed are exercised by opening a terminal and a conversation before readiness. Every instrumentation run asserts that the last-resort handler never ran.

Each task lands as its own commit after its checks pass, in dependency order: shared contracts, then platform robustness, action safety and UI states.

## Decision and completion

Accepted on 2026-10-04 with the recommended answer to each open question. The Workbench stays visible and inert during reconnection, reconnection is automatic with bounded backoff, and failures that reach the last-resort handler are logged and reported in every build.

The implementation follows the design with these differences. `Readiness` was not introduced as a separate type: the chat opening pane and the terminal notice present Waiting and Blocked directly from `ChatEnvironmentState` and `EnvironmentHealth`. Only Chat, the explorer, the chat rail, the launcher and shell commands were converted to `launchAction` with local reporting. Editor, terminal, proxy and settings launch sites rely on the scope handlers, which report a generic Snackbar instead of a local error row. Maintained behavior is described in [connection](../../app/connection.md), [Workbench adapters](../../app/workbench.md) and the [design system](../../ux/design-system.md#readiness-and-failures).

Host evidence: `app-unit` (including `FailureTest`, `WorkspaceRpcTest` and `AgentHubEnvironmentTest`), `lint`, `documentation` and `android-apk` pass. Device evidence on the isolated API 28 AVD: `ConnectionBoundaryAcceptanceTest` passes. It closes an online connection, sees the reconnect card over the inert Workbench, reconnects without user action with drafts and configuration intact, and leaves the `UnhandledFailures` defect count unchanged. `WorkbenchEngineAcceptanceTest#savedEnvironmentConfigBuildsAndConfirmedRestartReconnectsTerminal` also passes.

Two `WorkbenchEngineAcceptanceTest` failures predate this change and fail identically on its base commit. `integratedWorkbenchFileSessionsAndNavigation` asserts that the root listing hides `.workspace`, which the product now shows as a protected folder. `terminalRunsInsideEnvironmentAndAcceptsFilePathAlternative` allows 15 seconds for the first terminal panel, while a fresh workspace prepares its environment for longer.

Remaining limits: a new terminal still opens its panel only after the environment is usable, unlike a new conversation; the remaining launch sites still need local error rows; the backoff policy and the Offline state after the last attempt have no automated test; and a lost connection is observed only through store failure, so an Engine that hangs without closing its transport is not detected.
