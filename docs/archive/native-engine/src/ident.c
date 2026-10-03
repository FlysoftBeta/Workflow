/* ident.c — virtual credential syscalls with the kernel's permission rules
 * (kernel/sys.c: setuid/setreuid/setresuid/setfsuid, groups; security/
 * commoncap.c: capability transitions, capget/capset, credential prctls). */
#define _GNU_SOURCE
#include "engine/ident.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/prctl.h>
#include <sys/stat.h>
#include <sys/syscall.h>

#include "engine/guest.h"
#include "engine/log.h"
#include "engine/mem.h"

#define NOCHG ((uint32_t)-1)
#define A(i) ((uint32_t)eng_arg(r, (i)))

#define SECBIT_NOROOT        (1u << 0)
#define SECBIT_NO_SETUID_FIXUP (1u << 2)
#define SECBIT_KEEP_CAPS     (1u << 4)
#define SECBIT_NO_CAP_AMBIENT_RAISE (1u << 6)
#define SECURE_ALL_BITS      0x55u
#define SECURE_ALL_LOCKS     (SECURE_ALL_BITS << 1)

/* CAP_FS_MASK: caps that follow fsuid 0 <-> non-0 */
#define CAP_FS_MASK ((1ull << 0) | (1ull << 1) | (1ull << 2) | (1ull << 3) | (1ull << 4) | \
                     (1ull << 9) | (1ull << 27) | (1ull << 32))

static int is_one_of(uint32_t v, uint32_t a, uint32_t b, uint32_t c) { return v == a || v == b || v == c; }

static int put_u32s(eng_task *t, eng_regs *r, int first_arg, const uint32_t *v, int n) {
    for (int i = 0; i < n; i++) {
        uint64_t a = eng_arg(r, first_arg + i);
        if (!a || eng_mem_write(t->tid, a, &v[i], 4) != 4) return -EFAULT;
    }
    return 0;
}

/* commoncap cap_emulate_setxuid / setfsuid fixups, after ids changed */
static void fix_setxuid(eng_cred *c, const eng_cred *old) {
    if (c->securebits & SECBIT_NO_SETUID_FIXUP) return;
    if ((old->ruid == 0 || old->euid == 0 || old->suid == 0) &&
        (c->ruid != 0 && c->euid != 0 && c->suid != 0)) {
        if (!(c->securebits & SECBIT_KEEP_CAPS)) { c->cap_prm = 0; c->cap_eff = 0; }
        c->cap_amb = 0;
    }
    if (old->euid == 0 && c->euid != 0) c->cap_eff = 0;
    if (old->euid != 0 && c->euid == 0) c->cap_eff = c->cap_prm;
}

static void fix_setfsuid(eng_cred *c, uint32_t old_fsuid) {
    if (c->securebits & SECBIT_NO_SETUID_FIXUP) return;
    if (old_fsuid == 0 && c->fsuid != 0) c->cap_eff &= ~CAP_FS_MASK;
    if (old_fsuid != 0 && c->fsuid == 0) c->cap_eff |= c->cap_prm & CAP_FS_MASK;
}

static int set_resuid(eng_task *t, uint32_t ru, uint32_t eu, uint32_t su) {
    eng_cred old = t->cr;
    if (!eng_capable(t, ENG_CAP_SETUID)) {
        if (ru != NOCHG && !is_one_of(ru, old.ruid, old.euid, old.suid)) return -EPERM;
        if (eu != NOCHG && !is_one_of(eu, old.ruid, old.euid, old.suid)) return -EPERM;
        if (su != NOCHG && !is_one_of(su, old.ruid, old.euid, old.suid)) return -EPERM;
    }
    if (ru != NOCHG) t->cr.ruid = ru;
    if (eu != NOCHG) t->cr.euid = eu;
    if (su != NOCHG) t->cr.suid = su;
    t->cr.fsuid = t->cr.euid;
    fix_setxuid(&t->cr, &old);
    return 0;
}

