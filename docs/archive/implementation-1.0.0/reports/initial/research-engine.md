# Container engine — research, feasibility & design

Status: **research + measured feasibility probes + design. No product engine code written.**
Date 2026-09-26. Author: container-engine architect agent.

Scope: the private-image container engine from the brief (`.prompt_tmp/initial/prompt.md`
§container engine): a PRoot-style **user-space, same-UID compatibility runtime** that runs a
Debian trixie (glibc) rootfs as the ordinary Workflow app UID, with a custom loader shipped in
`nativeLibraryDir`, `ptrace` syscall interception, xattr-backed file identity, fake root via
`sudo su`, bind mounts and binfmt. This is **not** a security sandbox — there is no namespace or
kernel isolation; a malicious guest is out of scope. On-disk image format and `container.json`
are owned by `docs/report/initial/research-image.md`; this report covers only the execution
engine and cross-references that doc for the schema.

All probe sources are in `artifacts/engine-research/src/`, binaries in `.../bin/` (rebuild with
`artifacts/engine-research/build.sh`), consolidated results in `.../results/matrix.json`, raw
tablet JSON in `.../results/tablet-*.json`. AVDs created for this work: `artifacts/avd/workflow-api37-ps16k`
and `workflow-api29`. The latter was created but **not booted** because API37 already covered the
new-policy case and host RAM was tight, so API 29 x86 remains an unmeasured matrix row. The
reference-source audit (termux @d4d2a19, proot-me @2265984) was done by a read-only sub-agent and
is summarised in §3. Probes touch only their own children and their own
temporary files; none modified the app, ran Gradle, or altered the emulators' system state.

---

## 1. Existing material — what is reusable

| Artifact | Verdict |
| --- | --- |
| `docs/native-engine-design.md` | **Keep as the contract.** It is honest ("loader/ptrace engine not implemented"), defines the G0–G5 gates, the interface data model (`RuntimeRoot`/`EngineProbe`/`InstallRequest`/…), the syscall-class table, xattr durability semantics, atomic-generation install and the Android/kernel acceptance matrix. This report refines and *measures* against it; it does not replace it. |
| `tools/engine-probes/` (`ptrace_probe.c`, `run.py`) + `artifacts/engine-probes/result.json` | **Reusable host baseline.** Good hygiene (own children only, writes under ignored dirs). Host-only (Fedora 7.2 x86_64). My probes extend it onto the real Android matrix and add syscall-void/return-injection, `process_vm_*`, seccomp-RET_TRACE ordering, per-arch register rewriting, page size and exec-policy tests. |
| `runtime/workflow_daemon/engine.py` | **Reusable as the compatibility (proot) backend and lifecycle skeleton only.** It correctly refuses to pretend a host shell is a guest (`engine:native` raises until implemented), validates `container.json`, fingerprints config, applies packages/nvm/uv in a worker thread, publishes state events. It is *not* the native engine and calls `proot` as a separate binary. Keep the lifecycle/reconcile/state-event shape; the native engine plugs in behind the same data model. |
| `runtime/workflow_daemon/image.py` | **Reusable installer logic, needs hardening.** Good: rejects absolute/`..`/duplicate members, defers symlinks, validates hardlink targets, writes `attributes.json` fallback + `user.workflow.*` xattrs, keeps host mode readable, records device nodes as virtual. Gaps (already noted in the design doc): the three separate `os.replace` calls are **not** an atomic generation commit; no `generations/<id>/` + `current.json` pointer; no power-fail recovery; xattr uid/gid/mode is path-keyed and breaks on rename/hardlink/inode-reuse. |
| `runtime/image/Dockerfile` | **Reusable image recipe.** `debian:13-slim`, user `work` (uid 1000, passwordless `sudo`), the required apt set, nvm + uv. This is the source the flat `image.tar.zst` is built from (see research-image.md). |
| `artifacts/engine-references/{proot-me,termux-proot}` | **Read-only reference.** Pinned revisions recorded in the design doc. GPL-2.0-or-later. Used for algorithm study; consequences of deriving vs. re-implementing are analysed in §3. |
| `app/.../runtime/local/LocalCodexSession.kt`, `LocalProxyManager.kt`, `cpp/pty_process.c`, `cpp/CMakeLists.txt` | **Reusable Android integration pattern.** They already `ProcessBuilder(nativeLibraryDir/lib*.so …)` and run a JNI PTY with `-Wl,-z,max-page-size=16384`. The engine reuses exactly this launch path and PTY. |

