/* meta.c — metadata xattr store, DAC checks and hardlink emulation. */
#define _GNU_SOURCE
#include "engine/meta.h"

#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/file.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <sys/xattr.h>
#include <time.h>
#include <unistd.h>

#include "engine/guest.h"
#include "engine/ident.h"
#include "engine/log.h"

/* ---- xattr codec -------------------------------------------------------------- */

static int parse_meta(const char *s, eng_meta *m) {
    unsigned ver, uid, gid, mode, nlink, maj, min;
    if (sscanf(s, "%u %u %u %o %u %u,%u", &ver, &uid, &gid, &mode, &nlink, &maj, &min) != 7 || ver != 1)
        return -1;
    m->uid = uid; m->gid = gid; m->mode = mode; m->nlink = nlink; m->major = maj; m->minor = min;
    m->present = 1;
    return 0;
}

/* The object a host path names: fd paths (/proc/N/fd/M) are followed so the
 * store decision uses the real location. */
static const char *object_path(const char *host, char *buf, size_t cap) {
    if (strncmp(host, "/proc/", 6) || !strstr(host, "/fd/")) return host;
    ssize_t n = readlink(host, buf, cap - 1);
    if (n <= 0) return host;
    buf[n] = 0;
    return buf[0] == '/' ? buf : host;
}

int eng_meta_in_store(struct eng_guest *g, const char *host) {
    if (!g) return 1;
    char buf[PATH_MAX];
    return eng_guest_locate(g, object_path(host, buf, sizeof buf)) == ENG_LOC_ROOTFS;
}

int eng_meta_read(struct eng_guest *g, const char *host, int nofollow, const struct stat *st, eng_meta *m) {
    memset(m, 0, sizeof *m);
    char buf[PATH_MAX];
    const char *obj = object_path(host, buf, sizeof buf);
    int loc = g ? eng_guest_locate(g, obj) : ENG_LOC_ROOTFS;
    if (loc == ENG_LOC_ROOTFS && !S_ISLNK(st->st_mode)) {
        char val[128];
        ssize_t n = nofollow ? lgetxattr(host, ENG_META_XATTR, val, sizeof val - 1)
                             : getxattr(host, ENG_META_XATTR, val, sizeof val - 1);
        if (n > 0) {
            val[n] = 0;
            if (parse_meta(val, m) == 0) {
                /* the real type wins unless the xattr describes a placeholder */
                if (!eng_meta_is_placeholder(m, st->st_mode))
                    m->mode = (st->st_mode & S_IFMT) | (m->mode & 07777);
                return 0;
            }
        }
    }
    /* no metadata: binds present their owner and the host permission bits;
     * the rootfs presents root and the host bits (installer/engine always
     * write metadata, so this is only for foreign files) */
    if (loc >= 0) { m->uid = g->binds[loc].uid; m->gid = g->binds[loc].gid; }
    if (S_ISLNK(st->st_mode)) {
        if (loc == ENG_LOC_ROOTFS) {
            /* symlinks carry no xattr: present them as owned by their directory's owner */
            char dir[PATH_MAX];
            snprintf(dir, sizeof dir, "%s", obj);
            char *sl = strrchr(dir, '/');
            if (sl && sl != dir) {
                *sl = 0;
                struct stat dst;
                eng_meta dm;
                if (lstat(dir, &dst) == 0 && !S_ISLNK(dst.st_mode) && eng_meta_read(g, dir, 1, &dst, &dm) == 0) {
                    m->uid = dm.uid;
                    m->gid = dm.gid;
                }
            }
        }
        m->mode = S_IFLNK | 0777;
    } else {
        m->mode = st->st_mode & (S_IFMT | 0777);
    }
    m->present = 0;
    return 0;
}

int eng_meta_get(struct eng_guest *g, const char *host, int nofollow, const struct stat *st, eng_meta *m) {
    /* No cache: ctime granularity is coarse (jiffies on 4.x kernels), so a
     * ctime-keyed cache served stale data after two updates in one tick. */
    return eng_meta_read(g, host, nofollow, st, m);
}

