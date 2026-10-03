/* exec.c — exec planning.  Mirrors the kernel's execve decisions in guest
 * space: resolve, permission, script (#!) chains with the kernel's argv
 * rewriting, ELF ABI check, PT_INTERP resolution inside the guest. */
#define _GNU_SOURCE
#include "engine/exec.h"

#include <elf.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "engine/guest.h"
#include "engine/ident.h"
#include "engine/loader_proto.h"
#include "engine/log.h"
#include "engine/meta.h"
#include "engine/path.h"

#if defined(__aarch64__)
#define EM_SELF EM_AARCH64
#else
#define EM_SELF EM_X86_64
#endif

#define BINPRM_BUF 256
#define MAX_SCRIPT_DEPTH 5   /* like the kernel's bprm->have_execfd-free recursion limit */
#define MAX_PREPEND 16

typedef struct {
    char *s[MAX_PREPEND];
    int n;
} strlist;

static void sl_free(strlist *l) { for (int i = 0; i < l->n; i++) free(l->s[i]); l->n = 0; }
static int sl_push_front(strlist *l, const char *s) {
    if (l->n >= MAX_PREPEND) return -E2BIG;
    memmove(&l->s[1], &l->s[0], sizeof(char *) * (size_t)l->n);
    l->s[0] = strdup(s);
    l->n++;
    return 0;
}
static void sl_pop_front(strlist *l) {
    if (!l->n) return;
    free(l->s[0]);
    memmove(&l->s[0], &l->s[1], sizeof(char *) * (size_t)(l->n - 1));
    l->n--;
}

/* Parse "#!interp [arg]" like fs/binfmt_script.c.  Returns 0, or -ENOEXEC. */
static int parse_shebang(const char *buf, size_t n, char *interp, size_t icap, char *arg, size_t acap, int *has_arg) {
    const char *end = memchr(buf, '\n', n);
    int truncated = end == NULL;
    if (!end) end = buf + n;
    const char *p = buf + 2;
    while (p < end && (*p == ' ' || *p == '\t')) p++;
    const char *e = end;
    while (e > p && (e[-1] == ' ' || e[-1] == '\t' || e[-1] == '\r' || e[-1] == 0)) e--;
    if (p >= e) return -ENOEXEC;
    const char *ie = p;
    while (ie < e && *ie != ' ' && *ie != '\t' && *ie) ie++;
    if (truncated && ie == e) return -ENOEXEC;   /* interpreter name itself cut off */
    size_t il = (size_t)(ie - p);
    if (il >= icap) return -ENAMETOOLONG;
    memcpy(interp, p, il);
    interp[il] = 0;
    const char *a = ie;
    while (a < e && (*a == ' ' || *a == '\t')) a++;
    *has_arg = a < e;
    if (*has_arg) {
        size_t al = (size_t)(e - a);
        if (al >= acap) al = acap - 1;
        memcpy(arg, a, al);
        arg[al] = 0;
    }
    return 0;
}

/* Read PT_INTERP of an ELF already validated for this ABI.  Returns 1 with
 * interp filled, 0 if static, -errno on malformed input. */
static int elf_interp(int fd, const Elf64_Ehdr *eh, char *interp, size_t cap) {
    if (eh->e_phentsize != sizeof(Elf64_Phdr) || eh->e_phnum == 0 || eh->e_phnum > 256) return -ENOEXEC;
    Elf64_Phdr ph[256];
    size_t sz = (size_t)eh->e_phnum * sizeof(Elf64_Phdr);
    if (pread(fd, ph, sz, (off_t)eh->e_phoff) != (ssize_t)sz) return -ENOEXEC;
    int loads = 0;
    for (int i = 0; i < eh->e_phnum; i++) {
        if (ph[i].p_type == PT_LOAD) loads++;
        if (ph[i].p_type != PT_INTERP) continue;
        if (ph[i].p_filesz < 2 || ph[i].p_filesz >= cap) return -ENOEXEC;
        if (pread(fd, interp, ph[i].p_filesz, (off_t)ph[i].p_offset) != (ssize_t)ph[i].p_filesz) return -ENOEXEC;
        interp[ph[i].p_filesz] = 0;
        if (interp[ph[i].p_filesz - 1] != 0) return -ENOEXEC;
        return 1;
    }
    return loads ? 0 : -ENOEXEC;
}

