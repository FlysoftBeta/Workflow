/* sysprobe.c — measures which syscalls the calling process's seccomp filter traps, independent of
 * glibc actually loading (on 16 KiB kernels Debian's 4 KiB-aligned libc never gets that far).
 * Packaged into the harness as libwftest-sysprobe.so; built by engine/environment/runtime/tests/device/run.sh.
 *
 *   sysprobe            bare: fork per syscall, child resets SIGSYS to SIG_DFL and calls it;
 *                       a child killed by SIGSYS = BLOCKED by the filter.
 *   sysprobe --inline   call every syscall in-process (run under the engine: blocked ones must come
 *                       back as ENOSYS and appear as SIGSYS lines in WORKFLOW_ENGINE_LOG=3 output).
 *
 * Every call uses arguments the kernel rejects (NULL pointers, fd -1, size 0, bad flags) so nothing
 * is created, changed or closed. Output: one line per syscall, then "blocked: ..." / "enosys: ...".
 * Exit 0 unless the probe itself fails.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <unistd.h>

struct probe { const char *name; long nr; long a[6]; };

#define P(n, ...) {#n, __NR_##n, {__VA_ARGS__}},
static const struct probe probes[] = {
    /* controls, allowed everywhere */
    P(getpid, 0)
    P(openat, -1, 0, 0, 0)
    P(newfstatat, -1, 0, 0, 0)
    /* legacy (non-*at) calls glibc still issues on x86_64 */
#ifdef __NR_access
    P(access, 0, 0)
    P(open, 0, 0, 0)
    P(stat, 0, 0)
    P(lstat, 0, 0)
    P(creat, 0, 0)
    P(mkdir, 0, 0)
    P(rmdir, 0)
    P(unlink, 0)
    P(rename, 0, 0)
    P(link, 0, 0)
    P(symlink, 0, 0)
    P(readlink, 0, 0, 0)
    P(chmod, 0, 0)
    P(chown, 0, -1, -1)
    P(lchown, 0, -1, -1)
    P(mknod, 0, 0, 0)
    P(utimes, 0, 0)
    P(futimesat, -1, 0, 0)
    P(dup2, -1, -1)
    P(pipe, 0)
    P(poll, 0, 1, 0)
    P(select, -1, 0, 0, 0, 0)
    P(getdents, -1, 0, 0)
    P(getpgrp, 0)
    P(epoll_create, 0)
    P(epoll_wait, -1, 0, 0, 0)
    P(inotify_init, 0)
    P(eventfd, -1)
    P(signalfd, -1, 0, 0)
    P(alarm, 0)
    P(time, 0)
    P(uselib, 0)
#endif
    /* calls newer than the old app allowlists */
    P(set_robust_list, 0, 0)
    P(rseq, 0, 0, 0, 0)
    P(statx, -1, 0, 0, 0, 0)
    P(faccessat2, -1, 0, 0, 0)
    P(clone3, 0, 0)
    P(openat2, -1, 0, 0, 0)
    P(close_range, 10, 5, 0)
    P(pidfd_open, -1, 0)
    P(pidfd_getfd, -1, -1, 0)
    P(pidfd_send_signal, -1, 0, 0, 0)
    P(epoll_pwait2, -1, 0, 0, 0, 0, 0)
    P(process_madvise, -1, 0, 0, 0, 0)
    P(mount_setattr, -1, 0, 0, 0, 0)
    P(landlock_create_ruleset, 0, 0, 0)
    P(memfd_secret, -1)
    P(futex_waitv, 0, 0, 0, 0, 0)
    P(cachestat, -1, 0, 0, 0)
    P(fchmodat2, -1, 0, 0, 0)
#ifdef __NR_map_shadow_stack
    P(map_shadow_stack, 0, 0, -1)
