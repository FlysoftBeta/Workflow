/* sigsys_helper.c — models a guest process running under Android's app seccomp
 * filter.  It installs a SECCOMP_RET_TRAP filter on a chosen "new" syscall
 * (default statx) so that calling it raises SIGSYS, exactly as the zygote
 * filter does on older API levels.  Run UNDER the engine, the tracer must catch
 * the SIGSYS and inject -ENOSYS; the process then observes ENOSYS and falls
 * back.  Run WITHOUT the engine, the default SIGSYS action kills it.
 *
 * exit 0  = statx returned ENOSYS (engine emulated it) AND the fstatat fallback
 *           succeeded AND ordinary syscalls still work.
 * exit 10 = statx did not return ENOSYS
 * exit 11 = fallback fstatat failed
 * exit 12 = seccomp install failed
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <linux/audit.h>
#include <linux/filter.h>
#include <linux/seccomp.h>
#include <stddef.h>
#include <stdio.h>
#include <sys/prctl.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <unistd.h>

#if defined(__aarch64__)
#define AUDIT_ARCH_SELF AUDIT_ARCH_AARCH64
#elif defined(__x86_64__)
#define AUDIT_ARCH_SELF AUDIT_ARCH_X86_64
#else
#error unsupported
#endif

static int install_trap(int nr) {
    struct sock_filter filter[] = {
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, arch)),
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, AUDIT_ARCH_SELF, 1, 0),
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ALLOW),
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, nr)),
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, (unsigned)nr, 0, 1),
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_TRAP),
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ALLOW),
    };
    struct sock_fprog prog = {.len = sizeof(filter) / sizeof(filter[0]), .filter = filter};
    if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0) return -1;
    if (syscall(__NR_seccomp, SECCOMP_SET_MODE_FILTER, 0, &prog) != 0) return -1;
    return 0;
}

int main(void) {
#ifndef __NR_statx
    return 0; /* nothing to test on this arch */
#else
    if (install_trap(__NR_statx) != 0) return 12;

    /* Direct statx syscall: seccomp RET_TRAP -> SIGSYS -> engine injects ENOSYS. */
    struct statx stx;
    long r = syscall(__NR_statx, AT_FDCWD, "/", AT_STATX_SYNC_AS_STAT, STATX_BASIC_STATS, &stx);
    if (!(r == -1 && errno == ENOSYS)) return 10;

    /* Fallback path glibc would take: newfstatat on "/" must succeed. */
    struct stat st;
    if (fstatat(AT_FDCWD, "/", &st, 0) != 0) return 11;

    /* Ordinary syscalls unaffected. */
    if (getpid() <= 0) return 13;
    (void)write(1, "ok\n", 3);
    return 0;
#endif
}
