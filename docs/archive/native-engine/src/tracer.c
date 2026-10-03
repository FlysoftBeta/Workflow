/* tracer.c — the scheduler.  See tracer.h and docs/engine.md. */
#define _GNU_SOURCE
#include "engine/tracer.h"

#include <errno.h>
#include <fcntl.h>
#include <sched.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/prctl.h>
#include <sys/stat.h>
#include <sys/ptrace.h>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <unistd.h>

#include "engine/exec.h"
#include "engine/guest.h"
#include "engine/ident.h"
#include "engine/meta.h"
#include "engine/loader_proto.h"
#include <stddef.h>
#include "engine/log.h"
#include "engine/mem.h"
#include "engine/sysinv.h"
#include "engine/appfilter.h"

#include <linux/audit.h>
#include <linux/filter.h>
#include <linux/seccomp.h>
#include <sys/utsname.h>

#ifndef PTRACE_EVENT_SECCOMP
#define PTRACE_EVENT_SECCOMP 7
#endif
#ifndef PTRACE_O_TRACESECCOMP
#define PTRACE_O_TRACESECCOMP 0x80
#endif

#ifndef PTRACE_LISTEN
#define PTRACE_LISTEN 0x4208
#endif
#ifndef PTRACE_EVENT_STOP
#define PTRACE_EVENT_STOP 128
#endif

#define NBUCKETS 1024

struct eng_tracer {
    eng_task *buckets[NBUCKETS];
    size_t ntasks;
    pid_t main_pid;
    int main_status, main_seen;
    struct eng_guest *guest;
    int sysinfo;          /* 1: PTRACE_GET_SYSCALL_INFO works; 0: no; -1: unknown */
    int fast;             /* seccomp RET_TRACE fast path active */
    unsigned long seccomp_stops;
    int check_toggle;     /* test mode: use the toggle, count disagreements */
    unsigned long toggle_mismatch, syscall_stops;
};

typedef struct eng_task_node { eng_task t; struct eng_task_node *next; } node;

static unsigned bucket(pid_t tid) { return (unsigned)tid % NBUCKETS; }

eng_task *eng_task_find(struct eng_tracer *tr, pid_t tid) {
    for (node *n = (node *)tr->buckets[bucket(tid)]; n; n = n->next)
        if (n->t.tid == tid) return &n->t;
    return NULL;
}

struct eng_guest *eng_tracer_guest(struct eng_tracer *tr) { return tr ? tr->guest : NULL; }

static eng_task *task_new(struct eng_tracer *tr, pid_t tid) {
    node *n = calloc(1, sizeof *n);
    if (!n) { ENG_ERR("out of memory"); exit(125); }
    n->t.tid = tid;
    n->t.tgid = tid;
    n->t.in_use = 1;
    n->t.at_syscall_entry = 1;
    n->t.slot = -1;
    n->t.umask_old = -1;
    n->t.tr = tr;
    n->next = (node *)tr->buckets[bucket(tid)];
    tr->buckets[bucket(tid)] = &n->t;
    tr->ntasks++;
    return &n->t;
}

static void mm_put(eng_mm *mm) { if (mm && --mm->refs <= 0) free(mm); }
static void proc_put(eng_proc *p) { if (p && --p->refs <= 0) free(p); }

static void task_del(struct eng_tracer *tr, pid_t tid) {
    node **pp = (node **)&tr->buckets[bucket(tid)];
    for (; *pp; pp = &(*pp)->next) {
        if ((*pp)->t.tid != tid) continue;
        node *n = *pp;
        *pp = n->next;
        eng_scratch_release(&n->t);
        mm_put(n->t.mm);
        proc_put(n->t.proc);
        free(n->t.plan);
        free(n);
        tr->ntasks--;
        return;
    }
}

static eng_proc *proc_new(const char *exe) {
    eng_proc *p = calloc(1, sizeof *p);
    p->refs = 1;
    if (exe) snprintf(p->exe, sizeof p->exe, "%s", exe);
    return p;
}

/* ---- helpers exported to handlers ---------------------------------------- */

