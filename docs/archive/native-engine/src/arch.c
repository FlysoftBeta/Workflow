/* arch.c — aarch64 + x86_64 register/syscall implementation.
 * Uses GETREGSET/SETREGSET (measured working on kernels 4.4/4.14/6.12 on both
 * arches; see artifacts/engine-research/results/matrix.json). */
#define _GNU_SOURCE
#include "engine/arch.h"

#include <elf.h>
#include <errno.h>
#include <string.h>
#include <sys/ptrace.h>
#include <sys/syscall.h>
#include <sys/uio.h>

int eng_regs_get(pid_t tid, eng_regs *r) {
    struct iovec io = {r, sizeof(*r)};
    return ptrace(PTRACE_GETREGSET, tid, (void *)NT_PRSTATUS, &io) == 0 ? 0 : -1;
}

int eng_regs_set(pid_t tid, const eng_regs *r) {
    struct iovec io = {(void *)r, sizeof(*r)};
    return ptrace(PTRACE_SETREGSET, tid, (void *)NT_PRSTATUS, &io) == 0 ? 0 : -1;
}

#if defined(ENG_ARCH_AARCH64)

long eng_sysno(const eng_regs *r) { return (long)r->regs[8]; }
uint64_t eng_arg(const eng_regs *r, int i) { return r->regs[i]; }
void eng_set_arg(eng_regs *r, int i, uint64_t v) { r->regs[i] = v; }
uint64_t eng_ret(const eng_regs *r) { return r->regs[0]; }
void eng_set_ret(eng_regs *r, uint64_t v) { r->regs[0] = v; }
uint64_t eng_pc(const eng_regs *r) { return r->pc; }
void eng_set_pc(eng_regs *r, uint64_t v) { r->pc = v; }
uint64_t eng_sp(const eng_regs *r) { return r->sp; }
void eng_set_sp(eng_regs *r, uint64_t v) { r->sp = v; }

int eng_syscall_void(pid_t tid, eng_regs *r) {
    (void)r;
    int nr = __NR_getpid;
    struct iovec io = {&nr, sizeof(nr)};
    return ptrace(PTRACE_SETREGSET, tid, (void *)NT_ARM_SYSTEM_CALL, &io) == 0 ? 0 : -1;
}

void eng_restore_args(eng_regs *cur, const eng_regs *orig) {
    for (int i = 1; i <= 5; i++) cur->regs[i] = orig->regs[i];
}

int eng_syscall_set(pid_t tid, eng_regs *r, long nr) {
    (void)r;
    int n = (int)nr;
    struct iovec io = {&n, sizeof(n)};
    return ptrace(PTRACE_SETREGSET, tid, (void *)NT_ARM_SYSTEM_CALL, &io) == 0 ? 0 : -1;
}

int eng_syscall_insn_len(void) { return 4; } /* svc #0 */

#elif defined(ENG_ARCH_X86_64)

long eng_sysno(const eng_regs *r) { return (long)r->orig_rax; }
uint64_t eng_arg(const eng_regs *r, int i) {
    switch (i) {
        case 0: return r->rdi;
        case 1: return r->rsi;
        case 2: return r->rdx;
        case 3: return r->r10;
        case 4: return r->r8;
        default: return r->r9;
    }
}
void eng_set_arg(eng_regs *r, int i, uint64_t v) {
    switch (i) {
        case 0: r->rdi = v; break;
        case 1: r->rsi = v; break;
        case 2: r->rdx = v; break;
        case 3: r->r10 = v; break;
        case 4: r->r8 = v; break;
        default: r->r9 = v; break;
    }
}
uint64_t eng_ret(const eng_regs *r) { return r->rax; }
void eng_set_ret(eng_regs *r, uint64_t v) { r->rax = v; }
uint64_t eng_pc(const eng_regs *r) { return r->rip; }
void eng_set_pc(eng_regs *r, uint64_t v) { r->rip = v; }
uint64_t eng_sp(const eng_regs *r) { return r->rsp; }
void eng_set_sp(eng_regs *r, uint64_t v) { r->rsp = v; }

int eng_syscall_void(pid_t tid, eng_regs *r) {
    r->orig_rax = (unsigned long)__NR_getpid;
    return eng_regs_set(tid, r);
}

void eng_restore_args(eng_regs *cur, const eng_regs *orig) {
    cur->rdi = orig->rdi; cur->rsi = orig->rsi; cur->rdx = orig->rdx;
    cur->r10 = orig->r10; cur->r8 = orig->r8; cur->r9 = orig->r9;
    cur->orig_rax = orig->orig_rax;
}

int eng_syscall_set(pid_t tid, eng_regs *r, long nr) {
    r->orig_rax = (unsigned long)nr;
    return eng_regs_set(tid, r);
}

int eng_syscall_insn_len(void) { return 2; } /* 0f 05 syscall */

#endif

#ifndef PTRACE_GET_SYSCALL_INFO
#define PTRACE_GET_SYSCALL_INFO 0x420e
#endif
int eng_syscall_info_op(pid_t tid) {
    unsigned char info[96];
    long n = ptrace(PTRACE_GET_SYSCALL_INFO, tid, (void *)sizeof(info), info);
    if (n < 0) return -1;
    return info[0];
}

const char *eng_sysname(long nr) {
    switch (nr) {
#ifdef __NR_openat
        case __NR_openat: return "openat";
#endif
#ifdef __NR_openat2
        case __NR_openat2: return "openat2";
#endif
#ifdef __NR_execve
        case __NR_execve: return "execve";
#endif
#ifdef __NR_execveat
        case __NR_execveat: return "execveat";
#endif
#ifdef __NR_statx
        case __NR_statx: return "statx";
#endif
#ifdef __NR_newfstatat
        case __NR_newfstatat: return "newfstatat";
#endif
#ifdef __NR_faccessat
        case __NR_faccessat: return "faccessat";
#endif
#ifdef __NR_faccessat2
        case __NR_faccessat2: return "faccessat2";
#endif
#ifdef __NR_clone3
        case __NR_clone3: return "clone3";
#endif
#ifdef __NR_close_range
        case __NR_close_range: return "close_range";
#endif
#ifdef __NR_fchmodat2
        case __NR_fchmodat2: return "fchmodat2";
#endif
#ifdef __NR_rseq
        case __NR_rseq: return "rseq";
#endif
#ifdef __NR_getpid
        case __NR_getpid: return "getpid";
#endif
        default: return "?";
    }
}
