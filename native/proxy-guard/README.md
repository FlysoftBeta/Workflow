# Mihomo root guardian

This is an executable PIE, named `libworkflow_proxy_guard.so` so PackageManager
can extract it into `nativeLibraryDir`. It is not a JNI library and must not be
loaded with `System.loadLibrary`. API 28 and Linux kernels without pidfd are
supported. Production requires effective UID 0 before forking.

Build both `arm64-v8a` and `x86_64` using the pinned Android NDK:

```sh
native/proxy-guard/build.sh \
  --ndk /path/to/android-sdk/ndk/30.0.15729638 \
  --output-dir /absolute/generated/jniLibs
```

The default output directory is `artifacts/local-runtime/proxy-guard`. No device,
Gradle, root shell, download, or network access is involved in this build script.
The caller must package each ABI's output and enable native-library extraction.

## CLI and control protocol

Invoke with separate argv entries; do not build an unescaped shell command from
paths. The three paths must be absolute and already exist. `runUuid` is a UUID in
the canonical 36-character hexadecimal-and-dashes form.

```text
libworkflow_proxy_guard.so supervise kernelAbs directoryAbs configAbs runUuid [--tun-cleanup table ruleIndex device]
```

Guardian standard input is a private control pipe retained by the app. The child
gets `/dev/null` as stdin. Child stdout and stderr both go to guardian stderr,
which the app must drain to a bounded log. Guardian stdout contains JSONL only;
do not merge stdout and stderr when launching through `su`.

After successful `execve` and initial `/proc` identity verification:

```json
{"event":"started","uid":0,"pid":123,"startTime":456,"guardPid":122,"guardStartTime":450,"runId":"00000000-0000-0000-0000-000000000001"}
```

This reports verified process launch, not REST/controller readiness or TUN
capability. The manager must check those capabilities separately. `startTime`
and `guardStartTime` are unsigned `/proc/<pid>/stat` field 22 tick counts, not
wall-clock times. JSON emits numeric values; Kotlin `Long` handles realistic
device uptimes. `uid` is guardian effective UID inherited by the fork child.

For an explicit TUN device with auto-route, `--tun-cleanup` enables scoped netlink cleanup.
Before forking, the guardian requires the device to be absent and both IPv4 and IPv6
policy rules to be empty in the given priority range (index through index+10) and
routing table. After reaping its child it deletes only complete rule records in
that range whose target is the owned table, a goto inside the range, or nop, and
whose input interface is loopback or the configured device. Foreign targets and
interfaces are retained. It never flushes a table, changes DNS, or signals an
unrelated process. This also runs after a kernel crash or SIGKILL. The TUN's routes
vanish with its non-persistent interface. Guardian SIGKILL cannot run this cleanup;
existing residual rules must block a later start, never be guessed to be owned.
Auto-redirect nftables rules remain Mihomo's graceful-shutdown responsibility.
Before an owned stop, the guardian rechecks for unfamiliar scoped rules. If any
appeared, or the check fails, it uses SIGKILL and its own exact cleanup instead of
letting Mihomo's graceful teardown sweep a foreign rule in the priority range.

The only command is an exact newline-terminated record:

```text
stop 00000000-0000-0000-0000-000000000001 123 456
```

The run ID, child PID and start time must all match this guardian's memory. The
guardian additionally rechecks `/proc` start time and canonical executable path.
Wrong/overlong/malformed commands produce a nonfatal error and leave the child
running. Empty lines and extra whitespace are not accepted. A command is bounded
to 255 bytes, and fragmented reads are supported.

```json
{"event":"error","message":"Stop rejected: control or owned child identity mismatch","fatal":false}
```

Startup failures report `event:error` with `fatal:true` and exit without a
`started` event. Child exec/setup handshake is bounded to 5 seconds. On a valid
stop, EOF, or guardian SIGTERM/SIGHUP/SIGINT, the child process group normally gets SIGTERM,
then SIGKILL after 2 seconds if the child has not exited. Final reporting follows
actual `waitpid` reap:

```json
{"event":"exit","pid":123,"exitCode":143,"forced":false}
```

`exitCode` is the real child exit status, or `128 + signal`. `forced` indicates
the guardian had to escalate the still-live child to SIGKILL. The guardian exits
with that code too. A natural child exit also emits `exit`; remaining members of
its group are killed before the leader is reaped. The kernel may defer delivery
to a task in uninterruptible sleep, so the final reap cannot promise a wall-clock
bound merely because SIGKILL has been sent.

## Ownership and failure boundaries

The guardian only signals its own forked child/session. `waitid(..., WNOWAIT)`
observes termination without reaping. Until all signals have been sent, that
unreaped child pins its PID against reuse. The group leader's PID also cannot be
reused during group cleanup. `/proc` checks validate normal control but are not
used as a standalone PID-ownership substitute. There is no arbitrary PID kill
command and no stop-by-persisted-record path. If the manager loses the live
process/control handle, persisted identity is for read-only diagnosis only.

Child setup uses only fixed-stack state and syscall wrappers after fork, sets a
new session, closes inherited descriptors, and sets `PR_SET_PDEATHSIG(SIGKILL)`
with a parent-PID race check. Guardian SIGKILL therefore kills its direct child,
but cannot guarantee Mihomo's graceful route restoration or cleanup of all
descendants that escaped its process group. EOF provides the ordinary graceful
cleanup path when the app process dies. Nonblocking JSON output prevents a full
control-output pipe from blocking cleanup; a lost output reader triggers cleanup
when an event write fails. The app must keep reading both pipes while running.

## Host acceptance

```sh
native/test-host.sh proxy-guard
```

This compiles a real fake-kernel ELF and runs it as a supervised child. It tests
launch metadata, cwd/argv/stdin and stream separation, normal and forced stop,
EOF, guardian termination/death, natural exit, executable replacement, malformed
and stale controls, exec failure, and names containing spaces/parentheses/newline
in `/proc/stat`. Production UID rejection is also checked when the host user is
not root. The host-only `WORKFLOW_PROXY_GUARD_HOST_TEST` flag permits testing as
the current UID; compilation explicitly fails if that flag is combined with
`__ANDROID__`. These tests do not establish Android root/TUN acceptance.