int eng_task_void(eng_task *t, eng_regs *r, long result) {
    if (eng_syscall_void(t->tid, r) != 0 && errno != ESRCH)
        ENG_WARN("void syscall tid=%d: %s", t->tid, strerror(errno));
    t->void_pending = 1;
    t->inject_result = result;
    return 0;   /* regs already committed by eng_syscall_void */
}

/* ---- resume ---------------------------------------------------------------- */

/* Plain mode: every syscall stops (PTRACE_SYSCALL).  Fast path: only the
 * syscalls the seccomp filter marks RET_TRACE stop (PTRACE_EVENT_SECCOMP =
 * entry); PTRACE_SYSCALL is used only to reach the exit stop of one of them. */
static int g_fast;
static void resume_mode(pid_t tid, int sig, int want_syscall) {
    int req = (!g_fast || want_syscall) ? PTRACE_SYSCALL : PTRACE_CONT;
    if (ptrace(req, tid, 0, (void *)(intptr_t)sig) != 0 && errno != ESRCH)
        ENG_DBG("resume tid=%d: %s", tid, strerror(errno));
}
static void resume_task(eng_task *t, int sig) { resume_mode(t->tid, sig, t->await_exit); }
#define resume(tid, sig) resume_mode((tid), (sig), 0)

/* ---- stop / signal handling of the engine itself ------------------------- */

static volatile sig_atomic_t g_stop_sig, g_alarm;
static void on_stop_signal(int s) { g_stop_sig = s; }
static void on_alarm(int s) { (void)s; g_alarm = 1; }

static void install_engine_signals(void) {
    struct sigaction sa;
    memset(&sa, 0, sizeof sa);
    sa.sa_handler = SIG_IGN;
    sigaction(SIGINT, &sa, NULL);
    sigaction(SIGQUIT, &sa, NULL);
    sigaction(SIGHUP, &sa, NULL);
    sigaction(SIGPIPE, &sa, NULL);
    sa.sa_handler = on_stop_signal;   /* no SA_RESTART: interrupt waitpid */
    sigaction(SIGTERM, &sa, NULL);
    sa.sa_handler = on_alarm;
    sigaction(SIGALRM, &sa, NULL);
}

static void reset_child_signals(void) {
    struct sigaction sa;
    memset(&sa, 0, sizeof sa);
    sa.sa_handler = SIG_DFL;
    int sigs[] = {SIGINT, SIGQUIT, SIGHUP, SIGPIPE, SIGTERM, SIGALRM};
    for (size_t i = 0; i < sizeof sigs / sizeof sigs[0]; i++) sigaction(sigs[i], &sa, NULL);
    sigset_t none;
    sigemptyset(&none);
    sigprocmask(SIG_SETMASK, &none, NULL);
}

static void signal_all(struct eng_tracer *tr, int sig) {
    for (unsigned b = 0; b < NBUCKETS; b++)
        for (node *n = (node *)tr->buckets[b]; n; n = n->next)
            if (n->t.tid == n->t.tgid) kill(n->t.tgid, sig);
}

/* ---- markers from our loader ---------------------------------------------- */

static int is_marker(const eng_regs *r) {
    return eng_sysno(r) == __NR_getpid && eng_arg(r, 0) == ENG_MARK_A && eng_arg(r, 1) == ENG_MARK_B;
}

static void handle_marker(eng_task *t, eng_regs *r) {
    uint64_t op = eng_arg(r, 2);
    if (op == ENG_MARK_OP_QUERY) {
        if (!t->plan) { eng_task_void(t, r, -ENOENT); return; }
        if (eng_arg(r, 4) < t->plan_len) { eng_task_void(t, r, -E2BIG); return; }
        if (eng_mem_write(t->tid, eng_arg(r, 3), t->plan, t->plan_len) != (ssize_t)t->plan_len) {
            eng_task_void(t, r, -EFAULT);
            return;
        }
        long len = t->plan_len;
        eng_exec_discard(t);
        eng_task_void(t, r, len);
    } else if (op == ENG_MARK_OP_DONE) {
        eng_scratch_release(t);
        mm_put(t->mm);
        eng_mm *mm = calloc(1, sizeof *mm);
        mm->refs = 1;
        mm->base = eng_arg(r, 3);
        mm->size = (uint32_t)eng_arg(r, 4);
        mm->nslots = mm->size / ENG_SLOT_SIZE;
        if (mm->nslots > sizeof mm->used) mm->nslots = sizeof mm->used;
        t->mm = mm;
        proc_put(t->proc);
        t->proc = proc_new(t->plan_exe);
        t->phase = ENG_PH_GUEST;
        ENG_DBG("tid=%d guest phase, exe=%s scratch=%#llx", t->tid, t->plan_exe, (unsigned long long)mm->base);
        eng_task_void(t, r, 0);
    } else {
        eng_task_void(t, r, -EINVAL);
    }
}