/* Open and check one exec candidate.  Returns fd or -errno. */
static int open_candidate(eng_task *t, const eng_resolved *res, struct stat *st) {
    int fd = open(res->host, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return -errno;
    if (fstat(fd, st) != 0) { int e = errno; close(fd); return -e; }
    if (!S_ISREG(st->st_mode)) { close(fd); return -EACCES; }
    eng_meta vm;
    eng_meta_get(eng_tracer_guest(t->tr), res->host, 0, st, &vm);
    if (eng_meta_is_placeholder(&vm, st->st_mode)) { close(fd); return -EACCES; }
    int rc = eng_meta_may_exec(t, res->host, st);
    if (rc) { close(fd); return rc; }
    return fd;
}

static const eng_binfmt *match_binfmt(const eng_guest *g, const unsigned char *buf, size_t n, const char *filename) {
    for (int i = g->nbinfmt - 1; i >= 0; i--) {   /* newest first */
        const eng_binfmt *b = &g->binfmt[i];
        if (b->type == 'E') {
            const char *base = strrchr(filename, '/');
            base = base ? base + 1 : filename;
            const char *dot = strrchr(base, '.');
            if (dot && !strcmp(dot + 1, b->ext)) return b;
            continue;
        }
        if (b->offset + b->len > n) continue;
        unsigned k = 0;
        while (k < b->len && (buf[b->offset + k] & b->mask[k]) == b->magic[k]) k++;
        if (k == b->len) return b;
    }
    return NULL;
}

static int build_plan(eng_task *t, const char *exe_host, const char *interp_host,
                      const char *execfn, const char *comm, uint32_t skip, const strlist *pre, uint32_t flags) {
    size_t need = sizeof(eng_load_plan) + strlen(exe_host) + 1 + (interp_host ? strlen(interp_host) + 1 : 0) +
                  strlen(execfn) + 1 + strlen(comm) + 1;
    for (int i = 0; i < pre->n; i++) need += strlen(pre->s[i]) + 1;
    if (need > ENG_PLAN_MAX) return -E2BIG;
    char *buf = calloc(1, need);
    if (!buf) return -ENOMEM;
    eng_load_plan *pl = (eng_load_plan *)buf;
    size_t off = sizeof *pl;
#define PUT(field, str) do { size_t _l = strlen(str) + 1; memcpy(buf + off, (str), _l); pl->field = (uint32_t)off; off += _l; } while (0)
    PUT(exe_off, exe_host);
    if (interp_host) PUT(interp_off, interp_host);
    PUT(execfn_off, execfn);
    PUT(comm_off, comm);
    if (pre->n) {
        pl->prepend_off = (uint32_t)off;
        for (int i = 0; i < pre->n; i++) { size_t l = strlen(pre->s[i]) + 1; memcpy(buf + off, pre->s[i], l); off += l; }
    }
#undef PUT
    pl->magic = ENG_PLAN_MAGIC;
    pl->version = ENG_PLAN_VERSION;
    pl->size = (uint32_t)off;
    pl->flags = flags;
    pl->argv_skip = skip;
    pl->n_prepend = (uint32_t)pre->n;
    pl->uid = t->cr.ruid; pl->euid = t->cr.euid; pl->gid = t->cr.rgid; pl->egid = t->cr.egid;
    free(t->plan);
    t->plan = buf;
    t->plan_len = (uint32_t)off;
    return 0;
}

static int prepare(eng_task *t, int dirfd, const char *path, int at_flags) {
    const eng_guest *g = eng_tracer_guest(t->tr);
    char execfn[PATH_MAX];
    eng_resolved res;
    int rc;

    if ((at_flags & AT_EMPTY_PATH) && path[0] == 0) {
        if (dirfd == AT_FDCWD) return -ENOENT;
        char gp[PATH_MAX];
        snprintf(execfn, sizeof execfn, "/dev/fd/%d", dirfd);
        rc = eng_task_fd_path(t, 0, dirfd, gp, sizeof gp);
        memset(&res, 0, sizeof res);
        if (rc == 0) {
            snprintf(res.guest, sizeof res.guest, "%s", gp);
            eng_guest_to_host(g, gp, res.host, sizeof res.host);
        } else if (rc == -ENOENT) {
            snprintf(res.host, sizeof res.host, "/proc/%d/fd/%d", t->tid, dirfd);
            snprintf(res.guest, sizeof res.guest, "%s", execfn);
        } else {
            return rc;
        }
    } else {
        if (!path[0]) return -ENOENT;
        if (path[0] == '/' || dirfd == AT_FDCWD) snprintf(execfn, sizeof execfn, "%s", path);
        else snprintf(execfn, sizeof execfn, "/dev/fd/%d/%s", dirfd, path);
        int fl = (at_flags & AT_SYMLINK_NOFOLLOW) ? 0 : ENG_RES_FOLLOW;
        rc = eng_resolve(t, dirfd, path, fl, &res);
        if (rc) return rc;
        struct stat lst;
        if ((at_flags & AT_SYMLINK_NOFOLLOW) && lstat(res.host, &lst) == 0 && S_ISLNK(lst.st_mode)) return -ELOOP;
    }

    /* task name: basename of the path given (for fd-based execs the file's own
     * name, like current kernels), at most 15 bytes */
    char comm[16];
    {
        const char *src = (at_flags & AT_EMPTY_PATH) && path[0] == 0 ? res.guest : path;
        const char *b = strrchr(src, '/');
        b = b && b[1] ? b + 1 : src;
        snprintf(comm, sizeof comm, "%s", b);
    }
    /* fd-based exec of a script whose fd is close-on-exec: the interpreter could
     * never open /dev/fd/N, so the kernel fails it with ENOENT */
    int path_inaccessible = 0;
    if (dirfd != AT_FDCWD && path[0] != '/') {
        char fi[64], buf[512];
        snprintf(fi, sizeof fi, "/proc/%d/fdinfo/%d", t->tid, dirfd);
        int ff = open(fi, O_RDONLY | O_CLOEXEC);
        if (ff >= 0) {
            ssize_t k = read(ff, buf, sizeof buf - 1);
            close(ff);
            if (k > 0) {
                buf[k] = 0;
                char *fl = strstr(buf, "flags:");
                if (fl && (strtoul(fl + 6, NULL, 8) & O_CLOEXEC)) path_inaccessible = 1;
            }
        }
    }

    strlist pre = {.n = 0};
    uint32_t skip = 0;
    char filename[PATH_MAX];
    snprintf(filename, sizeof filename, "%s", execfn);
    for (int depth = 0;; depth++) {
        struct stat st;
        int fd = open_candidate(t, &res, &st);
        if (fd < 0) { sl_free(&pre); return fd; }
        unsigned char buf[BINPRM_BUF];
        ssize_t n = pread(fd, buf, sizeof buf, 0);
        if (n < 0) { int e = errno; close(fd); sl_free(&pre); return -e; }

        /* binfmt rules first, like binfmt_misc (registered at the front) */
        const eng_binfmt *bf = match_binfmt(g, buf, (size_t)n, filename);
        if (bf) {
            close(fd);
            if (depth >= MAX_SCRIPT_DEPTH) { sl_free(&pre); return -ELOOP; }
            if (depth == 0 && path_inaccessible) { sl_free(&pre); return -ENOENT; }
            if (!bf->preserve_argv0) { if (pre.n) sl_pop_front(&pre); else skip++; }
            if ((rc = sl_push_front(&pre, filename)) || (rc = sl_push_front(&pre, bf->interp))) {
                sl_free(&pre);
                return rc;
            }
            snprintf(filename, sizeof filename, "%s", bf->interp);
            rc = eng_resolve(t, AT_FDCWD, bf->interp, ENG_RES_FOLLOW, &res);
            if (rc) { sl_free(&pre); return rc == -ENOTDIR ? -ENOENT : rc; }
            continue;
        }

        if (n >= 2 && buf[0] == '#' && buf[1] == '!') {
            close(fd);
            if (depth >= MAX_SCRIPT_DEPTH) { sl_free(&pre); return -ELOOP; }
            if (depth == 0 && path_inaccessible) { sl_free(&pre); return -ENOENT; }
            char interp[PATH_MAX], arg[PATH_MAX];
            int has_arg = 0;
            rc = parse_shebang((const char *)buf, (size_t)n, interp, sizeof interp, arg, sizeof arg, &has_arg);
            if (rc) { sl_free(&pre); return rc; }
            if (pre.n) sl_pop_front(&pre); else skip++;
            if ((rc = sl_push_front(&pre, filename)) ||
                (has_arg && (rc = sl_push_front(&pre, arg))) ||
                (rc = sl_push_front(&pre, interp))) { sl_free(&pre); return rc; }
            snprintf(filename, sizeof filename, "%s", interp);
            rc = eng_resolve(t, AT_FDCWD, interp, ENG_RES_FOLLOW, &res);
            if (rc) { sl_free(&pre); return rc; }
            continue;
        }

        if (n >= (ssize_t)sizeof(Elf64_Ehdr) && !memcmp(buf, ELFMAG, SELFMAG)) {
            const Elf64_Ehdr *eh = (const Elf64_Ehdr *)buf;
            if (eh->e_ident[EI_CLASS] != ELFCLASS64 || eh->e_ident[EI_DATA] != ELFDATA2LSB ||
                eh->e_machine != EM_SELF || (eh->e_type != ET_EXEC && eh->e_type != ET_DYN)) {
                close(fd); sl_free(&pre);
                return -ENOEXEC;   /* foreign ABI: binfmt rules (M4) */
            }
            char interp[PATH_MAX];
            int hi = elf_interp(fd, eh, interp, sizeof interp);
            close(fd);
            if (hi < 0) { sl_free(&pre); return hi; }
            eng_resolved ires;
            if (hi) {
                rc = eng_resolve(t, AT_FDCWD, interp, ENG_RES_FOLLOW, &ires);
                if (rc) { sl_free(&pre); return rc == -ENOTDIR ? -ENOENT : rc; }
                struct stat ist;
                int ifd = open_candidate(t, &ires, &ist);
                if (ifd < 0) { sl_free(&pre); return ifd; }
                close(ifd);
            }
            uint32_t flags = g->no_filemap ? ENG_PLAN_NO_FILEMAP : 0;
            /* virtual set-user/group-ID of the ELF that will actually run */
            eng_meta em;
            eng_meta_get((eng_guest *)g, res.host, 0, &st, &em);
            uint32_t neu, neg;
            t->plan_setid = eng_ident_setid(t, em.mode, em.uid, em.gid, &neu, &neg);
            t->plan_euid = neu;
            t->plan_egid = neg;
            if (neu != t->cr.ruid || neg != t->cr.rgid) flags |= ENG_PLAN_SECURE;
            if (g->test_pagesz) flags |= ENG_PLAN_PAGESZ;
            rc = build_plan(t, res.host, hi ? ires.host : NULL, execfn, comm, skip, &pre, flags);
            if (rc == 0) {
                eng_load_plan *pl = (eng_load_plan *)t->plan;
                pl->pagesz = g->test_pagesz;
                pl->euid = neu;
                pl->egid = neg;
            }
            sl_free(&pre);
            if (rc) return rc;
            snprintf(t->plan_exe, sizeof t->plan_exe, "%s", res.guest);
            return 0;
        }
        close(fd);
        sl_free(&pre);
        return -ENOEXEC;
    }
}

int eng_exec_prepare(eng_task *t, int dirfd, const char *path, int at_flags) {
    return prepare(t, dirfd, path, at_flags);
}

int eng_exec_prepare_initial(eng_task *t, const char *path) {
    return prepare(t, AT_FDCWD, path, 0);
}

void eng_exec_discard(eng_task *t) {
    free(t->plan);
    t->plan = NULL;
    t->plan_len = 0;
}
