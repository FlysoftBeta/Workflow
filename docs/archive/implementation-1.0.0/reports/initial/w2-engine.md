# W2 — container engine: progress report

Living report for workstream W2 (engine). Updated at each milestone. Long-term design lives in
[docs/engine.md](../../engine.md); this file holds evidence, the matrix and known gaps.

## Status by milestone

| M | Scope | Host (Fedora x86_64, Debian trixie amd64) | AVD api28 x86_64 (k4.4, 4K) | AVD api37 x86_64 (k6.12, 16K) | Tablet (arm64, k4.14, f2fs) |
| --- | --- | --- | --- | --- | --- |
| M0 | tracer core, SIGSYS→ENOSYS, EXITKILL | **green** 8/8 | **green** (default cases 24/24) | **green** (pass-through cases) | pending (disconnected) |
| M1 | loader (anon/file segments, PT_INTERP, 4K/16K) | **green** 10/10 incl. 16K simulation | **green** | **green for static guests**; dynamic Debian amd64 = platform limit (glibc refuses 4K-linked libs on 16K) | pending |
| M2 | path translator, binds, /proc | **green** 77/77 oracle, also behind the app filter in both kernel orders | **green** (bash, pipelines) | n/a (platform limit) | pending |
| M3 | metadata store, hardlink emulation, crash injection | **green** 23/23 | **green** (44-case oracle byte-identical, persistence, dpkg, crash+fsck) | installer/verify/fsck green; guest cases n/a | pending |
| M4 | exec (shebang/execveat/binfmt), fake identity, sudo su | **green** 5/5 (21-case podman oracle) | **green** (21-case oracle byte-identical in the app sandbox) | n/a (platform limit) | pending |
| M5 | installer, Debian workload, seccomp fast path, lifecycle | **green**: installer (25/25 fixtures, 28444/28444 rows = reference), fast path, acceptance 13/13 groups, soak (see log) | **green** 29/29 (installer 5 s, apt over the network, bionic-in-guest) | installer + bionic-in-guest green | pending |

Latest totals: `test-host.sh` 68/68; `accept-host.sh` 13/13 (+ soak); `gen/check.sh` OK; AVD api28
24/24 + 29/29; AVD api37-16k 10/24 + 7/29 (every failure = glibc page-size platform limit) + 3/3
bionic diagnostics. Not run: tablet (disconnected), any 4K x86_64 image with kernel ≥ 4.8 (none installed).

## Inputs

- Host guest rootfs: `artifacts/engine/rootfs-amd64/`, from `docker.io/library/debian:13-slim`
  amd64 pinned `sha256:7792b1f7702a86946cd518db72b6a407302c3e9bc1635634368b878189e8221c`
  (skopeo copy → OCI layout `artifacts/engine/debian-13-slim-amd64`, single layer extracted).

## Design decisions taken during implementation (with reason)

1. **Void = rewrite to `getpid`, not `-1`.** The kernel runs seccomp *after* the ptrace
   syscall-entry stop (x86 and arm64 `syscall_trace_enter`), so a syscall voided to `-1` would be
   re-checked by Android's zygote allowlist and raise SIGSYS. `getpid` is allowed everywhere and
   side-effect free; its result is overwritten at the exit stop.
2. **Guest exec = kernel exec of the separate loader.** `execve(guest)` is rewritten to
   `execve(nativeLibraryDir/libworkflow-loader.so, same argv, same envp)`. The kernel does real exec
   semantics (new mm, CLOEXEC, thread teardown, signal reset); the loader then fetches a *load
   plan* from the tracer via a marker syscall, maps exe + PT_INTERP, and builds the guest stack on
   the real process stack. argv strings are untouched, so `/proc/self/cmdline` stays right.
3. **Loader maps file-backed first, anonymous copy as fallback** (EACCES/EPERM, or segment offsets
   not congruent with the runtime page size). File mappings share page cache (node ≈ 100 MB).
4. **SEIZE-based scheduler** so group-stops (Ctrl-Z) can use PTRACE_LISTEN instead of being
   swallowed.
5. **Per-mm scratch slots** (4 MiB MAP_NORESERVE region from the loader, 16 KiB slot per thread)
   for translated paths: threads sharing an mm cannot race on one buffer.
6. **Modified argument registers are restored at the exit stop** (x86_64: args + orig_rax;
   arm64: x1–x5, kernel restores x0 from orig_x0), so a restarted syscall (ERESTARTSYS) is
   re-translated from the guest's original arguments instead of being translated twice.