int eng_meta_write(const char *host, int nofollow, const eng_meta *m) {
    char buf[128];
    int n = snprintf(buf, sizeof buf, "1 %u %u %o %u %u,%u", m->uid, m->gid, m->mode, m->nlink, m->major, m->minor);
    int rc = nofollow ? lsetxattr(host, ENG_META_XATTR, buf, (size_t)n, 0)
                      : setxattr(host, ENG_META_XATTR, buf, (size_t)n, 0);
    return rc == 0 ? 0 : -errno;
}

int eng_meta_is_placeholder(const eng_meta *m, mode_t host_mode) {
    uint32_t vt = m->mode & S_IFMT;
    return S_ISREG(host_mode) && (vt == S_IFCHR || vt == S_IFBLK || vt == S_IFIFO || vt == S_IFSOCK);
}

int eng_meta_fifo_path(struct eng_guest *g, const struct stat *ph, char *out, size_t cap) {
    char dir[PATH_MAX];
    snprintf(dir, sizeof dir, "%s%s", g->root, ENG_STORE_GUEST);
    mkdir(dir, 0700);
    snprintf(dir, sizeof dir, "%s%s/fifo", g->root, ENG_STORE_GUEST);
    mkdir(dir, 0700);
    int n = snprintf(out, cap, "%s/%llx-%llx", dir, (unsigned long long)ph->st_dev, (unsigned long long)ph->st_ino);
    if (n <= 0 || (size_t)n >= cap) return -ENAMETOOLONG;
    if (mkfifo(out, 0600) != 0 && errno != EEXIST) return -errno;
    return 0;
}

void eng_meta_apply_stat(const eng_meta *m, struct stat *st) {
    st->st_uid = m->uid;
    st->st_gid = m->gid;
    uint32_t vt = m->mode & S_IFMT;
    if (eng_meta_is_placeholder(m, st->st_mode)) {
        st->st_mode = m->mode;
        st->st_rdev = (vt == S_IFCHR || vt == S_IFBLK) ? makedev(m->major, m->minor) : 0;
        st->st_size = 0;
    } else {
        st->st_mode = (st->st_mode & S_IFMT) | (m->mode & 07777);
    }
    if (m->nlink) st->st_nlink = m->nlink;
}

/* struct statx offsets are ABI-stable (uapi): uid@20 gid@24 mode(u16)@28 nlink@16
 * size@40 rdev_major@128 rdev_minor@132 */
void eng_meta_apply_statx(const eng_meta *m, void *stx) {
    unsigned char *b = stx;
    uint32_t v;
    uint16_t mode;
    memcpy(&mode, b + 28, 2);
    memcpy(b + 20, &m->uid, 4);
    memcpy(b + 24, &m->gid, 4);
    uint32_t vt = m->mode & S_IFMT;
    if (eng_meta_is_placeholder(m, mode)) {
        mode = (uint16_t)m->mode;
        uint32_t zero32 = 0;
        int dev = vt == S_IFCHR || vt == S_IFBLK;
        memcpy(b + 128, dev ? &m->major : &zero32, 4);
        memcpy(b + 132, dev ? &m->minor : &zero32, 4);
        uint64_t zero = 0;
        memcpy(b + 40, &zero, 8);
    } else {
        mode = (uint16_t)((mode & S_IFMT) | (m->mode & 07777));
    }
    memcpy(b + 28, &mode, 2);
    if (m->nlink) { v = m->nlink; memcpy(b + 16, &v, 4); }
}

/* ---- DAC ------------------------------------------------------------------------ */

int eng_in_group(const eng_task *t, uint32_t gid, int use_real) {
    if ((use_real ? t->cr.rgid : t->cr.fsgid) == gid) return 1;
    for (uint32_t i = 0; i < t->cr.ngroups; i++) if (t->cr.groups[i] == gid) return 1;
    return 0;
}

