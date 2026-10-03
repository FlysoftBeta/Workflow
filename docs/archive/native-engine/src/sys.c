/* sys.c — syscall dispatch for guest-phase tasks.
 *
 * Entry handlers translate guest paths into host paths written to per-thread
 * scratch, enforce virtual DAC, emulate metadata calls (chmod/chown/link/
 * access/mknod of devices), or refuse escape hatches.  Exit handlers overlay
 * virtual metadata (stat/statx), record metadata of created objects, drop
 * hardlink names, and filter getdents/listxattr.  Identity calls: ident.c.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/inotify.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <time.h>
#include <sys/sysmacros.h>
#include <unistd.h>

#include "engine/exec.h"
#include "engine/guest.h"
#include "engine/ident.h"
#include "engine/log.h"
#include "engine/mem.h"
#include "engine/meta.h"
#include "engine/path.h"
#include "engine/sysinv.h"
#include "engine/appfilter.h"
#include "engine/tracer.h"

#ifndef AT_EMPTY_PATH
#define AT_EMPTY_PATH 0x1000
#endif
#ifndef RENAME_EXCHANGE
#define RENAME_EXCHANGE (1 << 1)
#endif
#ifndef __O_TMPFILE
#define __O_TMPFILE 020000000
#endif

#define G(t) ((eng_guest *)eng_tracer_guest((t)->tr))
#define ARG(i) ((long)eng_arg(r, (i)))

/* ---- helpers ------------------------------------------------------------------- */

static long read_path(eng_task *t, uint64_t addr, char *buf) {
    if (!addr) return -EFAULT;
    ssize_t n = eng_mem_read_cstr(t->tid, addr, buf, PATH_MAX);
    if (n < 0) return -EFAULT;
    if (n >= PATH_MAX - 1) return -ENAMETOOLONG;
    return n;
}

static int put_path(eng_task *t, eng_regs *r, int arg, const char *host) {
    uint64_t a = eng_scratch_put_str(t, r, host);
    if (!a) return -ENOMEM;
    eng_set_arg(r, arg, a);
    t->regs_modified = 1;
    return 0;
}

/* Resolve the path argument.  *empty is set (and 0 returned) for "" when
 * AT_EMPTY_PATH allows it: the call then operates on the dirfd. */
static int resolve_arg(eng_task *t, eng_regs *r, int dfd_arg, int path_arg, int flags,
                       int allow_empty, eng_resolved *res, int *empty) {
    char path[PATH_MAX];
    long n = read_path(t, eng_arg(r, path_arg), path);
    if (empty) *empty = 0;
    if (n < 0) return (int)n;
    if (n == 0 && allow_empty) { if (empty) *empty = 1; return 0; }
    int dfd = dfd_arg >= 0 ? (int)eng_arg(r, dfd_arg) : AT_FDCWD;
    int rc = eng_resolve(t, dfd, path, flags, res);
    if (eng_log_enabled(ENG_LOG_TRACE))
        ENG_TRACE("tid=%d resolve \"%s\" -> %s (%d)", t->tid, path, rc ? "-" : res->host, rc);
    return rc;
}

static void fd_proc_path(eng_task *t, int fd, char *out, size_t cap) {
    snprintf(out, cap, "/proc/%d/fd/%d", t->tid, fd);
}

static int host_stat(const char *host, int nofollow, struct stat *st) {
    return (nofollow ? lstat(host, st) : stat(host, st)) == 0 ? 0 : -errno;
}

static int parent_dir(const char *host, char *out, size_t cap) {
    snprintf(out, cap, "%s", host);
    size_t n = strlen(out);
    while (n > 1 && out[n - 1] == '/') out[--n] = 0;
    char *sl = strrchr(out, '/');
    if (!sl) return -ENOENT;
    if (sl == out) sl[1] = 0; else *sl = 0;
    return 0;
}

/* W|X on the directory that will receive a new entry. */
static int may_create(eng_task *t, const char *entry_host, eng_meta *pm) {
    char dir[PATH_MAX];
    if (parent_dir(entry_host, dir, sizeof dir)) return -ENOENT;
    struct stat st;
    if (stat(dir, &st) != 0) return -errno;
    eng_meta m;
    eng_meta_get(G(t), dir, 0, &st, &m);
    if (pm) *pm = m;
    return eng_meta_permission(t, &m, W_OK | X_OK, 0);
}

/* W|X on the directory holding an entry to remove, plus the sticky-bit rule. */
static int may_delete(eng_task *t, const char *entry_host) {
    eng_meta dm;
    int rc = may_create(t, entry_host, &dm);
    if (rc) return rc;
    if ((dm.mode & S_ISVTX) && !eng_capable(t, ENG_CAP_FOWNER)) {
        struct stat st;
        if (lstat(entry_host, &st) != 0) return 0;
        eng_meta vm;
        eng_meta_get(G(t), entry_host, 1, &st, &vm);
        if (t->cr.fsuid != vm.uid && t->cr.fsuid != dm.uid) return -EPERM;
    }
    return 0;
}

static int in_rootfs(eng_task *t, const char *host) {
    eng_guest *g = G(t);
    return g->rootlen == 0 ||
           (strncmp(host, g->root, g->rootlen) == 0 && (host[g->rootlen] == '/' || host[g->rootlen] == 0));
}

/* gid for a new object: the parent's if it is set-group-ID, else the fsgid */
static uint32_t new_gid(eng_task *t, const eng_meta *parent) {
    return (parent->mode & S_ISGID) ? parent->gid : t->cr.fsgid;
}

/* ---- open / creat --------------------------------------------------------------- */

static const char *virtual_device(const eng_meta *m) {
    if ((m->mode & S_IFMT) != S_IFCHR) return NULL;
    if (m->major == 1) switch (m->minor) {
        case 3: return "/dev/null"; case 5: return "/dev/zero"; case 7: return "/dev/full";
        case 8: return "/dev/random"; case 9: return "/dev/urandom";
    }
    if (m->major == 5 && m->minor == 0) return "/dev/tty";
    if (m->major == 5 && m->minor == 2) return "/dev/ptmx";
    return NULL;
}

static int h_open(eng_task *t, eng_regs *r, int dfd_arg, int path_arg, int flags_arg, int mode_arg) {
    long flags = flags_arg >= 0 ? ARG(flags_arg) : (O_CREAT | O_WRONLY | O_TRUNC);
    int tmpfile = (flags & __O_TMPFILE) == __O_TMPFILE;
    int creat = (flags & O_CREAT) && !tmpfile;
    int follow = !((flags & O_NOFOLLOW) || (creat && (flags & O_EXCL)));
    eng_resolved res;
    int rc = resolve_arg(t, r, dfd_arg, path_arg, (follow ? ENG_RES_FOLLOW : 0) | (creat ? ENG_RES_MISSING_OK : 0),
                         0, &res, NULL);
    if (rc) return eng_task_void(t, r, rc);
    const char *host = res.host;
    if (!res.verbatim && !res.magic) {
        if (!res.exists || tmpfile) {
            eng_meta pm;
            char probe[PATH_MAX + 4];
            snprintf(probe, sizeof probe, "%s/x", res.host);   /* O_TMPFILE: the dir itself */
            if ((rc = may_create(t, tmpfile ? probe : res.entry, &pm))) return eng_task_void(t, r, rc);
            uint32_t mode = (uint32_t)ARG(mode_arg) & 07777;
            t->fix_mode = S_IFREG | (mode & ~t->cr.umask & 07777);
            t->fix_aux = new_gid(t, &pm);
            if ((t->fix_mode & S_ISGID) && !eng_in_group(t, (uint32_t)t->fix_aux, 0) &&
                !eng_capable(t, ENG_CAP_FSETID))
                t->fix_mode &= ~(uint32_t)S_ISGID;
            if (eng_meta_in_store(G(t), tmpfile ? res.host : res.entry)) {
                /* host bits: always owner rw (the virtual mode is in the store) */
                eng_set_arg(r, mode_arg, (mode | 0600) & 0777);
                t->fixup = ENG_FIX_CREATE_FD;
            } else {
                eng_set_arg(r, mode_arg, mode & 0777);   /* bind: the host bits are the mode */
            }
        } else {
            struct stat st;
            if (stat(res.host, &st) == 0) {
                eng_meta m;
                eng_meta_get(G(t), res.host, 0, &st, &m);
                int mask = 0;
                if (!(flags & O_PATH)) {
                    int acc = flags & O_ACCMODE;
                    if (acc == O_RDONLY || acc == O_RDWR) mask |= R_OK;
                    if (acc == O_WRONLY || acc == O_RDWR || (flags & O_TRUNC)) mask |= W_OK;
                }
                if (S_ISDIR(st.st_mode) && (mask & W_OK)) mask &= ~W_OK;   /* kernel: EISDIR */
                if (mask && (rc = eng_meta_permission(t, &m, mask, 0))) return eng_task_void(t, r, rc);
                const char *dev = (flags & O_PATH) ? NULL : virtual_device(&m);
                if (dev && S_ISREG(st.st_mode)) host = dev;
                if (flags & O_PATH) {
                    /* an O_PATH handle names the placeholder itself (fchmodat fallbacks, fstat) */
                } else if (S_ISREG(st.st_mode) && (m.mode & S_IFMT) == S_IFIFO) {
                    if ((rc = eng_meta_fifo_path(G(t), &st, t->fix_path, sizeof t->fix_path)))
                        return eng_task_void(t, r, rc);
                    host = t->fix_path;
                } else if (eng_meta_is_placeholder(&m, st.st_mode) && !dev) {
                    return eng_task_void(t, r, (m.mode & S_IFMT) == S_IFSOCK ? -ENXIO : -ENODEV);
                }
            }
        }
    }
    if ((rc = put_path(t, r, path_arg, host))) return eng_task_void(t, r, rc);
    return 1;
}