7. **After PTRACE_EVENT_EXEC a syscall-exit stop still follows** — the entry/exit toggle must not be
   reset at the exec event (M0 code did; fixed). Where PTRACE_GET_SYSCALL_INFO exists the host
   tests cross-check the toggle against it.

## Log

- 2026-09-27: tree `native/engine/` created; M0 sources written (arch/mem/log/tracer/proxy/main).
- 2026-09-27: **M0 green** on Fedora x86_64. `native/engine/test-host.sh m0` → 8/8. Covers:
  exit-code fidelity (0/1/42), stdout passthrough, signal encoding (SIGSEGV→139),
  fork/exec/vfork/thread stress with full reaping (no orphans), and SIGSYS(statx)→ENOSYS
  emulation via a RET_TRAP filter that models the zygote app filter (control: bare helper
  dies on SIGSYS 159). Evidence: `WORKFLOW_ENGINE_LOG=3 engine run -- sigsys_helper` logs
  `SIGSYS ... statx -> ENOSYS`; guest then falls back to fstatat and exits 0. Confirmed the
  SIGSYS PC needs no adjustment (kernel leaves PC past the trapped insn).
  Primitives (GETREGSET/SETREGSET, void via NT_ARM_SYSTEM_CALL/orig_rax, return inject,
  process_vm_*, EXITKILL) are the measured-green ones from the research matrix.
- 2026-09-27: **M1 loader green** (loader in isolation). `native/engine/test-host.sh m1` → 2/2.
  Freestanding loader (`src/loader.c`) maps PT_LOAD via ANONYMOUS memory + byte copy +
  mprotect-to-final-flags (no execve, no file-backed PROT_EXEC — the anon path measured green on
  all targets), follows PT_INTERP, reserves ET_DYN span with a PROT_NONE mmap for a collision-free
  randomized base, builds argv/env/auxv (AT_PHDR/PHENT/PHNUM/BASE/ENTRY/PAGESZ/RANDOM/EXECFN/
  UID/GID/SECURE + host HWCAP/CLKTCK/SYSINFO_EHDR). Validated with a static no-libc ET_EXEC guest
  (`test/guest_static.c`) that reads the stack the loader built and checks argv[0], envc>=1,
  AT_PAGESZ∈{4096,16384}, AT_RANDOM≠0. Loader→guest phase handoff uses a sentinel
  getpriority(0x77,"WFLD") the tracer voids and treats as "guest phase begins".
  NOTE: host engine and production .so must be PIE (a non-PIE engine sits at 0x400000 and an
  ET_EXEC guest collides). Dynamic glibc /bin/true needs M2 path translation (deferred to M2 suite).
- 2026-09-27: **Rework to the final architecture + M2 green.** `test-host.sh` → 24/24.
  - Loader is now a separate freestanding static-pie (`loader/loader.c`, ~5 KB text, zero
    relocations — the build fails if any appear, 16 KiB-aligned segments). Every guest exec,
    including the first, is a kernel exec of the loader; the loader fetches the plan with the
    QUERY marker, maps exe + PT_INTERP (file-backed, anon fallback), reports its 4 MiB scratch with
    DONE, builds the guest stack on the real stack and jumps.
  - Scheduler rewritten on PTRACE_SEIZE with a ready/go pipe handshake; tasks whose first stop
    precedes the parent's fork event are held until linked; engine is a child subreaper; SIGTERM
    forwards to the tree then SIGKILLs after `WORKFLOW_ENGINE_STOP_GRACE` (default 3 s).
  - Resolver (`src/path.c`) + dispatcher (`src/sys.c`): all path-bearing syscalls on both ABIs,
    `getcwd` and `/proc` magic-link `readlink` emulated, exec planning with shebang chains
    (`src/exec.c`), io_uring/openat2/clone3 → ENOSYS, handles/mounts/ptrace → EPERM/EOPNOTSUPP.
  - Evidence: Debian's own `ld-linux-x86-64.so.2` and `libc.so.6` are mapped from the rootfs
    (checked via `/proc/self/maps`); bash 5.2.37, coreutils, pipelines and nested execs work.
  - Oracle: `test/guest/paths.sh` (77 cases: symlink kinds, loops, above-root, trailing slash,
    errno parity, Unicode/spaces, ~800-byte paths, /proc/self/{cwd,root,exe,fd}, /dev/fd,
    mv/rm/cp/find, shebang edge cases incl. 126/127) run under rootless podman over the same
    rootfs → `test/guest/paths.golden`; engine output is byte-identical. `ENGINE_ORACLE=1`
    re-runs podman to detect drift.
  - Toggle cross-check: with `WORKFLOW_ENGINE_CHECK_TOGGLE=1` the tracer uses its own entry/exit
    toggle (what 4.x kernels need) and compares with PTRACE_GET_SYSCALL_INFO at every stop:
    0 mismatches over the fork/exec stress and the whole paths workload.
  - Tests run on a fresh reflink copy of the pristine rootfs (an early run had polluted it via
    `ln -s` into a symlinked dir; pristine tree re-extracted from the pinned layer).
