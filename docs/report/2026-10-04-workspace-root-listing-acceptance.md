# Protected `.workspace` root listing on the emulator

`WorkbenchEngineAcceptanceTest#integratedWorkbenchFileSessionsAndNavigation` asserted that the root listing never contains `.workspace`. That contradicts the current contract, in which `.workspace/` always appears first in the root listing as a protected folder, as the FileWork test `root_listing_shows_the_protected_workspace_folder_with_only_allowlisted_children` establishes on the host. The assertion failed on base `725da0e` and stopped the test before any later step ran.

## Test changes

The test now checks the contract through the packaged Engine:

- With hidden files both off and on, the first root entry is the `.workspace` directory, and the explorer row carries its protected-folder description.
- The Engine refuses to move `.workspace`.
- The `.workspace` listing contains `config.json` and only allowlisted entries (`agents`, `proxy`, `services`, `config.json`), although `state/` exists on disk; listing `state`, `environment` or `cache` is refused. The run logged that `cache/` existed on disk.
- Credential-named sentinel files (`auth.json`, `.credentials.json` and `.claude.json` with placeholder content, never real secrets) are written into the agent homes only when absent, are absent from the home listings and refused by `openFile`, and are removed afterwards with any home directory the test created.

Nothing else in the method referred to `env.json`, the former tools location or upload staging, so commit `d9ee2f9` left no other stale assertion. With the first failure gone, the later steps exposed a timing fault: the session archive's Snackbar (long duration, with an undo action) could still be current at the trash step, so the test could read it and press its undo. The trash step now waits for its own "已删除「…」" message and checks that the file is gone before pressing undo.

## Verification

All runs are under `artifacts/workflow/runs/coordinator/`. Device runs used the wrapper-managed `workflow-tablet-api28` AVD (API 28, x86_64) with only the method above selected; the daily tablet was not used.

| Source | APK build | Device run | Result |
| --- | --- | --- | --- |
| `725da0e` with this change | `20261004T132304Z-e261cca6` | `20261004T132733Z-d221cf56` | Failed at the chat step |
| `bf18a63` (`main`, including readiness `aed790e`) with this change | `20261004T133638Z-c1aa6911` | `20261004T134005Z-a32af6a0` | Passed, 1 test, zero skips |

On `725da0e`, every step through the protected listing, editing, conflict, archive, recreation, layout, trash/undo, Settings and Proxy passed. `newConversation` then failed with "chat requires a ready environment" because the Server refuses chat while the environment prepares, a gate from `b44bf22` rather than `d9ee2f9`. The readiness commit `aed790e`, now on `main`, makes chat commands wait for the environment and opens a new conversation at once; on `main`, the whole method passed, including the native chat panel and attachment.

The base-run APK SHA-256 values were `102e4e99ae04ac82c5505ed1ac9ca7e773f180da5cf718f1f6ea326e46d5b471` (app) and `459d0ee25ca9289b5eb17e86c0ba22107a0d3b261048d945963949e451f48e9e` (test); the `main`-run values were `e3ba254e7eab8f834c0d033a3f02f8c1bf091fc93517dd8226fc98121b8a0def` and `b33d3e4233b3f2ec629043756c01d3cea6bee6d619a5e973e67522cd4c2e8e1f`.

This establishes the protected listing and allowlist visibility through the app on that emulator. It does not cover guest masking on a device, cache removal and rebuild, the declaration build path in `config.json`, physical ARM64 or newer Android versions.
