# Codex login, terminal startup and blank file creation

The reported terminal failure was reproduced with the pinned Codex 0.157.1 Linux musl executable. Its default interactive startup requires a complete local daemon package, while Workflow supplies the standalone CLI. The Android filename `libcodex.so` is a packaging name for the unchanged ELF executable, not a shared-library conversion.

The Server now supplies a managed `/usr/local/bin/codex` launcher that passes `--no-daemon` and preserves the remaining arguments. A runtime bind overlays the image entry point, so existing extracted generations receive the correction when starting a new process. Chat still starts the bundled App Server directly. The launcher is included in Gradle's Engine input inventory.

The login adapter now reconciles pending login against account reads, matches completion notifications to the active login ID, and ends waiting on cancellation, process exit or a fifteen-minute deadline. An authenticated account refresh clears the old login flow; failed login feedback survives logged-out refreshes. Synthetic regressions cover notification loss, stale completion, timeout, cancellation and process exit. These checks establish recovery behavior, not the original account's network or authorization outcome.

Submitting an empty or whitespace-only new file or folder name cancels the inline creation row without sending a file operation. The confirmation button and keyboard Done action share that behavior. Empty renames retain their existing validation.

## Verification

All run directories below are under `artifacts/workflow/runs/coordinator/`. Their `run.json` files retain the exact source fingerprint, command and result. No source was edited during a recorded check.

| Check | Result | Run |
| --- | --- | --- |
| Agent JVM suite | 53 tests passed | `20261003T080532Z-f4b057a5` |
| Rust Server suite | 22 tests passed | `20261003T080643Z-cb7eec2b` |
| Android app and instrumentation build | Passed; immutable APK pair recorded | `20261003T080943Z-38b1f18d` |
| API 28 x86_64 emulator | Two tests passed, zero skips | `20261003T081046Z-5948b2bf` |

The emulator selected `FilesFeatureTest#blankCreationCancelsForButtonAndIme` and `EngineIntegrationTest#bundledCodexStartsFromLoginShellWithoutDaemonPackage`. The latter uses the packaged Server, runtime, loader, customized image and Codex executable under the Android app UID, starts a login shell, and reaches the interactive Codex welcome/login screen. It does not authenticate an account or run a model turn.

The app APK SHA-256 was `e4effb581c29e6bbba25314a9cdbd107a9a66a53aa71fc2996471f4350696465`; the instrumentation APK SHA-256 was `f89294fa8cecf838f95fe3f30190f636cf44c115c02f86c76e7a0723f14e592b`. These identify the emulator-tested Debug artifacts, not a tablet Release installation.

A separate host runtime comparison used a disposable copied root filesystem and isolated guest home with the same pinned binary. The original entry point produced the reported missing-package error; the corrected entry point reached the welcome/login screen. Version, resume-help, fork-help and App Server help also succeeded. Local evidence is in `artifacts/codex-login-fixes/`.

The first JVM run exposed an incorrect return type in the new JUnit tests and an existing generated coverage-table trailing-newline mismatch. Both were corrected before the successful run. The first host PTY probes also required a real terminal size and a valid isolated guest home; these harness failures are retained in the work history and are not device failures.

Real browser authorization completion, physical ARM64 tablet acceptance and authenticated model turns remain unverified. The daily tablet, its accounts, network settings and other applications were not modified.