- 2026-09-27: NDK cross-build (`build-android.sh`) green for arm64-v8a and x86_64 (engine PIE on
  linker64; loader static-pie, zero relocations, 16 KiB segments). Device harness and syscall
  inventory delegated to subagents (own paths: `android-harness/`, `device/`; `gen/`, `sysinv.*`).
  **Tablet: disconnected from adb as of this entry — tablet matrix row pending.**
- 2026-09-28 07:10: **Lead handover.** New W2 lead took over after the previous lead stopped mid-M3.
  Findings on the inherited tree (no finer history than the baseline snapshot, where all of
  `native/` is new):
  - `test-host.sh` rebuilt from the current sources: **24/24 green** (M0 8, M1 8, M2 8 incl. 77/77
    path oracle), so the in-progress M3/M4 code did not regress M0–M2.
  - Syscall inventory helper output (`gen/`, `sysinv.*`) is **complete**: `gen/check.sh` passes
    (Linux v7.2.8 tables pinned by sha256, deterministic generation, NDK r30 + host header
    cross-check, 2077/2031/2043 checks on x86_64/forced-x86_64/forced-aarch64, arm64 NDK link).
  - M3 code present but untested: `src/meta.c` (single xattr `user.workflow.meta` = `1 UID GID
    MODE NLINK MAJ,MIN`, DAC checks, hardlink store `/.workflow-engine/links/<id>` + stub
    symlinks, convert journal + `WORKFLOW_ENGINE_CRASH_AT` points, flock for multi-instance),
    `src/sys.c` handlers (open/stat/statx/access/mkdir/mknod/symlink/unlink/rename/link/chmod/
    chown/xattr/getdents/umask), `src/ident.c` (virtual uid/gid/groups/caps), set-id in
    `src/exec.c`. `test/guest/meta.sh` (48 cases) existed with no suite wired in.
  - First run of `meta.sh` engine vs podman (pinned image): 42/48 identical. Real engine bugs:
    (1) **stale nlink** (`ln h2 h3` shows 2 instead of 3) — the metadata cache is keyed on ctime,
    whose granularity is coarse (jiffies on 4.x), so two updates in one tick serve stale data;
    (2) **no capability model**: `setpriv --reuid` fails "reactivate capabilities" because
    `capset` after dropping euid is refused (kernel keeps the permitted set with KEEPCAPS).
    Oracle limits (not engine bugs): rootless podman cannot `mknod c` and shows `/dev/null` as
    65534 — those rows need a curated golden.
  - Decision: **finish, not redo** — the design matches the planner decisions (xattr store,
    emulated hardlinks, relocatable guest-absolute stub text); gaps to close: cache, capability
    model + prctl caps, bind policy (no xattrs in binds per environment.md §2.6), stub rename out
    of the rootfs → EXDEV, umask never masking host owner bits, persistence/crash/multi-instance
    tests. Harness helper still running (engine-api37-16k on :5600, its own); not touched.
