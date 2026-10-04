# Opening a terminal before the environment is ready

New terminal previously opened nothing until the environment was usable, because `EngineTerminalBackend.create` waited for it before sending `terminal.create`. On a fresh workspace this took longer than the 15 seconds that `WorkbenchEngineAcceptanceTest#terminalRunsInsideEnvironmentAndAcceptsFilePathAlternative` allows for the panel, so that test failed on base commit 725da0e and on the readiness commit aed790e.

Commit 8aa2f80 implements the [proposal](../development/proposals/terminal-before-environment.md):

- Engine's `terminal.create` allocates and persists the terminal whether or not the environment is usable. Without one, it returns `starting` metadata without a process, and the first `terminal.attach` after the environment becomes usable starts the shell.
- Android opens the panel with that Engine ID at once. It attaches a `starting` terminal instead of reading it, and waits for the environment, including through a failed build that the user can retry from the terminal's notice.
- A path pasted or dropped into the new terminal waits for its shell.

The wire schema is unchanged, because `starting` already existed.

## Verification

All runs are under `artifacts/workflow/runs/coordinator/` and were taken against the primary checkout at 8aa2f80, unless stated otherwise. No source was edited during a recorded check.

| Check | Result | Run |
| --- | --- | --- |
| Rust Server suite | Passed, including `creation_without_a_usable_environment_keeps_identity_until_attach` | `20261004T131509Z-1af46e67` |
| App unit suites | Passed, including both `TerminalHostTest` cases | `20261004T131606Z-4e6dff1f` |
| Lint | Passed | `20261004T131851Z-bbf233ff` |
| Android app and instrumentation build | Passed; immutable APK pair recorded | `20261004T132200Z-dea5b62f` |
| Documentation | Passed in a linked worktree at the same commit | `20261004T131655Z-c35bd862` |
| API 28 x86_64 emulator | Four tests passed, zero skips | `20261004T132610Z-d701d957` |

The documentation run in the primary checkout, `20261004T131509Z-dc544ba5`, failed only on pages inside nested worktrees under `.claude/worktrees/`. All 234 reported links belonged to those copies. The checker now skips nested checkouts.

The emulator run used `emulator-5860` (ranchu, API 28) with app data cleared before instrumentation. It ran `terminalRunsInsideEnvironmentAndAcceptsFilePathAlternative` first, followed by `savedEnvironmentConfigBuildsAndConfirmedRestartReconnectsTerminal` and both `TerminalAttachmentTest` cases.

In the first test, the environment was prepared from scratch:

| Time | Event |
| --- | --- |
| 21:28:10.9 | The terminal panel's page loaded |
| 21:28:23.5 | The first guest process started |
| 21:28:27.4 | The terminal reported Running |

The panel was therefore shown about 16.5 seconds before the shell ran. The test then pasted a quoted path through the menu alternative, and the shell wrote it into a workspace file. The restart test confirmed that a saved declaration rebuilt the environment and that the confirmed restart kept the terminal's cwd, its panel and an unsaved draft. Logcat recorded no `UnhandledFailures` defect.

The app APK SHA-256 was `6f13eafe03894dea949ad432a39e6760a1e87fa55918f4b7d7403cdcfa8f97b4`, and the instrumentation APK SHA-256 was `f7515a303dc19be72f9dc96db53aa718b4d25cf704e006449761e711998ce764`. These are emulator-tested Debug artifacts.

## Limits

- **Waiting through a failed build.** No device test drives this path: the panel waits, the user retries from the notice, and the shell starts. It is covered only by the client's structure and the host tests.
- **Paths delivered while waiting.** No instrumented test drops or pastes a path into a terminal that is still waiting.
- **Untested hardware.** The physical ARM64 tablet, rotation and newer Android versions were not exercised. The daily tablet was not used.