#endif
    P(copy_file_range, -1, 0, -1, 0, 0, 0)
    P(preadv2, -1, 0, 0, 0, 0, 0)
    P(pwritev2, -1, 0, 0, 0, 0, 0)
    P(pkey_alloc, -1, -1)
    P(pkey_mprotect, 0, 0, 0, -1)
    P(io_uring_setup, 0, 0)
    P(io_pgetevents, 0, 0, 0, 0, 0, 0)
    P(membarrier, -1, 0, 0)
    P(getrandom, 0, 0, 0)
    P(memfd_create, 0, 0)
    P(userfaultfd, -1)
    P(sched_setattr, -1, 0, 0)
    P(renameat2, -1, 0, -1, 0, -1)
    P(execveat, -1, 0, 0, 0, 0)
    P(mlock2, 0, 0, -1)
    P(kcmp, -1, -1, -1, 0, 0)
    P(name_to_handle_at, -1, 0, 0, 0, -1)
    P(open_by_handle_at, -1, 0, 0)
    P(setns, -1, 0)
    P(unshare, -1)
    P(bpf, -1, 0, 0)
    P(seccomp, -1, 0, 0)
    P(syncfs, -1)
    P(sync_file_range, -1, 0, 0, 0)
    P(process_vm_readv, -1, 0, 0, 0, 0, 0)
    P(kexec_file_load, -1, -1, 0, 0, -1)
    P(quotactl, 0, 0, 0, 0)
    P(add_key, 0, 0, 0, 0, 0)
    P(keyctl, -1, 0, 0, 0, 0)
    P(get_mempolicy, 0, 0, 0, 0, -1)
    P(set_mempolicy, -1, 0, 0)
    P(mbind, 0, 0, -1, 0, 0, -1)
    P(migrate_pages, -1, 0, 0, 0)
    P(move_pages, -1, 0, 0, 0, 0, -1)
    P(fanotify_init, -1, -1)
    P(perf_event_open, 0, 0, 0, 0, 0)
    P(clock_adjtime, -1, 0)
    P(adjtimex, 0)
    P(acct, 0)
    P(swapon, 0, 0)
    P(chroot, 0)
    P(pivot_root, 0, 0)
    P(mount, 0, 0, 0, 0, 0)
    P(umount2, 0, -1)
    P(open_tree, -1, 0, -1)
    P(fsopen, 0, -1)
    P(setxattr, 0, 0, 0, 0, -1)
};
#undef P

static void emit(const struct probe *p, const char *verdict, int err) {
    printf("%-24s %4ld %-8s %s\n", p->name, p->nr, verdict, err ? strerror(err) : "ok");
}

int main(int argc, char **argv) {
    int inl = argc > 1 && !strcmp(argv[1], "--inline");
    char blocked[4096] = "", enosys[4096] = "";
    size_t n = sizeof probes / sizeof probes[0];
    setvbuf(stdout, NULL, _IOLBF, 0);
    for (size_t i = 0; i < n; i++) {
        const struct probe *p = &probes[i];
        int err = 0, sig = 0;
        if (inl) {
            long r = syscall(p->nr, p->a[0], p->a[1], p->a[2], p->a[3], p->a[4], p->a[5]);
            err = r == -1 ? errno : 0;
        } else {
            pid_t c = fork();
            if (c < 0) { perror("fork"); return 2; }
            if (c == 0) {
                signal(SIGSYS, SIG_DFL);
                long r = syscall(p->nr, p->a[0], p->a[1], p->a[2], p->a[3], p->a[4], p->a[5]);
                _exit(r == -1 ? (errno & 0xff) : 0);
            }
            int st;
            if (waitpid(c, &st, 0) != c) { perror("waitpid"); return 2; }
            if (WIFSIGNALED(st)) sig = WTERMSIG(st);
            else err = WEXITSTATUS(st);
        }
        char *list = NULL;
        if (sig == SIGSYS) { emit(p, "BLOCKED", 0); list = blocked; }
        else if (sig) { printf("%-24s %4ld SIGNAL   %d\n", p->name, p->nr, sig); }
        else if (err == ENOSYS) { emit(p, "ENOSYS", err); list = enosys; }
        else emit(p, "allowed", err);
        if (list) { strncat(list, " ", 4095 - strlen(list)); strncat(list, p->name, 4095 - strlen(list)); }
    }
    printf("blocked:%s\nenosys:%s\n", blocked, enosys);
    return 0;
}