static int set_resgid(eng_task *t, uint32_t rg, uint32_t eg, uint32_t sg) {
    if (!eng_capable(t, ENG_CAP_SETGID)) {
        if (rg != NOCHG && !is_one_of(rg, t->cr.rgid, t->cr.egid, t->cr.sgid)) return -EPERM;
        if (eg != NOCHG && !is_one_of(eg, t->cr.rgid, t->cr.egid, t->cr.sgid)) return -EPERM;
        if (sg != NOCHG && !is_one_of(sg, t->cr.rgid, t->cr.egid, t->cr.sgid)) return -EPERM;
    }
    if (rg != NOCHG) t->cr.rgid = rg;
    if (eg != NOCHG) t->cr.egid = eg;
    if (sg != NOCHG) t->cr.sgid = sg;
    t->cr.fsgid = t->cr.egid;
    return 0;
}

static int h_setuid(eng_task *t, uint32_t u) {
    if (u == NOCHG) return -EINVAL;
    eng_cred old = t->cr;
    if (eng_capable(t, ENG_CAP_SETUID)) t->cr.ruid = t->cr.suid = u;
    else if (u != t->cr.ruid && u != t->cr.suid) return -EPERM;
    t->cr.euid = t->cr.fsuid = u;
    fix_setxuid(&t->cr, &old);
    return 0;
}

static int h_setgid(eng_task *t, uint32_t g) {
    if (g == NOCHG) return -EINVAL;
    if (eng_capable(t, ENG_CAP_SETGID)) t->cr.rgid = t->cr.sgid = g;
    else if (g != t->cr.rgid && g != t->cr.sgid) return -EPERM;
    t->cr.egid = t->cr.fsgid = g;
    return 0;
}

static int h_setreuid(eng_task *t, uint32_t ru, uint32_t eu) {
    eng_cred old = t->cr;
    if (!eng_capable(t, ENG_CAP_SETUID)) {
        if (ru != NOCHG && ru != old.ruid && ru != old.euid) return -EPERM;
        if (eu != NOCHG && !is_one_of(eu, old.ruid, old.euid, old.suid)) return -EPERM;
    }
    if (ru != NOCHG) t->cr.ruid = ru;
    if (eu != NOCHG) t->cr.euid = eu;
    if (ru != NOCHG || (eu != NOCHG && eu != old.ruid)) t->cr.suid = t->cr.euid;
    t->cr.fsuid = t->cr.euid;
    fix_setxuid(&t->cr, &old);
    return 0;
}

static int h_setregid(eng_task *t, uint32_t rg, uint32_t eg) {
    uint32_t old_r = t->cr.rgid;
    if (!eng_capable(t, ENG_CAP_SETGID)) {
        if (rg != NOCHG && rg != t->cr.rgid && rg != t->cr.egid) return -EPERM;
        if (eg != NOCHG && !is_one_of(eg, t->cr.rgid, t->cr.egid, t->cr.sgid)) return -EPERM;
    }
    if (rg != NOCHG) t->cr.rgid = rg;
    if (eg != NOCHG) t->cr.egid = eg;
    if (rg != NOCHG || (eg != NOCHG && eg != old_r)) t->cr.sgid = t->cr.egid;
    t->cr.fsgid = t->cr.egid;
    return 0;
}

static long h_setfsuid(eng_task *t, uint32_t f) {
    uint32_t old = t->cr.fsuid;
    if (f != NOCHG && (eng_capable(t, ENG_CAP_SETUID) || f == t->cr.ruid || f == t->cr.euid ||
                       f == t->cr.suid || f == t->cr.fsuid)) {
        t->cr.fsuid = f;
        fix_setfsuid(&t->cr, old);
    }
    return old;
}

static long h_setfsgid(eng_task *t, uint32_t f) {
    long old = t->cr.fsgid;
    if (f != NOCHG && (eng_capable(t, ENG_CAP_SETGID) || f == t->cr.rgid || f == t->cr.egid ||
                       f == t->cr.sgid || f == t->cr.fsgid))
        t->cr.fsgid = f;
    return old;
}