---

## 2. Feasibility matrix (measured)

Method: NDK r30 clang (`~/Android/Sdk/ndk/30.0.15729638`), API-28 sysroot, static-ish PIE.
Real device via `adb shell run-as top.flysoftbeta.workflow` (**genuine `untrusted_app` domain**,
the app targets SDK 37). Emulators booted headless one at a time under
`artifacts/avd`. On emulators the kernel-level capabilities were read in the `shell` domain and the
SELinux exec policy probed with root + `runcon`/`chcon` (see the important caveat below).

| Capability | tablet API28 arm64 k4.14 (real app UID) | AVD API28 x86_64 k4.4 | AVD API37 ps16k x86_64 k6.12 |
| --- | --- | --- | --- |
| Page size | 4096 | 4096 | **16384** |
| exec from `nativeLibraryDir` | **yes** (existing app runs libcodex/libmihomo/libpty) | n/a | n/a (real app confirms) |
| exec from app **data** dir | **yes** (real untrusted_app) | runcon: denied¹ | runcon: **denied**¹ |
| anon `PROT_READ\|WRITE\|EXEC` mmap | yes | yes | yes |
| `mprotect` add `PROT_EXEC` (W^X toggle) | yes | yes | yes |
| file `mmap(PROT_EXEC, MAP_PRIVATE)` | yes | yes | yes (shell) |
| `PTRACE_TRACEME`+`SETOPTIONS` (SYSGOOD/FORK/VFORK/CLONE/EXEC/EXIT/EXITKILL) | yes | yes | yes |
| syscall-entry/exit stops | yes | yes | yes |
| `GETREGSET NT_PRSTATUS` read syscall nr | yes | yes | yes |
| `PTRACE_GET_SYSCALL_INFO` | **no (EIO)** | **no (EIO)** | **yes** |
| `process_vm_readv` tracee memory | yes | yes | yes |
| `process_vm_writev` (writable target) | yes | yes | yes |
| `PTRACE_PEEKDATA` fallback | yes | yes | yes |
| **void syscall** (arm64 `NT_ARM_SYSTEM_CALL`=-1 / x86 `orig_rax`=-1) | yes | yes | yes |
| **inject return value** + guest observes it | **yes (end-to-end)** | yes | yes |
| install 2nd seccomp filter (`NO_NEW_PRIVS`+`SECCOMP_SET_MODE_FILTER`) | yes | yes | yes |
| `SECCOMP_RET_TRACE` → `PTRACE_EVENT_SECCOMP` | yes | yes | yes |
| seccomp event ordering | **before** syscall-entry stop | before | before |
| `PTRACE_GETEVENTMSG` fork child tid / seccomp `RET_DATA` | **correct / correct** | not re-measured | not re-measured |
| Android zygote app seccomp filter present in the probe process | **no** (`run-as`: `Seccomp:0`); real app process: `Seccomp:2` | n/a (shell) | n/a (shell) |
| `setxattr user.workflow.*` regular file | yes | yes | yes |
| `setxattr user.*` directory | yes | yes | yes |
| xattr survives `rename` | yes | yes | yes |
| **`link()` (hardlink) create** | **no — EACCES** | **no — EACCES** | **no — EACCES** |
| `lsetxattr` on symlink | no — EPERM | no — EPERM | no — EPERM |
| filesystem | f2fs (user_xattr,inline_xattr) | ext4 | ext4 |

¹ **Critical caveat on the exec-from-data denials.** A `runcon`-entered `untrusted_app` on the
emulator is **not** a faithful reproduction of a zygote-spawned app: on the API28 **x86_64 AVD**
it denied exec-from-data, yet the **real API28 tablet** (`run-as`, genuine domain) **allowed** it.
So emulator `runcon` exec-denials prove only "SELinux enforces the check", not that a real app is
blocked. The trustworthy statements are: (a) exec-from-data is **allowed on the API28 tablet**
(measured, real domain); (b) it is a function of the **device platform policy** and is documented
as removed for `untrusted_app` on **API29+ devices**; (c) therefore the loader must live in
`nativeLibraryDir` unconditionally, which the existing app already proves works at targetSdk 37.

