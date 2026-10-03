# Android device controls

The Android capability center is implemented in `ui/SettingsScreens.kt`; reusable permission and device operations live in `platform/DeviceCapabilities.kt`. Reading `snapshot()` never requests a permission or runs `su`. Optional permissions are independent of Termux activation.

## Floating controls

`WorkflowOverlayService.start(context)` requires the caller to be a foreground Activity and the user to have enabled `SYSTEM_ALERT_WINDOW`. It creates a `TYPE_APPLICATION_OVERLAY` window and an ongoing foreground notification. Android 14+ uses the manifest-declared `specialUse` foreground-service type; Android 9 uses the ordinary foreground-service overload. It does not start at boot or restart after force-stop.

The collapsed dock can be dragged and animates to the closest edge. Its position is stored under `.workspace/.state/overlay-position.json`. The expanded panel consumes outside touches and collapses on outside tap or Back. Screen-off hides it; unlocking restores it. Revoking the overlay app-op stops the service. The ongoing notification can stop it at any time. `running: StateFlow<Boolean>` represents actual service lifetime; the settings switch uses this instead of claiming that a saved preference implies a running service.

Each expansion reads `WorkspaceRepository(filesDir/.workspace)` off the main thread. Only IDs in `config.quickApps` that still exist in `state.apps` are shown. Apps are added explicitly in the launcher; its context menu adds/removes quick-switch entries. External apps use the package manager's launch intent. Built-ins deliver an explicit `MainActivity` intent with `destination=WORKBENCH|PROXY|SETTINGS`; the manage action uses `APPS`.

## Optional access

| Capability | Permission / mechanism | Behavior without access |
| --- | --- | --- |
| Floating controls | `SYSTEM_ALERT_WINDOW`, granted in system settings | The service is not started; settings opens the corresponding grant page. |
| Global brightness | `WRITE_SETTINGS`, granted in system settings | The expanded panel offers the grant page. Moving the slider switches to manual brightness and writes the system value. |
| Lock screen | `DevicePolicyManager` with the declared force-lock receiver | A user lock action may try `su` instead; failure is surfaced. The capability center can revoke the admin grant. |
| Monochrome display | Explicit `su -c` execution as UID 0 | Actual root denial/timeout is shown. The UI never treats `su` presence as authorization. |
| Running notification | Notification runtime permission on Android 13+ | Optional; the foreground service remains subject to Android's task-manager display rules. |

The root executor has a 15-second timeout and bounds captured output. Root checking is an explicit user action. Grayscale writes and reads back Android's secure daltonizer settings; prior color settings are preserved in `.workspace/.state/device-controls.json` for restoration. Root lock uses the sleep key event and checks that the screen became noninteractive. Device-admin lock uses `lockNow()`.

The root grant checked here is for the Android application. A Termux daemon running a root TUN requires its own working root execution path. Neither a checked switch nor a discovered `su` executable certifies that a proxy TUN is active.

## Environment activation

Termux is mandatory for the runtime. The app saves `activationComplete` only when the authenticated runtime health endpoint reports `ready=true`; Codex installation is optional at activation. A successful historical activation allows reopening existing local work while the service is offline. Optional overlay, notification, brightness, and admin grants do not block activation.