static long h_getgroups(eng_task *t, eng_regs *r) {
    long size = (long)(int)A(0);
    if (size < 0) return -EINVAL;
    if (size == 0) return t->cr.ngroups;
    if ((uint32_t)size < t->cr.ngroups) return -EINVAL;
    size_t n = t->cr.ngroups * sizeof(uint32_t);
    if (n && eng_mem_write(t->tid, eng_arg(r, 1), t->cr.groups, n) != (ssize_t)n) return -EFAULT;
    return t->cr.ngroups;
}

static long h_setgroups(eng_task *t, eng_regs *r) {
    long size = (long)(int)A(0);
    if (!eng_capable(t, ENG_CAP_SETGID)) return -EPERM;
    if (size < 0 || size > 65536) return -EINVAL;
    uint32_t n = (uint32_t)size;
    const uint32_t cap = sizeof t->cr.groups / sizeof t->cr.groups[0];
    if (n > cap) {
        ENG_WARN("setgroups: %u groups, keeping the first %u", n, cap);
        n = cap;
    }
    if (n && eng_mem_read(t->tid, eng_arg(r, 1), t->cr.groups, n * 4) != (ssize_t)(n * 4)) return -EFAULT;
    t->cr.ngroups = n;
    return 0;
}

#define CAP_V1 0x19980330u
#define CAP_V2 0x20071026u
#define CAP_V3 0x20080522u

static long h_capget(eng_task *t, eng_regs *r) {
    uint32_t hdr[2];
    if (eng_mem_read(t->tid, eng_arg(r, 0), hdr, sizeof hdr) != (ssize_t)sizeof hdr) return -EFAULT;
    int words;
    if (hdr[0] == CAP_V1) words = 1;
    else if (hdr[0] == CAP_V2 || hdr[0] == CAP_V3) words = 2;
    else {
        uint32_t v = CAP_V3;
        eng_mem_write(t->tid, eng_arg(r, 0), &v, 4);
        return eng_arg(r, 1) ? -EINVAL : 0;
    }
    if (hdr[1] != 0 && hdr[1] != (uint32_t)t->tgid && hdr[1] != (uint32_t)t->tid) {
        eng_task *o = eng_task_find(t->tr, (pid_t)hdr[1]);
        if (!o) return -ESRCH;
        t = o == NULL ? t : o;
    }
    if (!eng_arg(r, 1)) return 0;
    uint32_t data[6];
    for (int w = 0; w < 2; w++) {
        data[w * 3 + 0] = (uint32_t)(t->cr.cap_eff >> (32 * w));
        data[w * 3 + 1] = (uint32_t)(t->cr.cap_prm >> (32 * w));
        data[w * 3 + 2] = (uint32_t)(t->cr.cap_inh >> (32 * w));
    }
    size_t n = (size_t)words * 12;
    return eng_mem_write(t->tid, eng_arg(r, 1), data, n) == (ssize_t)n ? 0 : -EFAULT;
}

static long h_capset(eng_task *t, eng_regs *r) {
    uint32_t hdr[2];
    if (eng_mem_read(t->tid, eng_arg(r, 0), hdr, sizeof hdr) != (ssize_t)sizeof hdr) return -EFAULT;
    int words;
    if (hdr[0] == CAP_V1) words = 1;
    else if (hdr[0] == CAP_V2 || hdr[0] == CAP_V3) words = 2;
    else {
        uint32_t v = CAP_V3;
        eng_mem_write(t->tid, eng_arg(r, 0), &v, 4);
        return -EINVAL;
    }
    if (hdr[1] != 0 && hdr[1] != (uint32_t)t->tgid && hdr[1] != (uint32_t)t->tid) return -EPERM;
    uint32_t data[6] = {0};
    size_t n = (size_t)words * 12;
    if (eng_mem_read(t->tid, eng_arg(r, 1), data, n) != (ssize_t)n) return -EFAULT;
    uint64_t eff = 0, prm = 0, inh = 0;
    for (int w = 0; w < words; w++) {
        eff |= (uint64_t)data[w * 3 + 0] << (32 * w);
        prm |= (uint64_t)data[w * 3 + 1] << (32 * w);
        inh |= (uint64_t)data[w * 3 + 2] << (32 * w);
    }
    eff &= ENG_CAP_FULL; prm &= ENG_CAP_FULL; inh &= ENG_CAP_FULL;
    if (words == 1) {   /* v1 only touches the low word */
        eff |= t->cr.cap_eff & ~0xffffffffull;
        prm |= t->cr.cap_prm & ~0xffffffffull;
        inh |= t->cr.cap_inh & ~0xffffffffull;
    }
    /* cap_capset: inheritable within old inh|prm (or anything with SETPCAP)
     * and within inh|bset; permitted may only shrink; effective within new permitted */
    uint64_t allowed_inh = t->cr.cap_inh | (eng_capable(t, ENG_CAP_SETPCAP) ? ENG_CAP_FULL : t->cr.cap_prm);
    if (inh & ~allowed_inh) return -EPERM;
    if (inh & ~(t->cr.cap_inh | t->cr.cap_bnd)) return -EPERM;
    if (prm & ~t->cr.cap_prm) return -EPERM;
    if (eff & ~prm) return -EPERM;
    t->cr.cap_eff = eff;
    t->cr.cap_prm = prm;
    t->cr.cap_inh = inh;
    t->cr.cap_amb &= prm & inh;
    return 0;
}

