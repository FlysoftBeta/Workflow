/* arch.h — per-ABI register / syscall abstraction.
 *
 * Guest arch == host arch (planner decision). Each build targets exactly one
 * ABI: aarch64 (devices) or x86_64 (emulators/host tests). No 32-bit guests.
 * The tracer never writes a struct of one ABI into a tracee of another.
 */
#ifndef WORKFLOW_ENGINE_ARCH_H
#define WORKFLOW_ENGINE_ARCH_H

#include <stdint.h>
#include <sys/types.h>
#include <sys/user.h>

#if defined(__aarch64__)
#define ENG_ARCH_NAME "arm64"
#define ENG_ARCH_AARCH64 1
#elif defined(__x86_64__)
#define ENG_ARCH_NAME "x86_64"
#define ENG_ARCH_X86_64 1
#else
#error "workflow engine supports aarch64 and x86_64 guests only"
#endif

typedef struct user_regs_struct eng_regs;

/* Read/write the general-purpose register set of a stopped tracee.
 * Return 0 on success, -1 on error (errno set). */
int  eng_regs_get(pid_t tid, eng_regs *r);
int  eng_regs_set(pid_t tid, const eng_regs *r);

/* Syscall number currently being entered (valid at syscall-entry stop). */
long eng_sysno(const eng_regs *r);

/* Syscall arguments 0..5 (as raw machine words). */
uint64_t eng_arg(const eng_regs *r, int i);

/* Overwrite a syscall argument register in a local copy of regs (caller must
 * eng_regs_set to commit).  Used to redirect a path pointer to scratch. */
void eng_set_arg(eng_regs *r, int i, uint64_t v);

/* The syscall return value register (rax / x0). */
uint64_t eng_ret(const eng_regs *r);
void     eng_set_ret(eng_regs *r, uint64_t v);

/* Program counter / instruction pointer. */
uint64_t eng_pc(const eng_regs *r);
void     eng_set_pc(eng_regs *r, uint64_t v);

/* Stack pointer. */
uint64_t eng_sp(const eng_regs *r);
void     eng_set_sp(eng_regs *r, uint64_t v);

/* Void the pending syscall: rewrite it to getpid (allowed by every Android
 * app seccomp policy, no side effects) and let the caller inject the real
 * result at the exit stop.  NOT -1: the kernel re-runs seccomp after the
 * ptrace entry stop, and Android's allowlist would trap -1 with SIGSYS.
 * `r` must be the regs read at the entry stop; on x86_64 it is modified and
 * committed.  Return 0 on success. */
int eng_syscall_void(pid_t tid, eng_regs *r);

/* Change the syscall number that the kernel will execute (entry stop only).
 * Used to translate e.g. openat2 -> openat, statx -> fstatat when we choose to
 * let the kernel run an older equivalent.  Return 0 on success. */
int eng_syscall_set(pid_t tid, eng_regs *r, long nr);

/* Length in bytes of the instruction that raises a syscall (svc #0 = 4 on
 * arm64, syscall = 2 on x86_64).  Used to step the PC past a SIGSYS'd
 * instruction when emulating a seccomp-blocked syscall. */
int eng_syscall_insn_len(void);

/* Restore the guest's original argument registers (and, on x86_64, the
 * syscall number) into `cur` at the exit stop, keeping the return value.
 * arm64 restores x1..x5 only: x0 is the return value and the kernel restores
 * it from orig_x0 on restart; x8 is never modified (NT_ARM_SYSTEM_CALL). */
void eng_restore_args(eng_regs *cur, const eng_regs *orig);

/* PTRACE_GET_SYSCALL_INFO op of the current stop: 1 entry, 2 exit, 3 seccomp,
 * 0 none, -1 unsupported (kernel < 5.3). */
int eng_syscall_info_op(pid_t tid);

/* Human name for a syscall number (best-effort, for logs). */
const char *eng_sysname(long nr);

#endif /* WORKFLOW_ENGINE_ARCH_H */
