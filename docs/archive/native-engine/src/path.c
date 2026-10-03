/* path.c — resolve guest paths to host paths with chroot semantics.
 *
 * Works on canonical guest paths.  Each component is looked up on the host
 * with lstat(host_of(candidate)); symlinks are read and spliced into the
 * remaining path (absolute targets restart at guest "/"), ".." never climbs
 * above guest "/", binds switch the host prefix, hidden paths do not exist.
 * /proc/self and /proc/thread-self are rewritten to the tracee's ids (the
 * tracer's own /proc/self would be wrong), and /proc magic links (cwd, root,
 * exe, fd/N) are resolved to guest paths.  This is not a security boundary:
 * there is an unavoidable TOCTOU window between this walk and the kernel's. */
#define _GNU_SOURCE
#include "engine/path.h"

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "engine/guest.h"
#include "engine/ident.h"
#include "engine/log.h"
#include "engine/meta.h"

#define MAX_LINKS 40

static int read_host_link(const char *hostpath, char *out, size_t cap) {
    ssize_t n = readlink(hostpath, out, cap - 1);
    if (n < 0) return -errno;
    out[n] = 0;
    return 0;
}

/* kernel text of a /proc magic link -> guest path; -ENOENT if not mappable */
static int link_text_to_guest(const eng_guest *g, char *text, char *out, size_t cap) {
    size_t n = strlen(text);
    static const char del[] = " (deleted)";
    if (n > sizeof del - 1 && !strcmp(text + n - (sizeof del - 1), del)) text[n - (sizeof del - 1)] = 0;
    if (text[0] != '/') { snprintf(out, cap, "%s", text); return -ENOENT; }
    if (eng_host_to_guest(g, text, out, cap) != 0) { snprintf(out, cap, "%s", text); return -ENOENT; }
    return 0;
}

int eng_task_cwd(eng_task *t, char *out, size_t cap) {
    char p[64], text[PATH_MAX];
    snprintf(p, sizeof p, "/proc/%d/cwd", t->tid);
    int rc = read_host_link(p, text, sizeof text);
    if (rc) return rc;
    return link_text_to_guest(eng_tracer_guest(t->tr), text, out, cap);
}

int eng_task_fd_path(eng_task *t, pid_t pid, int fd, char *out, size_t cap) {
    char p[64], text[PATH_MAX];
    snprintf(p, sizeof p, "/proc/%d/fd/%d", pid ? pid : t->tid, fd);
    int rc = read_host_link(p, text, sizeof text);
    if (rc) return rc == -ENOENT ? -EBADF : rc;
    return link_text_to_guest(eng_tracer_guest(t->tr), text, out, cap);
}

/* X permission on a directory we look up names in (skipped for fs root). */
static int may_search(eng_task *t, const char *guest_dir) {
    if (t->cr.cap_eff & ((1ull << ENG_CAP_DAC_OVERRIDE) | (1ull << ENG_CAP_DAC_READ_SEARCH))) return 0;
    const eng_guest *g = eng_tracer_guest(t->tr);
    char host[PATH_MAX];
    if (eng_guest_to_host(g, guest_dir, host, sizeof host)) return 0;
    struct stat st;
    if (stat(host, &st) != 0 || !S_ISDIR(st.st_mode)) return 0;
    eng_meta m;
    eng_meta_get((eng_guest *)g, host, 0, &st, &m);
    return eng_meta_permission(t, &m, X_OK, 0);
}

static int is_num(const char *s, size_t n) {
    if (!n) return 0;
    for (size_t i = 0; i < n; i++) if (s[i] < '0' || s[i] > '9') return 0;
    return 1;
}

/* Is `cand` a /proc magic link?  Returns 1 and fills kind/pid/fd.
 * kind: 'c' cwd, 'r' root, 'e' exe, 'f' fd/N */
static int proc_magic(const char *cand, char *kind, pid_t *pid, int *fd) {
    if (strncmp(cand, "/proc/", 6)) return 0;
    const char *p = cand + 6, *s = strchr(p, '/');
    if (!s || !is_num(p, (size_t)(s - p))) return 0;
    *pid = (pid_t)atoi(p);
    p = s + 1;
    if (!strncmp(p, "task/", 5)) {             /* /proc/N/task/M/... */
        const char *q = p + 5, *e = strchr(q, '/');
        if (!e || !is_num(q, (size_t)(e - q))) return 0;
        *pid = (pid_t)atoi(q);
        p = e + 1;
    }
    if (!strcmp(p, "cwd")) { *kind = 'c'; return 1; }
    if (!strcmp(p, "root")) { *kind = 'r'; return 1; }
    if (!strcmp(p, "exe")) { *kind = 'e'; return 1; }
    if (!strncmp(p, "fd/", 3) && is_num(p + 3, strlen(p + 3))) { *kind = 'f'; *fd = atoi(p + 3); return 1; }
    return 0;
}

