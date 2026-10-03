# Engine device/AVD acceptance harness

Runs `libworkflow-engine.so` inside a **real app process**: the instrumentation test of the harness app
`top.flysoftbeta.workflow.engineharness` executes in the app's own zygote-forked process, so every engine
run inherits Android's app seccomp filter and the `untrusted_app` SELinux domain (not `run-as`, not
`adb shell`). The engine is started with `ProcessBuilder` from `applicationInfo.nativeLibraryDir`.

| Path | Role |
| --- | --- |
| `engine/environment/runtime/tests/android-harness/` | standalone Gradle project (not in the root build): debuggable app, minSdk 28, targetSdk 37, `useLegacyPackaging` (extractNativeLibs=true); androidTest `EngineAcceptanceTest` |
| `engine/environment/runtime/tests/device/run.sh` | host driver: build harness APKs, install, provision rootfs, push cases, instrument, pull, summarize |
| `engine/environment/runtime/tests/device/mkrootfs.py` | OCI layout -> app-extractable tar (hardlinks copied, device nodes/FIFOs dropped, whiteouts applied, owner rwx on dirs) |
| `engine/environment/runtime/tests/device/summarize.py` | pass/fail table, G0 facts, nativeLibraryDir sha256 check, SIGSYS survey |
| `engine/environment/runtime/tests/device/sysprobe.c` | seccomp filter probe (built by `run.sh` with the NDK, packaged as `libwftest-sysprobe.so`) |
| `engine/environment/runtime/tests/device/cases/default.json` | initial case file |
| `artifacts/engine/device/<avd-or-serial>/` | results of the last run on that target |

## Running

```sh
engine/environment/runtime/test-android.sh
ONLY=guest-bash,guest-true engine/environment/runtime/test-android.sh
```

The entry point builds Rust runtime/loader, compiles the C test probes, and stages them under
`artifacts/engine/rust-harness/<abi>`. Only that test copy aliases `libworkflow-runtime.so` to the
harness's historical `libworkflow-engine.so` name. Production reserves the latter name for Server.
It then runs `test-app-sandbox.sh` via `tools/with-emulator.sh`, sharing the single device lock,
1536 MiB emulator limit and disk-backed ANDROID_TMP. The device driver accepts only that wrapper's
explicit emulator serial; it does not boot AVDs or run against the daily tablet.

`ONLY=ids` filters cases, `HARNESS_BUILD=0` reuses the previously built harness APKs,
`REPROVISION=1` rebuilds the isolated guest root, and `WORKSPACE_IMAGE=1` supplies the customized
workspace image for M5. The outer entry point sets `ENGINE_BUILD=0` because it already staged Rust.
Gradle uses `flock artifacts/.gradle.lock ./gradlew -p engine/environment/runtime/tests/android-harness ...`.

## What one run does

1. Gradle stages Rust executables and probes into nativeLibraryDir, preserving executable bytes for
   SHA-256 checks. No archived C implementation is linked or executed.
2. `mkrootfs.py` prepares the pinned OCI rootfs fixture. Upload uses `adb shell -T`, checks SHA-256,
   extracts as the harness UID, validates counts, then publishes `files/rootfs`. M5 additionally
   receives the real customized image and index; the basic fixture is not a production image fallback.
3. `am instrument -w -e cases cases.json` runs `EngineAcceptanceTest.runCases` in the zygote-forked
   app process, recording G0 facts and every case result.
4. Results are pulled to `artifacts/engine/device/<name>/`: JSON, stdout/stderr, instrumentation log,
   process listing, summary and SIGSYS survey. The summary verifies runtime/loader hashes.

The JUnit test fails on harness errors; individual case verdicts are in `results.json`/`summary.txt`.
Test compilation is not an on-device pass.

## G0 facts (`g0.json`)

API level/release/fingerprint, ABIs, `uname`, `Os.sysconf(_SC_PAGESIZE)`, pid/uid/ppid (the summary
names the parent from `ps.txt`, expected `zygote64`), `/proc/self/attr/current`, `/proc/self/status`
(`Seccomp`, `Seccomp_filters`, `NoNewPrivs`, `CapEff`, `CapBnd`, `TracerPid`, `Uid`, `Gid`), Yama
`ptrace_scope`, targetSdk/debuggable/extractNativeLibs, nativeLibraryDir with its SELinux label and a
listing (size, executable, sha256, label; the summary compares sha256 with the host build), the
rootfs state and label, and the `/data` mount line.