/* ---- stat family ------------------------------------------------------------------ */

static int h_stat(eng_task *t, eng_regs *r, int dfd_arg, int path_arg, int flag_arg, int buf_arg,
                  int nofollow_always, int statx) {
    long flags = flag_arg >= 0 ? ARG(flag_arg) : 0;
    int nofollow = nofollow_always || (flags & AT_SYMLINK_NOFOLLOW);
    eng_resolved res;
    int empty = 0;
    int rc = resolve_arg(t, r, dfd_arg, path_arg, nofollow ? 0 : ENG_RES_FOLLOW,
                         (flags & AT_EMPTY_PATH) != 0, &res, &empty);
    if (rc) return eng_task_void(t, r, rc);
    int changed = 0;
    if (empty) {
        fd_proc_path(t, (int)ARG(dfd_arg), t->fix_path, sizeof t->fix_path);
        t->fix_nofollow = 0;
    } else {
        if ((rc = put_path(t, r, path_arg, res.host))) return eng_task_void(t, r, rc);
        changed = 1;
        if (res.magic || res.verbatim) return changed;   /* /proc objects: kernel view */
        snprintf(t->fix_path, sizeof t->fix_path, "%s", res.host);
        t->fix_nofollow = nofollow && !res.stub;
    }
    t->fix_addr = eng_arg(r, buf_arg);
    t->fixup = statx ? ENG_FIX_STATX : ENG_FIX_STAT;
    return changed;
}

static int h_fstat(eng_task *t, eng_regs *r) {
    fd_proc_path(t, (int)ARG(0), t->fix_path, sizeof t->fix_path);
    t->fix_nofollow = 0;
    t->fix_addr = eng_arg(r, 1);
    t->fixup = ENG_FIX_STAT;
    return 0;
}

static void x_stat(eng_task *t) {
    struct stat st;
    if (eng_mem_read(t->tid, t->fix_addr, &st, sizeof st) != (ssize_t)sizeof st) return;
    eng_meta m;
    if (eng_meta_get(G(t), t->fix_path, t->fix_nofollow, &st, &m)) return;
    eng_meta_apply_stat(&m, &st);
    eng_mem_write(t->tid, t->fix_addr, &st, sizeof st);
}

static void x_statx(eng_task *t) {
    unsigned char b[256];
    if (eng_mem_read(t->tid, t->fix_addr, b, sizeof b) != (ssize_t)sizeof b) return;
    struct stat st;
    memset(&st, 0, sizeof st);
    uint16_t mode; uint64_t ino; uint32_t dmaj, dmin; int64_t cs; uint32_t cn;
    memcpy(&mode, b + 28, 2); memcpy(&ino, b + 32, 8);
    memcpy(&dmaj, b + 136, 4); memcpy(&dmin, b + 140, 4);
    memcpy(&cs, b + 96, 8); memcpy(&cn, b + 104, 4);
    st.st_mode = mode; st.st_ino = ino; st.st_dev = makedev(dmaj, dmin);
    st.st_ctim.tv_sec = cs; st.st_ctim.tv_nsec = cn;
    eng_meta m;
    if (eng_meta_get(G(t), t->fix_path, t->fix_nofollow, &st, &m)) return;
    eng_meta_apply_statx(&m, b);
    eng_mem_write(t->tid, t->fix_addr, b, sizeof b);
}

/* ---- access ------------------------------------------------------------------------ */

static int h_access(eng_task *t, eng_regs *r, int dfd_arg, int path_arg, int mode_arg, int flag_arg) {
    long flags = flag_arg >= 0 ? ARG(flag_arg) : 0;
    long mode = ARG(mode_arg);
    if (mode & ~(long)7) return eng_task_void(t, r, -EINVAL);
    eng_resolved res;
    int empty = 0;
    int rc = resolve_arg(t, r, dfd_arg, path_arg, (flags & AT_SYMLINK_NOFOLLOW) ? 0 : ENG_RES_FOLLOW,
                         (flags & AT_EMPTY_PATH) != 0, &res, &empty);
    if (rc) return eng_task_void(t, r, rc);
    char host[PATH_MAX];
    if (empty) fd_proc_path(t, (int)ARG(dfd_arg), host, sizeof host);
    else if (res.magic || res.verbatim) { if ((rc = put_path(t, r, path_arg, res.host))) return eng_task_void(t, r, rc); return 1; }
    else snprintf(host, sizeof host, "%s", res.host);
    struct stat st;
    int nf = (flags & AT_SYMLINK_NOFOLLOW) && !res.stub;
    if ((rc = host_stat(host, nf, &st))) return eng_task_void(t, r, rc);
    if (mode == F_OK) return eng_task_void(t, r, 0);
    eng_meta m;
    eng_meta_get(G(t), host, nf, &st, &m);
    return eng_task_void(t, r, eng_meta_permission(t, &m, (int)mode, !(flags & AT_EACCESS)));
}

/* ---- creation: mkdir, mknod, symlink ------------------------------------------------ */

static int h_mkdir(eng_task *t, eng_regs *r, int dfd_arg, int path_arg, int mode_arg) {
    eng_resolved res;
    int rc = resolve_arg(t, r, dfd_arg, path_arg, ENG_RES_MISSING_OK, 0, &res, NULL);
    if (rc) return eng_task_void(t, r, rc);
    if (res.exists) return eng_task_void(t, r, -EEXIST);
    eng_meta pm;
    if ((rc = may_create(t, res.entry, &pm))) return eng_task_void(t, r, rc);
    uint32_t mode = (uint32_t)ARG(mode_arg) & 07777;
    t->fix_mode = S_IFDIR | (mode & ~t->cr.umask & 01777) | (pm.mode & S_ISGID);
    t->fix_aux = new_gid(t, &pm);
    snprintf(t->fix_path, sizeof t->fix_path, "%s", res.entry);
    int store = eng_meta_in_store(G(t), res.entry);
    eng_set_arg(r, mode_arg, store ? (mode | 0700) & 0777 : mode & 0777);
    if ((rc = put_path(t, r, path_arg, res.entry))) return eng_task_void(t, r, rc);
    if (store) t->fixup = ENG_FIX_CREATE_PATH;
    return 1;
}

static int h_mknod(eng_task *t, eng_regs *r, int dfd_arg, int path_arg, int mode_arg, int dev_arg) {
    eng_resolved res;
    int rc = resolve_arg(t, r, dfd_arg, path_arg, ENG_RES_MISSING_OK, 0, &res, NULL);
    if (rc) return eng_task_void(t, r, rc);
    if (res.exists) return eng_task_void(t, r, -EEXIST);
    eng_meta pm;
    if ((rc = may_create(t, res.entry, &pm))) return eng_task_void(t, r, rc);
    uint32_t mode = (uint32_t)ARG(mode_arg);
    uint32_t type = mode & S_IFMT;
    if (!type) type = S_IFREG;
    uint32_t perm = mode & 07777 & ~t->cr.umask;
    int store = eng_meta_in_store(G(t), res.entry);
    if (type == S_IFCHR || type == S_IFBLK || (store && (type == S_IFIFO || type == S_IFSOCK))) {
        if ((type == S_IFCHR || type == S_IFBLK) && (!eng_capable(t, ENG_CAP_MKNOD) || !store))
            return eng_task_void(t, r, -EPERM);
        /* placeholder: regular host file, the type lives in the metadata */
        int fd = open(res.entry, O_CREAT | O_EXCL | O_WRONLY | O_CLOEXEC, 0600);
        if (fd < 0) return eng_task_void(t, r, -errno);
        close(fd);
        dev_t dev = (type == S_IFCHR || type == S_IFBLK) ? (dev_t)eng_arg(r, dev_arg) : 0;
        eng_meta m = {.uid = t->cr.fsuid, .gid = new_gid(t, &pm), .mode = type | perm,
                      .major = major(dev), .minor = minor(dev), .present = 1};
        rc = eng_meta_write(res.entry, 0, &m);
        if (rc) unlink(res.entry);
        return eng_task_void(t, r, rc);
    }
    if (type != S_IFREG && type != S_IFIFO && type != S_IFSOCK) return eng_task_void(t, r, -EINVAL);
    t->fix_mode = type | perm;
    t->fix_aux = new_gid(t, &pm);
    snprintf(t->fix_path, sizeof t->fix_path, "%s", res.entry);
    eng_set_arg(r, mode_arg, type | (store ? (perm | 0600) & 0777 : (uint32_t)ARG(mode_arg) & 0777));
    if ((rc = put_path(t, r, path_arg, res.entry))) return eng_task_void(t, r, rc);
    if (store) t->fixup = ENG_FIX_CREATE_PATH;
    return 1;
}

static int h_symlink(eng_task *t, eng_regs *r, int dfd_arg, int path_arg) {
    eng_resolved res;
    int rc = resolve_arg(t, r, dfd_arg, path_arg, ENG_RES_MISSING_OK, 0, &res, NULL);
    if (rc) return eng_task_void(t, r, rc);
    if (res.exists) return eng_task_void(t, r, -EEXIST);
    if ((rc = may_create(t, res.entry, NULL))) return eng_task_void(t, r, rc);
    if ((rc = put_path(t, r, path_arg, res.entry))) return eng_task_void(t, r, rc);
    return 1;
}

/* ---- removal / rename / link ----------------------------------------------------------- */