#ifndef PR_CAP_AMBIENT
#define PR_CAP_AMBIENT 47
#define PR_CAP_AMBIENT_IS_SET 1
#define PR_CAP_AMBIENT_RAISE 2
#define PR_CAP_AMBIENT_LOWER 3
#define PR_CAP_AMBIENT_CLEAR_ALL 4
#endif

/* Returns 1 and sets *res when the prctl is a credential option. */
static int h_prctl(eng_task *t, eng_regs *r, long *res) {
    long op = (long)eng_arg(r, 0);
    uint64_t a2 = eng_arg(r, 1);
    switch (op) {
        case PR_GET_KEEPCAPS: *res = (t->cr.securebits & SECBIT_KEEP_CAPS) ? 1 : 0; return 1;
        case PR_SET_KEEPCAPS:
            if (a2 > 1) { *res = -EINVAL; return 1; }
            if (t->cr.securebits & (SECBIT_KEEP_CAPS << 1)) { *res = -EPERM; return 1; }
            if (a2) t->cr.securebits |= SECBIT_KEEP_CAPS; else t->cr.securebits &= ~SECBIT_KEEP_CAPS;
            *res = 0;
            return 1;
        case PR_CAPBSET_READ:
            *res = a2 > ENG_CAP_LAST ? -EINVAL : (long)((t->cr.cap_bnd >> a2) & 1);
            return 1;
        case PR_CAPBSET_DROP:
            if (a2 > ENG_CAP_LAST) { *res = -EINVAL; return 1; }
            if (!eng_capable(t, ENG_CAP_SETPCAP)) { *res = -EPERM; return 1; }
            t->cr.cap_bnd &= ~(1ull << a2);
            *res = 0;
            return 1;
        case PR_GET_SECUREBITS: *res = t->cr.securebits; return 1;
        case PR_SET_SECUREBITS: {
            uint32_t nb = (uint32_t)a2, ob = t->cr.securebits;
            if ((((ob & SECURE_ALL_LOCKS) >> 1) & (ob ^ nb)) || ((ob & SECURE_ALL_LOCKS) & ~nb) ||
                (nb & ~(SECURE_ALL_LOCKS | SECURE_ALL_BITS)) || !eng_capable(t, ENG_CAP_SETPCAP)) {
                *res = -EPERM;
                return 1;
            }
            t->cr.securebits = nb;
            *res = 0;
            return 1;
        }
        case PR_CAP_AMBIENT: {
            uint64_t cap = eng_arg(r, 2);
            if (a2 == PR_CAP_AMBIENT_CLEAR_ALL) { t->cr.cap_amb = 0; *res = 0; return 1; }
            if (cap > ENG_CAP_LAST) { *res = -EINVAL; return 1; }
            uint64_t bit = 1ull << cap;
            if (a2 == PR_CAP_AMBIENT_IS_SET) { *res = (t->cr.cap_amb & bit) ? 1 : 0; return 1; }
            if (a2 == PR_CAP_AMBIENT_LOWER) { t->cr.cap_amb &= ~bit; *res = 0; return 1; }
            if (a2 == PR_CAP_AMBIENT_RAISE) {
                if (!(t->cr.cap_prm & bit) || !(t->cr.cap_inh & bit) ||
                    (t->cr.securebits & SECBIT_NO_CAP_AMBIENT_RAISE)) { *res = -EPERM; return 1; }
                t->cr.cap_amb |= bit;
                *res = 0;
                return 1;
            }
            *res = -EINVAL;
            return 1;
        }
        case PR_GET_NO_NEW_PRIVS: *res = t->cr.nnp; return 1;
        case PR_GET_DUMPABLE: *res = !t->cr.undumpable; return 1;
        case PR_SET_DUMPABLE:
            if (a2 > 1) { *res = -EINVAL; return 1; }
            t->cr.undumpable = a2 == 0;
            *res = 0;
            return 1;
        default: return 0;
    }
}