## Case file format (JSON)

```json
{
  "defaults": {"exe": "${LIB}/libworkflow-engine.so", "timeout": 60, "cwd": "${FILES}",
               "env": {}, "clearEnv": false, "noLeftovers": true},
  "setup": [{"copy": "${LIB}/libwftest-gueststatic.so", "to": "${ROOT}/guest_static", "mode": "755"}],
  "cases": [
    {"id": "guest-echo", "argv": ["run", "--root", "${ROOT}", "--", "/bin/echo", "hi"],
     "clearEnv": true, "env": {"PATH": "/usr/bin:/bin"}, "exit": 0, "stdout": "\\Ahi\\n\\z"}
  ]
}
```

Case fields (all but `id` optional; case values override `defaults`, `env` maps are merged and a JSON
`null` value removes a variable):

| Field | Meaning |
| --- | --- |
| `id` | `[A-Za-z0-9._-]+`, names `out/<id>.stdout` / `.stderr` |
| `exe` | program to start (default the engine) |
| `argv` | arguments after `exe` |
| `env`, `clearEnv` | environment changes; `clearEnv` starts from an empty environment (guest cases) |
| `cwd` | host working directory of the engine process |
| `timeout` | seconds; SIGTERM at the deadline, SIGKILL 6 s later; a timeout is a failure |
| `exit` | expected exit code or list of codes (absent = not checked). A process killed by signal N is reported as 128+N |
| `stdout`, `stderr` | Java regex (MULTILINE) that must be found in the full output |
| `stdoutNot`, `stderrNot` | regex that must not be found |
| `noLeftovers` | default true: fail if any process of the app UID other than the harness survives the case |
| `abis`, `minApi`, `maxApi`, `enabled` | skip conditions (verdict SKIP) |
| `tags` | `sigsys-survey` marks survey cases; `self-sigsys` excludes a case's SIGSYS lines from the app-filter list (the sigsys helper installs its own filter) |
| `note` | free text, copied to the result |

Placeholders in `exe`, `argv`, `env` values, `cwd` and setup paths: `${LIB}` nativeLibraryDir, `${ROOT}`
`filesDir/rootfs`, `${FILES}` filesDir, `${CACHE}` cacheDir, `${RESULTS}` results dir, `${ABI}` primary ABI.

Instrumentation arguments: `-e cases NAME` (file in filesDir, default `cases.json`), `-e only a,b`,
`-e results DIR`. To add cases without rebuilding: edit a case file before starting the wrapper and run the driver with `HARNESS_BUILD=0 ENGINE_BUILD=0` inside that wrapper. Never modify a running script.

## SIGSYS survey

Two independent measurements:

- **Filter probe** (`sysprobe-bare`, no engine): `libwftest-sysprobe.so` forks once per syscall; the child
  resets SIGSYS to SIG_DFL and calls it with arguments the kernel rejects (NULL/-1/0, nothing is
  created or changed). A child killed by SIGSYS = blocked by the app filter; ENOSYS without SIGSYS =
  the kernel lacks it. This works even where no glibc guest can start (16 KiB kernels).
  `sysprobe-engine` runs the same calls in-process under the engine; the summary checks that its SIGSYS
  lines equal the bare blocked list and that every blocked call came back as ENOSYS.
- **Workload survey**: cases run with `WORKFLOW_ENGINE_LOG=3` log `SIGSYS tid=N syscall=NR(name) -> ENOSYS`
  for every syscall the app filter trapped and the engine answered with ENOSYS. `summarize.py` collects
  them per target (names from the NDK uapi headers) into `summary.txt` and `sigsys.json`. Only syscalls
  the workload actually reached appear: an early failure hides later ones.

Evidence of the last run per target: `artifacts/engine/device/<target>/summary.txt` (+ `results.json`,
`out/`, `instrument.log`).