static int h_unlink(eng_task *t, eng_regs *r, int dfd_arg, int path_arg, int rmdir) {
    eng_resolved res;
    int rc = resolve_arg(t, r, dfd_arg, path_arg, 0, 0, &res, NULL);
    if (rc) return eng_task_void(t, r, rc);
    if (!res.verbatim && !res.magic && (rc = may_delete(t, res.entry))) return eng_task_void(t, r, rc);
    if ((rc = put_path(t, r, path_arg, res.entry))) return eng_task_void(t, r, rc);
    if (res.stub && !rmdir) {
        snprintf(t->fix_id, sizeof t->fix_id, "%s", res.stub_id);
        t->fixup = ENG_FIX_DROP_LINK;
    }
    return 1;
}

static int h_rename(eng_task *t, eng_regs *r, int d1, int p1, int d2, int p2, int flag_arg) {
    long flags = flag_arg >= 0 ? ARG(flag_arg) : 0;
    eng_resolved a, b;
    int rc = resolve_arg(t, r, d1, p1, 0, 0, &a, NULL);
    if (rc) return eng_task_void(t, r, rc);
    rc = resolve_arg(t, r, d2, p2, ENG_RES_MISSING_OK, 0, &b, NULL);
    if (rc) return eng_task_void(t, r, rc);
    if ((rc = may_delete(t, a.entry))) return eng_task_void(t, r, rc);
    if (b.exists ? (rc = may_delete(t, b.entry)) : (rc = may_create(t, b.entry, NULL)))
        return eng_task_void(t, r, rc);
    int sa = eng_meta_in_store(G(t), a.entry), sb = eng_meta_in_store(G(t), b.entry);
    /* a hardlink name only means something inside the rootfs: moving one out
     * is a cross-device move (mv copies the content) */
    if ((a.stub && !sb) || (b.stub && (flags & RENAME_EXCHANGE) && !sa)) return eng_task_void(t, r, -EXDEV);
    if ((rc = put_path(t, r, p1, a.entry)) || (rc = put_path(t, r, p2, b.entry))) return eng_task_void(t, r, rc);
    if (b.exists && b.stub && !(flags & RENAME_EXCHANGE) && !(a.stub && !strcmp(a.stub_id, b.stub_id))) {
        snprintf(t->fix_id, sizeof t->fix_id, "%s", b.stub_id);
        t->fixup = ENG_FIX_DROP_LINK;
    } else if (!sa && sb && !(flags & RENAME_EXCHANGE)) {
        /* moving in from a bind: keep the owner the guest saw */
        unsigned u = 0, gg = 0;
        eng_guest_default_owner(G(t), a.entry, &u, &gg);
        t->fix_mode = u;
        t->fix_aux = gg;
        snprintf(t->fix_path, sizeof t->fix_path, "%s", b.entry);
        t->fixup = ENG_FIX_RENAME_IN;
    }
    return 1;
}

static int h_link(eng_task *t, eng_regs *r, int d1, int p1, int d2, int p2, int flag_arg) {
    long flags = flag_arg >= 0 ? ARG(flag_arg) : 0;
    if (flags & AT_EMPTY_PATH) return eng_task_void(t, r, -EPERM);   /* needs CAP_DAC_READ_SEARCH */
    eng_resolved a, b;
    int rc = resolve_arg(t, r, d1, p1, (flags & AT_SYMLINK_FOLLOW) ? ENG_RES_FOLLOW : 0, 0, &a, NULL);
    if (rc) return eng_task_void(t, r, rc);
    rc = resolve_arg(t, r, d2, p2, ENG_RES_MISSING_OK, 0, &b, NULL);
    if (rc) return eng_task_void(t, r, rc);
    if (b.exists) return eng_task_void(t, r, -EEXIST);
    int ra = in_rootfs(t, a.entry), rb = in_rootfs(t, b.entry);
    if (!ra || !rb) return eng_task_void(t, r, ra != rb ? -EXDEV : -EPERM);
    if ((rc = may_create(t, b.entry, NULL))) return eng_task_void(t, r, rc);
    struct stat st;
    if (lstat(a.host, &st) == 0 && S_ISDIR(st.st_mode)) return eng_task_void(t, r, -EPERM);
    return eng_task_void(t, r, eng_link_create(G(t), a.entry, b.entry));
}

/* ---- chmod / chown ------------------------------------------------------------------------ */

/* Resolve the object of a metadata call: fd variant (path_arg < 0) or path. */
static int meta_target(eng_task *t, eng_regs *r, int fd_arg, int dfd_arg, int path_arg, int nofollow,
                       int allow_empty, char *host, struct stat *st, int *nf) {
    eng_resolved res;
    int empty = 0;
    if (path_arg < 0) {
        fd_proc_path(t, (int)ARG(fd_arg), host, PATH_MAX);
        *nf = 0;
    } else {
        int rc = resolve_arg(t, r, dfd_arg, path_arg, nofollow ? 0 : ENG_RES_FOLLOW, allow_empty, &res, &empty);
        if (rc) return rc;
        if (empty) { fd_proc_path(t, (int)ARG(dfd_arg), host, PATH_MAX); *nf = 0; }
        else { snprintf(host, PATH_MAX, "%s", res.host); *nf = nofollow && !res.stub; }
    }
    return host_stat(host, *nf, st);
}

static int h_chmod(eng_task *t, eng_regs *r, int fd_arg, int dfd_arg, int path_arg, int mode_arg, int flag_arg) {
    long flags = flag_arg >= 0 ? ARG(flag_arg) : 0;
    char host[PATH_MAX];
    struct stat st;
    int nf;
    int rc = meta_target(t, r, fd_arg, dfd_arg, path_arg, (flags & AT_SYMLINK_NOFOLLOW) != 0,
                         (flags & AT_EMPTY_PATH) != 0, host, &st, &nf);
    if (rc) return eng_task_void(t, r, rc);
    if (S_ISLNK(st.st_mode)) return eng_task_void(t, r, -EOPNOTSUPP);
    int lk = eng_meta_lock(G(t));
    eng_meta m;
    eng_meta_get(G(t), host, nf, &st, &m);
    if (t->cr.fsuid != m.uid && !eng_capable(t, ENG_CAP_FOWNER)) { eng_meta_unlock(lk); return eng_task_void(t, r, -EPERM); }
    uint32_t mode = (uint32_t)ARG(mode_arg) & 07777;
    if (!S_ISDIR(st.st_mode) && !eng_in_group(t, m.gid, 0) && !eng_capable(t, ENG_CAP_FSETID))
        mode &= ~(uint32_t)S_ISGID;
    if (!eng_meta_in_store(G(t), host)) {
        /* bind: the host permission bits are the mode (no set-id) */
        rc = chmod(host, mode & 0777) == 0 ? 0 : -errno;
    } else {
        m.mode = (m.mode & S_IFMT) | mode;
        rc = eng_meta_write(host, nf, &m);
        uint32_t hm = (mode & 0777) | (S_ISDIR(st.st_mode) ? 0700 : 0600);
        if (rc == -ENOTSUP || rc == -EOPNOTSUPP) rc = chmod(host, mode & 0777) == 0 ? 0 : -errno;
        else if (rc == 0 && (st.st_mode & 07777) != hm) chmod(host, hm);
    }
    eng_meta_unlock(lk);
    return eng_task_void(t, r, rc);
}

static int h_chown(eng_task *t, eng_regs *r, int fd_arg, int dfd_arg, int path_arg, int uid_arg, int gid_arg,
                   int flag_arg, int nofollow_always) {
    long flags = flag_arg >= 0 ? ARG(flag_arg) : 0;
    char host[PATH_MAX];
    struct stat st;
    int nf;
    int rc = meta_target(t, r, fd_arg, dfd_arg, path_arg, nofollow_always || (flags & AT_SYMLINK_NOFOLLOW),
                         (flags & AT_EMPTY_PATH) != 0, host, &st, &nf);
    if (rc) return eng_task_void(t, r, rc);
    uint32_t nu = (uint32_t)ARG(uid_arg), ng = (uint32_t)ARG(gid_arg);
    int lk = eng_meta_lock(G(t));
    eng_meta m;
    eng_meta_get(G(t), host, nf, &st, &m);
    rc = 0;
    if (!eng_capable(t, ENG_CAP_CHOWN)) {
        if (nu != (uint32_t)-1 && nu != m.uid) rc = -EPERM;
        else if (ng != (uint32_t)-1 && (t->cr.fsuid != m.uid || (ng != m.gid && !eng_in_group(t, ng, 0)))) rc = -EPERM;
        else if (t->cr.fsuid != m.uid && (nu != (uint32_t)-1 || ng != (uint32_t)-1)) rc = -EPERM;
    }
    /* symlink owners and bind objects are not persisted: accept, keep the view */
    if (rc || S_ISLNK(st.st_mode) || !eng_meta_in_store(G(t), host)) { eng_meta_unlock(lk); return eng_task_void(t, r, rc); }
    if (nu != (uint32_t)-1) m.uid = nu;
    if (ng != (uint32_t)-1) m.gid = ng;
    if (!S_ISDIR(st.st_mode) && (nu != (uint32_t)-1 || ng != (uint32_t)-1)) {
        m.mode &= ~(uint32_t)S_ISUID;
        if (m.mode & S_IXGRP) m.mode &= ~(uint32_t)S_ISGID;
    }
    rc = eng_meta_write(host, nf, &m);
    if (rc == -ENOTSUP || rc == -EOPNOTSUPP) rc = eng_capable(t, ENG_CAP_CHOWN) ? 0 : -EPERM;
    eng_meta_unlock(lk);
    return eng_task_void(t, r, rc);
}