/* Guest text of a magic link; returns 0 (guest path in out), -ENOENT when the
 * target is not a guest path (raw kernel text in out), or -errno. */
static int magic_target(eng_task *t, char kind, pid_t pid, int fd, char *out, size_t cap) {
    const eng_guest *g = eng_tracer_guest(t->tr);
    char p[64], text[PATH_MAX];
    switch (kind) {
        case 'r': snprintf(out, cap, "/"); return 0;
        case 'e': {
            eng_task *o = eng_task_find(t->tr, pid);
            if (o && o->proc && o->proc->exe[0]) { snprintf(out, cap, "%s", o->proc->exe); return 0; }
            snprintf(p, sizeof p, "/proc/%d/exe", pid);
            break;
        }
        case 'c': snprintf(p, sizeof p, "/proc/%d/cwd", pid); break;
        default: snprintf(p, sizeof p, "/proc/%d/fd/%d", pid, fd); break;
    }
    int rc = read_host_link(p, text, sizeof text);
    if (rc) return rc;
    return link_text_to_guest(g, text, out, cap);
}

int eng_resolve(eng_task *t, int dirfd, const char *path, int flags, eng_resolved *out) {
    const eng_guest *g = eng_tracer_guest(t->tr);
    memset(out, 0, offsetof(eng_resolved, magic_text));
    out->magic_text[0] = 0;
    out->stub = 0;
    out->stub_id[0] = 0;
    out->entry[0] = 0;
    if (!path[0]) return -ENOENT;
    size_t plen = strlen(path);
    if (plen >= PATH_MAX) return -ENAMETOOLONG;

    char cur[PATH_MAX];
    if (path[0] == '/') {
        strcpy(cur, "/");
    } else {
        int rc = dirfd == AT_FDCWD ? eng_task_cwd(t, cur, sizeof cur)
                                   : eng_task_fd_path(t, 0, dirfd, cur, sizeof cur);
        if (rc == -ENOENT && dirfd != AT_FDCWD) {
            /* fd names something outside the guest: let the kernel resolve. */
            int n = snprintf(out->host, sizeof out->host, "/proc/%d/fd/%d/%s", t->tid, dirfd, path);
            if (n <= 0 || (size_t)n >= sizeof out->host) return -ENAMETOOLONG;
            snprintf(out->guest, sizeof out->guest, "%s", path);
            snprintf(out->entry, sizeof out->entry, "%s", out->host);
            out->verbatim = 1;
            return 0;
        }
        if (rc) return rc;
    }

    int trailing = plen > 0 && path[plen - 1] == '/';
    char todo[2 * PATH_MAX + 2];
    snprintf(todo, sizeof todo, "%s", path);
    size_t pos = 0;
    int links = 0;

    for (;;) {
        while (todo[pos] == '/') pos++;
        if (!todo[pos]) break;
        size_t start = pos;
        while (todo[pos] && todo[pos] != '/') pos++;
        size_t clen = pos - start;
        size_t after = pos;
        while (todo[after] == '/') after++;
        int last = todo[after] == 0;
        if (clen >= NAME_MAX + 1) return -ENAMETOOLONG;
        char comp[NAME_MAX + 1];
        memcpy(comp, todo + start, clen);
        comp[clen] = 0;

        if (!strcmp(comp, ".")) continue;
        if (strncmp(cur, "/proc", 5) && strncmp(cur, "/sys", 4)) {
            int prc = may_search(t, cur);
            if (prc) return prc;
        }
        if (!strcmp(comp, "..")) {
            char *sl = strrchr(cur, '/');
            if (sl && sl != cur) *sl = 0; else strcpy(cur, "/");
            continue;
        }

        /* /proc/self, /proc/thread-self -> the tracee's ids */
        if (!strcmp(cur, "/proc") && (!strcmp(comp, "self") || !strcmp(comp, "thread-self"))) {
            char tgt[64];
            if (comp[0] == 's') snprintf(tgt, sizeof tgt, "%d", t->tgid);
            else snprintf(tgt, sizeof tgt, "%d/task/%d", t->tgid, t->tid);
            char rest[2 * PATH_MAX + 2];
            snprintf(rest, sizeof rest, "%s%s", tgt, todo + pos);
            snprintf(todo, sizeof todo, "%s", rest);
            pos = 0;
            continue;
        }

        char cand[PATH_MAX];
        int n = snprintf(cand, sizeof cand, "%s%s%s", cur, strcmp(cur, "/") ? "/" : "", comp);
        if (n <= 0 || (size_t)n >= sizeof cand) return -ENAMETOOLONG;

        if (eng_guest_is_hidden(g, cand)) {
            if (last && (flags & ENG_RES_MISSING_OK)) return -EACCES;
            return -ENOENT;
        }

        int follow_here = !last || (flags & ENG_RES_FOLLOW) || trailing;

        char kind; pid_t mpid; int mfd = -1;
        if (proc_magic(cand, &kind, &mpid, &mfd)) {
            char tgt[PATH_MAX];
            int rc = magic_target(t, kind, mpid, mfd, tgt, sizeof tgt);
            if (rc && rc != -ENOENT) return rc;
            if (!follow_here) {
                /* readlink/lstat of the magic link itself */
                snprintf(out->magic_text, sizeof out->magic_text, "%s", tgt);
                out->magic = 1;
                snprintf(out->guest, sizeof out->guest, "%s", cand);
                eng_guest_to_host(g, cand, out->host, sizeof out->host);
                snprintf(out->entry, sizeof out->entry, "%s", out->host);
                out->exists = 1;
                return 0;
            }
            if (rc == -ENOENT) {
                /* not a guest path (pipe, socket, host file): kernel resolves */
                char h[PATH_MAX];
                eng_guest_to_host(g, cand, h, sizeof h);
                int m = snprintf(out->host, sizeof out->host, "%s%s", h, todo + pos);
                if (m <= 0 || (size_t)m >= sizeof out->host) return -ENAMETOOLONG;
                snprintf(out->guest, sizeof out->guest, "%s", cand);
                snprintf(out->entry, sizeof out->entry, "%s", out->host);
                out->verbatim = 1;
                out->exists = 1;
                return 0;
            }
            if (++links > MAX_LINKS) return -ELOOP;
            char rest[2 * PATH_MAX + 2];
            int m = snprintf(rest, sizeof rest, "%s%s", tgt, todo + pos);
            if (m <= 0 || (size_t)m >= sizeof rest) return -ENAMETOOLONG;
            snprintf(todo, sizeof todo, "%s", rest);
            pos = 0;
            strcpy(cur, "/");
            continue;
        }

        char host[PATH_MAX];
        if (eng_guest_to_host(g, cand, host, sizeof host)) return -ENAMETOOLONG;
        struct stat st;
        if (lstat(host, &st) != 0) {
            int e = errno;
            if (e == ENOENT && last && (flags & ENG_RES_MISSING_OK)) {
                snprintf(cur, sizeof cur, "%s", cand);
                out->exists = 0;
                goto done;
            }
            return -e;
        }
        if (S_ISLNK(st.st_mode)) {
            char tgt[PATH_MAX];
            int rc = read_host_link(host, tgt, sizeof tgt);
            if (rc) return rc;
            char id[64];
            if (eng_link_is_stub_text(tgt, id, sizeof id)) {
                /* hardlink stub: a regular file for every purpose */
                if (!last) return -ENOTDIR;
                snprintf(out->guest, sizeof out->guest, "%s", cand);
                snprintf(out->entry, sizeof out->entry, "%s", host);
                if (eng_link_object_path((eng_guest *)g, id, out->host, sizeof out->host)) return -ENAMETOOLONG;
                snprintf(out->stub_id, sizeof out->stub_id, "%s", id);
                out->stub = 1;
                out->exists = 1;
                if (trailing) return -ENOTDIR;
                return 0;
            }
            if (!follow_here) goto plain;
            if (++links > MAX_LINKS) return -ELOOP;
            if (!tgt[0]) return -ENOENT;
            char rest[2 * PATH_MAX + 2];
            int m = snprintf(rest, sizeof rest, "%s%s", tgt, todo + pos);
            if (m <= 0 || (size_t)m >= sizeof rest) return -ENAMETOOLONG;
            snprintf(todo, sizeof todo, "%s", rest);
            pos = 0;
            if (tgt[0] == '/') strcpy(cur, "/");
            continue;
        }
    plain:
        if (!last && !S_ISDIR(st.st_mode)) return -ENOTDIR;
        snprintf(cur, sizeof cur, "%s", cand);
        out->exists = 1;
        if (last) break;
    }
    if (!strcmp(cur, "/")) out->exists = 1;
done:
    snprintf(out->guest, sizeof out->guest, "%s", cur);
    if (eng_guest_to_host(g, cur, out->host, sizeof out->host)) return -ENAMETOOLONG;
    snprintf(out->entry, sizeof out->entry, "%s", out->host);
    if (trailing) {
        size_t hl = strlen(out->host);
        if (hl + 1 >= sizeof out->host) return -ENAMETOOLONG;
        if (out->host[hl - 1] != '/') { out->host[hl] = '/'; out->host[hl + 1] = 0; }
    }
    if ((flags & ENG_RES_DIR_ONLY) && out->exists) {
        struct stat st;
        if (stat(out->host, &st) == 0 && !S_ISDIR(st.st_mode)) return -ENOTDIR;
    }
    return 0;
}
