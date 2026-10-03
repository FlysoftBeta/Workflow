/* tracer.h — single-threaded ptrace scheduler and per-task state.
 *
 * One engine process per launched guest process tree.  A single
 * waitpid(-1, __WALL) loop owns every tid in the tree.  All tracees are
 * attached with PTRACE_SEIZE semantics (initial child seized; descendants
 * auto-attached) so group-stops can be handled with PTRACE_LISTEN. */
#ifndef WORKFLOW_ENGINE_TRACER_H
#define WORKFLOW_ENGINE_TRACER_H

#include <limits.h>
#include <signal.h>
#include <stdint.h>
#include <sys/types.h>

#include "engine/arch.h"

struct eng_tracer;
struct eng_guest;

typedef enum {
    ENG_PH_BOOT = 0,   /* engine code in the forked child before its first exec: pass-through */
    ENG_PH_LOADER,     /* our loader runs (host paths): pass-through except markers */
    ENG_PH_GUEST,      /* guest code: full translation */
} eng_phase;

/* Address space shared by CLONE_VM tasks; holds the scratch slot table. */
typedef struct eng_mm {
    int      refs;
    uint64_t base;            /* scratch region in the tracee (0 = none) */
    uint32_t size;
    uint32_t nslots;
    uint8_t  used[512];
} eng_mm;

/* Per-process (thread-group) info shared by all threads of a tgid. */
typedef struct eng_proc {
    int  refs;
    char exe[PATH_MAX];       /* guest path for /proc/<pid>/exe */
} eng_proc;

/* Virtual credentials of a task: ids, groups, capability sets, umask.  The
 * real app uid never changes; the guest sees and is checked against these. */
typedef struct eng_cred {
    uint32_t ruid, euid, suid, fsuid;
    uint32_t rgid, egid, sgid, fsgid;
    uint32_t ngroups;
    uint32_t groups[32];
    uint64_t cap_eff, cap_prm, cap_inh, cap_amb, cap_bnd;
    uint32_t securebits;      /* SECBIT_* (KEEP_CAPS mirrored from PR_SET_KEEPCAPS) */
    int      nnp;             /* guest asked for PR_SET_NO_NEW_PRIVS: set-id is ignored */
    int      undumpable;      /* virtual PR_SET_DUMPABLE 0 (the real flag must stay 1:
                               * a non-dumpable tracee's memory is closed to the tracer) */
    uint32_t umask;           /* guest umask (the kernel gets umask & 077) */
} eng_cred;

typedef struct eng_task {
    pid_t tid;
    pid_t tgid;
    int   in_use;
    int   linked;             /* parent's fork/clone event processed (state inherited) */
    int   held;               /* first stop arrived before the parent's event: not resumed yet */
    eng_phase phase;
    int   at_syscall_entry;   /* next syscall-stop is an entry stop */
    int   await_exit;         /* seccomp fast path: resume with PTRACE_SYSCALL for the exit stop */

    long  sysno;              /* syscall number captured at entry */
    int   void_pending;       /* entry voided the syscall; inject result at exit */
    long  inject_result;
    int   regs_modified;      /* entry changed args/nr: restore at exit */
    eng_regs entry_regs;      /* registers as the guest issued them */

    eng_mm   *mm;
    int       slot;           /* scratch slot index, -1 if none */
    uint32_t  slot_off;       /* bump allocator within the slot, reset per syscall */
    uint64_t  stack_scratch;  /* fallback allocation below sp (per syscall) */

    eng_proc *proc;

    /* pending exec: plan bytes built at execve entry, consumed by the loader */
    void     *plan;
    uint32_t  plan_len;
    char      plan_exe[PATH_MAX];

    /* exit-side fix-ups */
    int      fixup;           /* ENG_FIX_* */
    uint64_t fix_addr;
    uint64_t fix_len;
    long     fix_aux;
    int      fix_nofollow;
    uint32_t fix_mode;        /* virtual mode for a created object */
    char     fix_id[64];      /* hardlink object losing a name */
    char     fix_path[PATH_MAX];   /* host path for metadata lookups at exit */

    /* credentials to install when the pending exec succeeds (set-id) */
    int      plan_setid;
    uint32_t plan_euid, plan_egid;

    /* legacy x86_64 syscall rewritten to its *at/2 form (sys.c) */
    int      lg_fix;          /* LG_*: exit-side conversion */
    uint64_t lg_a, lg_b;
    int      lg_restart;      /* SIGSYS'd call re-issued under nr lg_nr ... */
    long     lg_nr;
    eng_regs lg_saved;        /* ... restore these argument registers at its exit */

    eng_cred cr;              /* virtual credentials (ident.c) */
    int      umask_old;       /* pending umask: virtual value to return */

    struct eng_tracer *tr;
} eng_task;

enum {
    ENG_FIX_NONE = 0,
    ENG_FIX_STAT,             /* overlay metadata onto struct stat at fix_addr */
    ENG_FIX_STATX,            /* ... onto struct statx */
    ENG_FIX_CREATE_FD,        /* new file: result is an fd, write metadata */
    ENG_FIX_CREATE_PATH,      /* new object at fix_path: write metadata */
    ENG_FIX_DROP_LINK,        /* success removed a name of hardlink object fix_id */
    ENG_FIX_RENAME_IN,        /* object moved from a bind into the rootfs: record its owner */
    ENG_FIX_SOCKNAME,         /* AF_UNIX address returned at fix_addr/fix_len(ptr): host -> guest */
    ENG_FIX_GETDENTS,         /* hide entries / present stubs as regular files */
    ENG_FIX_LISTXATTR,        /* remove engine-private names */
};

typedef struct eng_run_cfg {
    char **argv;              /* guest argv, argv[0] as the user typed it */
    const char *exe;          /* guest path to execute (PATH-resolved), NULL = argv[0] */
    char **envp;
    const char *cwd;          /* guest cwd (default "/") */
    struct eng_guest *guest;  /* NULL: M0 host pass-through (no loader) */
    int use_seccomp_fastpath;
    uint32_t uid, gid;        /* initial virtual credentials */
    int test_app_filter;      /* test: install appfilter.h as RET_TRAP in the child, like zygote */
    int wait_all;             /* keep running until every guest process is gone (default:
                               * when the main process exits, the rest get SIGTERM, then
                               * SIGKILL after the stop grace) */
} eng_run_cfg;

int eng_tracer_run(const eng_run_cfg *cfg);

eng_task *eng_task_find(struct eng_tracer *tr, pid_t tid);
struct eng_guest *eng_tracer_guest(struct eng_tracer *tr);

/* Syscall dispatch (sys.c).  Return 1 if regs were changed. */
int eng_sys_entry(eng_task *t, eng_regs *r);
int eng_sys_exit(eng_task *t, eng_regs *r);
int eng_sys_sigsys(eng_task *t, eng_regs *r, siginfo_t *si);

/* Helpers for handlers ---------------------------------------------------- */
/* Void the current syscall (rewritten to getpid) and make it return `result`. */
int eng_task_void(eng_task *t, eng_regs *r, long result);
/* Allocate `n` bytes of scratch in the tracee for this syscall; returns the
 * tracee address or 0. */
uint64_t eng_scratch_alloc(eng_task *t, const eng_regs *r, size_t n);
/* Copy a string into tracee scratch; returns tracee address or 0. */
uint64_t eng_scratch_put_str(eng_task *t, const eng_regs *r, const char *s);
void eng_scratch_release(eng_task *t);

#endif
