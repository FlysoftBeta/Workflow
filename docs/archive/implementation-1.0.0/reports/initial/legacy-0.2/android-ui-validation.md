# Android UI validation

Environment: task-owned `emulator-5580`, Android 9 / API 28, 1080 × 1920 phone display. One final wide-layout screenshot uses a temporary 240 dpi density override on the same emulator; the original density was restored afterward. Only this emulator was modified. Screenshots are under `artifacts/ui-review/`. Final reviewed app version: 0.1.0.

The connection test uses the real Python daemon on the host, reached through `adb reverse` from the Android localhost endpoint. The app's real pairing-token check and `health.ready` response were exercised. This is not evidence that Termux installation or a native guest works on a physical device.

## Verified interactions

- Launcher initially shows only the three built-in apps. Third-party apps require explicit registration. The final launcher scrolls as one grid and all app tiles are reachable in landscape (`15-launcher-landscape-fixed.png`).
- The capability center scrolls on the phone and does not require optional device grants for runtime activation.
- The full-width editor accepts an exact two-line note. Force-stopping and reopening preserves its working-resource draft while the source file remains empty. Saving removes the draft and writes the exact text to `notes.md`.
- Files and Chat switches retain the same note; the file editor remains editable in the opposite paradigm’s side panel. The final landscape empty-state button is fully visible (`19-workbench-landscape-final.png`).
- Archiving a session with a new third-line draft presents an explicit preservation choice (`18-archive-draft-decision.png`). “保留草稿并归档” followed by restoration recovers all three draft lines while the source file still has the two saved lines.
- The authenticated “保存并检测” action reports connected and persists `activationComplete=true`; no activation fixture was injected (`09-activation-ready.png`).
- Android's overlay settings grant returns to the app. Enabling the dock starts a real foreground service and displays the overlay (`04-overlay-collapsed.png`, `05-overlay-expanded.png`).
- Dragging the dock to the left snaps it to that edge and persists `right=false`; outside tapping collapses the expanded panel (`06-overlay-drag-snapped.png`).
- The brightness settings grant returns as authorized. Moving the actual overlay slider changes `Settings.System.screen_brightness` from 102 to 203.
- SAF runtime export produces a valid 16,714-byte gzip archive containing 15 entries and `install-termux.sh`; its SHA-256 matches the bundled `.bundle` byte for byte.
- A temporary session was named through its menu, moved to the persistent section, and retained its name and current screen after rotation (`14-session-landscape.png`).
- Root probing reports that `su` is unavailable or denied on this emulator. No successful root operation is claimed.
- Repeated delivery of the same Settings destination works after HOME navigation. Rotating the Apps page retains Apps instead of replaying an earlier Settings intent.
- Wide-layout review shows the real file browser, multiple tabs, JSON syntax highlighting, and a separate conversation panel (`20-workbench-tablet-final.png`). “本机文件” and the editor status bar distinguish local files from the Termux runtime. The image displays container settings only; no pairing token is visible.
- The final overlay uses the app’s green palette and displays the real granted admin/brightness states (`21-overlay-final.png`).
- The device-admin grant shows the system force-lock disclosure. After activation, the overlay reports the admin grant; tapping lock changes `dumpsys power` to `mWakefulness=Asleep`. The test then wakes/unlocks the emulator.

## Defects found and fixed during review

- The first APK's fixed launcher header pushed apps out of the landscape viewport (`03-launcher-landscape-initial.png`). The launcher was changed to one scrolling grid with full-span headers.
- Android 9 rejects `ACTION_ADD_DEVICE_ADMIN` with `FLAG_ACTIVITY_NEW_TASK`. The grant now starts from the foreground Activity; the system grant and actual lock operation passed afterward.
- AAPT expands `.gz` assets and removes the `.gz` suffix. This made the first runtime-export attempt fail after creating an empty SAF document (`10-export-result.png`). The bundled archive was renamed to `.bundle` to preserve its gzip bytes, with the exported filename remaining `workflow-runtime.tar.gz`.
- Empty Sora editors were measured at only 33 pixels wide because the embedded Android view lacked `fillMaxWidth()`. The modifier was added; the rebuilt APK reports the full 1080-pixel editor width and passes input, saving, and draft-recovery checks.
- Activity recreation replayed an old navigation intent after rotation. Navigation and the controller/runtime are now retained through a ViewModel, and each delivered intent is consumed once.

## Outcome and limits

The executed Android UI checks pass; no blocking UI defect was observed in the final reviewed APK. Earlier screenshots with `initial` or defect descriptions record intermediate builds, not the final presentation.

Actual Termux package installation, physical-device root approval/grayscale/TUN execution, background behavior on vendor Android builds, and live Codex conversation turns were not verified by this UI pass. The connected-daemon activation check, offline rendering/protocol instrumentation, and native Android permission/control checks are separate evidence; none is represented as end-to-end native-guest certification.

Recommended final image: `artifacts/ui-review/20-workbench-tablet-final.png`. Phone landscape alternative: `artifacts/ui-review/19-workbench-landscape-final.png`.