- 2026-09-28 (cont.): **Planner-reported device bugs fixed (host-verified; AVD re-run in progress).**
  1. *API28 x86_64 (k4.4): legacy syscalls SIGSYS'd by the app filter.* The dispatcher now rewrites
     31 legacy x86_64 calls (open/creat/access/stat/lstat/mkdir/rmdir/unlink/rename/link/symlink/
     readlink/chmod/chown/lchown/mknod/utime(s)/futimesat/dup2/pipe/poll/select/getdents/getpgrp/
     epoll_create/epoll_wait/inotify_init/eventfd/signalfd/alarm/time) to their `*at`/`*2`
     equivalents, restoring every guest register at the exit (dup2(fd,fd) → fcntl F_GETFD,
     select writes the remaining timeval back, old `linux_dirent` rebuilt in place, alarm via
     setitimer, time emulated). Two paths because **4.4 runs seccomp before the ptrace entry stop**
     (≥4.8 after): entry-stop rewrite, and on SIGSYS a re-issue of the modern call (PC rewound,
     args restored at its exit). New `test/legacy_probe.c` (46 raw-syscall checks; `--filter`
     installs the measured API28 trap set; `--exec` runs a program behind it) and suite `legacy`:
     probe passes in both orders, and the 77-case path oracle is byte-identical behind the filter
     in both orders (SIGSYS order simulated with `WORKFLOW_ENGINE_TEST_LEGACY=sigsys`).
  2. *16 KiB pages: loader clobbered segments sharing a page.* Anonymous path now maps the image
     union once, copies every segment, and mprotects each page with the union of its segments'
     flags; the file-backed path is only used when no two segments share a runtime page. Test hook
     `--test-page-size 16384` makes the loader behave as on a 16 KiB kernel (anonymous copies);
     `guest_static` now links 4 KiB/separate-code (R, RX, R, RW share one 16 KiB page) and checks
     data/bss/phdr → passes. **Platform limit found:** Debian *amd64* libraries are linked for
     4 KiB pages and glibc's own ld.so refuses them on a 16 KiB kernel ("ELF load command
     address/offset not page-aligned") — reproduced by the simulation; not an engine bug. Debian
     arm64 is linked with 64 KiB alignment (checked `libc.so.6`, `bash` from the arm64 base image),
     so 16 KiB arm64 devices are unaffected. The API37 16K x86_64 AVD can therefore only run static
     guests; dynamic Debian rows there are "n/a (platform)".
  - Also: capability model (see M3/M4 below), credential struct `eng_cred`, fault logging.
  - Host: `test-host.sh` **31/31** (m0 8, m1 10, m2 8, legacy 5).