/* ---- syscall stops ---------------------------------------------------------- */

static void syscall_stop(struct eng_tracer *tr, eng_task *t, int seccomp_event) {
    eng_regs r;
    if (eng_regs_get(t->tid, &r) != 0) { resume(t->tid, 0); return; }
    tr->syscall_stops++;

    int entry = t->at_syscall_entry;
    if (seccomp_event) {
        tr->seccomp_stops++;
        if (!t->at_syscall_entry) {
            /* the RET_TRACE event of a call whose ptrace entry stop we already
             * handled (a SIGSYS re-issue resumed with PTRACE_SYSCALL) */
            resume_task(t, 0);
            return;
        }
        entry = 1;
    }
    if (tr->sysinfo != 0 && !seccomp_event) {
        int op = eng_syscall_info_op(t->tid);
        if (op < 0) tr->sysinfo = 0;
        else {
            tr->sysinfo = 1;
            int k_entry = op == 1;
            if (op == 1 || op == 2) {
                if (k_entry != entry) {
                    tr->toggle_mismatch++;
                    ENG_WARN("entry/exit toggle mismatch tid=%d nr=%ld (toggle %s, kernel %s)", t->tid,
                             eng_sysno(&r), entry ? "entry" : "exit", k_entry ? "entry" : "exit");
                }
                if (!tr->check_toggle) entry = k_entry;
            }
        }
    }

    if (entry) {
        t->at_syscall_entry = 0;
        t->sysno = eng_sysno(&r);
        t->entry_regs = r;
        t->regs_modified = 0;
        t->void_pending = 0;
        t->fixup = ENG_FIX_NONE;
        if (t->lg_restart && t->sysno != t->lg_nr) t->lg_restart = 0;   /* not our re-issue */
        if (!t->lg_restart) {
            /* a SIGSYS re-issue keeps the scratch it already filled */
            t->slot_off = 0;
            t->stack_scratch = 0;
            t->lg_fix = 0;
        }
        int changed = 0;
        if (t->phase == ENG_PH_LOADER && is_marker(&r)) {
            handle_marker(t, &r);
        } else if (t->phase == ENG_PH_GUEST && tr->guest) {
            changed = eng_sys_entry(t, &r);
        }
        if (changed) eng_regs_set(t->tid, &r);
        /* fast path: skip the exit stop when nothing happens there */
        t->await_exit = !tr->fast || t->void_pending || t->regs_modified || t->fixup || t->lg_fix || t->plan ||
                        t->umask_old >= 0 || t->lg_restart || t->sysno == __NR_execve || t->sysno == __NR_execveat;
        if (!t->await_exit) t->at_syscall_entry = 1;
    } else {
        t->at_syscall_entry = 1;
        t->await_exit = 0;
        int changed = 0;
        if (t->phase == ENG_PH_GUEST && tr->guest) changed = eng_sys_exit(t, &r);
        if (t->void_pending) {
            eng_set_ret(&r, (uint64_t)t->inject_result);
            t->void_pending = 0;
            changed = 1;
        }
        if (t->regs_modified) {
            eng_restore_args(&r, &t->entry_regs);
            t->regs_modified = 0;
            changed = 1;
        }
        if (t->lg_restart) {
            eng_restore_args(&r, &t->lg_saved);   /* the guest's original legacy call */
            t->lg_restart = 0;
            changed = 1;
        }
        if (t->plan && (long)eng_ret(&r) < 0 && (t->sysno == __NR_execve || t->sysno == __NR_execveat))
            eng_exec_discard(t);   /* kernel exec of the loader failed; process unchanged */
        if (eng_log_enabled(ENG_LOG_TRACE)) {
            const char *nm = eng_sysinv_name(t->sysno);
            ENG_TRACE("tid=%d %s(%#llx, %#llx, %#llx, %#llx) = %lld", t->tid, nm ? nm : "?",
                      (unsigned long long)eng_arg(&t->entry_regs, 0), (unsigned long long)eng_arg(&t->entry_regs, 1),
                      (unsigned long long)eng_arg(&t->entry_regs, 2), (unsigned long long)eng_arg(&t->entry_regs, 3),
                      (long long)eng_ret(&r));
        }
        if (changed) eng_regs_set(t->tid, &r);
    }
    resume_task(t, 0);
}