**Second caveat: `run-as` is not the app's own process tree.** The `run-as` probe had no
zygote seccomp filter (`Seccomp:0`). The running Workflow app process has one (`Seccomp:2`).
Guest processes started by the real app inherit that filter. So any guest syscall outside
Android's app allowlist arrives as `SIGSYS`. Older API levels have narrower allowlists, which
matters for the syscalls glibc 2.41 tries first: `clone3`, `faccessat2`, `rseq`, `close_range`
and `statx`. The engine must turn those `SIGSYS` stops into truthful `ENOSYS` results, or into
emulation through an older equivalent, so that glibc takes its normal fallback path. It does not
try to remove the platform filter. termux/proot does the same (`tracee/seccomp.c`). None of this
is measured yet: it needs a probe launched through the app's own `ProcessBuilder` path (M0
on-device gate).

**Device note.** During the last probe batch (2026-09-27) the tablet showed the lock screen. I
stopped using it, as the safety rules require. The cleanup commands (remove `files/probe` and
`/data/local/tmp/*.arm64`) returned no error, but I did not re-check the result. The `.workspace`
contents had been verified intact just before that batch.

### What the matrix means for the engine

- **The core PRoot mechanism is viable on the whole matrix.** Trace own children, read
  registers, read/write tracee memory, and — proven end-to-end — **void a syscall and inject a
  return value the guest actually observes**. This is the irreducible primitive a path/identity
  proxy needs, and it works on 4.4, 4.14 and 6.12, arm64 and x86_64.
- **Old vs new kernel split is exactly one feature:** `PTRACE_GET_SYSCALL_INFO` is absent on
  4.x (EIO) and present on 6.x. The tracer must derive entry/exit + arch from per-arch
  `GETREGSET`/`SETREGSET` on old kernels, and may use `GET_SYSCALL_INFO` as an optimisation on new
  ones. Register access itself (`NT_PRSTATUS`, arm64 `NT_ARM_SYSTEM_CALL`) works everywhere.
- **seccomp acceleration is available even on 4.14** and the event arrives *before* the
  syscall-entry stop, so the "only path/identity syscalls stop the tracer" optimisation is real.
  But it is an **optimisation, not a requirement** — the engine must stay correct on plain
  `PTRACE_SYSCALL`, and Android's own installed seccomp/SELinux denials do not disappear because we
  add a second filter.
- **W^X forces the loader design.** No dependence on exec-from-writable-data. Loader ships in
  `nativeLibraryDir`; the guest ELF is **loaded by mapping, not by `execve`**.
- **16 KiB pages are real** (API37). The loader must use the runtime `sysconf(_SC_PAGESIZE)` for
  all segment rounding/BSS zeroing; never hardcode 4096. NDK link flags already set
  `max-page-size=16384`.
- **Hardlinks are denied to the app on every target** (EACCES), and symlink `user.*` xattrs are
  EPERM. Both are decisive for the metadata layer (see §4 and §5): dpkg/apt create hardlinks and
  the guest expects them, so the engine **must emulate hardlinks** (link→symlink + shared-object
  metadata, like termux/proot `link2symlink`) and must key file identity on something other than a
  symlink-borne xattr.

### Residual capability that is still unverified (top G1 risk)

