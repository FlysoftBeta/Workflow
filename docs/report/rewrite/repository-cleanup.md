# Repository cleanup after the Rust rewrite

Date: 2026-10-03. This change reorganizes production/reference boundaries and current documentation.
It does not alter the Server protocol, connection implementation, editor implementation or release tools.

## Production boundary

- `core/environment` now contains only the shell's `EnvironmentStatus`, `Stage` and `ActivationReason`
  types. The former Kotlin `EnvironmentReconciler`, `EngineStore`, planner, image/config records and
  codecs are in `core/src/testFixtures/`, alongside the frozen reference workspace writer.
- `JvmFileSystem`, `FileWatcher` and `ManualFileWatcher` have no production callers and are also test
  fixtures. The only AndroidFileSystem caller is an instrumented reference-store terminal test, so that
  adapter moved to `app/src/androidTest/`. The unused AndroidFileWatcher is archived.
- The production `core.jar` was inspected: no reference workspace writer, environment writer/planner,
  JvmFileSystem or FileWatcher classes remain. Shared status classes are present. The existing
  test-fixtures dependencies preserve both JVM and instrumented reference tests.
- `web/` now vendors only xterm, its fit/web-links addons and core-js. The chat/Markdown source,
  generated pages and marked/KaTeX/DOMPurify assets, npm lockfile and licenses are preserved in
  `docs/archive/web-chat/`. Those dependencies are absent from the current npm lock and APK asset tree.
  Native Compose chat dependencies and notices are unchanged.
- The full obsolete C engine/loader/generator and licenses are preserved in `docs/archive/native-engine/`.
  Active C probes, guest oracles and Android harness moved to `engine/runtime/tests/`; they are tests,
  not a production C runtime. `engine/runtime/test-host.sh` is now the current host acceptance entry,
  and it only builds Rust. Android test scripts use the new paths and the shared emulator wrapper.
  The archived independent AVD boot script is not an active entry point.
- Unreferenced old `runtime/proxy` example configuration is archived under `docs/archive/legacy-runtime/`.
  Existing ignored runtime artifacts were left intact.

## Documentation

README, AGENTS and current product/architecture/workspace/engine/testing/status/dependency docs now
agree on `<workspace-root>/.workspace/`, Engine ownership, three Rust executables, complete customized
images for both ABIs, native Compose chat and the retained xterm WebView. AGENTS no longer has
contradictory pre-rewrite state/path ownership rules. Current configuration references use env.json.

Obsolete architecture and state-layout documents are preserved in `docs/archive/initial/`.
`docs/report/README.md` indexes the existing initial/rewrite evidence and records the old-to-new source
paths; historical reports and their original test counts/hashes were not rewritten. Current Markdown
navigation links were checked. Long-lived docs distinguish implementation, host tests, emulator tests,
real-account/device acceptance and the remaining 16 KiB amd64 glibc limitation.

## Verification

- `flock artifacts/.gradle.lock ./gradlew :core:test :core:jar :app:compileEmulatorDebugKotlin
  :app:compileEmulatorDebugAndroidTestKotlin`: successful after all Kotlin source moves.
  Core JUnit XML: **212 tests, 0 failures/errors/skips**. Log:
  `artifacts/repository-cleanup/gradle-final.log` (first Gradle invocation).
- `ENGINE_HOST_BUILD="$PWD/artifacts/repository-cleanup/runtime-host" engine/runtime/test-host.sh`: **68/68 pass** after deterministic image-clock fixture setup. Full path/metadata/identity, legacy/SIGSYS and plain/fast oracles run against Rust. Log: `artifacts/repository-cleanup/runtime-host-final.log`.
- Moved standalone harness `assembleDebug assembleDebugAndroidTest` with `ANDROID_HOME` and
  `-PengineOut=<absolute-repo>/artifacts/engine/rust-harness`: successful, 68 tasks. APK runtime/loader bytes exactly
  match the staged Rust executables. Log: `artifacts/repository-cleanup/harness-build.log`.
  An earlier direct invocation omitted ANDROID_HOME and correctly failed SDK discovery; the device
  driver already exports it. No on-device run is claimed by this cleanup.
- `npm run build` and `npm test`: **17/17 terminal tests pass**. All four current vendor packages'
  shipped bytes match their SHA-256 manifest. Retired chat dependencies are absent from package-lock.
  Logs: `artifacts/repository-cleanup/web-tests.log`, `asset-boundary.txt`.
- Rust syscall generator `--check`, shell syntax checks and regenerated M5 JSON consistency pass.
  Active scripts no longer reference `native/engine` or its generator/build outputs.

## Image-clock fixture correction

The first full relocated host suite passed 64/68 checks. All four failures were byte-identical except
for `chage`'s last password-change date: the new customized image was built on October 3 while the
existing real-kernel golden records September 27. The guest identity fixture now explicitly sets that
single disposable account field to 2026-09-27 before its assertions, and fails setup if that operation
fails. The semantic golden is unchanged. The M5 case generator embeds the same fixture, so both host
and app-sandbox runs use the deterministic input. No production account or image was changed.