static void sigsys_stop(eng_task *t) {
    siginfo_t si;
    eng_regs r;
    if (ptrace(PTRACE_GETSIGINFO, t->tid, 0, &si) != 0 || eng_regs_get(t->tid, &r) != 0) {
        resume_task(t, SIGSYS);
        return;
    }
    if (eng_sys_sigsys(t, &r, &si)) {
        eng_regs_set(t->tid, &r);
        if (t->lg_restart) {
            /* the re-issued call may be one the fast-path filter lets through:
             * stop at its entry and exit anyway to convert and restore */
            t->at_syscall_entry = 1;
            t->await_exit = 1;
        }
        resume_task(t, 0);
    } else {
        resume_task(t, SIGSYS);
    }
}

/* ---- events ------------------------------------------------------------------ */

static void on_new_child(struct eng_tracer *tr, eng_task *parent, pid_t ctid, int event) {
    eng_task *c = eng_task_find(tr, ctid);
    if (!c) c = task_new(tr, ctid);
    c->linked = 1;
    unsigned long flags = 0;
    if (event == PTRACE_EVENT_CLONE) {
        eng_regs pr;
        if (eng_regs_get(parent->tid, &pr) == 0) flags = eng_arg(&pr, 0);
    }
    int thread = event == PTRACE_EVENT_CLONE && (flags & CLONE_THREAD);
    int share_vm = event == PTRACE_EVENT_VFORK || (event == PTRACE_EVENT_CLONE && (flags & CLONE_VM));
    c->tgid = thread ? parent->tgid : ctid;
    c->phase = parent->phase;
    c->cr = parent->cr;
    if (parent->mm) {
        if (share_vm) { c->mm = parent->mm; c->mm->refs++; }
        else {
            c->mm = calloc(1, sizeof *c->mm);
            c->mm->refs = 1;
            c->mm->base = parent->mm->base;
            c->mm->size = parent->mm->size;
            c->mm->nslots = parent->mm->nslots;
        }
    }
    if (parent->proc) {
        if (thread) { c->proc = parent->proc; c->proc->refs++; }
        else c->proc = proc_new(parent->proc->exe);
    }
    if (c->held) { c->held = 0; resume(ctid, 0); }
}

static void on_exec(struct eng_tracer *tr, eng_task *t) {
    unsigned long former = 0;
    ptrace(PTRACE_GETEVENTMSG, t->tid, 0, &former);
    if (former && (pid_t)former != t->tid) {
        /* a non-leader thread exec'd and took over the leader's tid */
        eng_task *o = eng_task_find(tr, (pid_t)former);
        if (o) {
            free(t->plan);
            t->plan = o->plan; o->plan = NULL;
            t->plan_len = o->plan_len;
            memcpy(t->plan_exe, o->plan_exe, sizeof t->plan_exe);
            t->cr = o->cr;
            t->plan_setid = o->plan_setid; t->plan_euid = o->plan_euid; t->plan_egid = o->plan_egid;
            t->at_syscall_entry = o->at_syscall_entry;
            task_del(tr, (pid_t)former);
        }
    }
    eng_scratch_release(t);
    mm_put(t->mm);
    t->mm = NULL;
    t->regs_modified = 0;
    t->void_pending = 0;
    t->phase = tr->guest ? ENG_PH_LOADER : ENG_PH_BOOT;
    if (tr->guest) eng_ident_exec(t);   /* set-id transition + saved ids */
    /* keep at_syscall_entry: the execve's syscall-exit stop follows */
}