/* ---- readlink / getcwd / exec --------------------------------------------------------------- */

static int h_readlink(eng_task *t, eng_regs *r, int dfd_arg, int path_arg, int buf_arg, int sz_arg) {
    eng_resolved res;
    int empty = 0;
    int rc = resolve_arg(t, r, dfd_arg, path_arg, 0, 1, &res, &empty);
    if (rc) return eng_task_void(t, r, rc);
    if (empty) return 0;
    if (res.stub) return eng_task_void(t, r, -EINVAL);
    if (res.magic) {
        long bufsz = ARG(sz_arg);
        if (bufsz <= 0) return eng_task_void(t, r, -EINVAL);
        size_t len = strlen(res.magic_text);
        if ((long)len > bufsz) len = (size_t)bufsz;
        if (eng_mem_write(t->tid, eng_arg(r, buf_arg), res.magic_text, len) != (ssize_t)len)
            return eng_task_void(t, r, -EFAULT);
        return eng_task_void(t, r, (long)len);
    }
    if ((rc = put_path(t, r, path_arg, res.entry))) return eng_task_void(t, r, rc);
    return 1;
}

static int h_getcwd(eng_task *t, eng_regs *r) {
    char cwd[PATH_MAX];
    int rc = eng_task_cwd(t, cwd, sizeof cwd);
    if (rc == -ENOENT) return 0;
    if (rc) return eng_task_void(t, r, rc);
    {
        /* an unlinked cwd: the kernel's getcwd fails with ENOENT */
        char p[64], host[PATH_MAX];
        struct stat a, b;
        snprintf(p, sizeof p, "/proc/%d/cwd", t->tid);
        if (stat(p, &a) == 0 && a.st_nlink == 0) return eng_task_void(t, r, -ENOENT);
        if (eng_guest_to_host(G(t), cwd, host, sizeof host) == 0 &&
            stat(p, &a) == 0 && (stat(host, &b) != 0 || a.st_ino != b.st_ino || a.st_dev != b.st_dev))
            return eng_task_void(t, r, -ENOENT);
    }
    size_t need = strlen(cwd) + 1;
    if ((uint64_t)ARG(1) < need) return eng_task_void(t, r, -ERANGE);
    if (eng_mem_write(t->tid, eng_arg(r, 0), cwd, need) != (ssize_t)need) return eng_task_void(t, r, -EFAULT);
    return eng_task_void(t, r, (long)need);
}

static int h_exec(eng_task *t, eng_regs *r, int is_at) {
    char path[PATH_MAX];
    int dfd = is_at ? (int)ARG(0) : AT_FDCWD;
    int pidx = is_at ? 1 : 0;
    int fl = is_at ? (int)ARG(4) : 0;
    long n = read_path(t, eng_arg(r, pidx), path);
    if (n < 0) return eng_task_void(t, r, n);
    int rc = eng_exec_prepare(t, dfd, path, fl);
    if (rc) {
        ENG_DBG("exec %s refused: %s", path, strerror(-rc));
        return eng_task_void(t, r, rc);
    }
    uint64_t a = eng_scratch_put_str(t, r, G(t)->loader);
    if (!a) { eng_exec_discard(t); return eng_task_void(t, r, -ENOMEM); }
    if (is_at) {
        uint64_t argv = eng_arg(r, 2), envp = eng_arg(r, 3);
        eng_set_arg(r, 0, a);
        eng_set_arg(r, 1, argv);
        eng_set_arg(r, 2, envp);
        eng_syscall_set(t->tid, r, __NR_execve);
    } else {
        eng_set_arg(r, 0, a);
    }
    t->regs_modified = 1;
    return 1;
}

/* ---- generic path-only syscalls ------------------------------------------------------------- */

enum { FL_FOLLOW, FL_NOFOLLOW, FL_ATFLAG, FL_INOTIFY };
typedef struct { long nr; signed char dfd, path, flagarg; unsigned char mode, nullok, dironly, entry; } pathsys;

static const pathsys PATHSYS[] = {
    {__NR_truncate, -1, 0, -1, FL_FOLLOW, 0, 0, 0},
#ifdef __NR_utime
    {__NR_utime, -1, 0, -1, FL_FOLLOW, 0, 0, 0},
#endif
#ifdef __NR_utimes
    {__NR_utimes, -1, 0, -1, FL_FOLLOW, 0, 0, 0},
#endif
#ifdef __NR_futimesat
    {__NR_futimesat, 0, 1, -1, FL_FOLLOW, 1, 0, 0},
#endif
    {__NR_utimensat, 0, 1, 3, FL_ATFLAG, 1, 0, 0},
    {__NR_statfs, -1, 0, -1, FL_FOLLOW, 0, 0, 0},
    {__NR_chdir, -1, 0, -1, FL_FOLLOW, 0, 1, 0},
    {__NR_inotify_add_watch, -1, 1, 2, FL_INOTIFY, 0, 0, 0},
#ifdef __NR_rmdir
    {__NR_rmdir, -1, 0, -1, FL_NOFOLLOW, 0, 0, 1},
#endif
};

static int h_pathsys(eng_task *t, eng_regs *r, const pathsys *d) {
    long f = d->flagarg >= 0 ? ARG(d->flagarg) : 0;
    uint64_t addr = eng_arg(r, d->path);
    if (!addr && d->nullok) return 0;
    int follow = d->mode == FL_FOLLOW || (d->mode == FL_ATFLAG && !(f & AT_SYMLINK_NOFOLLOW)) ||
                 (d->mode == FL_INOTIFY && !(f & IN_DONT_FOLLOW));
    eng_resolved res;
    int empty = 0;
    int rc = resolve_arg(t, r, d->dfd, d->path, (follow ? ENG_RES_FOLLOW : 0) | (d->dironly ? ENG_RES_DIR_ONLY : 0),
                         d->mode == FL_ATFLAG && (f & AT_EMPTY_PATH), &res, &empty);
    if (rc) return eng_task_void(t, r, rc);
    if (empty) return 0;
    if (d->nr == __NR_chdir && !res.verbatim) {
        struct stat st;
        if (stat(res.host, &st) == 0) {
            eng_meta m;
            eng_meta_get(G(t), res.host, 0, &st, &m);
            if ((rc = eng_meta_permission(t, &m, X_OK, 0))) return eng_task_void(t, r, rc);
        }
    }
    if ((rc = put_path(t, r, d->path, d->entry ? res.entry : res.host))) return eng_task_void(t, r, rc);
    return 1;
}

/* ---- xattr ------------------------------------------------------------------------------------ */

enum { XA_GET, XA_SET, XA_LIST, XA_REMOVE };

static int h_xattr(eng_task *t, eng_regs *r, int op, int path_arg, int nofollow) {
    if (op != XA_LIST) {
        char name[256];
        ssize_t n = eng_mem_read_cstr(t->tid, eng_arg(r, 1), name, sizeof name);
        if (n < 0) return eng_task_void(t, r, -EFAULT);
        if (!strncmp(name, ENG_META_PREFIX, sizeof ENG_META_PREFIX - 1))
            return eng_task_void(t, r, op == XA_GET ? -ENODATA : -EPERM);
        /* the host's SELinux labels (Android, Fedora) are not the guest's:
         * Debian runs without an LSM, where these calls see no attribute */
        if (!strcmp(name, "security.selinux"))
            return eng_task_void(t, r, op == XA_GET ? -ENODATA : -EOPNOTSUPP);
        if (op == XA_SET && !strcmp(name, "security.capability")) return eng_task_void(t, r, -EOPNOTSUPP);
    } else {
        t->fix_addr = eng_arg(r, 1);
        t->fix_len = eng_arg(r, 2);
        t->fixup = ENG_FIX_LISTXATTR;
    }
    if (path_arg < 0) return 0;
    eng_resolved res;
    int rc = resolve_arg(t, r, -1, path_arg, nofollow ? 0 : ENG_RES_FOLLOW, 0, &res, NULL);
    if (rc) { t->fixup = 0; return eng_task_void(t, r, rc); }
    if ((rc = put_path(t, r, path_arg, res.host))) { t->fixup = 0; return eng_task_void(t, r, rc); }
    return 1;
}

static void x_listxattr(eng_task *t, long ret) {
    if (ret <= 0 || !t->fix_addr || !t->fix_len) return;
    char *buf = malloc((size_t)ret);
    if (!buf) return;
    if (eng_mem_read(t->tid, t->fix_addr, buf, (size_t)ret) == ret) {
        char *out = malloc((size_t)ret);
        size_t o = 0;
        for (long i = 0; i < ret;) {
            size_t l = strnlen(buf + i, (size_t)(ret - i));
            if (strncmp(buf + i, ENG_META_PREFIX, sizeof ENG_META_PREFIX - 1) && strcmp(buf + i, "security.selinux")) {
                memcpy(out + o, buf + i, l + 1);
                o += l + 1;
            }
            i += (long)l + 1;
        }
        if ((long)o != ret) {
            eng_mem_write(t->tid, t->fix_addr, out, o);
            t->inject_result = (long)o;
            t->void_pending = 1;   /* reuse the injection path to set the result */
        }
        free(out);
    }
    free(buf);
}

/* ---- getdents64 ------------------------------------------------------------------------------ */

struct dirent64_hdr { uint64_t ino; int64_t off; uint16_t reclen; uint8_t type; } __attribute__((packed));

