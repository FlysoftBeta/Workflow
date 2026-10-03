# Workflow-owned Android runtime

This implementation replaces the earlier requirement to pair with a Termux daemon for basic local work. Workflow owns the execution UID and the canonical directory `File(context.filesDir, ".workspace").canonicalFile`. `WorkspaceRepository`, the local PTY, and a local Codex process must all use that same directory. A legacy Termux connection is a separate compatibility backend; it is not this runtime's filesystem owner.

## What is implemented

The initial executable is Android's installed `/system/bin/sh -i`. `libworkflow_pty.so` allocates a real PTY, creates a child session and controlling terminal, connects stdin/stdout/stderr, changes into the selected workspace directory, and execs that system shell. It uses neither an HTTP daemon nor a copied pairing token, `su`, an adb reverse tunnel, or Termux. This is an **Android system terminal**, not a Debian guest, a virtual root environment, or a completed loader/ptrace engine.

The JNI library is loaded on the IO dispatcher when a terminal is requested. Loading failure returns an operation error and does not stop Launcher or file editing. The library is a normal APK native library; this PTY backend does not need to execute code from the app's writable data directory or extract an executable `.so` through PackageManager.

The terminal starts with `HOME` equal to the canonical workspace, `PWD` equal to its selected subdirectory, and `WORKFLOW_WORKSPACE` identifying the same root. Temporary files and per-terminal shell histories live under `.workspace/.state/runtime/`. System tools available on a particular Android installation determine which commands can run. Python, Node, Git and Debian tools are not implied by a working shell.

## Kotlin interface

Package: `top.flysoftbeta.workflow.runtime.local`.

```kotlin
val runtime = LocalRuntime.get(context)
val root: File = runtime.workspace
val terminal = runtime.createTerminal(rows = 24, columns = 80, cwd = "").getOrThrow()
val id: String = terminal.id
val pid: Int = terminal.pid
val state: StateFlow<LocalTerminalState> = terminal.state
terminal.write("pwd\n")         // Result<Unit>, UTF-8
terminal.write(byteArrayOf(3))   // Ctrl-C through the PTY line discipline
terminal.resize(37, 111)         // TIOCSWINSZ, including foreground SIGWINCH
terminal.close()                // HUP/CONT, bounded grace, KILL if required, then wait/reap
terminal.awaitExit()            // real exit status; 128 + signal when signalled
```

`LocalTerminalState` contains real decoded `output`, `outputStartOffset`, `exitCode`, `error`, `sequence`, and `truncated`. The retained text represents the half-open stream window `[outputStartOffset, outputStartOffset + output.length)` in UTF-16 code units, matching Kotlin and JavaScript string indices. The start offset counts every discarded code unit, including an extra low-surrogate trim, and never moves backward. A live renderer can append only the suffix after its prior stream end even when the retained window slides or its text happens to repeat. UTF-8 bytes split between kernel reads are preserved. Output is capped at one million UTF-16 characters; truncation is explicit and never splits a surrogate pair. Output contains no invented success markers. Lifecycle status is separate from command output. Offsets do not create a VT screen snapshot: changing streams, rebuilding a renderer or falling behind the oldest retained offset requires an explicit reset/gap path. A truncated raw transcript cannot promise exact restoration of alternate-screen modes, cursor/parser state or discarded escape-sequence prefixes. Eight pure-JVM window tests cover the real one-million-character limit, repeated windows, surrogate boundaries, continuous deltas, genuine gaps and offsets beyond the integer range.

`LocalRuntime.terminal(id)` and `terminals()` refer only to processes owned by the current Android application process. There are at most eight live terminals and eight retained exited terminals. Activity recreation may attach to the same in-memory session. An ID from a prior app process does not recreate a live process or make an old PID safe to signal.

Initial `cwd` must be an existing canonical path inside the workspace; absolute paths and traversal outside it are rejected. This validates the startup request, not an execution sandbox: the user's shell can subsequently access whatever its Android UID is allowed to access. Unsaved Working Resources remain drafts until explicitly saved, so terminal tools see the actual saved file. The editor captures text and its base digest together before accepting input. That base travels with every working draft; it is not inferred from whatever the shell has most recently written. Successful saves acknowledge their exact written text/digest, so typing during or immediately after a save can rebase only to that known version. Digest-based save conflict checks remain necessary when shell or Codex writes race an editor draft.

## Native lifecycle and safety

All strings, argument/environment arrays, descriptor limits, termios values and the exec-status pipe are prepared in the parent. After fork the child performs only syscall-style operations: close, parent-death setup, signal reset, setsid, controlling-terminal ioctl, dup2, chdir, descriptor closing and execve. It does not call JNI, allocate memory, log through Java or C stdio, or call setenv.

A close-on-exec pipe reports failures by stage and errno. The parent returns a started terminal only after successful exec closes that pipe. The handshake has a bounded deadline. File descriptors above stdio are closed before exec, except the temporary error-report descriptor.

Every read/write/resize operation duplicates the master descriptor while holding the session lock. Concurrent release cannot turn an outstanding operation into access to an unrelated reused descriptor. Waiting first uses `waitid(..., WNOWAIT)`; the session is marked exited before `waitpid` releases the PID. Stop therefore never signals a stale, reallocated child PID. A cancelled terminal-creation coroutine closes any newly spawned terminal whose result would otherwise be discarded.