int eng_meta_permission(eng_task *t, const eng_meta *m, int mask, int use_real) {
    /* generic_permission: mode bits first, then CAP_DAC_READ_SEARCH/OVERRIDE.
     * access(2) (use_real) runs with the real ids and, for a non-root real
     * uid, no capabilities (do_faccessat's override creds). */
    uint32_t uid = use_real ? t->cr.ruid : t->cr.fsuid;
    uint64_t caps = use_real ? (t->cr.ruid == 0 ? t->cr.cap_prm : 0) : t->cr.cap_eff;
    unsigned bits;
    if (uid == m->uid) bits = (m->mode >> 6) & 7;
    else if (eng_in_group(t, m->gid, use_real)) bits = (m->mode >> 3) & 7;
    else bits = m->mode & 7;
    unsigned want = ((mask & R_OK) ? 4 : 0) | ((mask & W_OK) ? 2 : 0) | ((mask & X_OK) ? 1 : 0);
    if ((bits & want) == want) return 0;
    int dir = S_ISDIR(m->mode);
    if ((caps >> ENG_CAP_DAC_READ_SEARCH) & 1) {
        if (dir && !(mask & W_OK)) return 0;
        if (!dir && want == 4) return 0;
    }
    if ((caps >> ENG_CAP_DAC_OVERRIDE) & 1) {
        if (dir || !(mask & X_OK) || (m->mode & 0111)) return 0;
    }
    return -EACCES;
}

int eng_meta_may_exec(eng_task *t, const char *host, const struct stat *st) {
    eng_meta m;
    eng_meta_get(eng_tracer_guest(t->tr), host, 0, st, &m);
    return eng_meta_permission(t, &m, X_OK, 0);
}

/* ---- hardlink store ------------------------------------------------------------ */

int eng_link_is_stub_text(const char *text, char *id, size_t cap) {
    size_t pl = sizeof ENG_LINK_PREFIX - 1;
    if (strncmp(text, ENG_LINK_PREFIX, pl)) return 0;
    const char *p = text + pl;
    size_t n = strlen(p);
    if (n == 0 || n >= cap || strchr(p, '/')) return 0;
    memcpy(id, p, n + 1);
    return 1;
}

static int store_dir(eng_guest *g, char *out, size_t cap) {
    int n = snprintf(out, cap, "%s%s/links", g->root, ENG_STORE_GUEST);
    return (n > 0 && (size_t)n < cap) ? 0 : -ENAMETOOLONG;
}

int eng_link_object_path(struct eng_guest *g, const char *id, char *out, size_t cap) {
    int n = snprintf(out, cap, "%s%s/links/%s", g->root, ENG_STORE_GUEST, id);
    return (n > 0 && (size_t)n < cap) ? 0 : -ENAMETOOLONG;
}

/* One lock serialises every metadata read-modify-write (chmod/chown and the
 * hardlink store's NLINK updates) across the engine instances sharing the
 * rootfs: separate locks let a chmod write back a stale NLINK.  One fd per
 * process; callers never nest it. */
static int g_meta_lock_fd = -1;
int eng_meta_lock(struct eng_guest *g) {
    if (!g) return -1;
    if (g_meta_lock_fd < 0) {
        char top[PATH_MAX], p[PATH_MAX + 16];
        snprintf(top, sizeof top, "%s%s", g->root, ENG_STORE_GUEST);
        mkdir(top, 0700);
        snprintf(p, sizeof p, "%s/meta.lock", top);
        g_meta_lock_fd = open(p, O_RDWR | O_CREAT | O_CLOEXEC, 0600);
        if (g_meta_lock_fd < 0) return -1;
    }
    while (flock(g_meta_lock_fd, LOCK_EX) != 0 && errno == EINTR) {}
    return g_meta_lock_fd;
}
void eng_meta_unlock(int token) { if (token >= 0) flock(token, LOCK_UN); }

static int store_lock(eng_guest *g) {
    char d[PATH_MAX];
    if (store_dir(g, d, sizeof d)) return -1;
    int fd = eng_meta_lock(g);   /* creates the store's top directory */
    mkdir(d, 0700);
    return fd;
}
static void store_unlock(int fd) { eng_meta_unlock(fd); }

static int crash_point(const char *name) {
    const char *c = getenv("WORKFLOW_ENGINE_CRASH_AT");
    if (c && !strcmp(c, name)) {
        ENG_WARN("crash injection at %s", name);
        _exit(99);
    }
    return 0;
}