static void x_getdents(eng_task *t, long ret) {
    if (ret <= 0) return;
    eng_guest *g = G(t);
    char dirhost[PATH_MAX], dirguest[PATH_MAX];
    char link[64];
    snprintf(link, sizeof link, "/proc/%d/fd/%d", t->tid, (int)t->fix_aux);
    ssize_t dn = readlink(link, dirhost, sizeof dirhost - 1);
    if (dn <= 0) return;
    dirhost[dn] = 0;
    int have_guest = eng_host_to_guest(g, dirhost, dirguest, sizeof dirguest) == 0;
    unsigned char *buf = malloc((size_t)ret);
    if (!buf) return;
    if (eng_mem_read(t->tid, t->fix_addr, buf, (size_t)ret) != ret) { free(buf); return; }
    long o = 0, changed = 0;
    for (long i = 0; i + (long)sizeof(struct dirent64_hdr) <= ret;) {
        struct dirent64_hdr h;
        memcpy(&h, buf + i, sizeof h);
        if (h.reclen == 0 || i + h.reclen > ret) break;
        const char *name = (const char *)buf + i + sizeof h;
        int drop = 0;
        if (have_guest && g->nhides) {
            char gp[PATH_MAX];
            snprintf(gp, sizeof gp, "%s%s%s", dirguest, strcmp(dirguest, "/") ? "/" : "", name);
            drop = eng_guest_is_hidden(g, gp);
        }
        if (!drop && h.type == 10 /* DT_LNK */) {
            char hp[PATH_MAX], txt[PATH_MAX], id[64];
            snprintf(hp, sizeof hp, "%s/%s", dirhost, name);
            ssize_t n = readlink(hp, txt, sizeof txt - 1);
            if (n > 0) {
                txt[n] = 0;
                if (eng_link_is_stub_text(txt, id, sizeof id)) {
                    char obj[PATH_MAX];
                    struct stat st;
                    h.type = 8; /* DT_REG */
                    if (eng_link_object_path(g, id, obj, sizeof obj) == 0 && stat(obj, &st) == 0) h.ino = st.st_ino;
                    memcpy(buf + i, &h, sizeof h);
                    changed = 1;
                }
            }
        }
        if (drop) changed = 1;
        else {
            if (o != i) memmove(buf + o, buf + i, h.reclen);
            o += h.reclen;
        }
        i += h.reclen;
    }
    if (changed) {
        eng_mem_write(t->tid, t->fix_addr, buf, (size_t)o);
        if (o != ret) { t->inject_result = o; t->void_pending = 1; }
    }
    free(buf);
}

/* ---- default policy (versioned inventory, gen/syscalls.tsv) ------------------------------------ */

/* Syscalls without an explicit handler: pass the audited ones, refuse the rest.
 * A path-, metadata- or exec-carrying call must never reach the kernel
 * untranslated, and numbers newer than the inventory are unknown. */
static int default_policy(eng_task *t, eng_regs *r, long nr) {
    eng_sc_class c = eng_sysinv_class(nr);
    static unsigned char warned[1024];
    switch (c) {
        case ENG_SC_PASS: case ENG_SC_FD: case ENG_SC_PROC: case ENG_SC_SOCK: case ENG_SC_ID:
            return 0;
        case ENG_SC_EPERM:
            return eng_task_void(t, r, -EPERM);
        default:
            if (nr >= 0 && nr < (long)sizeof warned && !warned[nr]) {
                warned[nr] = 1;
                ENG_INFO("refusing syscall %ld (%s, class %s) with ENOSYS", nr,
                         eng_sysinv_name(nr) ? eng_sysinv_name(nr) : "unknown", eng_sysinv_class_name(c));
            }
            return eng_task_void(t, r, -ENOSYS);
    }
}

/* ---- AF_UNIX pathname sockets --------------------------------------------------------------------
 * bind/connect/sendto/sendmsg addresses are guest paths; the kernel gets the
 * host path.  sun_path holds 108 bytes and host paths on Android are long, so
 * a longer one goes through an alias: <sockdir>/<hash of the host directory>
 * is a symlink to that directory (connect and bind resolve symlinks in the
 * directory part), deterministic so every instance and process agrees.
 * Addresses the kernel returns (getsockname/getpeername/accept) are mapped
 * back to guest paths.  Abstract and unnamed addresses pass unchanged. */
#define SUN_PATH_MAX 108
struct wf_sockaddr_un { uint16_t family; char path[SUN_PATH_MAX]; };

static uint64_t fnv1a(const char *s) {
    uint64_t h = 1469598103934665603ull;
    for (; *s; s++) { h ^= (unsigned char)*s; h *= 1099511628211ull; }
    return h;
}

/* host path -> sun_path (maybe via alias).  Returns 0 or -errno. */
static int sock_host_name(eng_guest *g, const char *host, char *out) {
    if (strlen(host) < SUN_PATH_MAX) { snprintf(out, SUN_PATH_MAX, "%s", host); return 0; }
    char dir[PATH_MAX];
    snprintf(dir, sizeof dir, "%s", host);
    char *sl = strrchr(dir, '/');
    if (!sl || !g->sockdir[0]) return -ENAMETOOLONG;
    *sl = 0;
    const char *base = sl + 1;
    char alias[PATH_MAX];
    snprintf(alias, sizeof alias, "%s/%016llx", g->sockdir, (unsigned long long)fnv1a(dir));
    char cur[PATH_MAX];
    ssize_t n = readlink(alias, cur, sizeof cur - 1);
    if (n < 0 || (size_t)n != strlen(dir) || memcmp(cur, dir, (size_t)n)) {
        mkdir(g->sockdir, 0700);
        char tmp[PATH_MAX + 32];
        snprintf(tmp, sizeof tmp, "%s.%d", alias, (int)getpid());
        unlink(tmp);
        if (symlink(dir, tmp) != 0 || rename(tmp, alias) != 0) { unlink(tmp); return -ENAMETOOLONG; }
    }
    if (strlen(alias) + 1 + strlen(base) >= SUN_PATH_MAX) return -ENAMETOOLONG;
    snprintf(out, SUN_PATH_MAX, "%s/%s", alias, base);
    return 0;
}

/* Translate the sockaddr at arg `addr_arg` (length in arg `len_arg`, or given
 * via *lenp for msghdr).  Writes the new sockaddr to scratch; returns 0 and
 * sets new_addr and new_len, 1 if unchanged, or -errno. */
static int sock_translate(eng_task *t, eng_regs *r, uint64_t addr, uint64_t len, int creating,
                          uint64_t *new_addr, uint64_t *new_len) {
    if (!addr || len <= 2 || len > sizeof(struct wf_sockaddr_un)) return 1;
    struct wf_sockaddr_un sa;
    memset(&sa, 0, sizeof sa);
    if (eng_mem_read(t->tid, addr, &sa, (size_t)len) != (ssize_t)len) return -EFAULT;
    if (sa.family != 1 /* AF_UNIX */ || sa.path[0] == 0) return 1;
    char gp[SUN_PATH_MAX + 1];
    size_t pl = (size_t)len - 2;
    memcpy(gp, sa.path, pl);
    gp[pl] = 0;
    eng_resolved res;
    int rc = eng_resolve(t, AT_FDCWD, gp, creating ? ENG_RES_MISSING_OK : ENG_RES_FOLLOW, &res);
    if (rc) return rc;
    if (creating && res.exists) return -EADDRINUSE;
    struct wf_sockaddr_un out;
    memset(&out, 0, sizeof out);
    out.family = 1;
    if ((rc = sock_host_name(G(t), creating ? res.entry : res.host, out.path))) return rc;
    size_t ol = 2 + strlen(out.path) + 1;
    uint64_t a = eng_scratch_alloc(t, r, sizeof out);
    if (!a || eng_mem_write(t->tid, a, &out, ol) != (ssize_t)ol) return -EFAULT;
    *new_addr = a;
    *new_len = ol;
    return 0;
}

static int h_sockaddr(eng_task *t, eng_regs *r, int addr_arg, int len_arg, int creating) {
    uint64_t na, nl;
    int rc = sock_translate(t, r, eng_arg(r, addr_arg), eng_arg(r, len_arg), creating, &na, &nl);
    if (rc < 0) return eng_task_void(t, r, rc);
    if (rc == 1) return 0;
    eng_set_arg(r, addr_arg, na);
    eng_set_arg(r, len_arg, nl);
    t->regs_modified = 1;
    return 1;
}

struct wf_msghdr { uint64_t name; uint32_t namelen; uint32_t pad; uint64_t iov, iovlen, control, controllen; int32_t flags; int32_t pad2; };

static int h_sendmsg(eng_task *t, eng_regs *r) {
    struct wf_msghdr m;
    uint64_t ma = eng_arg(r, 1);
    if (!ma || eng_mem_read(t->tid, ma, &m, sizeof m) != (ssize_t)sizeof m) return 0;   /* kernel reports EFAULT */
    uint64_t na, nl;
    int rc = sock_translate(t, r, m.name, m.namelen, 0, &na, &nl);
    if (rc < 0) return eng_task_void(t, r, rc);
    if (rc == 1) return 0;
    m.name = na;
    m.namelen = (uint32_t)nl;
    uint64_t a = eng_scratch_alloc(t, r, sizeof m);
    if (!a || eng_mem_write(t->tid, a, &m, sizeof m) != (ssize_t)sizeof m) return eng_task_void(t, r, -EFAULT);
    eng_set_arg(r, 1, a);
    t->regs_modified = 1;
    return 1;
}

