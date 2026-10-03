# W8: settings and floating controls

Implements `docs/ui.md` §4.11/4.13 and `docs/product.md` §9/11. The settings provider replaces the last Settings placeholder through `PanelWiring`; feature packages do not import one another.

## Ownership and behavior

- `feature/settings`: adaptive categories (220dp category column at 720dp; otherwise drilled-in detail and Back in the existing panel header), theme/density/shared editor-terminal font size, accounts, environment, overlay, default Home, permissions, version and bundled license viewer. Settings writes use `WorkspaceStore.updateConfig` and surface invalid-config/write failures. Slider preview is transient and commits on release.
- Accounts use `AppGraph.agentHub` plus usable environment health. They never install or start a host fallback. Codex offers its supported device-code/browser/API-key methods; Claude offers its actual terminal/browser or token/API-key methods. Login URLs open only HTTP(S); secrets stay in unsaved input state and are cleared on submission/dismissal. System browser completion and terminal authentication refresh are explicit. No credentials or agent requests are approved automatically.
- Environment is a read of `EngineController.health`; retry appears for failed builds, restart only for `NeedsRestart` and after a concrete interruption confirmation. Existing usable generations remain accurately labeled if a later build fails.
- `platform/apps/InstalledApps` is the single process cache of launchable apps, labels and icons, shared by Launcher/settings/overlay; package labels and icons load on IO. The former feature-local catalog moved to platform, keeping feature boundaries intact.
- `WorkflowOverlayService` owns its native 48dp touch dock and Compose/M3 Expressive expanded panel. Position persists below `.workflow/overlay` on IO using atomic replacement and finite clamping. The 44dp glyph circle rests half offscreen, fades after 3 seconds, enlarges on drag, and uses a damped 280ms edge snap. Pending agent requests add a dot; lock/screen-off hides the dock. Rotation preserves side and vertical fraction.
- Expanded controls show four columns and at most three visible app rows; scroll includes launcher order, deduplicated extras, then the edit entry. Outside, Back, Home and screen-off dismiss. Edit deep-links to Settings/Overlay/Extra apps. Grayscale and lock are disabled until measured capability is available and have explicit grant controls. Operations read back actual system values; inline errors expire after 2 seconds and announce through a polite accessibility live region. Lock dismisses before the system action; if the action fails while the device is still unlocked, the panel reopens with the real error. Service failures that cannot use the overlay remain visible in its settings. No settings/overlay error uses a Toast.
- Capabilities are permission reads, explicit root probes, and verified actions. Brightness checks manual mode and actual stored brightness. Grayscale keeps/restores the preexisting daltonizer setting under `.workflow/device`; su exit status and readback must agree. Both device-admin and root lock paths verify noninteractive power state. No legacy `.state` path remains.
- Overlay's running switch combines actual service state and overlay AppOp. Saving an enabled preference is not presented as a successful service launch. Foreground restoration uses `restoreIfEnabled`; no boot receiver or background auto-grant.
- The screen/Home receiver uses `ContextCompat.registerReceiver(..., RECEIVER_NOT_EXPORTED)` on every API level. Real system broadcasts still dismiss/hide the controls; other apps cannot send arbitrary dismissal broadcasts.

## Verification

Host build/unit evidence and device acceptance are separate. Only disposable API 28 emulators are used; no daily-device permission, route, application or system setting is touched.

- `:app:assembleEmulatorDebug` and `:app:assembleEmulatorDebugAndroidTest`: pass during implementation (`artifacts/w8-build.log`).
- `OverlayPositionTest`: pass; finite, overflow and corrupt saved fractions remain on screen (`artifacts/w8-tests-build.log`).
- Final build, unit test and lint: pass (`artifacts/w8-final/rebuild.log`, `artifacts/w8-final/final-checks.log`; lint has zero errors). The final test runner requires both an explicit `settingsEmulator=true` flag and emulator hardware, so capability mutation tests are skipped on a daily device.
- Final API 28 instrumentation: **13/13 passed, zero skips**, on the disposable `workflow-tablet-api28` (1920×1200 at 261 dpi), including rotation to portrait. `artifacts/w8-final/instrumentation.log` reports `OK (13 tests)` in 32.756 seconds. `artifacts/w8-final/logcat.txt` has no fatal application exception. APK fingerprints are in `artifacts/w8-final/apk-sha256.txt`.