static int obj_adjust(const char *obj, int delta) {
    struct stat st;
    if (stat(obj, &st) != 0) return -errno;
    eng_meta m;
    eng_meta_read(NULL, obj, 0, &st, &m);
    if (!m.present) { m.uid = 0; m.gid = 0; m.mode = st.st_mode; }
    long n = (long)m.nlink + delta;
    if (n < 0) n = 0;
    m.nlink = (uint32_t)n;
    return eng_meta_write(obj, 0, &m);
}

static void new_id(char *id, size_t cap) {
    unsigned long long r = 0;
    int fd = open("/dev/urandom", O_RDONLY | O_CLOEXEC);
    if (fd >= 0) { if (read(fd, &r, sizeof r) != (ssize_t)sizeof r) r = 0; close(fd); }
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    r ^= (unsigned long long)ts.tv_nsec << 20 ^ (unsigned long long)ts.tv_sec ^ (unsigned long long)getpid() << 40;
    snprintf(id, cap, "%016llx", r);
}

int eng_link_create(struct eng_guest *g, const char *old_entry, const char *new_entry) {
    struct stat st;
    if (lstat(old_entry, &st) != 0) return -errno;
    struct stat nst;
    if (lstat(new_entry, &nst) == 0) return -EEXIST;
    int lk = store_lock(g);
    if (lk < 0) return -EIO;
    char id[64], obj[PATH_MAX], text[PATH_MAX], sd[PATH_MAX];
    int rc = 0;
    store_dir(g, sd, sizeof sd);
    if (S_ISLNK(st.st_mode)) {
        char t[PATH_MAX];
        ssize_t n = readlink(old_entry, t, sizeof t - 1);
        if (n < 0) { rc = -errno; goto out; }
        t[n] = 0;
        if (!eng_link_is_stub_text(t, id, sizeof id)) {
            /* hardlink to a plain symlink: same text, no shared identity */
            rc = symlink(t, new_entry) == 0 ? 0 : -errno;
            goto out;
        }
        eng_link_object_path(g, id, obj, sizeof obj);
    } else if (S_ISREG(st.st_mode) || S_ISFIFO(st.st_mode) || S_ISSOCK(st.st_mode)) {
        /* convert: journal, move content into the store, stub the old name */
        new_id(id, sizeof id);
        eng_link_object_path(g, id, obj, sizeof obj);
        char jpath[PATH_MAX + 64];
        snprintf(jpath, sizeof jpath, "%s/journal-%s", sd, id);
        int jf = open(jpath, O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC, 0600);
        if (jf < 0) { rc = -errno; goto out; }
        size_t l = strlen(old_entry);
        if (write(jf, old_entry, l) != (ssize_t)l || fsync(jf) != 0) { rc = -EIO; close(jf); unlink(jpath); goto out; }
        close(jf);
        crash_point("link-journaled");
        if (rename(old_entry, obj) != 0) { rc = -errno; unlink(jpath); goto out; }
        crash_point("link-moved");
        snprintf(text, sizeof text, "%s%s", ENG_LINK_PREFIX, id);
        if (symlink(text, old_entry) != 0) {
            rc = -errno;
            rename(obj, old_entry);   /* undo */
            unlink(jpath);
            goto out;
        }
        crash_point("link-stubbed");
        eng_meta m;
        eng_meta_read(g, obj, 0, &st, &m);
        if (!m.present) m.mode = st.st_mode;
        m.nlink = 1;
        eng_meta_write(obj, 0, &m);
        unlink(jpath);
    } else {
        rc = -EPERM;   /* directories (and anything else) cannot be hard linked */
        goto out;
    }
    /* add the new name: count first (a crash leaves nlink too high = leak, safe) */
    if ((rc = obj_adjust(obj, +1))) goto out;
    crash_point("link-counted");
    snprintf(text, sizeof text, "%s%s", ENG_LINK_PREFIX, id);
    if (symlink(text, new_entry) != 0) { rc = -errno; obj_adjust(obj, -1); }
out:
    store_unlock(lk);
    return rc;
}