static void x_sockname(eng_task *t, long ret) {
    if (ret < 0 || !t->fix_addr || !t->fix_len) return;
    uint32_t len;
    if (eng_mem_read(t->tid, t->fix_len, &len, 4) != 4 || len <= 2 || len > sizeof(struct wf_sockaddr_un)) return;
    struct wf_sockaddr_un sa;
    memset(&sa, 0, sizeof sa);
    uint32_t cap = (uint32_t)t->fix_aux;
    uint32_t rd = len < cap ? len : cap;
    if (rd <= 2 || eng_mem_read(t->tid, t->fix_addr, &sa, rd) != (ssize_t)rd) return;
    if (sa.family != 1 || sa.path[0] == 0 || rd < len) return;
    char host[PATH_MAX], guest[PATH_MAX];
    snprintf(host, sizeof host, "%.*s", (int)(len - 2), sa.path);
    eng_guest *g = G(t);
    size_t sdl = strlen(g->sockdir);
    if (sdl && !strncmp(host, g->sockdir, sdl) && host[sdl] == '/') {
        /* alias: <sockdir>/<hash>/<name> -> directory the alias points to */
        char alias[PATH_MAX], dir[PATH_MAX];
        snprintf(alias, sizeof alias, "%s", host);
        char *sl = strrchr(alias, '/');
        if (!sl) return;
        *sl = 0;
        ssize_t n = readlink(alias, dir, sizeof dir - 1);
        if (n <= 0) return;
        dir[n] = 0;
        snprintf(host, sizeof host, "%s/%s", dir, sl + 1);
    }
    if (eng_host_to_guest(g, host, guest, sizeof guest)) return;
    size_t gl = strlen(guest);
    if (gl >= SUN_PATH_MAX) return;
    struct wf_sockaddr_un out;
    memset(&out, 0, sizeof out);
    out.family = 1;
    memcpy(out.path, guest, gl + 1);
    uint32_t olen = (uint32_t)(2 + gl + 1);
    uint32_t wr = olen < cap ? olen : cap;
    eng_mem_write(t->tid, t->fix_addr, &out, wr);
    eng_mem_write(t->tid, t->fix_len, &olen, 4);
}

static int h_sockname(eng_task *t, eng_regs *r, int addr_arg, int lenp_arg) {
    t->fix_addr = eng_arg(r, addr_arg);
    t->fix_len = eng_arg(r, lenp_arg);
    uint32_t cap = 0;
    if (t->fix_len && eng_mem_read(t->tid, t->fix_len, &cap, 4) != 4) cap = 0;
    t->fix_aux = cap;
    t->fixup = ENG_FIX_SOCKNAME;
    return 0;
}

/* ---- legacy x86_64 syscalls ---------------------------------------------------------------------
 * Android's x86_64 app seccomp filter only allows what bionic uses, so glibc's
 * open/stat/access/dup2/pipe/poll/select/... raise SIGSYS.  They are rewritten
 * to their *at / *2 equivalents: at the entry stop on kernels >= 4.8 (seccomp
 * runs after the ptrace stop and sees the new number), and by re-issuing the
 * call from the SIGSYS stop on older kernels (4.4: seccomp runs first).  Every
 * guest arg register is restored at the exit.  aarch64 has no legacy calls. */
#if defined(__x86_64__)
enum { LG_NONE, LG_DUP2_SAME, LG_SELECT, LG_GETDENTS, LG_ALARM };

struct lg_timeval { int64_t sec, usec; };
struct lg_timespec { int64_t sec, nsec; };

/* Convert timeval[n] at guest `a` to timespec[n] in scratch.  0 on NULL. */
static int lg_tv_to_ts(eng_task *t, const eng_regs *r, uint64_t a, int n, uint64_t *out) {
    *out = 0;
    if (!a) return 0;
    struct lg_timeval tv[2];
    struct lg_timespec ts[2];
    if (eng_mem_read(t->tid, a, tv, sizeof tv[0] * (size_t)n) != (ssize_t)(sizeof tv[0] * (size_t)n)) return -EFAULT;
    for (int i = 0; i < n; i++) { ts[i].sec = tv[i].sec; ts[i].nsec = tv[i].usec * 1000; }
    uint64_t s = eng_scratch_alloc(t, r, sizeof ts[0] * (size_t)n);
    if (!s || eng_mem_write(t->tid, s, ts, sizeof ts[0] * (size_t)n) != (ssize_t)(sizeof ts[0] * (size_t)n)) return -EFAULT;
    *out = s;
    return 0;
}

static uint64_t lg_put(eng_task *t, const eng_regs *r, const void *v, size_t n) {
    uint64_t s = eng_scratch_alloc(t, r, n);
    if (s && eng_mem_write(t->tid, s, v, n) != (ssize_t)n) s = 0;
    return s;
}

#define SETA(...) do { uint64_t _v[] = {__VA_ARGS__}; \
    for (size_t _i = 0; _i < sizeof _v / sizeof _v[0]; _i++) eng_set_arg(r, (int)_i, _v[_i]); } while (0)

/* Returns 0: not a legacy call; 1: rewritten, *nr is the new number and the
 * args in r are set; 2: emulated, *res is the result. */
static int legacy_convert(eng_task *t, eng_regs *r, long *nr, long *res) {
    uint64_t a0 = eng_arg(r, 0), a1 = eng_arg(r, 1), a2 = eng_arg(r, 2), a3 = eng_arg(r, 3), a4 = eng_arg(r, 4);
    const uint64_t CWD = (uint64_t)(long)AT_FDCWD;
    t->lg_fix = LG_NONE;
    uint64_t s;
    int rc;
    switch (*nr) {
        case __NR_open: *nr = __NR_openat; SETA(CWD, a0, a1, a2); return 1;
        case __NR_creat: *nr = __NR_openat; SETA(CWD, a0, O_CREAT | O_WRONLY | O_TRUNC, a1); return 1;
        case __NR_access: *nr = __NR_faccessat; SETA(CWD, a0, a1); return 1;
        case __NR_stat: *nr = __NR_newfstatat; SETA(CWD, a0, a1, 0); return 1;
        case __NR_lstat: *nr = __NR_newfstatat; SETA(CWD, a0, a1, AT_SYMLINK_NOFOLLOW); return 1;
        case __NR_mkdir: *nr = __NR_mkdirat; SETA(CWD, a0, a1); return 1;
        case __NR_rmdir: *nr = __NR_unlinkat; SETA(CWD, a0, AT_REMOVEDIR); return 1;
        case __NR_unlink: *nr = __NR_unlinkat; SETA(CWD, a0, 0); return 1;
        case __NR_rename: *nr = __NR_renameat; SETA(CWD, a0, CWD, a1); return 1;
        case __NR_link: *nr = __NR_linkat; SETA(CWD, a0, CWD, a1, 0); return 1;
        case __NR_symlink: *nr = __NR_symlinkat; SETA(a0, CWD, a1); return 1;
        case __NR_readlink: *nr = __NR_readlinkat; SETA(CWD, a0, a1, a2); return 1;
        case __NR_chmod: *nr = __NR_fchmodat; SETA(CWD, a0, a1); return 1;
        case __NR_chown: *nr = __NR_fchownat; SETA(CWD, a0, a1, a2, 0); return 1;
        case __NR_lchown: *nr = __NR_fchownat; SETA(CWD, a0, a1, a2, AT_SYMLINK_NOFOLLOW); return 1;
        case __NR_mknod: *nr = __NR_mknodat; SETA(CWD, a0, a1, a2); return 1;
        case __NR_utimes:
            if ((rc = lg_tv_to_ts(t, r, a1, 2, &s))) { *res = rc; return 2; }
            *nr = __NR_utimensat; SETA(CWD, a0, s, 0); return 1;
        case __NR_futimesat:
            if ((rc = lg_tv_to_ts(t, r, a2, 2, &s))) { *res = rc; return 2; }
            *nr = __NR_utimensat; SETA(a0, a1, s, 0); return 1;
        case __NR_utime: {
            s = 0;
            if (a1) {
                int64_t ub[2];
                if (eng_mem_read(t->tid, a1, ub, sizeof ub) != (ssize_t)sizeof ub) { *res = -EFAULT; return 2; }
                struct lg_timespec ts[2] = {{ub[0], 0}, {ub[1], 0}};
                if (!(s = lg_put(t, r, ts, sizeof ts))) { *res = -EFAULT; return 2; }
            }
            *nr = __NR_utimensat; SETA(CWD, a0, s, 0); return 1;
        }
        case __NR_dup2:
            if ((int)a0 == (int)a1) { *nr = __NR_fcntl; SETA(a0, F_GETFD); t->lg_fix = LG_DUP2_SAME; t->lg_a = a0; return 1; }
            *nr = __NR_dup3; SETA(a0, a1, 0); return 1;
        case __NR_pipe: *nr = __NR_pipe2; SETA(a0, 0); return 1;
        case __NR_poll: {
            s = 0;
            if ((int)a2 >= 0) {
                struct lg_timespec ts = {(int)a2 / 1000, (int64_t)((int)a2 % 1000) * 1000000};
                if (!(s = lg_put(t, r, &ts, sizeof ts))) { *res = -EFAULT; return 2; }
            }
            *nr = __NR_ppoll; SETA(a0, a1, s, 0, 8); return 1;
        }
        case __NR_select:
            if ((rc = lg_tv_to_ts(t, r, a4, 1, &s))) { *res = rc; return 2; }
            *nr = __NR_pselect6; SETA(a0, a1, a2, a3, s, 0);
            if (s) { t->lg_fix = LG_SELECT; t->lg_a = s; t->lg_b = a4; }
            return 1;
        case __NR_getdents:
            *nr = __NR_getdents64; SETA(a0, a1, a2); t->lg_fix = LG_GETDENTS; t->lg_a = a1; return 1;
        case __NR_getpgrp: *nr = __NR_getpgid; SETA(0); return 1;
        case __NR_epoll_create:
            if ((int)a0 <= 0) { *res = -EINVAL; return 2; }
            *nr = __NR_epoll_create1; SETA(0); return 1;
        case __NR_epoll_wait: *nr = __NR_epoll_pwait; SETA(a0, a1, a2, a3, 0, 8); return 1;
        case __NR_inotify_init: *nr = __NR_inotify_init1; SETA(0); return 1;
        case __NR_eventfd: *nr = __NR_eventfd2; SETA(a0, 0); return 1;
        case __NR_signalfd: *nr = __NR_signalfd4; SETA(a0, a1, a2, 0); return 1;
        case __NR_alarm: {
            int64_t it[8] = {0, 0, (int64_t)(uint32_t)a0, 0, 0, 0, 0, 0};   /* new, old */
            if (!(s = lg_put(t, r, it, sizeof it))) { *res = -EFAULT; return 2; }
            *nr = __NR_setitimer; SETA(0 /*ITIMER_REAL*/, s, s + 32);
            t->lg_fix = LG_ALARM; t->lg_a = s + 32;
            return 1;
        }
        case __NR_time: {
            int64_t now = (int64_t)time(NULL);
            if (a0 && eng_mem_write(t->tid, a0, &now, 8) != 8) { *res = -EFAULT; return 2; }
            *res = now;
            return 2;
        }
        default: return 0;
    }
}