int eng_ident_entry(eng_task *t, eng_regs *r) {
    long res;
    switch (t->sysno) {
        case __NR_getuid: res = t->cr.ruid; break;
        case __NR_geteuid: res = t->cr.euid; break;
        case __NR_getgid: res = t->cr.rgid; break;
        case __NR_getegid: res = t->cr.egid; break;
        case __NR_getresuid: { uint32_t v[3] = {t->cr.ruid, t->cr.euid, t->cr.suid}; res = put_u32s(t, r, 0, v, 3); break; }
        case __NR_getresgid: { uint32_t v[3] = {t->cr.rgid, t->cr.egid, t->cr.sgid}; res = put_u32s(t, r, 0, v, 3); break; }
        case __NR_setuid: res = h_setuid(t, A(0)); break;
        case __NR_setgid: res = h_setgid(t, A(0)); break;
        case __NR_setreuid: res = h_setreuid(t, A(0), A(1)); break;
        case __NR_setregid: res = h_setregid(t, A(0), A(1)); break;
        case __NR_setresuid: res = set_resuid(t, A(0), A(1), A(2)); break;
        case __NR_setresgid: res = set_resgid(t, A(0), A(1), A(2)); break;
        case __NR_setfsuid: res = h_setfsuid(t, A(0)); break;
        case __NR_setfsgid: res = h_setfsgid(t, A(0)); break;
        case __NR_getgroups: res = h_getgroups(t, r); break;
        case __NR_setgroups: res = h_setgroups(t, r); break;
        case __NR_capget: res = h_capget(t, r); break;
        case __NR_capset: res = h_capset(t, r); break;
        case __NR_prctl:
            if ((long)eng_arg(r, 0) == PR_SET_NO_NEW_PRIVS && eng_arg(r, 1) == 1) {
                t->cr.nnp = 1;       /* also applied by the kernel (seccomp needs it) */
                return 0;
            }
            if (!h_prctl(t, r, &res)) return 0;
            break;
        case __NR_umask:
            /* the kernel never masks owner bits, so the engine can always read
             * and write what the guest creates; the guest sees its own umask */
            t->umask_old = (int)t->cr.umask;
            t->cr.umask = A(0) & 0777;
            eng_set_arg(r, 0, t->cr.umask & 077);
            t->regs_modified = 1;
            return 1;
        default: return -1;
    }
    return eng_task_void(t, r, res);
}

int eng_ident_exit(eng_task *t, eng_regs *r) {
    if (t->sysno == __NR_umask && t->umask_old >= 0) {
        eng_set_ret(r, (uint64_t)t->umask_old);
        t->umask_old = -1;
        return 1;
    }
    return 0;
}

int eng_ident_setid(const eng_task *t, uint32_t mode, uint32_t uid, uint32_t gid, uint32_t *neuid, uint32_t *negid) {
    *neuid = t->cr.euid;
    *negid = t->cr.egid;
    if (t->cr.nnp) return 0;
    int setid = 0;
    if (mode & S_ISUID) { *neuid = uid; setid = 1; }
    if ((mode & S_ISGID) && (mode & S_IXGRP)) { *negid = gid; setid = 1; }
    return setid;
}