void eng_link_drop(struct eng_guest *g, const char *id) {
    char obj[PATH_MAX];
    if (eng_link_object_path(g, id, obj, sizeof obj)) return;
    crash_point("link-drop");   /* the kernel already removed the name */
    int lk = store_lock(g);
    struct stat st;
    if (stat(obj, &st) == 0) {
        eng_meta m;
        eng_meta_read(g, obj, 0, &st, &m);
        if (m.nlink <= 1) unlink(obj);
        else obj_adjust(obj, -1);
    }
    store_unlock(lk);
}

int eng_link_recover(struct eng_guest *g) {
    char sd[PATH_MAX];
    if (store_dir(g, sd, sizeof sd)) return -ENAMETOOLONG;
    DIR *d = opendir(sd);
    if (!d) return 0;
    int lk = store_lock(g);
    int fixed = 0;
    struct dirent *e;
    while ((e = readdir(d))) {
        if (strncmp(e->d_name, "journal-", 8)) continue;
        const char *id = e->d_name + 8;
        char jpath[PATH_MAX + 300], entry[PATH_MAX], obj[PATH_MAX];
        snprintf(jpath, sizeof jpath, "%s/%s", sd, e->d_name);
        int jf = open(jpath, O_RDONLY | O_CLOEXEC);
        if (jf < 0) continue;
        ssize_t n = read(jf, entry, sizeof entry - 1);
        close(jf);
        if (n <= 0) { unlink(jpath); continue; }
        entry[n] = 0;
        eng_link_object_path(g, id, obj, sizeof obj);
        struct stat est, ost;
        int have_entry = lstat(entry, &est) == 0;
        int have_obj = stat(obj, &ost) == 0;
        if (!have_entry && have_obj) {
            rename(obj, entry);                    /* moved but not stubbed: undo */
        } else if (have_entry && S_ISLNK(est.st_mode) && have_obj) {
            eng_meta m;                            /* stubbed: make sure nlink >= 1 */
            eng_meta_read(g, obj, 0, &ost, &m);
            if (!m.present) m.mode = ost.st_mode;
            if (m.nlink == 0) { m.nlink = 1; eng_meta_write(obj, 0, &m); }
        }
        unlink(jpath);
        fixed++;
    }
    closedir(d);
    store_unlock(lk);
    if (fixed) ENG_INFO("hardlink store: recovered %d interrupted operation(s)", fixed);
    return fixed;
}

/* ---- instances and fsck ------------------------------------------------------------ */

int eng_instance_lock(struct eng_guest *g, int exclusive) {
    char top[PATH_MAX], p[PATH_MAX + 32];
    snprintf(top, sizeof top, "%s%s", g->root, ENG_STORE_GUEST);
    mkdir(top, 0700);
    snprintf(p, sizeof p, "%s/instances.lock", top);
    int fd = open(p, O_RDWR | O_CREAT | O_CLOEXEC, 0600);
    if (fd < 0) return -errno;
    int op = exclusive ? (LOCK_EX | LOCK_NB) : LOCK_SH;
    while (flock(fd, op) != 0) {
        if (errno == EINTR) continue;
        int e = errno;
        close(fd);
        return -e;
    }
    return fd;
}

typedef struct { char id[24]; unsigned names; unsigned nlink; int have_obj; } fsck_ent;
static struct { fsck_ent *v; size_t n, cap; } g_fsck;

static fsck_ent *fsck_find(const char *id) {
    for (size_t i = 0; i < g_fsck.n; i++) if (!strcmp(g_fsck.v[i].id, id)) return &g_fsck.v[i];
    if (g_fsck.n == g_fsck.cap) {
        size_t nc = g_fsck.cap ? g_fsck.cap * 2 : 64;
        fsck_ent *nv = realloc(g_fsck.v, nc * sizeof *nv);
        if (!nv) return NULL;
        g_fsck.v = nv;
        g_fsck.cap = nc;
    }
    fsck_ent *e = &g_fsck.v[g_fsck.n++];
    memset(e, 0, sizeof *e);
    snprintf(e->id, sizeof e->id, "%s", id);
    return e;
}