/* Exit-side conversions.  `ret` is the result the guest will see. */
static long legacy_exit(eng_task *t, long ret) {
    int fix = t->lg_fix;
    t->lg_fix = LG_NONE;
    switch (fix) {
        case LG_DUP2_SAME: return ret < 0 ? ret : (long)(int)t->lg_a;
        case LG_SELECT: {
            struct lg_timespec ts;
            if (eng_mem_read(t->tid, t->lg_a, &ts, sizeof ts) == (ssize_t)sizeof ts) {
                struct lg_timeval tv = {ts.sec, ts.nsec / 1000};
                eng_mem_write(t->tid, t->lg_b, &tv, sizeof tv);
            }
            return ret;
        }
        case LG_GETDENTS: {
            /* linux_dirent64 {ino, off, reclen, type, name} -> linux_dirent
             * {ino, off, reclen, name, ..., type at reclen-1}: same reclen */
            if (ret <= 0) return ret;
            unsigned char *b = malloc((size_t)ret);
            if (!b) return ret;
            if (eng_mem_read(t->tid, t->lg_a, b, (size_t)ret) == ret) {
                for (long i = 0; i + 19 < ret;) {
                    uint16_t rl;
                    memcpy(&rl, b + i + 16, 2);
                    if (rl < 20 || i + rl > ret) break;
                    unsigned char type = b[i + 18];
                    size_t nl = strnlen((char *)b + i + 19, (size_t)rl - 19);
                    memmove(b + i + 18, b + i + 19, nl + 1);
                    b[i + rl - 1] = type;
                    i += rl;
                }
                eng_mem_write(t->tid, t->lg_a, b, (size_t)ret);
            }
            free(b);
            return ret;
        }
        case LG_ALARM: {
            int64_t old[4];
            if (ret < 0 || eng_mem_read(t->tid, t->lg_a, old, sizeof old) != (ssize_t)sizeof old) return ret;
            long secs = (long)old[2];
            if (old[3] >= 500000) secs++;
            if (!secs && old[3]) secs = 1;
            return secs;
        }
        default: return ret;
    }
}
#endif

/* ---- dispatch -------------------------------------------------------------------------------- */

static int dispatch(eng_task *t, eng_regs *r);
static int sys_exit_fix(eng_task *t, int fix, long ret);

int eng_sys_entry(eng_task *t, eng_regs *r) {
    /* Number -1: a call seccomp already refused.  On pre-4.8 kernels the
     * filter runs before this stop and sets the number to -1 (RET_TRAP/ERRNO);
     * the kernel then skips it and delivers SIGSYS, where eng_sys_sigsys takes
     * over.  Voiding it here would turn it into getpid and lose the call. */
    if (t->sysno == -1) return 0;
#if defined(__x86_64__)
    if (t->lg_restart) return dispatch(t, r);   /* re-issued from a SIGSYS stop: already converted */
    /* test hook: WORKFLOW_ENGINE_TEST_LEGACY=sigsys leaves legacy calls to the
     * SIGSYS re-issue path, as on a pre-4.8 kernel behind the app filter */
    static int entry_rewrite = -1;
    if (entry_rewrite < 0) {
        const char *e = getenv("WORKFLOW_ENGINE_TEST_LEGACY");
        entry_rewrite = !(e && !strcmp(e, "sigsys"));
    }
    long nr = t->sysno, res = 0;
    if (!entry_rewrite) {
        /* let seccomp see the calls Android's API 28 x86_64 app filter traps
         * (measured by the device harness), as a pre-4.8 kernel would */
        static const long api28_trapped[] = {ENG_APPFILTER_LEGACY, ENG_APPFILTER_OTHER};
        for (size_t i = 0; i < sizeof api28_trapped / sizeof api28_trapped[0]; i++)
            if (api28_trapped[i] == nr) return 0;
    }
    int lg = legacy_convert(t, r, &nr, &res);
    if (lg == 2) return eng_task_void(t, r, res);
    if (lg == 1) {
        r->orig_rax = (unsigned long)nr;
        t->sysno = nr;
        t->regs_modified = 1;       /* restores the guest's args and number at the exit */
        dispatch(t, r);
        return 1;
    }
#endif
    return dispatch(t, r);
}