- 2026-09-28: **M3 green on the host** — `test-host.sh m3` 23/23; full host run **54/54**
  (m0 8, m1 10, m2 8, m3 23, legacy 5).
  - Fixes: metadata cache removed (stale NLINK); **one metadata lock** (`.workflow-engine/meta.lock`,
    flock) for chmod/chown RMW *and* hardlink counts — with two locks a concurrent chmod wrote
    back a stale NLINK (caught by the two-instance test: 121 of 122 names); capability model
    (`eng_cred`: eff/prm/inh/amb/bnd, securebits/KEEPCAPS, commoncap setxuid/setfsuid/exec
    rules, capget/capset, PR_CAPBSET_*, PR_CAP_AMBIENT, virtual no_new_privs honoured for set-id);
    DAC via CAP_DAC_OVERRIDE/READ_SEARCH/FOWNER/FSETID/CHOWN/MKNOD; umask: the kernel gets
    `umask & 077` so host owner bits are never masked, the guest sees its own value.
  - **Binds keep no metadata** (environment.md §2.6): owner = bind owner (work), mode = host bits;
    chown accepted without effect; device nodes and hardlinks EPERM; a hardlink name moved out of
    the rootfs → EXDEV (mv copies); a file moved in from a bind keeps the presented owner.
  - **FIFO/socket placeholders**: the kernel refuses `user.*` xattrs on FIFOs, so mkfifo/mknod
    in the rootfs create a regular placeholder with the type in the xattr (like devices); open
    redirects to a real FIFO `.workflow-engine/fifo/<dev>-<ino>` made on demand. (Previously a
    FIFO created by `work` came out root-owned 0600 → EACCES for `work`, e.g. make's jobserver.)
    Known limit: `d_type` of placeholders is DT_REG (`find -type p/c` misses them; `ls -l`, stat
    are right).
  - `engine fsck --root R [--repair]`: recovers journals, walks the rootfs counting stub names
    per object, reports dangling/orphan/count problems; `--repair` only when no other instance
    holds the shared `instances.lock` (exit 3 otherwise). Crash windows can only **overcount**
    NLINK (never delete data); repair recomputes.
  - Tests (`suite_m3`): 44-case metadata oracle (`test/guest/meta.sh` vs `meta.golden` = podman
    output except 3 rows rootless podman cannot produce, documented; `ENGINE_ORACLE=1` re-checks
    that only those rows differ); xattr hiding/protection (`test/meta_probe.c`); persistence across
    instances incl. set-id, FIFO, char device; FIFO data between non-root processes; **dpkg**:
    a package built in the guest with owners/set-id/setgid dir/hardlink, `dpkg -i` → a new
    instance sees `4755 root root 2`, `640 daemon daemon`, `2775 root staff`, `dpkg --verify`
    clean, `dpkg -r` leaves a consistent store; **crash injection** at 5 points (convert:
    journaled/moved/stubbed, add-name, drop) → content intact, fsck clean after repair, NLINK ==
    names; **two instances** racing 120 links + chmod/chown loops → exact count, no lost update;
    concurrent removal; repair refused while an instance runs; bind policy (5 cases).
- 2026-09-28: AVD **engine-api28** (x86_64, k4.4) re-run with the legacy fixes: **24/24 PASS** (was 19/24). On 4.4 the seccomp-refused call reaches the ptrace entry stop with number -1 (rax = real number); the engine now passes it through untouched and re-issues at the SIGSYS stop (log: `access(21) -> re-issued as 269` ×105, dup2→dup3, readlink→readlinkat, creat/mkdir/rmdir/getpgrp). AVD **engine-api37-16k**: 10/24 — all static/pass-through cases pass incl. the 4 KiB-linked `guest_static` whose segments share 16 KiB pages (loader fix verified on a real 16 KiB kernel); the 14 dynamic cases fail with glibc's own "libc.so.6: ELF load command address/offset not page-aligned" (platform limit, see above).
- 2026-09-28: **M4 green on the host** (`test-host.sh m4` 5/5).
  - `test/guest/ident.sh` (21 cases) run as `work` in the real workspace image, engine vs podman
    on the same image imported as a plain rootfs (`localhost/wf-engine-oracle:<sha12>`, made by
    `wfimage to-tar`): **byte-identical** — `id`, set-id bits, `sudo -n id/su/-i/-u nobody`,
    `sudo su work`, SUDO_* env, `su` auth failure through `unix_chkpwd` (virtual setgid shadow),
    `chage -l` (setgid), /etc/shadow EACCES for work / readable for root, root-created file
    ownership, shebang chains (5 levels OK, 6 → ELOOP like the kernel), `gcc` in the guest, an
    in-guest C probe for execveat (dirfd-relative, AT_EMPTY_PATH, O_PATH fd, AT_SYMLINK_NOFOLLOW
    → ELOOP, script by fd → `/dev/fd/N` argv, CLOEXEC script fd → ENOENT, dir/non-exec → EACCES),
    getauxval(AT_SECURE)=1 for set-uid and set-gid copies, `/proc/self/comm`, and sudo refusing a
    world-writable sudoers (it checks the virtual mode). Plus: `sudo su` shell reports uid 0 while
    `/proc/self/status` shows the unchanged host uid.
  - Fixes found by it: **PR_SET_DUMPABLE virtualised** (sudo sets it; a non-dumpable tracee's
    memory is closed to the tracer — process_vm_* and PEEK/POKE — so every translated path gave
    EFAULT); task name (comm) set by the loader to the executed basename (was "loader");
    getcwd of an unlinked cwd → ENOENT; CLOEXEC script fd → ENOENT; NETLINK_AUDIT socket →
    EPROTONOSUPPORT (sudo/login skip audit quietly, as without kernel audit).
  - **binfmt table** (engine-owned, never the host's binfmt_misc): register-string rules from
    `--binfmt RULE|@FILE` and the guest's `/usr/lib/binfmt.d`, `/etc/binfmt.d` (*.conf, /etc wins,
    systemd-binfmt order); M (offset/magic/mask) and E (extension) types; flag P honoured,
    O/C/F accepted (interpreter gets the path, as qemu-user handles without AT_EXECFD); checked
    before shebang/ELF like binfmt_misc. Test: magic, extension, P, PATH search, an arm64 ELF
    routed to a fake qemu interpreter; without a rule → ENOEXEC (126). No QEMU is shipped.
  - **AF_UNIX pathname sockets** (M5 item, needed by sudo's syslog): bind/connect/sendto/sendmsg
    translated; paths over 108 bytes via a deterministic alias `<socket-dir>/<fnv64 of host
    dir>` → directory symlink; getsockname/getpeername/accept map back to guest paths; abstract
    names untouched. Checked with python (stream + datagram, 60-char dir, getsockname equality).
  - Debug aid: `WORKFLOW_ENGINE_LOG=4` prints every guest syscall with args and result.
- 2026-09-28: **M5 part 1 — installer + seccomp fast path (host).** `test-host.sh` **64/64**.
  - `install --image F|- (--index image.json|--sha256 H) --target GEN [--profile]`: C
    implementation of environment.md §2.5 with vendored **zstd 1.5.7** single-file decoder
    (`third_party/zstd-1.5.7`, BSD, release sha256 pinned in its README), own SHA-256 and JSON
    reader. Validates metadata (format/version/type/profile/arch/requires incl. content-implied
    capabilities), attributes.tsv (header, invariants 1/2/4/5/6, sha256, counts), then streams
    once: ustar+PAX only (path/linkpath/size/mtime), member↔row order/type (inv. 3), openat/
    mkdirat/symlinkat relative to its own dir fds with O_NOFOLLOW|O_EXCL, no host set-id,
    `user.workflow.meta` per object, hardlink objects in `.workflow-engine/links`, special files
    as placeholders, seeds to `GEN/seeds/<store>/` with host bits = mode&0777, mtimes, data after
    the archive end refused, whole-stream sha256 + size checked before success, syncfs; any
    failure removes GEN. Real amd64 workspace image: **2.9–3.4 s, 139 MB RSS** (128 MiB zstd
    window). `clone --from --to` (reflink/copy + xattrs + hardlink store), `verify --generation`
    (metadata presence + fsck), `remove --generation` (no symlink following; refuses while an
    instance holds the generation).
  - Evidence: wfimage conformance fixtures **25/25** (good installs; all 23 bad variants rejected
    with exit 65 and no target left); engine install vs `wfimage install` reference on the
    real image: **28444/28444 rows identical** (uid/gid/mode/NLINK/content sha256/symlink text/
    mtimes/seeds bits) — `test/cmp_install.py`; clone vs reference rootfs 16589/16589.
    **Finding for W3:** the reference installer creates seed files with `os.open(mode)` under the
    process umask, so `/opt/toolchains/uv/python/.lock` (0666) becomes 0644 there; §2.5 says
    `mode & 0777` — the engine is exact; the comparison runs the reference with umask 0.
  - **Seccomp RET_TRACE fast path**, on by default where the kernel is ≥ 4.8 (`--no-seccomp` /
    `WORKFLOW_ENGINE_SECCOMP=0` off). Filter in the guest's first process: inventory classes
    PASS/FD/PROC + recv* run without stopping, everything else (and loader markers via getpid
    arg match, prctl, socket, umask, x86 legacy calls) stops; foreign-arch calls (int 0x80) →
    ENOSYS; x32 → traced/refused. Exit stops only requested when the entry changed something.
    Measured: 100× `cat` 4708 vs 22682 stops; a bash/cat/ls loop 0.76 s vs 1.16 s. Found and
    fixed: with the Android filter stacked, TRAP beats TRACE, so a SIGSYS re-issue of a legacy
    call whose modern form is fast-allowed (dup3/fcntl/pselect6/setitimer) ran without stops and
    lost its register restore → the re-issue now resumes with PTRACE_SYSCALL and ignores the
    duplicate RET_TRACE event. Suite `plain` re-runs the path, metadata, identity and legacy
    oracles with the fast path off: all identical.
  - Guest env no longer inherits `WORKFLOW_ENGINE_*`.
- 2026-09-28: **M5 part 2 — acceptance suite §4.6 on the host with the real workspace image.**
  `native/engine/test/accept-host.sh install workload pty lifecycle` → **13/13** groups, guest
  workload 25/25 + 5/5 persistence checks (log: `artifacts/engine/accept/`). Layout exactly as
  environment.md §4 (`.workspace/.workflow/engine/generations/1` from the C installer, seeds moved
  into `home/work` and `toolchains`), started like the app's ProcessLauncher (clean env from
  `metadata.environment`, binds `/workspace` (+hide `.workflow`), `/home/work`,
  `/opt/toolchains`, `/etc/resolv.conf`, `--socket-dir`). Covered: (1) `id`=work, `sudo su`
  → uid 0 with host uid unchanged; (2) `apt-get update/install jq strace` over the network,
  a new instance sees `dpkg` status and root ownership; (3) gcc, cmake; (4) `uv venv` +
  `uv pip install six`, `npm install is-number` + node; (5) `git clone` (GitHub) + commit;
  (6) hardlinks in the rootfs (`ln`, `cp -l`, count 3) and the refused-then-copied case in the
  workspace bind; (7) AF_UNIX stream over a 70-char directory + abstract; (8) **PTY** via a real
  pty (`test/pty_check.py`): size, resize → SIGWINCH/$COLUMNS, background job, Ctrl-C → 130,
  Ctrl-Z/fg, pipeline, `more`, `top`/`watch`, `nano`, clean exit; (9) Unicode/deep/long paths,
  `/proc/self/exe`; (10) 500 execs, 16 threads, 100 posix_spawn (vfork), `make -j8`;
  (11) `strace -f true` → "Operation not permitted", no hang; (12) io_uring_setup → ENOSYS;
  (14) SIGTERM reaps the tree incl. a TERM-ignoring child after the grace, SIGKILL of the engine
  kills the guests (EXITKILL), installer killed midway leaves only `2.partial` (removed by
  `remove`), clone+verify, `remove` refused while an instance runs. `verify` after the whole
  workload: every object has metadata, hardlink store consistent (found and fixed: runtime
  mount points `/dev/*`, `/dev/shm` created without metadata). (13) hour-long soak: running.
  - Codex model: bionic binaries from `nativeLibraryDir` in the guest need `/system/bin/linker64`
    → on Android the engine now binds `/system /apex /vendor /product /system_ext /odm
    /linkerconfig` at their own paths; the app adds `--bind nativeLibraryDir:nativeLibraryDir`.
    Device case `m5-bionic-in-guest` checks it.
  - `docs/engine.md` written (design, CLI/ProcessLauncher contract, layout, syscall policy,
    limits); `native/engine/README.md` replaced (was a stale placeholder).
  - AVD engine-api28 re-run with everything so far: default cases **24/24** (fast path off there:
    kernel 4.4).
- 2026-09-28: **First device run of the M3–M5 cases** (`device/cases/m5.json`, generated by
  `cases/gen_m5.py`; `run.sh` gained `WORKSPACE_IMAGE=1` to upload the real image and
  `RESULT_SUFFIX`). engine-api28: 16/27 — installer 5.2 s on the device, verify, id, gcc,
  uv-CPython, node, git, AF_UNIX, pipeline all PASS; failures showed three real engine bugs the
  host could not see: (1) **set*id calls are in the app filter**: on 4.4 (trap before the entry
  stop) and on ≥4.8 with the fast path (TRAP beats TRACE) they arrived as SIGSYS → ENOSYS, so
  sudo failed ("unable to change to root gid"). Fix: every call the dispatcher emulates completely
  is now answered at the SIGSYS stop. (2) glibc's `fchmodat(AT_SYMLINK_NOFOLLOW)` fallback
  (fchmodat2 is trapped) opens with O_PATH and chmods `/proc/self/fd/N`; O_PATH opens of FIFO
  placeholders were redirected to the hidden real FIFO → EOPNOTSUPP. Fix: O_PATH never redirects.
  (3) deep binds (`nativeLibraryDir` at its own path) need their mount point in the rootfs → the
  engine creates missing mount points (root 0755). Plus a test bug (crash case in one instance).
  - New host suite **appfilter**: `--test-app-filter` installs the measured app filter
    (`include/engine/appfilter.h`: 31 legacy + 51 newer calls + set*id family) in the child
    before its first exec, like zygote; the metadata and identity oracles are byte-identical
    behind it in both kernel orders. `test-host.sh` **68/68**.
- 2026-09-28: **AVD engine-api28 (x86_64, Android 9, k4.4, real app process under zygote's
  seccomp filter): default cases 24/24, M3–M5 cases 28/29.** Passing on the device: C installer
  (real 189 MB image, 5 s), commit, `verify`, `id`, `sudo su` (guest uid 0, `/proc/self/status`
  shows the app uid), **the 44-case metadata oracle and the 21-case identity/exec oracle
  byte-identical to the podman goldens**, persistence across instances (set-id, FIFO, char dev),
  dpkg build/install/verify, crash injection → fsck finds the overcount → repair → content intact,
  gcc, uv CPython 3.14 from the toolchain store, node, git, AF_UNIX over a long path with
  `--socket-dir cacheDir/s`, pipeline, **bionic `libwftest-forkstress.so` from nativeLibraryDir
  running inside the guest** (Codex model), `verify` after the workload. Fixed on the way:
  guest sees no `security.selinux` (xattr hidden, `/sys/fs/selinux` hidden — `ls -l` printed the
  SELinux '.' marker on Android); oracle scripts shipped as files (bash names them in messages);
  dpkg check as root and without directory link counts (fs-dependent: btrfs 1, ext4/f2fs 2);
  harness manifest gained INTERNET (the real app has it). Remaining FAIL `m5-apt-network`: DNS
  resolution fails in this emulator (the host has no /etc/resolv.conf for the emulator's slirp
  DNS 10.0.2.3); public resolvers added to the case for the next run.
- 2026-09-28: **AVD engine-api37-16k (x86_64, Android 17 preview, k6.12, 16 KiB pages):**
  default 10/24, M3–M5 7/29. Everything that does not load glibc passes: probe, SIGSYS
  emulation, fork/exec stress in pass-through, the SIGSYS survey, `guest_static` with segments
  sharing 16 KiB pages (file-map and anonymous), not-found, **the C installer on a 16K kernel
  (189 MB image), commit, verify, fsck --repair, verify-after**. Every other failure is
  `libc.so.6: ELF load command address/offset not page-aligned` from Debian amd64's own ld.so
  (4 KiB-linked libraries on a 16 KiB kernel) — the platform limit reproduced on the host by
  `--test-page-size 16384`; not fixable in the engine without re-linking Debian. The bionic
  `libwftest-forkstress.so` itself started inside the guest there (its Debian `/bin/true`
  children then hit the limit) and printed "failed to find generated linker configuration
  /linkerconfig/ld.config.txt" — open item, needs a look on a 4K API 30+ device.
  - No 4K x86_64 system image with a kernel ≥ 4.8 is installed (android-29 is x86-only), so the
    fast path with dynamic guests *inside a real app process* is only covered by the host
    simulation (`--test-app-filter`, both kernel orders) until the tablet (k4.14, fast path on)
    is back. Suggest adding an API 34/35 x86_64 image (not downloaded: ~1.5 GB).
- 2026-09-28: **Soak bug + instance semantics.** The first 60-min soak never ran its loop: the
  test started `http.server` in the background and curled it before it listened, the main
  shell exited with status 2, and the engine then waited 4.6 h for the leftover server (it only
  exited at ECHILD). Two fixes: the soak waits for the server; and **an instance now ends with its
  main process** (remaining tasks get SIGTERM, then SIGKILL after the stop grace; engine exits
  with the main status) — otherwise an agent's `awaitExit` would hang on any daemon it left
  behind. `--wait-all` keeps the old behaviour. Verified: `sh -c 'sleep & (trap "" TERM; sleep)
  &'` → engine exits after 3.0 s, no survivors; 1-min soak 264 rounds (make -j4 + HTTP), RSS
  flat at 2.6 MB, no orphans. 60-min soak restarted.
- 2026-09-28: **Bionic programs inside the guest (Codex model), API37 AVD diagnosis.** Path
  tracing at `WORKFLOW_ENGINE_LOG=4` (resolve "guest" -> host) and a bind-table dump at level 3
  showed: `stat("/linkerconfig")` is **EACCES for an app** (SELinux) while
  `/linkerconfig/ld.config.txt` is readable, so the directory bind was silently skipped and the
  bionic linker fell back to its default config; bionic's libc also looked for
  `/dev/__properties__` (system properties) and `/dev/socket` (netd dnsproxyd — bionic DNS).
  Now bound on Android: the linkerconfig *file*, `/dev/__properties__`, `/dev/socket`. Re-run
  (`device/cases/diag-linkerconfig.json`, fast path on and off): warning gone, properties read,
  3/3.
- 2026-09-28: **AVD engine-api28 final: default 24/24, M3–M5 29/29** with the latest engine
  (instance-ends-with-main, Android binds, SELinux label hiding). `m5-apt-network` now passes:
  the failure was not DNS in the engine — a network diagnosis case (`cases/diag-net.json`)
  showed the guest reaching 10.0.2.3 and 8.8.8.8 and connecting over TCP; apt resolves as
  `_apt`, and `resolv.conf` written by the app (umask 077) was 0600, which a bind presents
  as-is. Contract in engine.md: files the app writes for the guest must be 0644.