/* ---- seccomp fast path -------------------------------------------------------------
 * A filter installed in the guest's first process (inherited by every
 * descendant) returns RET_TRACE only for syscalls the dispatcher must see;
 * the rest run without stopping.  Allowed without a stop: inventory classes
 * PASS, FD and PROC and AF_UNIX-free receive calls, except the ones the
 * dispatcher still handles (prctl, socket, getpid markers, the x86_64 legacy
 * calls it rewrites).  Foreign-arch calls (x86 int 0x80) fail with ENOSYS.
 * Needs kernel >= 4.8 (seccomp after the ptrace entry stop, re-checked after
 * RET_TRACE); otherwise, or if installing fails, every syscall stops. */
#if defined(__x86_64__)
#define ENG_AUDIT_ARCH AUDIT_ARCH_X86_64
#else
#define ENG_AUDIT_ARCH AUDIT_ARCH_AARCH64
#endif

static int never_allow(long nr) {
    static const long list[] = {
        __NR_getpid, __NR_prctl, __NR_socket, __NR_umask,
#if defined(__x86_64__)
        __NR_time, __NR_getpgrp, __NR_alarm, __NR_pipe, __NR_dup2, __NR_poll, __NR_select, __NR_eventfd,
        __NR_signalfd, __NR_epoll_create, __NR_epoll_wait, __NR_inotify_init, __NR_getdents,
#endif
    };
    for (size_t i = 0; i < sizeof list / sizeof list[0]; i++) if (list[i] == nr) return 1;
    return 0;
}

static int fast_allowed(long nr) {
    if (never_allow(nr)) return 0;
    switch (eng_sysinv_class(nr)) {
        case ENG_SC_PASS: case ENG_SC_FD: case ENG_SC_PROC: return 1;
        default: break;
    }
    return nr == __NR_recvfrom || nr == __NR_recvmsg || nr == __NR_recvmmsg;
}

int eng_fast_filter_count(void) {
    int n = 0;
    for (long nr = 0; nr <= eng_sysinv_max(); nr++) n += fast_allowed(nr);
    return n;
}

/* Runs in the forked child before its first exec.  Returns 0 or -errno. */
static int install_fast_filter(void) {
    struct sock_filter f[1024];
    unsigned n = 0;
#define EMIT(...) do { if (n >= sizeof f / sizeof f[0]) return -E2BIG; f[n++] = (struct sock_filter)__VA_ARGS__; } while (0)
    const uint32_t TRACE = SECCOMP_RET_TRACE, ALLOW = SECCOMP_RET_ALLOW, NOSYS = SECCOMP_RET_ERRNO | ENOSYS;
    EMIT(BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, arch)));
    EMIT(BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, ENG_AUDIT_ARCH, 1, 0));
    EMIT(BPF_STMT(BPF_RET | BPF_K, NOSYS));
    EMIT(BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, nr)));
#if defined(__x86_64__)
    EMIT(BPF_JUMP(BPF_JMP | BPF_JGE | BPF_K, 0x40000000u, 0, 1));   /* x32 */
    EMIT(BPF_STMT(BPF_RET | BPF_K, TRACE));
#endif
    /* getpid: only the loader's markers stop */
    EMIT(BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, __NR_getpid, 0, 6));
    EMIT(BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, args[0])));
    EMIT(BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, (uint32_t)ENG_MARK_A, 0, 3));
    EMIT(BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, args[0]) + 4));
    EMIT(BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, (uint32_t)(ENG_MARK_A >> 32), 0, 1));
    EMIT(BPF_STMT(BPF_RET | BPF_K, TRACE));
    EMIT(BPF_STMT(BPF_RET | BPF_K, ALLOW));
    for (long nr = 0; nr <= eng_sysinv_max(); nr++) {
        if (!fast_allowed(nr)) continue;
        EMIT(BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, (uint32_t)nr, 0, 1));
        EMIT(BPF_STMT(BPF_RET | BPF_K, ALLOW));
    }
    EMIT(BPF_STMT(BPF_RET | BPF_K, TRACE));