static int dispatch(eng_task *t, eng_regs *r) {
    long nr = t->sysno;
    int h = eng_ident_entry(t, r);
    if (h >= 0) return h;
    for (size_t i = 0; i < sizeof PATHSYS / sizeof PATHSYS[0]; i++)
        if (PATHSYS[i].nr == nr) return h_pathsys(t, r, &PATHSYS[i]);
    switch (nr) {
#ifdef __NR_open
        case __NR_open: return h_open(t, r, -1, 0, 1, 2);
#endif
#ifdef __NR_creat
        case __NR_creat: return h_open(t, r, -1, 0, -1, 1);
#endif
        case __NR_openat: return h_open(t, r, 0, 1, 2, 3);
#ifdef __NR_stat
        case __NR_stat: return h_stat(t, r, -1, 0, -1, 1, 0, 0);
#endif
#ifdef __NR_lstat
        case __NR_lstat: return h_stat(t, r, -1, 0, -1, 1, 1, 0);
#endif
        case __NR_newfstatat: return h_stat(t, r, 0, 1, 3, 2, 0, 0);
#ifdef __NR_statx
        case __NR_statx: return h_stat(t, r, 0, 1, 2, 4, 0, 1);
#endif
        case __NR_fstat: return h_fstat(t, r);
#ifdef __NR_access
        case __NR_access: return h_access(t, r, -1, 0, 1, -1);
#endif
        case __NR_faccessat: return h_access(t, r, 0, 1, 2, -1);
#ifdef __NR_faccessat2
        case __NR_faccessat2: return h_access(t, r, 0, 1, 2, 3);
#endif
#ifdef __NR_mkdir
        case __NR_mkdir: return h_mkdir(t, r, -1, 0, 1);
#endif
        case __NR_mkdirat: return h_mkdir(t, r, 0, 1, 2);
#ifdef __NR_mknod
        case __NR_mknod: return h_mknod(t, r, -1, 0, 1, 2);
#endif
        case __NR_mknodat: return h_mknod(t, r, 0, 1, 2, 3);
#ifdef __NR_symlink
        case __NR_symlink: return h_symlink(t, r, -1, 1);
#endif
        case __NR_symlinkat: return h_symlink(t, r, 1, 2);
#ifdef __NR_unlink
        case __NR_unlink: return h_unlink(t, r, -1, 0, 0);
#endif
        case __NR_unlinkat: return h_unlink(t, r, 0, 1, (ARG(2) & AT_REMOVEDIR) != 0);
#ifdef __NR_rename
        case __NR_rename: return h_rename(t, r, -1, 0, -1, 1, -1);
#endif
#ifdef __NR_renameat
        case __NR_renameat: return h_rename(t, r, 0, 1, 2, 3, -1);
#endif
        case __NR_renameat2: return h_rename(t, r, 0, 1, 2, 3, 4);
#ifdef __NR_link
        case __NR_link: return h_link(t, r, -1, 0, -1, 1, -1);
#endif
        case __NR_linkat: return h_link(t, r, 0, 1, 2, 3, 4);
#ifdef __NR_chmod
        case __NR_chmod: return h_chmod(t, r, -1, -1, 0, 1, -1);
#endif
        case __NR_fchmod: return h_chmod(t, r, 0, -1, -1, 1, -1);
        case __NR_fchmodat: return h_chmod(t, r, -1, 0, 1, 2, -1);
#ifdef __NR_fchmodat2
        case __NR_fchmodat2: return h_chmod(t, r, -1, 0, 1, 2, 3);
#endif
#ifdef __NR_chown
        case __NR_chown: return h_chown(t, r, -1, -1, 0, 1, 2, -1, 0);
#endif
#ifdef __NR_lchown
        case __NR_lchown: return h_chown(t, r, -1, -1, 0, 1, 2, -1, 1);
#endif
        case __NR_fchown: return h_chown(t, r, 0, -1, -1, 1, 2, -1, 0);
        case __NR_fchownat: return h_chown(t, r, -1, 0, 1, 2, 3, 4, 0);
#ifdef __NR_readlink
        case __NR_readlink: return h_readlink(t, r, -1, 0, 1, 2);
#endif
        case __NR_readlinkat: return h_readlink(t, r, 0, 1, 2, 3);
        case __NR_getcwd: return h_getcwd(t, r);
        case __NR_execve: return h_exec(t, r, 0);
        case __NR_execveat: return h_exec(t, r, 1);
        case __NR_socket:
            /* no audit subsystem: sudo/login skip audit quietly instead of warning */
            if (ARG(0) == 16 /* AF_NETLINK */ && ARG(2) == 9 /* NETLINK_AUDIT */)
                return eng_task_void(t, r, -EPROTONOSUPPORT);
            return 0;
        case __NR_bind: return h_sockaddr(t, r, 1, 2, 1);
        case __NR_connect: return h_sockaddr(t, r, 1, 2, 0);
        case __NR_sendto: return h_sockaddr(t, r, 4, 5, 0);
        case __NR_sendmsg: return h_sendmsg(t, r);
        case __NR_getsockname: return h_sockname(t, r, 1, 2);
        case __NR_getpeername: return h_sockname(t, r, 1, 2);
        case __NR_accept: return h_sockname(t, r, 1, 2);
        case __NR_accept4: return h_sockname(t, r, 1, 2);
        case __NR_getdents64:
            t->fix_aux = ARG(0);
            t->fix_addr = eng_arg(r, 1);
            t->fixup = ENG_FIX_GETDENTS;
            return 0;
        case __NR_getxattr: return h_xattr(t, r, XA_GET, 0, 0);
        case __NR_lgetxattr: return h_xattr(t, r, XA_GET, 0, 1);
        case __NR_fgetxattr: return h_xattr(t, r, XA_GET, -1, 0);
        case __NR_setxattr: return h_xattr(t, r, XA_SET, 0, 0);
        case __NR_lsetxattr: return h_xattr(t, r, XA_SET, 0, 1);
        case __NR_fsetxattr: return h_xattr(t, r, XA_SET, -1, 0);
        case __NR_removexattr: return h_xattr(t, r, XA_REMOVE, 0, 0);
        case __NR_lremovexattr: return h_xattr(t, r, XA_REMOVE, 0, 1);
        case __NR_fremovexattr: return h_xattr(t, r, XA_REMOVE, -1, 0);
        case __NR_listxattr: return h_xattr(t, r, XA_LIST, 0, 0);
        case __NR_llistxattr: return h_xattr(t, r, XA_LIST, 0, 1);
        case __NR_flistxattr: return h_xattr(t, r, XA_LIST, -1, 0);

        /* escape hatches: refuse truthfully */
#ifdef __NR_io_uring_setup
        case __NR_io_uring_setup:
        case __NR_io_uring_enter:
        case __NR_io_uring_register:
#endif
#ifdef __NR_openat2
        case __NR_openat2:
#endif
#ifdef __NR_clone3
        case __NR_clone3:
#endif
#ifdef __NR_uselib
        case __NR_uselib:
#endif
            return eng_task_void(t, r, -ENOSYS);
        case __NR_name_to_handle_at: return eng_task_void(t, r, -EOPNOTSUPP);
        case __NR_open_by_handle_at:
        case __NR_mount:
        case __NR_umount2:
        case __NR_pivot_root:
        case __NR_chroot:
        case __NR_swapon:
        case __NR_swapoff:
        case __NR_acct:
        case __NR_ptrace:
        case __NR_fanotify_init:
        case __NR_fanotify_mark:
#ifdef __NR_open_tree
        case __NR_open_tree:
        case __NR_move_mount:
        case __NR_fsopen:
        case __NR_fsconfig:
        case __NR_fsmount:
        case __NR_fspick:
#endif
            return eng_task_void(t, r, -EPERM);
        default:
            return default_policy(t, r, nr);
    }
}

int eng_sys_exit(eng_task *t, eng_regs *r) {
    if (eng_ident_exit(t, r)) return 1;
    long ret = (long)eng_ret(r);
    int fix = t->fixup;
    t->fixup = ENG_FIX_NONE;
    if (t->void_pending) { t->lg_fix = 0; return 0; }
#if defined(__x86_64__)
    if (t->lg_fix) {
        int rc = sys_exit_fix(t, fix, ret);
        long cur = t->void_pending ? t->inject_result : ret;
        long nv = legacy_exit(t, cur);
        if (nv != cur) { t->inject_result = nv; t->void_pending = 1; }
        return rc;
    }
#endif
    return sys_exit_fix(t, fix, ret);
}

static int sys_exit_fix(eng_task *t, int fix, long ret) {
    eng_guest *g = G(t);
    switch (fix) {
        case ENG_FIX_STAT: if (ret == 0) x_stat(t); break;
        case ENG_FIX_STATX: if (ret == 0) x_statx(t); break;
        case ENG_FIX_CREATE_FD:
            if (ret >= 0) {
                char p[64];
                fd_proc_path(t, (int)ret, p, sizeof p);
                eng_meta m = {.uid = t->cr.fsuid, .gid = (uint32_t)t->fix_aux, .mode = t->fix_mode, .present = 1};
                eng_meta_write(p, 0, &m);
            }
            break;
        case ENG_FIX_CREATE_PATH:
            if (ret == 0) {
                eng_meta m = {.uid = t->cr.fsuid, .gid = (uint32_t)t->fix_aux, .mode = t->fix_mode, .present = 1};
                eng_meta_write(t->fix_path, 1, &m);
            }
            break;
        case ENG_FIX_DROP_LINK: if (ret == 0) eng_link_drop(g, t->fix_id); break;
        case ENG_FIX_RENAME_IN:
            if (ret == 0) {
                struct stat st;
                eng_meta m;
                if (lstat(t->fix_path, &st) == 0 && !S_ISLNK(st.st_mode) &&
                    eng_meta_read(g, t->fix_path, 1, &st, &m) == 0 && !m.present) {
                    m.uid = t->fix_mode;
                    m.gid = (uint32_t)t->fix_aux;
                    m.present = 1;
                    eng_meta_write(t->fix_path, 1, &m);
                }
            }
            break;
        case ENG_FIX_GETDENTS: x_getdents(t, ret); break;
        case ENG_FIX_SOCKNAME: x_sockname(t, ret); break;
        case ENG_FIX_LISTXATTR: x_listxattr(t, ret); break;
        default: break;
    }
    return 0;
}

int eng_sys_sigsys(eng_task *t, eng_regs *r, siginfo_t *si) {
    if (si->si_code != 1 /* SYS_SECCOMP */) return 0;
#if defined(__x86_64__)
    /* Pre-4.8 kernel: seccomp trapped a legacy call before our entry stop.
     * Re-issue it as the modern equivalent (the registers still hold the
     * call; rax was rolled back to the number), restore the args at its exit. */
    if (si->si_syscall == (int)r->rax && t->phase == ENG_PH_GUEST) {
        eng_regs saved = *r;
        long nr = si->si_syscall, res = 0;
        t->slot_off = 0;
        t->stack_scratch = 0;
        int lg = legacy_convert(t, r, &nr, &res);
        if (lg == 2) { eng_set_ret(r, (uint64_t)res); return 1; }
        if (lg == 1) {
            t->lg_saved = saved;
            t->lg_saved.orig_rax = (unsigned long)si->si_syscall;
            t->lg_restart = 1;
            t->lg_nr = nr;
            r->rax = (unsigned long)nr;
            eng_set_pc(r, eng_pc(r) - (uint64_t)eng_syscall_insn_len());
            ENG_DBG("SIGSYS tid=%d syscall=%d(%s) -> re-issued as %ld", t->tid, si->si_syscall,
                    eng_sysinv_name(si->si_syscall) ? eng_sysinv_name(si->si_syscall) : "?", nr);
            return 1;
        }
    }
#endif
    /* A call the dispatcher emulates completely (credentials, emulated links,
     * placeholders, refusals) is answered here too: the app filter's TRAP
     * wins over the fast path's TRACE, and pre-4.8 kernels trap before the
     * entry stop, so these never reach eng_sys_entry. */
    if (t->phase == ENG_PH_GUEST && eng_tracer_guest(t->tr)) {
        eng_regs saved = *r;
        long save = t->sysno;
        t->sysno = si->si_syscall;
        t->void_pending = 0;
        t->regs_modified = 0;
        t->fixup = ENG_FIX_NONE;
        t->slot_off = 0;
        t->stack_scratch = 0;
        dispatch(t, r);
        int emulated = t->void_pending;
        long res = t->inject_result;
        t->void_pending = 0;
        t->regs_modified = 0;
        t->fixup = ENG_FIX_NONE;
        t->umask_old = -1;
        t->sysno = save;
        *r = saved;
        if (emulated) {
            eng_set_ret(r, (uint64_t)res);
            ENG_DBG("SIGSYS tid=%d syscall=%d(%s) -> emulated (%ld)", t->tid, si->si_syscall,
                    eng_sysinv_name(si->si_syscall) ? eng_sysinv_name(si->si_syscall) : "?", res);
            return 1;
        }
    }
    eng_set_ret(r, (uint64_t)(-ENOSYS));
    ENG_DBG("SIGSYS tid=%d syscall=%d(%s) -> ENOSYS", t->tid, si->si_syscall,
            eng_sysinv_name(si->si_syscall) ? eng_sysinv_name(si->si_syscall) : "?");
    return 1;
}
