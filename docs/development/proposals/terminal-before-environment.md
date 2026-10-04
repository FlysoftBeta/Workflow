# Opening a terminal before the environment is ready

Status: accepted, implemented; verification pending. Owner: coordinator. Updated: 2026-10-04.

## Problem and intended outcome

The [readiness proposal](readiness-states.md) requires that a new terminal or conversation open its panel immediately and show the waiting state inside it. Conversations do this, but terminals do not. `TerminalPanelProvider.newTarget` calls `TerminalHost.create`, and `EngineTerminalBackend.create` waits for `EngineController.awaitUsable()` before it sends `terminal.create`. New terminal therefore shows nothing until the environment is usable, which can take more than a minute on a fresh workspace. If the environment fails during that wait, the click ends in a Snackbar and no panel.

The intended outcome is that New terminal, Open in terminal and Paste path into a terminal open a terminal panel at once. That panel shows the environment's preparation or failure at its top, using the existing notice. When the environment becomes usable, the shell starts in the same panel without another action. A path delivered to the new terminal is pasted once the shell runs, instead of being lost.

Engine continues to own terminal identity, and no process starts outside a verified environment.

## Experience and design

The terminal tab appears immediately, titled Terminal n with the ordinal Engine assigned. While the environment prepares, the [design-system](../../ux/design-system.md#readiness-and-failures) notice at the top of the terminal shows Preparing environment with its progress, and the terminal body stays empty. Input is ignored until the shell runs, special keys do nothing, and path drops onto the panel are not offered.

If the build fails, the notice shows the failure with Details and Retry. The terminal does not fail separately. Retry is the single action that unblocks it, and a successful retry starts the shell. Closing the panel while it waits only removes the view, as it does for any terminal.

A path delivered by Paste path or a drop onto an empty terminal region waits for the shell and is then pasted. If the shell cannot start, the path is discarded, matching an ended terminal.

## Implementation and ownership

The panel target must carry an Engine terminal ID, because the Engine-owned layout persists it and Engine reaps unreferenced terminals by it. Engine refuses `terminal.create` while no environment is active, so a client-side placeholder or client-proposed ID would be needed to open the panel early. Both would move identity ownership to the client. The smallest contract change instead lets Engine allocate the identity before the environment is usable.

**Contract (Engine Terminal and protocol).** `terminal.create` always allocates and persists the resource. When an environment is usable, it starts the process and returns `running` metadata, as before. Otherwise it returns `starting` metadata without a process, recording the requested directory, rows and columns. `terminal.attach` keeps its rule: it starts a resource that has no process, has not failed and has no exit code, which now includes a `starting` resource. It still fails with `environment_unavailable` while no environment is usable. The `starting` status already exists in the schema, so no method, field, notification or schema change is needed. A Server restart reloads a `starting` resource as `ended` without an exit code, which attach rehydrates in the same way.

**Client (`feature/terminal`, `platform/engine`).**

- `EngineTerminalBackend.create` sends `terminal.create` without waiting for the environment.
- `EngineTerminalBackend.attach` waits with `EngineController.awaitUsable(waitThroughFailure = true)` before `terminal.attach`. That wait enrolls an unused environment and waits through a failed build that the user can retry.
- `TerminalHost.create` attaches a `starting` resource instead of reading it, and `TerminalSession` maps `starting` to `TerminalStatus.Starting`.
- A path dropped or pasted onto a terminal waits for the session to leave `Starting`.

Because attachment no longer fails on an unusable environment, the terminal's separate environment-failure row and its Retry are removed. The notice presents that state.

## Verification and rollout

- **Rust:** `rust-server` covers a terminal created without a usable environment. It is persisted as `starting` with no process, attach is refused with `environment_unavailable`, and nothing is reaped.
- **Host:** `app-unit` and `lint` cover the client.
- **Device:** the isolated API 28 AVD runs `WorkbenchEngineAcceptanceTest#terminalRunsInsideEnvironmentAndAcceptsFilePathAlternative` on a fresh install. Its panel assertion allows 15 seconds while the environment prepares for longer, so it fails on base commit 725da0e and should pass with this change. `savedEnvironmentConfigBuildsAndConfirmedRestartReconnectsTerminal` guards the restart path.

## Decision and completion

The Engine-allocated `starting` resource was chosen over a client placeholder or a client-proposed ID, because it keeps identity with Engine and needs no schema change. Maintained behavior is described in [Engine Terminal](../../engine/terminal.md), the [protocol](../../engine/protocol.md#terminal-resources), [Workbench adapters](../../app/workbench.md#editor-and-terminal), [Workbench UX](../../ux/workbench.md#terminal-interaction) and the [design system](../../ux/design-system.md).