| Acceptance area | Measured result |
| --- | --- |
| Settings navigation and layout | Narrow category/detail Back and header action; wide two-column categories; external Extra apps entry passed. |
| Appearance | Theme, density and the shared font-size slider write through the store; font size 18 is read back. |
| Accounts/environment | Both login buttons are disabled while the actual environment is unusable; its state is displayed and no restart action is offered. No host authentication process or credential was used. |
| Permissions, Home, About | Six permission rows, current Home resolver state, version 1.0.0 and the bundled licenses render and pass assertions. |
| Overlay permission and dismissal | Real overlay AppOp grants permit the service; revocation stops it and a new start is rejected. Outside, Back, Home, screen-off and wake transitions passed. |
| Overlay position | A real drag snaps left, writes side/fraction, remains on the same edge after rotation, and restores after service restart. |
| Brightness | Permission denial causes failure; after granting the actual AppOp, manual mode and brightness 91 are read back. Previous mode/value/permission are restored. |
| Lock | Device-admin grant causes actual noninteractive power state and is revoked afterward. Revoking the grant after expansion exercises a real failed lock: the panel reopens with an inline error and the error disappears after two seconds. |
| Root and grayscale | The stock emulator denies app-level `su`; this is reported as `UNAVAILABLE`. Grayscale correctly fails and its system value stays unchanged. This is denial-path acceptance, not a successful root grayscale or root lock claim. |

Reproduction (all device execution is serialized, bounded to 1.5 GiB, and stopped by the wrapper):

```sh
WORKFLOW_EMULATOR_PORT=5850 tools/with-emulator.sh bash artifacts/w8-final/accept.sh
```

The script installs the preserved APKs with `--no-streaming`, runs `SettingsFeatureTest`, `DeviceControlsEmulatorTest` and `OverlayEmulatorTest` with `-e settingsEmulator true`, and rejects skipped tests. A final **5/5 settings-only pass** (`artifacts/w8-final/settings-screens.log`, 8.736 seconds) captures the settled Compose root directly, avoiding incomplete native test-window transition frames in the earlier system screenshots. It is reproducible through the same wrapper with `artifacts/w8-final/settings-screens.sh`.

The final screenshots under `artifacts/w8-final/screenshots/` were visually checked: `settings-wide-appearance.png`, `settings-narrow-categories.png`, `settings-accounts-environment-gate.png`, `settings-environment-not-ready.png`, `settings-permissions.png`, `settings-about.png`, `overlay-expanded.png`, `overlay-dragged-left.png`, `overlay-rotation.png` and `overlay-lock-failure-inline.png`. Settings captures show their actual panel bounds; overlay captures include the actual system window. `device-results.json` records the real capability outcomes.

The supplemental `settings-in-workbench.png`, captured immediately after instrumentation shutdown and an external MainActivity launch, still shows the launcher transition. It is not deep-link acceptance evidence; the app-shell acceptance run owns the full Activity-restoration check. The isolated settings panel and service acceptance above do not depend on that supplemental screenshot.

An initial concurrent emulator run filled the host `/tmp` user quota, causing a Kotlin incremental-cache write failure. That attempt is not acceptance. Later emulators place temporary qcow files on disk through the shared wrapper using `ANDROID_TMP` (TMPDIR alone does not affect qcow overlays), and use bounded memory/serial device execution.

## Boundaries

A real account OAuth roundtrip needs user credentials and is not fabricated by UI testing. A default-home choice and permissions are reported from Android after returning from its own settings. Successful root grayscale/root lock, API 33+ permission flows and OEM-specific behavior remain outside this API 28 stock-emulator acceptance. No daily-device setting, app, route, DNS, PIN or credential was touched.