Closing sends HUP and CONT to the shell and its foreground process group, escalates to KILL after the grace period, and waits for reaping. The child sets `PR_SET_PDEATHSIG`; killing the owning app process does not leave the shell running. These mechanisms are not complete descendant containment: a deliberately detached descendant that changes sessions and closes its terminal is outside this first backend's lifecycle guarantee. Full guest task tracking remains part of the future engine design. Active terminals and local Codex processes retain the independent `LocalRuntimeService` foreground service. Its notification returns to the workbench; it does not depend on the overlay service. Android force-stop and process eviction still terminate this runtime, and no old session is represented as a restored live process.

## Android background lifetime

`LocalRuntimeService` is an independent `specialUse` foreground service. A terminal request acquires a process lease and waits for successful `startForeground()` before spawning its shell. Android 12+ background-start rejection, permission errors, or the startup deadline return an explicit error; a rejected request does not create a hidden shell. The notification reports active owners and opens the workbench. Releasing the last terminal/Codex lease stops the service. No root permission, battery-optimization exemption, boot receiver or overlay grant is required.

A local Codex bridge calls `suspend LocalRuntimeService.retainCodex(context, generationId)` and `releaseCodex(context, generationId)`. If it has already created a process, a service-start failure is visible through the returned `Result` and `state.error`, and does not secretly kill that process. It can retry `refresh(context)` during a user foreground action. Unique generation IDs keep an old completion from releasing a newer run's lease.

## Open file observation

The Android editor observes each open file's parent directory, including atomic replacement, and the browser observes the currently displayed directory. Clean open files reload actual saved bytes while retaining the cursor. A working draft is retained and external changes produce a conflict notice; normal save still uses repository digest checking. Missing files have a distinct version from empty files, so deletion cannot silently become an empty-file overwrite. Deleting a clean, nonempty open file preserves its visible text as a working draft. Programmatic reloads are suppressed from editor change callbacks, so reading new disk content does not create an unintended user edit.

## Verification

Portable native tests:

```sh
runtime/android/tests/run-host-tests.sh
```

The host suite actually runs a PTY and checks tty descriptors, working directory, direct file writes, environment, Unicode bytes, input, resize observed by `stty`, Ctrl-C delivered to a real foreground sleep, exit code, timeout escalation and reaping, an interactive shell’s distinct foreground-job process group during close, exec/cwd errno propagation, and non-CLOEXEC parent descriptor isolation. The C core and JNI wrapper also compile with NDK 30 for arm64/API 28. Host success is not a claim of Android success.

`LocalRuntimeInstrumentedTest` runs inside Workflow's application UID. It checks repository-to-shell and shell-to-repository visibility in the same root, `id -u`, `pwd`, true tty descriptors, `stty size`, Ctrl-C, split UTF-8, exit/reap and invalid startup paths. It writes the actual Android API/ABI/UID/kernel/SELinux/cwd/results to `.workspace/.state/runtime/local-pty-verification.json` only after that test succeeds. A separate UI reviewer owns any real-device operations; running the binary as adb shell or root does not substitute for this evidence.

### Physical-device result · 2026-09-26

The authorized Android 9 / API 28 arm64 device passed the final headless suite: **9 tests passed** (`artifacts/physical-review/headless-runtime-tests.log`). The PTY evidence is `artifacts/physical-review/local-pty-device-result.json`:

- Actual child UID **10087**, matching Workflow; SELinux context is `u:r:untrusted_app` on kernel `4.14.98-gab0da1b3c98f-dirty`.
- Canonical root is `/data/data/top.flysoftbeta.workflow/files/.workspace`, with the test child in a unique subdirectory of that same root.
- Real tty descriptors, editor-to-shell and shell-to-editor file visibility, cwd, `stty` resize to 37 × 111, foreground Ctrl-C, Unicode across separate writes, exit code 7 and wait/reap all passed.
- The separate busy-shell stop and invalid-startup-directory test passed. Creating a terminal also verified that the independent foreground service had actually entered foreground state.

The first device run exposed an Android-specific UTF-8 failure: the previous implementation created a new `CharsetDecoder` for each read. Android's decoder can hold an incomplete prefix internally after consuming its input buffer, so carrying only `ByteBuffer.remaining()` lost that prefix. `StreamingUtf8Decoder` now preserves the decoder for the entire stream as well as any unconsumed input. Empty polls do not finish decoding; only actual EOF flushes it. Six pure-JVM regressions and the unchanged real-device split-code-point case pass after this correction. The diagnostic failure log is retained separately as development evidence, not a final failure.

The same final suite also passed packaged Codex startup/JSONL handshake, managed Codex stop/restart, four network-bridge tests, and read-only Mihomo/guardian packaging checks. It did **not** start a root proxy, enable TUN, alter device routes, or substitute an adb-reversed host service for the Android runtime. Live GUI file-observer/rotation behavior, long-duration background/doze behavior, account authorization, and Debian guest execution are separate acceptance scopes.