#undef EMIT
    struct sock_fprog prog = {(unsigned short)n, f};
    if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0) return -errno;
    if (prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &prog, 0, 0) != 0) return -errno;
    return 0;
}

/* Test hook (--test-app-filter): the measured Android app filter as RET_TRAP,
 * installed in the child before its first exec -- the place zygote's filter
 * occupies for a real app (inherited, set with no_new_privs the guest never
 * asked for). */
static int install_test_app_filter(void) {
#if defined(__x86_64__)
    static const int trapped[] = {ENG_APPFILTER_LEGACY, ENG_APPFILTER_OTHER};
    struct sock_filter f[256];
    unsigned n = 0, cnt = sizeof trapped / sizeof trapped[0];
    f[n++] = (struct sock_filter)BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, nr));
    for (unsigned i = 0; i < cnt; i++) {
        f[n++] = (struct sock_filter)BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, (uint32_t)trapped[i], 0, 1);
        f[n++] = (struct sock_filter)BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_TRAP);
    }
    f[n++] = (struct sock_filter)BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ALLOW);
    struct sock_fprog prog = {(unsigned short)n, f};
    if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0) return -errno;
    return prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &prog, 0, 0) == 0 ? 0 : -errno;
#else
    return -ENOSYS;
#endif
}

/* seccomp re-checks a changed syscall after RET_TRACE and runs after the
 * ptrace entry stop only since Linux 4.8. */
static int kernel_supports_fast(void) {
    struct utsname u;
    if (uname(&u) != 0) return 0;
    int maj = 0, min = 0;
    if (sscanf(u.release, "%d.%d", &maj, &min) != 2) return 0;
    return maj > 4 || (maj == 4 && min >= 8);
}

/* ---- run ---------------------------------------------------------------------- */

static const long SEIZE_OPTS =
    PTRACE_O_TRACESYSGOOD | PTRACE_O_TRACEFORK | PTRACE_O_TRACEVFORK |
    PTRACE_O_TRACECLONE | PTRACE_O_TRACEEXEC | PTRACE_O_TRACEEXIT | PTRACE_O_EXITKILL;

static int chdir_guest(const eng_run_cfg *cfg) {
    if (!cfg->guest) return cfg->cwd ? chdir(cfg->cwd) : 0;
    char host[PATH_MAX];
    const char *gc = cfg->cwd ? cfg->cwd : "/";
    if (eng_guest_to_host(cfg->guest, gc, host, sizeof host)) return -1;
    return chdir(host);
}