Whether `untrusted_app` on an **API29+ device** retains **`file execute`** permission for
`mmap(PROT_EXEC)` of an `app_data_file` (the guest ELF living in app data) is not yet measured on a
real device (emulator `runcon` can't answer it faithfully). The design therefore **does not depend
on it**: the loader can map segments into **anonymous** executable memory (allowed everywhere:
anon-RWX and `mprotect +X` passed on all targets) by reading the ELF with ordinary file I/O and
copying `PT_LOAD` bytes. mmap-backed loading of the file is an optimisation to confirm on a real
API29+ device during G1.

---

## 3. Build strategy — from-scratch vs. derive from termux/proot

**Recommendation: write a fresh engine in C (a small freestanding loader + a tracer/proxy core),
studying proot-me/termux-proot as reference for *correctness breadth*, and NOT linking or copying
their GPL-2.0 source into the app.** Ship it as `libworkflow-engine.so` (an `execve`-able ELF in
`nativeLibraryDir`), started as a separate process — never `dlopen`ed into the JVM.

Reasoning:

1. **Licensing.** proot is GPL-2.0-*or-later*. Copying/adapting its source makes the derived work
   GPL and imposes source-distribution obligations on that binary. Because the engine is a
   **separately executed binary** (its own process, arms-length via a pipe/PTY protocol), even a
   *verbatim* proot shipped alongside a proprietary app is the usual "mere aggregation / separate
   program" case, so the app does not become a derivative of proot. The proot binary itself stays
   GPL, including its loader (`loader/loader.c` is GPL-2.0-or-later). Bundling it in the APK
   therefore brings GPL distribution duties for that binary: the corresponding source, or a
   written offer, plus the license text. It is **not** acceptable to paste proot internals into a
   custom engine we want to keep under our own terms, because the whole engine would inherit
   GPL-2.0+. A clean re-implementation keeps the engine under our control and auditable. Stock
   proot can still be offered as an opt-in fallback. Today `engine.py` only execs a proot that the
   user installed.
2. **The xattr metadata requirement is not in proot.** proot's `fake_id0` keeps fake uid/gid/mode
   **in memory only** (termux's `USERLAND` branch adds a metadata file, with `TODO: USERLAND`
   holes, e.g. in `statx`). The brief mandates **durable** identity persisted in xattrs with
   hardlink/rename/unlink-open-fd correctness. That is a new subsystem either way; bolting a
   journaling `AttributeStore` onto proot's global-state extensions is more fragile than designing
   it in.
3. **Maintainability / footprint.** termux/proot is about 33.8 kLOC including syscall tables.
   Its core is about 21k, and about 1.3k of that is netlink faking. Extensions add 9.5k (fake_id0
   3.3k, sysvipc 2.3k, link2symlink 1.4k, kompat 1.0k); proot-me totals about 26.8k. Both use
   global static state and a single-threaded tracer doing synchronous host IO with one `lstat` per
   path component per syscall. Errors are handled with `assert()` aborts, and app-specific hacks
   keyed on names have accumulated. The build does not fit our NDK/CMake setup. A focused engine targeting exactly two ABIs (aarch64, x86_64), one guest type
   (`debian-trixie`), our image format, our bind model and our metadata store is far smaller and
   testable milestone-by-milestone.
4. **Correctness breadth is the hard part** and is where we *lean on* proot as an oracle: openat2,
   statx, faccessat2, execveat, clone3, `/proc/self/exe`, getcwd, symlink resolution, rename/link
   semantics, AF_UNIX `sun_path`, inotify path translation, shebang recursion — we port the
   *algorithms and edge-case lists* (which are facts, not copyrightable expression) and validate
   our outputs against a real Debian under stock proot and under a host chroot.
5. **The reference has gaps that directly affect our targets.** Source audit of termux @d4d2a19
   and proot-me @2265984:
   - `openat2` is rewritten to `openat` and its `RESOLVE_*` flags are dropped. proot-me passes it
     through untranslated on arm64.
   - `fchmodat2` has no syscall number in termux, so it passes through untranslated. glibc ≥ 2.39
     uses it on kernels ≥ 6.6, which includes our API37 6.12 target. proot-me reads the wrong
     argument register for its flags.
   - `faccessat2` ignores its flags.
   - `execveat` works only with `AT_FDCWD`.
   - `io_uring`, `fanotify` and `close_range` pass through.
   - `PTRACE_O_EXITKILL` is never used. If proot dies, the guests keep running.
   - Multithreaded `execve` is a TODO.
   - Job control is approximate because proot does not use `SEIZE`/`LISTEN`.
   - AF_UNIX paths longer than 108 bytes get a short name that dies at the next stop, so the
     client and server see different names.
   - The loader uses fixed load bases with no ASLR and file-backed `PROT_EXEC` maps with no
     anonymous-copy fallback, and `AT_SECURE` is always 0.
   - fake_id0 is not persisted. It forces `EPERM` results to 0, so `mknod` reports success but
     creates nothing. It temporarily chmods real host path components, which is racy and leaves
     modes changed after a crash. termux writes `getresuid` results with `poke_uint16`.
   - link2symlink stores absolute host paths, so the rootfs cannot be relocated. Its `.l2s.*`
     files show up in `getdents`.

   We take its *edge-case catalogue* and its SIGSYS/4.8-order/`GETEVENTMSG` handling ideas.
   We do not take the code.
6. **io_uring must be blocked/emulated**, not passed through: it lets the guest issue path/identity
   operations that never enter a traced syscall, silently bypassing the proxy. Same for any "escape
   hatch" (raw `openat2` with `RESOLVE_*`, `name_to_handle_at`). A from-scratch inventory lets us
   default-deny unknown/escaping syscalls (ENOSYS/EPERM) instead of proot's more permissive
   pass-through.

Fallback ladder: **native engine** (this design) → **stock proot** compatibility backend
(`engine.py`, GPL, separate binary, user-opt-in) → **explicit unavailable** (never a host shell
masquerading as a guest — the design doc's cardinal rule).

---

## 4. Design

### 4.1 Process & ownership model

- **One engine process per running guest session** ("run"), started by the Android service via
  `ProcessBuilder(nativeLibraryDir/libworkflow-engine.so, …)` with a PTY (reuse
  `cpp/pty_process.c`). The engine process is the **tracer**; every guest process/thread is a
  traced descendant in the **same app UID**. A single-threaded `waitpid(-1, __WALL)` scheduler owns
  all tids (proot's model) — simpler and correct across fork/vfork/clone/exec races; multiple
  concurrent guests = multiple engine processes, isolated by construction.
- **`runId` is a logical identity**; the engine tracks tid + start-time (and `pidfd` when
  available) rather than trusting a stored PID for kill. `PTRACE_O_EXITKILL` guarantees the guest
  tree dies with the engine; the service additionally reaps via the run handle.
- Kotlin ↔ engine speak a **structured, length-prefixed protocol** over a control fd (never shell
  string concatenation): `Install/Start/Stop/Remove` requests and `RunEvent` stream, matching the
  data model already in `native-engine-design.md`. PTY carries only guest stdio.

### 4.2 Components

1. **`ElfInspector`** — bounded, read-only validation of guest ELF + its `PT_INTERP` (ld-linux):
   magic/class/endian/machine, program-header bounds & overflow, `PT_INTERP` length/NUL, page
   alignment, ET_EXEC/ET_DYN, GNU_STACK. Cross-ABI or exec-stack ⇒ explicit reject.
2. **Loader** (`libworkflow-engine.so` acting in "load" phase) — the freestanding, W^X-safe loader.
   Because exec-from-data is unavailable on API29+ and mmap-PROT_EXEC of app data is unverified,
   the **default loader maps `PT_LOAD` segments into anonymous memory** (mmap RW, copy file bytes,
   zero `p_memsz−p_filesz` BSS, `mprotect` to final flags) at the runtime page size, builds the
   ABI stack (argv/env/auxv), maps the guest interpreter the same way, sets
   `AT_PHDR/PHENT/PHNUM/ENTRY`, `AT_BASE`=interp, `AT_EXECFN`, valid `AT_RANDOM`, `AT_PAGESZ`,
   `AT_SECURE`, and jumps to the interpreter entry. `AT_SECURE` is set correctly for virtual
   set-id execs such as `sudo`; proot always leaves it at 0. Load bases are **randomised** within
   an arch-specific window, whereas proot uses fixed bases with a "TODO: ASLR". A faster mmap-file
   path is enabled only where G1 proves the platform permits `PROT_EXEC` file mappings of app
   data. The reference loader uses *only* file-backed maps. This is also why Termux is widely
   reported to keep targetSdk 28 (not verified here). Our app targets 37, so the anonymous path
   is the default.
3. **Tracer / syscall scheduler** — the `waitpid` loop; per-`TaskState` = {ABI, tgid/tid,
   entry/exit phase, saved/rewritten regs, pending result, signal info, credentials, exec
   generation, shared fs/fd/vm refs (from clone flags), loader phase}. Baseline = `PTRACE_SYSCALL`
   + `TRACESYSGOOD` + FORK/VFORK/CLONE/EXEC/EXIT + **`EXITKILL`** (measured working; proot omits it);
   optional seccomp-RET_TRACE fast path (measured available) so only path/identity syscalls stop.
   The seccomp-vs-entry order is **auto-detected at the first event** (proot's approach), not taken
   from `uname`. A zero `GETEVENTMSG` is tolerated: fall back to the filter's own flags, and to the
   fork syscall's result for the child tid. That value was correct on our tablet but is known lost
   on some 4.14 vendor kernels. **`SIGSYS` handling:** a `SYS_SECCOMP` signal caused by Android's
   app filter is consumed. The call is either answered with a truthful `-ENOSYS` so glibc falls
   back, or emulated by an older equivalent: `statx`→`fstatat`, `faccessat2`→`faccessat` with its
   flags honoured in the tracer, `openat2`→`openat` *only when* the `RESOLVE_*` flags can be
   enforced by our resolver. The engine never tries to remove or evade the platform filter. Per-arch register tables (aarch64 & x86_64);
   syscall-number rewrite via `NT_ARM_SYSTEM_CALL` (arm64) / `orig_rax` (x86_64); "void + inject"
   confirmed working.
4. **Path translator** — canonicalises guest paths against the guest root and the **bind table**
   (longest-prefix match, per-component resolution, guest-absolute symlinks restart at guest `/`);
   resolves `dirfd`, `cwd`, `/proc/self/fd/N`, `/proc/self/cwd`, `/proc/self/exe`; guards TOCTOU by
   re-reading through the FD. Writes translated paths into a **writable scratch region in the
   tracee** (proven: `process_vm_writev` to writable memory works; the guest stack/argv area is the
   target, never rodata).
5. **Metadata store (`AttributeStore`)** — durable file identity. Since **hardlinks are denied**
   and **symlink xattrs are EPERM**, identity is a **versioned journal** keyed by a stable
   `objectId` (stored as `user.workflow.objectId` on xattr-capable regular inodes; link/lifecycle
   index for the rest), recording uid/gid/mode/special-type and the set of guest paths pointing at
   each object. `chmod/chown/stat/statx/access/exec` all read this one view; `+x` is virtual.
   Hardlinks are **emulated**: every guest link is a **relative** symlink into a hidden per-rootfs
   object area, and the link count lives in the journal. This avoids termux's absolute-host-path,
   filename-suffix scheme, so the rootfs stays relocatable. The object area is hidden from
   `getdents`. On f2fs the resolver must account for the casefold dentry bug that termux works
   around in `path/f2fs-bug.c`.
   Write-ahead + fsync ordering defined so a crash leaves the store readable or the generation
   unavailable, never silently wrong.
6. **Exec handler** — `execve`/`execveat` (incl. `AT_EMPTY_PATH`, dirfd, `/proc/self/fd`), shebang
   (length, arg, recursion limit, errno parity), and a **binfmt hook**: native ELF + guest shebang
   first; other ABIs default `ENOEXEC`; a later QEMU-user path uses the engine's own
   magic/mask→interpreter rules (never the host `binfmt_misc`), and any such runner must itself pass
   G1/G2. Successful exec commits new image/cwd/CLOEXEC/credential state; failure keeps the old
   process.
7. **Fake identity & sudo** — virtual uid/gid/groups/`AT_SECURE`; `sudo`/`su` flip the guest
   credential view to 0:0 (fake root) while the real app UID is unchanged; passwd/group/PAM/tty and
   set-id-on-exec semantics emulated; writing `security.capability` is refused truthfully, never
   faked as a granted kernel capability.
8. **Mount/bind table** — user-space guest→owner-root mappings (source ResourceRef/allowed root,
   guest target, rw policy, symlink policy, mappingId). `readOnly` stays **rejected** until every
   write path (open flags, existing writable FDs, `MAP_SHARED`, rename/unlink/link, truncate,
   metadata, `SCM_RIGHTS`) is consistently denied. `/dev` = explicit device whitelist; `/proc` =
   guest-identity presentation, not a raw host `/proc` passthrough.
9. **Lifecycle manager + image installer** — install/start/stop/delete; **atomic generation
   commit**: unpack into `generations/<id>/` (rootfs + metadata + attribute journal + verification),
   fsync, atomically flip `current.json`, fsync parent; running generations are never replaced in
   place; power-fail recovery test required. Uses the image format from research-image.md.

### 4.3 Syscall coverage plan (first release surface)

| Class | Implemented first | Semantics that must hold / not-yet boundary |
| --- | --- | --- |
| Path/FD | open/openat, `openat2`, *stat/`statx`, readlink*, getcwd/chdir/fchdir, rename/`renameat2`, link (→emulated), unlink, symlink, mkdir/rmdir, dup/fcntl/close/`close_range`, getdents64 | dirfd, `AT_EMPTY_PATH`, follow/no-follow, abs/rel symlinks, unlink-open-fd, CLOEXEC; `openat2` `RESOLVE_*` must be honoured or the call rejected — never silently pass a resolve flag we don't translate. |
| Exec/addr-space | execve/`execveat`, brk, mmap/mprotect/munmap/mremap, arch/TLS | loader vs guest syscall phases explicit; page-size at runtime; exec failure doesn't commit. |
| Identity/attrs | get/set*uid/gid, groups, umask, chmod/chown family, `fchmodat2`, access/`faccessat2`, xattr family | guest credential layer ≠ host UID; errno/return parity; set-id only virtual. |
| Proc/IPC | fork/vfork/clone/`clone3`, wait, kill/tgkill, signals, futex, socket/connect/bind/sendmsg | AF_UNIX pathname sockets + `SCM_RIGHTS` need FD/path registration. For `sun_path` > 108 bytes, a **run-scoped, deterministic** short alias lives in a short engine socket directory, so server and client resolve to the same name (fixes the proot defect). `clone3` is not blindly downgraded to clone. |
| Blocked/emulated | `io_uring_setup/enter/register` (**blocked** — escapes the proxy), mount/umount/namespaces, capabilities, real device mgmt, nested `ptrace`, `name_to_handle_at`/`open_by_handle_at`, fanotify | default-deny with truthful `ENOSYS`/`EPERM`; inotify path-translated or reported unsupported; nothing unknown passed through while claiming isolation. |

`/proc/self/exe`, getcwd and symlink resolution are validated against a real Debian under stock
proot as the oracle.

### 4.4 How the Android app starts it

`ProcessBuilder(File(applicationInfo.nativeLibraryDir, "libworkflow-engine.so").absolutePath,
"run", "--generation", id, …)` with the JNI PTY as controlling terminal, `-i` credentials and env
passed as structured argv/protocol, cwd = guest `/workspace` bound to `.workspace`. Identical
pattern to the working `LocalCodexSession`/`LocalProxyManager`. The engine binary is built by the
existing CMake external-native path (add an `arm64-v8a`+`x86_64` target with
`-Wl,-z,max-page-size=16384`) so PackageManager extracts it to `nativeLibraryDir`.

### 4.5 Staged milestones — each testable on the Linux host first, then the matrix

| M | Deliverable | Host-first test (Fedora x86_64 + podman Debian trixie rootfs) | Then device/AVD |
| --- | --- | --- | --- |
| **M0** | Tracer core + per-arch reg abstraction + void/inject + SIGSYS→ENOSYS/emulation + EXITKILL | run `/bin/true`, `/bin/echo` under bare tracer; assert entry/exit accounting, signal-once, no orphans; inject synthetic SIGSYS via a test seccomp filter | tablet + all AVDs. **Gate:** the tracer launched through the app's own `ProcessBuilder` (inherits the zygote filter). List which glibc-2.41 syscalls get SIGSYS per API level. |
| **M1** | Freestanding loader (anon-segment) loads guest `ld-linux` → `PT_INTERP` → `/bin/true` | exit 0 from real glibc `/bin/true`; auxv/stack correctness; 4K & 16K | tablet arm64, API37 ps16k (G1) |
| **M2** | Path translator + bind table + `/proc` presentation | `bash` starts, `pwd`, `ls`, symlink/dirfd cases vs host oracle | tablet + API28/37 |
| **M3** | Metadata store (objectId journal) + chmod/chown/stat + hardlink emulation | `dpkg`-style chown/chmod persists across restart; hardlink emulation; crash-injection recovery | tablet (f2fs) + API37 (ext4) |
| **M4** | Exec handler (shebang/execveat) + fake identity + `sudo su` | `sudo su` → guest uid 0, host uid unchanged; shebang recursion; AT_SECURE | tablet + AVDs |
| **M5** | Full Debian workload + seccomp fast-path + lifecycle/atomic install | acceptance suite below | full matrix, incl. release APK, upgrade, reboot, background-kill |

### 4.6 Acceptance test suite (concrete, G4/G5)

Each case records command, env fingerprint, expected vs. actual guest+host result, stderr,
duration; a pass needs *observed correct behaviour*, never "did not crash" or "compiled".

1. `id` → `work` uid 1000; `sudo su` then `whoami` → `root`, while `adb`-observed real uid stays
   the app UID.
2. `apt-get update && apt-get install -y <pkg>`; dpkg unpack performs chown/chmod that **persist**
   across engine restart (metadata store).
3. `build-essential`: compile & run a C program; `cmake` a small project.
4. Python via `uv` (venv, install a wheel), Node via `nvm` (`npm i`, run a script).
5. `git clone` a small repo, `git status`, commit.
6. Hardlink-heavy op (dpkg, `cp -l`, `ln`) works via emulation; link count observed correctly.
7. AF_UNIX socket round-trip (e.g. `python -c` server/client) incl. abstract & pathname sockets.
8. PTY: `bash` job control, `Ctrl-C`, `stty`/resize, pipelines, `less`.
9. Unicode + spaces + very long paths; deep symlink chains; `/proc/self/exe`, `getcwd`.
10. Hundreds of `fork`/`exec`, a multithreaded program, `vfork` user.
11. `strace`/`gdb` **in the guest** attempting to ptrace another guest process → refused clearly
    (nested ptrace not emulated), no engine deadlock.
12. `io_uring` probe in guest → blocked/emulated, path ops still proxied.
13. Long-running stability: an hour-long build / server, memory stable, no orphaned tids.
14. Lifecycle: stop reaps the whole tree; kill the engine → guest dies (`EXITKILL`); power-fail
    during install → recovers to old generation or clean unavailable; upgrade/reboot; background
    kill of the app service.

---

## 5. Effort / risk per milestone, and open questions

| Milestone | Effort | Risk | Why |
| --- | --- | --- | --- |
| M0 tracer core | M | **Low–Medium** | Primitives all measured-green on 4.4/4.14/6.12, arm64+x86_64. The SIGSYS inventory under the real zygote filter is unmeasured. |
| M1 loader | L | **High** | Freestanding loader + auxv/stack + static-PIE/ET_DYN layout + 16K pages; the "does anon-exec suffice / is mmap-PROT_EXEC of app data allowed on API29+ real device" question resolves here. |
| M2 path translator | L | Medium | Breadth of edge cases (dirfd, openat2 RESOLVE, /proc, TOCTOU); proot is the oracle. |
| M3 metadata store | L | **High** | Hardlink-denied + symlink-xattr-EPERM ⇒ must emulate hardlinks and journal identity with crash-safety; no ready-made design in proot (their fake_id0 is in-memory; termux USERLAND has TODO gaps). |
| M4 exec/identity | M | Medium | shebang/execveat corner cases; sudo/AT_SECURE/set-id parity. |
| M5 workload+lifecycle | L | Medium | Integration + atomic install/rollback + release-path/background-kill behaviour. |

Open questions for the planner:

1. **API29+ real-device access.** The whole W^X exec/mmap policy for `untrusted_app` on an API29+
   *device* is only documented, not measured (emulator `runcon` gave a false-negative vs the real
   API28 tablet). Can we get an API29–33 physical arm64 device, or is booting an API29+ AVD **with
   a minimal installed debuggable app** (to obtain a genuine app domain) acceptable? This gates M1.
2. **arm32 guests?** Design commits to aarch64 + x86_64 only. Confirm no 32-bit guest requirement
   (brief says "not promised"); it changes syscall tables and struct layouts materially.
3. **QEMU-user for foreign-ABI binaries** (e.g. amd64 `.deb` tooling on arm64): in scope now or a
   later phase? Affects the binfmt component and licensing (QEMU is GPL, separate binary).
4. **How aggressively to block `io_uring`/escape syscalls** vs. emulate — acceptable to `ENOSYS`
   `io_uring` (Debian tools fall back) rather than emulate its path ops?
5. **Metadata store durability budget:** is a per-operation fsync acceptable on f2fs for dpkg-heavy
   installs, or do we need a batched WAL with a bounded loss window?
6. **On-device probing through the app itself.** `run-as` does not reproduce the zygote seccomp
   filter. The SIGSYS inventory needs either a debug-only "run engine probe" entry point in the
   app, or permission to install a small debuggable probe APK on AVDs (never on the tablet). Which
   does the planner prefer?
7. **Ship stock proot as the compatibility backend** (GPL, separate binary, user-opt-in) in the
   shipped APK, or only wire the native engine + "unavailable"? `engine.py` already supports proot;
   confirm the licensing/product stance.

---

Probe evidence: `artifacts/engine-research/results/matrix.json` and `.../src/*.c`,
`.../bin/*.{arm64,x86_64}`. References (read-only, GPL-2.0): pinned proot revisions in
`docs/native-engine-design.md`. Image format & `container.json`:
`docs/report/initial/research-image.md`.
