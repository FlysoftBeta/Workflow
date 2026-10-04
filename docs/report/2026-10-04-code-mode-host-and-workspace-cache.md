# Code Mode host, editor highlighting and the `.workspace` cache

Branch `claude/happy-curie-6k434r`, based on `b14bd86`. Environment: cloud Linux container (Ubuntu 24.04, x86_64, no KVM) with Rust, Java 21, Gradle, and an Android SDK installed for this work at `/opt/android-sdk` (platform 37.0, build-tools 36/37, NDK 30.0.15729638); no emulator, customized image or Engine Android build. `tools/with-build-lock.sh` wrapped heavy Cargo and Gradle commands; Cargo used two jobs.

## Codex Code Mode host

Codex 0.157.1 reported "Code Mode is unavailable because failed to spawn code-mode host `/opt/workflow/tools/codex/bin/codex-code-mode-host`: host executable was not found. Code mode will fail closed". Codex's install context resolves `codex-code-mode-host` beside its own executable, and the tools payload carried only `codex`.

- `third_party/codex/manifest.json` pins the `codex-code-mode-host-{aarch64,x86_64}-unknown-linux-musl.tar.gz` assets of the same `rust-v0.157.1` release, with archive and binary SHA-256 and sizes. Each binary's digest matches the `hashedrekord` entry of the release's `.sigstore` bundle, its signature verifies against the bundle certificate, and that certificate names `openai/codex/.github/workflows/rust-release.yml@refs/tags/rust-v0.157.1`.
- `engine/tools/package.py` packages it as `codex/bin/codex-code-mode-host`; Environment's catalog validation and extraction and `tools/build-release.sh` require it.
- Checks: `cargo test -p workflow-environment tools::` passed, including a catalog without the host being rejected. `package.py --architecture amd64` produced a verified 135 MB payload containing both binaries. A minimal client for the host's length-prefixed stdio protocol opened a session and ran JavaScript (`[1,2,3]` squared and summed, result `14`), both natively and inside the Rust runtime (`workflow-engine run`) on the host.
- Not established: Code Mode on an Android device. Under `--test-app-filter` on this desktop host both the host and Codex itself abort, so that simulation does not represent the device, where Codex already runs.

## Editor highlighting

The user saw supported files (Python, JSON, Kotlin and others) without colors in the light theme, and later reported that highlighting works again without a change. The cause was not reproduced:

- The bundled grammars and themes tokenize real files with distinct colors in tm4e on the JVM. sora-editor 0.24.6's `ThemeRegistry`, `GrammarRegistry` and `TextMateColorScheme`, replayed in the app's exact call order, resolve the same colors.
- Under Robolectric, the real sora `CodeEditor` with `SoraGrammars.kt` and `EditorScheme.kt` colored every supported type in both themes, including after editor release, reopening, theme switches, edits and reloads.
- D8 at `--min-api 28` leaves no Java API newer than Android 9 in sora or its dependencies; only `RenderNode` and input APIs behind version checks remain.
- One candidate remains unverified: sora's `LockedSpans` readers fall back to an uncolored span when the analysis thread holds the spans lock, and the API 29+ `RenderNode` line cache, used only without word wrap, can retain such a line. The earlier device acceptance opened a wrapped Markdown file on API 28, where that path is not used.

`TextEditorController` now logs, under tag `WorkflowEditor`, a grammar loading failure or a failed TextMate setup, which previously left a file plain without a trace. The palette and editor behavior are unchanged.

## `.workspace` layout

At the user's request, the top level of `.workspace/` holds only configuration and persistent data, and everything rebuildable moves to the private `cache/` directory. By the user's decision there is no migration.

- The environment declaration is the `environment` section of `config.json`, typed in `ClientConfig` and `ConfigPatch` (whole-value replacement). New workspaces write it with empty package, variable and post-script lists; a configuration without it declares the image defaults and stays without it when other settings are saved. Change detection hashes only that section canonically, so other settings never schedule a build. Saving `config.json` through file APIs validates the declaration; settings updates only require it to parse. Validation messages now name `environment.<field>`.
- `cache/` holds `tools/` (payload, launchers, optional Claude, archive copy and tool state), `images/`, `generations/`, `toolchains/`, `network/resolv.conf` and `uploads/`. `environment/` keeps the lifecycle record and the persistent `stores/home/work`.
- When the Engine starts and an activation's generation or the toolchain store is missing, it forgets that activation. A lost active generation makes the environment unavailable with stage `cache_removed`, and declaration monitoring rebuilds it. A tools payload removed while the Engine runs is extracted again.
- The allowlist drops `env.json`, `uploads` and the read-only `agents/tools` class and adds private `cache`, which the guest also masks. A former `env.json`, `agents/tools/` or generations below `environment/` are ignored, so an existing workspace rebuilds its environment from defaults until the declaration is copied into `config.json`.
- Settings "编辑环境配置" opens `config.json` at the `environment` key. The exported contract and Kotlin bindings were regenerated.

Checks:

- `cargo test --manifest-path engine/Cargo.toml --locked --workspace -j 2`: 226 passed, 0 failed, including new tests for the declaration section, change detection, cache removal, an absent section surviving settings updates, and a bad declaration refused by file saves without blocking settings.
- `engine/server/tools/test-protocol.py` protocol and lifecycle fixtures: passed. The real-image `test-environment.py` was updated but not run, because no customized image is available here.
- The guest-mask checks of `runtime/test-host.sh` suite `m2`, run by hand with the host runtime and a minimal rootfs binding the host `/usr`, passed: agent credentials readable only at their home, `agents` and `cache` aliases masked, workspace walks reach no credential, only `config.json` listed, and tools mounted at `/opt/workflow/tools`. The full suite needs the pinned Debian rootfs and was not run.
- `cargo clippy --all-targets` for Environment, FileWork, Server, Workspace, Terminal and Chat: the same warnings as the base commit. The optional declaration is boxed so `ClientConfig` does not trip `large_enum_variant`.
- `tools/check-app-unit.sh`: `:app:android` 92, `:app:client` 245 (one existing skip) and `:app:proxy` 85 tests passed. `:app:android:compileX86_64DebugAndroidTestKotlin` passed.
- `tools/generate-client-protocol.py --check`, `tools/check-docs.py`, `test_engine_boundaries.py` and `test_workflow.py` passed. `test_client_boundary.py` has the same `AgentPaths.kt` failure as the base commit.

## Limits

No APK was built or installed, so the larger tools payload, Code Mode in a guest on a device, the editor change and the new layout are untested on Android, as are the instrumentation tests updated for `saveEnvironmentDeclaration`. Real-account Codex turns, isolated API 28 acceptance and physical ARM64 remain open as before.