void eng_ident_exec(eng_task *t) {
    eng_cred *c = &t->cr;
    int setid = 0;
    if (t->plan_setid) {
        if (t->plan_euid != NOCHG && t->plan_euid != c->euid) { c->euid = t->plan_euid; setid = 1; }
        if (t->plan_egid != NOCHG && t->plan_egid != c->egid) { c->egid = t->plan_egid; setid = 1; }
    }
    /* cap_bprm_creds_from_file without file capabilities */
    if (setid) c->cap_amb = 0;
    if (!(c->securebits & SECBIT_NOROOT) && (c->euid == 0 || c->ruid == 0)) {
        c->cap_prm = (c->cap_bnd | c->cap_inh) & ENG_CAP_FULL;
        c->cap_eff = c->euid == 0 ? c->cap_prm : c->cap_amb;
    } else {
        c->cap_prm = c->cap_amb;
        c->cap_eff = c->cap_amb;
    }
    c->suid = c->fsuid = c->euid;
    c->sgid = c->fsgid = c->egid;
    c->securebits &= ~SECBIT_KEEP_CAPS;
    c->undumpable = setid;   /* the kernel drops dumpable on a set-id exec */
    t->plan_setid = 0;
}

/* ---- initial groups from the guest's databases ------------------------------------ */

static int read_guest_file(struct eng_guest *g, const char *path, char **out) {
    char host[PATH_MAX];
    if (eng_guest_to_host(g, path, host, sizeof host)) return -1;
    FILE *f = fopen(host, "re");
    if (!f) return -1;
    size_t cap = 1 << 16, len = 0;
    char *buf = malloc(cap);
    size_t n;
    while (buf && (n = fread(buf + len, 1, cap - len - 1, f)) > 0) {
        len += n;
        if (len + 1 >= cap) { cap *= 2; buf = realloc(buf, cap); }
    }
    fclose(f);
    if (!buf) return -1;
    buf[len] = 0;
    *out = buf;
    return 0;
}

void eng_ident_init(struct eng_guest *g, eng_task *t, uint32_t uid, uint32_t gid) {
    eng_cred *c = &t->cr;
    memset(c, 0, sizeof *c);
    c->ruid = c->euid = c->suid = c->fsuid = uid;
    c->rgid = c->egid = c->sgid = c->fsgid = gid;
    c->cap_bnd = ENG_CAP_FULL;
    if (uid == 0) c->cap_prm = c->cap_eff = ENG_CAP_FULL;
    c->umask = 022;
    t->umask_old = -1;
    c->ngroups = 0;
    c->groups[c->ngroups++] = gid;
    if (!g) return;
    char *pw = NULL, *gr = NULL, name[256] = "";
    if (read_guest_file(g, "/etc/passwd", &pw) == 0) {
        for (char *save = NULL, *l = strtok_r(pw, "\n", &save); l; l = strtok_r(NULL, "\n", &save)) {
            char *f[4] = {0};
            char *p = l;
            for (int i = 0; i < 4 && p; i++) { f[i] = p; p = strchr(p, ':'); if (p) *p++ = 0; }
            if (f[2] && (uint32_t)strtoul(f[2], NULL, 10) == uid) { snprintf(name, sizeof name, "%s", f[0]); break; }
        }
        free(pw);
    }
    if (!name[0] || read_guest_file(g, "/etc/group", &gr) != 0) return;
    for (char *save = NULL, *l = strtok_r(gr, "\n", &save); l; l = strtok_r(NULL, "\n", &save)) {
        char *f[4] = {0};
        char *p = l;
        for (int i = 0; i < 4 && p; i++) { f[i] = p; p = strchr(p, ':'); if (p) *p++ = 0; }
        if (!f[2] || !f[3]) continue;
        uint32_t id = (uint32_t)strtoul(f[2], NULL, 10);
        for (char *s2 = NULL, *m = strtok_r(f[3], ",", &s2); m; m = strtok_r(NULL, ",", &s2)) {
            if (strcmp(m, name) || id == gid) continue;
            if (c->ngroups < sizeof c->groups / sizeof c->groups[0]) c->groups[c->ngroups++] = id;
        }
    }
    free(gr);
}
