# Engine device/AVD acceptance harness

Runs `libworkflow-engine.so` inside a **real app process**: the instrumentation test of the harness app
`top.flysoftbeta.workflow.engineharness` executes in the app's own zygote-forked process, so every engine
run inherits Android's app seccomp filter and the `untrusted_app` SELinux domain (not `run-as`, not
`adb shell`). The engine is started with `ProcessBuilder` from `applicationInfo.nativeLibraryDir`.

| Path | Role |
| --- | --- |
| `native/engine/android-harness/` | standalone Gradle project (not in the root build): debuggable app, minSdk 28, targetSdk 37, `useLegacyPackaging` (extractNativeLibs=true); androidTest `EngineAcceptanceTest` |
| `native/engine/device/run.sh` | host driver: build engine + APKs, install, provision rootfs, push cases, instrument, pull, summarize |
| `native/engine/device/mkrootfs.py` | OCI layout -> app-extractable tar (hardlinks copied, device nodes/FIFOs dropped, whiteouts applied, owner rwx on dirs) |
| `native/engine/device/summarize.py` | pass/fail table, G0 facts, nativeLibraryDir sha256 check, SIGSYS survey |
| `native/engine/device/sysprobe.c` | seccomp filter probe (built by `run.sh` with the NDK, packaged as `libwftest-sysprobe.so`) |
| `native/engine/device/avd.sh` | create/boot/stop `engine-*` AVDs headless; `register` re-announces an emulator to a restarted adb server |
| `native/engine/device/cases/default.json` | initial case file |
| `artifacts/engine/device/<avd-or-serial>/` | results of the last run on that target |

## Running

```sh
native/engine/device/avd.sh boot engine-api28 5600          # or engine-api37-16k; prints emulator-5600
native/engine/device/run.sh emulator-5600                   # default case file
native/engine/device/run.sh emulator-5600 my-cases.json     # any case file
ONLY=guest-bash,guest-true ENGINE_BUILD=0 HARNESS_BUILD=0 native/engine/device/run.sh emulator-5600
native/engine/device/avd.sh stop 5600
```

Switches: `SETTLE_SECONDS=120` (minimum device uptime before instrumenting: right after boot lmkd
on the 2 GiB 16K AVD killed the instrumented harness itself), `ENGINE_BUILD=0` (skip `native/engine/build-android.sh`), `HARNESS_BUILD=0` (skip Gradle),
`ONLY=ids`, `REPROVISION=1`, `UNINSTALL=1` (removes only the harness packages at the end).
Gradle always runs as `flock artifacts/.gradle.lock ./gradlew -p native/engine/android-harness ...`.
AVDs live in `ANDROID_AVD_HOME=artifacts/avd` (`engine-api28`: android-28 default x86_64, 4.4 kernel,
4 KiB pages; `engine-api37-16k`: android-37.0 google_apis_ps16k x86_64, 16 KiB pages).

On the tablet the driver only installs `adb install -r` the harness, uses `run-as` for the harness
package and runs `am instrument`; it never touches other packages and never unlocks the screen.

## What one run does

1. `build-android.sh <abi>`; Gradle stages `artifacts/engine/android/<abi>` into jniLibs:
   `libworkflow-engine.so`, `libworkflow-loader.so` (names kept), `test/sigsys_helper ->
   libwftest-sigsys.so`, `test/fork_stress -> libwftest-forkstress.so`, `test/guest_static ->
   libwftest-gueststatic.so` (`keepDebugSymbols` so the files stay byte-identical).
2. Rootfs: `mkrootfs.py` builds `artifacts/engine/device/rootfs/debian-13-slim-<abi>.tar` from the pinned
   OCI layout (arm64: `artifacts/image-research/debian-13-slim-arm64`, x86_64:
   `artifacts/engine/debian-13-slim-amd64`). If the device stamp `files/rootfs.stamp` differs from the
   tar's sha256 it is uploaded with `adb shell -T` (shell protocol v2; `adb exec-in` truncated the
   stream on API 28), sha256-verified on the device, extracted by toybox tar as the app user into
   `files/rootfs.new` (every `chown` fails with EPERM and is expected; any other error aborts), and the
   file/dir/symlink counts must equal the manifest before it is renamed to `files/rootfs`.
3. The case file is written to `files/cases.json`; `am instrument -w -e cases cases.json` runs
   `EngineAcceptanceTest.runCases`, which records G0 facts, runs the setup steps and every case.
4. `files/results/` is pulled to `artifacts/engine/device/<name>/` (`results.json`, `g0.json`,
   `out/<id>.stdout|stderr` full output, `instrument.log`, `ps.txt`, `summary.txt`, `sigsys.json`).

The JUnit test fails only on harness errors; case verdicts are in `results.json`/`summary.txt`.

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
`-e results DIR`. To add cases without rebuilding: edit a case file and run `HARNESS_BUILD=0
ENGINE_BUILD=0 run.sh SERIAL FILE`.

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