int eng_tracer_run(const eng_run_cfg *cfg) {
    struct eng_tracer *tr = calloc(1, sizeof *tr);
    tr->guest = cfg->guest;
    tr->sysinfo = -1;
    const char *ct = getenv("WORKFLOW_ENGINE_CHECK_TOGGLE");
    tr->check_toggle = ct && *ct == '1';

    int ready[2], go[2];
    if (pipe2(ready, O_CLOEXEC) || pipe2(go, O_CLOEXEC)) { ENG_ERR("pipe: %s", strerror(errno)); return -1; }

    install_engine_signals();
    prctl(PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0);   /* orphans stay our children */
    int fast_wanted = cfg->use_seccomp_fastpath && cfg->guest;
    if (fast_wanted && !kernel_supports_fast()) {
        ENG_INFO("kernel older than 4.8: seccomp fast path off");
        fast_wanted = 0;
    }
    if (cfg->guest) eng_link_recover(cfg->guest);   /* finish interrupted hardlink conversions */
    pid_t child = fork();
    if (child < 0) { ENG_ERR("fork: %s", strerror(errno)); return -1; }
    if (child == 0) {
        reset_child_signals();
        close(ready[0]);
        close(go[1]);
        umask(022);   /* Debian login default; Android apps run with 077 */
        if (chdir_guest(cfg) != 0) {
            fprintf(stderr, "workflow-engine: cannot enter working directory %s: %s\n",
                    cfg->cwd ? cfg->cwd : "/", strerror(errno));
            _exit(126);
        }
        /* first, like zygote: the app filter predates the engine */
        if (cfg->test_app_filter) {
            int arc = install_test_app_filter();
            if (arc) { fprintf(stderr, "workflow-engine: test app filter: %s\n", strerror(-arc)); _exit(126); }
        }
        char c = 'P';
        if (fast_wanted) {
            int frc = install_fast_filter();
            if (frc == 0) c = 'S';
            else fprintf(stderr, "workflow-engine: seccomp fast path unavailable (%s), tracing every syscall\n",
                         strerror(-frc));
        }
        if (write(ready[1], &c, 1) != 1) _exit(126);
        if (read(go[0], &c, 1) != 1) _exit(126);
        if (cfg->guest) {
            execve(cfg->guest->loader, cfg->argv, cfg->envp);
            fprintf(stderr, "workflow-engine: exec loader %s: %s\n", cfg->guest->loader, strerror(errno));
            _exit(127);
        }
        execvpe(cfg->argv[0], cfg->argv, cfg->envp);
        fprintf(stderr, "workflow-engine: exec %s: %s\n", cfg->argv[0], strerror(errno));
        _exit(127);
    }
    close(ready[1]);
    close(go[0]);
    tr->main_pid = child;

    char c;
    if (read(ready[0], &c, 1) != 1) {
        int st;
        waitpid(child, &st, 0);
        return WIFEXITED(st) ? WEXITSTATUS(st) : -1;
    }
    close(ready[0]);
    tr->fast = g_fast = c == 'S';
    long opts = SEIZE_OPTS | (tr->fast ? PTRACE_O_TRACESECCOMP : 0);
    if (ptrace(PTRACE_SEIZE, child, 0, (void *)opts) != 0) {
        ENG_ERR("PTRACE_SEIZE: %s", strerror(errno));
        kill(child, SIGKILL);
        return -1;
    }
    eng_task *m = task_new(tr, child);
    m->linked = 1;
    m->phase = ENG_PH_BOOT;
    eng_ident_init(cfg->guest, m, cfg->uid, cfg->gid);
    if (cfg->guest) {
        int rc = eng_exec_prepare_initial(m, cfg->exe ? cfg->exe : cfg->argv[0]);
        if (rc) {
            fprintf(stderr, "workflow-engine: %s: %s\n", cfg->exe ? cfg->exe : cfg->argv[0], strerror(-rc));
            kill(child, SIGKILL);
            waitpid(child, NULL, __WALL);
            return rc == -ENOENT ? 127 : 126;
        }
    }
    ptrace(PTRACE_INTERRUPT, child, 0, 0);
    int status;
    while (waitpid(child, &status, __WALL) < 0 && errno == EINTR) {}
    resume(child, 0);
    if (write(go[1], &c, 1) != 1) ENG_WARN("release child: %s", strerror(errno));
    close(go[1]);

    int stop_forwarded = 0;
    for (;;) {
        if (g_stop_sig && !stop_forwarded) {
            ENG_INFO("stop requested (signal %d): forwarding to guest", (int)g_stop_sig);
            signal_all(tr, g_stop_sig);
            stop_forwarded = 1;
            const char *gs = getenv("WORKFLOW_ENGINE_STOP_GRACE");
            alarm(gs ? (unsigned)atoi(gs) : 3);
        }
        if (g_alarm) {
            g_alarm = 0;
            ENG_INFO("grace period over: killing guest tree");
            signal_all(tr, SIGKILL);
        }
        pid_t tid = waitpid(-1, &status, __WALL);
        if (tid < 0) {
            if (errno == EINTR) continue;
            if (errno == ECHILD) break;
            ENG_ERR("waitpid: %s", strerror(errno));
            break;
        }
        if (WIFEXITED(status) || WIFSIGNALED(status)) {
            if (tid == tr->main_pid) {
                tr->main_status = WIFEXITED(status) ? WEXITSTATUS(status) : 128 + WTERMSIG(status);
                tr->main_seen = 1;
                task_del(tr, tid);
                if (!cfg->wait_all && tr->ntasks && !stop_forwarded) {
                    /* the instance ends with its main process, like a container's init */
                    ENG_INFO("main process exited: terminating %zu remaining task(s)", tr->ntasks);
                    signal_all(tr, SIGTERM);
                    stop_forwarded = 1;
                    const char *gs = getenv("WORKFLOW_ENGINE_STOP_GRACE");
                    alarm(gs ? (unsigned)atoi(gs) : 3);
                }
                continue;
            }
            task_del(tr, tid);
            continue;
        }
        if (!WIFSTOPPED(status)) continue;

        int sig = WSTOPSIG(status);
        unsigned event = (unsigned)status >> 16;
        eng_task *t = eng_task_find(tr, tid);
        if (!t) t = task_new(tr, tid);   /* child reported before its parent's event */
        if (!t->linked) {
            /* Never run a task before we know what it inherited (phase, creds,
             * mm): hold it until the parent's PTRACE_EVENT_{FORK,VFORK,CLONE}. */
            t->held = 1;
            continue;
        }

        switch (event) {
            case 0:
                break;
            case PTRACE_EVENT_FORK:
            case PTRACE_EVENT_VFORK:
            case PTRACE_EVENT_CLONE: {
                unsigned long ctid = 0;
                if (ptrace(PTRACE_GETEVENTMSG, tid, 0, &ctid) == 0 && ctid)
                    on_new_child(tr, t, (pid_t)ctid, (int)event);
                resume_task(t, 0);
                continue;
            }
            case PTRACE_EVENT_EXEC:
                on_exec(tr, t);
                resume_task(t, 0);
                continue;
            case PTRACE_EVENT_SECCOMP:
                syscall_stop(tr, t, 1);
                continue;
            case PTRACE_EVENT_STOP:
                if (sig == SIGSTOP || sig == SIGTSTP || sig == SIGTTIN || sig == SIGTTOU) {
                    /* group-stop: stay stopped until SIGCONT, keep being notified */
                    if (ptrace(PTRACE_LISTEN, tid, 0, 0) != 0 && errno != ESRCH)
                        ENG_DBG("PTRACE_LISTEN tid=%d: %s", tid, strerror(errno));
                } else {
                    resume_task(t, 0);   /* auto-attach / interrupt stop */
                }
                continue;
            default:   /* PTRACE_EVENT_EXIT and others */
                resume_task(t, 0);
                continue;
        }

        if (sig == (SIGTRAP | 0x80)) { syscall_stop(tr, t, 0); continue; }
        if (sig == SIGSYS) { sigsys_stop(t); continue; }
        if (eng_log_enabled(ENG_LOG_DEBUG) && (sig == SIGSEGV || sig == SIGBUS || sig == SIGILL)) {
            siginfo_t si;
            eng_regs rr;
            if (ptrace(PTRACE_GETSIGINFO, tid, 0, &si) == 0 && eng_regs_get(tid, &rr) == 0)
                ENG_DBG("tid=%d signal %d code=%d addr=%p pc=%#llx phase=%d", tid, sig, si.si_code, si.si_addr,
                        (unsigned long long)eng_pc(&rr), (int)t->phase);
        }
        resume_task(t, sig);   /* signal-delivery-stop: deliver once, unchanged */
    }

    if (tr->check_toggle || getenv("WORKFLOW_ENGINE_STATS"))
        fprintf(stderr, "workflow-engine: stats syscall_stops=%lu seccomp_stops=%lu toggle_mismatch=%lu sysinfo=%d fast=%d\n",
                tr->syscall_stops, tr->seccomp_stops, tr->toggle_mismatch, tr->sysinfo, tr->fast);
    int rc = tr->main_seen ? tr->main_status : -1;
    if (tr->check_toggle && tr->toggle_mismatch) rc = 124;
    free(tr);
    return rc;
}