/* Count stub names below dirfd (no symlink following; the store is skipped). */
static void fsck_walk(int dfd, int depth) {
    if (depth > 256) return;
    DIR *d = fdopendir(dfd);
    if (!d) { close(dfd); return; }
    struct dirent *e;
    while ((e = readdir(d))) {
        const char *n = e->d_name;
        if (!strcmp(n, ".") || !strcmp(n, "..")) continue;
        if (depth == 0 && !strcmp(n, &ENG_STORE_GUEST[1])) continue;
        unsigned char type = e->d_type;
        if (type == DT_UNKNOWN) {
            struct stat st;
            if (fstatat(dirfd(d), n, &st, AT_SYMLINK_NOFOLLOW) != 0) continue;
            type = S_ISDIR(st.st_mode) ? DT_DIR : S_ISLNK(st.st_mode) ? DT_LNK : DT_REG;
        }
        if (type == DT_DIR) {
            int fd = openat(dirfd(d), n, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
            if (fd >= 0) fsck_walk(fd, depth + 1);
        } else if (type == DT_LNK) {
            char txt[PATH_MAX], id[64];
            ssize_t k = readlinkat(dirfd(d), n, txt, sizeof txt - 1);
            if (k <= 0) continue;
            txt[k] = 0;
            if (!eng_link_is_stub_text(txt, id, sizeof id)) continue;
            fsck_ent *fe = fsck_find(id);
            if (fe) fe->names++;
        }
    }
    closedir(d);
}

int eng_link_fsck(struct eng_guest *g, int repair, void *outp) {
    FILE *out = outp ? outp : stdout;
    eng_link_recover(g);
    char sd[PATH_MAX];
    if (store_dir(g, sd, sizeof sd)) return -ENAMETOOLONG;
    memset(&g_fsck, 0, sizeof g_fsck);
    int lk = store_lock(g);
    DIR *d = opendir(sd);
    if (d) {
        struct dirent *e;
        while ((e = readdir(d))) {
            if (e->d_name[0] == '.') continue;
            if (!strncmp(e->d_name, "journal-", 8)) continue;
            char obj[PATH_MAX];
            if (eng_link_object_path(g, e->d_name, obj, sizeof obj)) continue;
            struct stat st;
            if (stat(obj, &st) != 0) continue;
            eng_meta m;
            eng_meta_read(NULL, obj, 0, &st, &m);
            fsck_ent *fe = fsck_find(e->d_name);
            if (fe) { fe->have_obj = 1; fe->nlink = m.nlink; }
        }
        closedir(d);
    }
    int problems = 0;
    int rfd = open(g->root, O_RDONLY | O_DIRECTORY | O_CLOEXEC);
    if (rfd < 0) {
        store_unlock(lk);
        free(g_fsck.v);
        return -errno;
    }
    fsck_walk(rfd, 0);
    for (size_t i = 0; i < g_fsck.n; i++) {
        fsck_ent *e = &g_fsck.v[i];
        char obj[PATH_MAX];
        eng_link_object_path(g, e->id, obj, sizeof obj);
        if (!e->have_obj) {
            fprintf(out, "dangling: %u name(s) of missing object %s\n", e->names, e->id);
            problems++;
        } else if (e->names == 0) {
            fprintf(out, "orphan: object %s has no names (nlink %u)%s\n", e->id, e->nlink, repair ? ": removed" : "");
            if (repair && unlink(obj) == 0) continue;
            problems++;
        } else if (e->names != e->nlink) {
            fprintf(out, "count: object %s nlink %u, names %u%s\n", e->id, e->nlink, e->names, repair ? ": fixed" : "");
            if (repair) {
                struct stat st;
                eng_meta m;
                if (stat(obj, &st) == 0 && eng_meta_read(NULL, obj, 0, &st, &m) == 0) {
                    if (!m.present) m.mode = st.st_mode;
                    m.nlink = e->names;
                    if (eng_meta_write(obj, 0, &m) == 0) continue;
                }
            }
            problems++;
        }
    }
    store_unlock(lk);
    fprintf(out, "hardlink store: %zu object(s), %d problem(s)%s\n", g_fsck.n, problems, repair ? " left" : "");
    free(g_fsck.v);
    memset(&g_fsck, 0, sizeof g_fsck);
    return problems;
}
