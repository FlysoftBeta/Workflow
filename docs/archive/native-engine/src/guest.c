#define _GNU_SOURCE
#include "engine/guest.h"

#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "engine/log.h"
#include "engine/meta.h"

#include <stdint.h>
#include <sys/sysmacros.h>
#include <sys/xattr.h>

eng_guest *eng_guest_open(const char *root) {
    eng_guest *g = calloc(1, sizeof(*g));
    if (!g) return NULL;
    if (!realpath(root, g->root)) { free(g); return NULL; }
    g->rootlen = strlen(g->root);
    if (g->rootlen == 1) g->rootlen = 0;   /* "/" as root: prefix is empty */
    g->rootfd = open(g->root, O_PATH | O_DIRECTORY | O_CLOEXEC);
    if (g->rootfd < 0) { free(g); return NULL; }
    eng_guest_add_hide(g, "/.workflow-engine");
    return g;
}

void eng_guest_close(eng_guest *g) {
    if (!g) return;
    if (g->rootfd >= 0) close(g->rootfd);
    free(g);
}

static void canon_guest(char *p) {
    /* strip trailing slashes (except root), collapse "//" */
    char *w = p, *r = p;
    while (*r) {
        *w++ = *r;
        if (*r == '/') while (r[1] == '/') r++;
        r++;
    }
    *w = 0;
    size_t n = strlen(p);
    while (n > 1 && p[n - 1] == '/') p[--n] = 0;
}

int eng_guest_add_bind(eng_guest *g, const char *host, const char *guest) {
    return eng_guest_add_bind_owned(g, host, guest, 1000, 1000);
}

int eng_guest_add_bind_owned(eng_guest *g, const char *host, const char *guest, unsigned uid, unsigned gid) {
    if (g->nbinds >= ENG_MAX_BINDS) return -ENOSPC;
    if (guest[0] != '/') return -EINVAL;
    eng_bind b;
    memset(&b, 0, sizeof b);
    char real[PATH_MAX];
    if (!realpath(host, real)) return -errno;
    snprintf(b.host, sizeof b.host, "%s", real);
    snprintf(b.guest, sizeof b.guest, "%s", guest);
    canon_guest(b.guest);
    if (!strcmp(b.guest, "/")) return -EINVAL;   /* the rootfs is "/" */
    b.glen = strlen(b.guest);
    b.hlen = strlen(b.host);
    b.uid = uid;
    b.gid = gid;
    if (b.hlen == 1) b.hlen = 0;
    /* replace an existing bind for the same guest path */
    for (int i = 0; i < g->nbinds; i++)
        if (!strcmp(g->binds[i].guest, b.guest)) { g->binds[i] = b; return 0; }
    int pos = g->nbinds;
    while (pos > 0 && g->binds[pos - 1].glen < b.glen) { g->binds[pos] = g->binds[pos - 1]; pos--; }
    g->binds[pos] = b;
    g->nbinds++;
    return 0;
}

int eng_guest_add_hide(eng_guest *g, const char *guest) {
    if (g->nhides >= ENG_MAX_HIDES) return -ENOSPC;
    snprintf(g->hides[g->nhides], PATH_MAX, "%s", guest);
    canon_guest(g->hides[g->nhides]);
    g->nhides++;
    return 0;
}

static int prefix_match(const char *path, const char *pre, size_t plen) {
    return strncmp(path, pre, plen) == 0 && (path[plen] == 0 || path[plen] == '/');
}

int eng_guest_locate(const eng_guest *g, const char *host) {
    int best = -1;
    for (int i = 0; i < g->nbinds; i++) {
        const eng_bind *b = &g->binds[i];
        if ((b->hlen == 0 || prefix_match(host, b->host, b->hlen)) && (best < 0 || b->hlen > g->binds[best].hlen))
            best = i;
    }
    size_t rl = g->rootlen;
    int in_root = rl == 0 || prefix_match(host, g->root, rl);
    if (best >= 0 && (!in_root || g->binds[best].hlen >= rl)) return best;
    return in_root ? ENG_LOC_ROOTFS : ENG_LOC_NONE;
}

void eng_guest_default_owner(const eng_guest *g, const char *host, unsigned *uid, unsigned *gid) {
    int loc = eng_guest_locate(g, host);
    if (loc >= 0) { *uid = g->binds[loc].uid; *gid = g->binds[loc].gid; return; }
    *uid = 0;
    *gid = 0;
}

int eng_guest_is_hidden(const eng_guest *g, const char *guest) {
    for (int i = 0; i < g->nhides; i++)
        if (prefix_match(guest, g->hides[i], strlen(g->hides[i]))) return 1;
    return 0;
}

int eng_guest_to_host(const eng_guest *g, const char *guest, char *host, size_t cap) {
    for (int i = 0; i < g->nbinds; i++) {
        const eng_bind *b = &g->binds[i];
        if (prefix_match(guest, b->guest, b->glen)) {
            int n = snprintf(host, cap, "%.*s%s", (int)b->hlen, b->host, guest + b->glen);
            if (n <= 0 || (size_t)n >= cap) return -ENAMETOOLONG;
            if (host[0] == 0) { host[0] = '/'; host[1] = 0; }
            return 0;
        }
    }
    int n = snprintf(host, cap, "%.*s%s", (int)g->rootlen, g->root, guest);
    if (n <= 0 || (size_t)n >= cap) return -ENAMETOOLONG;
    return 0;
}

int eng_host_to_guest(const eng_guest *g, const char *host, char *guest, size_t cap) {
    /* longest host-prefix match among binds, then the rootfs */
    const eng_bind *best = NULL;
    for (int i = 0; i < g->nbinds; i++) {
        const eng_bind *b = &g->binds[i];
        if (b->hlen == 0 || prefix_match(host, b->host, b->hlen))
            if (!best || b->hlen > best->hlen) best = b;
    }
    size_t rl = g->rootlen;
    int in_root = rl == 0 || prefix_match(host, g->root, rl);
    if (best && (!in_root || best->hlen >= rl)) {
        int n = snprintf(guest, cap, "%s%s", best->guest, host + best->hlen);
        return (n > 0 && (size_t)n < cap) ? 0 : -1;
    }
    if (in_root) {
        const char *rest = host + rl;
        int n = snprintf(guest, cap, "%s", *rest ? rest : "/");
        return (n > 0 && (size_t)n < cap) ? 0 : -1;
    }
    return -1;
}

/* Runtime mount points the engine adds to the rootfs get metadata like any
 * other object (root-owned), so `verify` stays clean. */
static void ensure_file(const eng_guest *g, const char *rel, const char *hostdev) {
    char p[PATH_MAX];
    snprintf(p, sizeof p, "%s/%s", g->root, rel);
    struct stat st, hs;
    if (lstat(p, &st) == 0) return;
    int f = open(p, O_CREAT | O_EXCL | O_WRONLY | O_CLOEXEC, 0600);
    if (f < 0) return;
    close(f);
    eng_meta m = {.uid = 0, .gid = 0, .mode = S_IFREG | 0666, .present = 1};
    if (stat(hostdev, &hs) == 0 && S_ISCHR(hs.st_mode)) {
        m.mode = S_IFCHR | 0666;
        m.major = major(hs.st_rdev);
        m.minor = minor(hs.st_rdev);
    }
    eng_meta_write(p, 1, &m);
}
static void ensure_dir(const eng_guest *g, const char *rel, uint32_t mode) {
    char p[PATH_MAX];
    snprintf(p, sizeof p, "%s/%s", g->root, rel);
    if (mkdir(p, 0700) != 0) return;
    eng_meta m = {.uid = 0, .gid = 0, .mode = S_IFDIR | mode, .present = 1};
    eng_meta_write(p, 1, &m);
}
static void ensure_link(const eng_guest *g, const char *rel, const char *target) {
    char p[PATH_MAX];
    snprintf(p, sizeof p, "%s/%s", g->root, rel);
    struct stat st;
    if (lstat(p, &st) == 0) return;
    if (symlink(target, p) != 0) ENG_DBG("placeholder symlink %s: %s", p, strerror(errno));
}

int eng_guest_default_binds(eng_guest *g) {
    static const char *devs[] = {"null", "zero", "full", "random", "urandom", "tty", "ptmx"};
    ensure_dir(g, "proc", 0555);
    ensure_dir(g, "sys", 0555);
    ensure_dir(g, "dev", 0755);
    ensure_dir(g, "dev/pts", 0755);
    ensure_dir(g, "dev/shm", 01777);
    ensure_link(g, "dev/fd", "/proc/self/fd");
    ensure_link(g, "dev/stdin", "/proc/self/fd/0");
    ensure_link(g, "dev/stdout", "/proc/self/fd/1");
    ensure_link(g, "dev/stderr", "/proc/self/fd/2");
    int rc = 0;
    if (access("/proc", F_OK) == 0) rc |= eng_guest_add_bind_owned(g, "/proc", "/proc", 0, 0);
    if (access("/sys", F_OK) == 0) eng_guest_add_bind_owned(g, "/sys", "/sys", 0, 0);
    eng_guest_add_hide(g, "/sys/fs/selinux");   /* libselinux in the guest: SELinux disabled */
    for (size_t i = 0; i < sizeof devs / sizeof devs[0]; i++) {
        char host[64], guest[64];
        snprintf(host, sizeof host, "/dev/%s", devs[i]);
        snprintf(guest, sizeof guest, "/dev/%s", devs[i]);
        if (access(host, F_OK) != 0) continue;
        char rel[64];
        snprintf(rel, sizeof rel, "dev/%s", devs[i]);
        ensure_file(g, rel, host);
        eng_guest_add_bind_owned(g, host, guest, 0, 0);
    }
    if (access("/dev/pts", F_OK) == 0) eng_guest_add_bind_owned(g, "/dev/pts", "/dev/pts", 0, 0);
#if defined(__ANDROID__)
    /* Android system directories, so a bionic program (e.g. an agent binary
     * from nativeLibraryDir, bound at its own path) runs inside the guest: its
     * PT_INTERP /system/bin/linker64, libraries and linker config resolve.
     * Debian uses none of these paths. */
    /* /dev/__properties__ (system properties) and /dev/socket (netd DNS proxy,
     * property service) are what bionic's libc talks to */
    static const char *sysdirs[] = {"/system", "/apex", "/vendor", "/product", "/system_ext", "/odm",
                                    "/dev/__properties__", "/dev/socket"};
    /* apps may read /linkerconfig/ld.config.txt but not stat /linkerconfig
     * itself (SELinux), so the file is bound, not the directory */
    {
        struct stat st;
        const char *lc = "/linkerconfig/ld.config.txt";
        if (stat(lc, &st) == 0 && S_ISREG(st.st_mode)) {
            eng_guest_make_mountpoint(g, lc, 0);
            eng_guest_add_bind_owned(g, lc, lc, 0, 0);
        }
    }
    for (size_t i = 0; i < sizeof sysdirs / sizeof sysdirs[0]; i++) {
        struct stat st;
        if (stat(sysdirs[i], &st) != 0 || !S_ISDIR(st.st_mode)) {
            ENG_DBG("android bind %s: %s", sysdirs[i], strerror(errno));
            continue;
        }
        eng_guest_make_mountpoint(g, sysdirs[i], 1);
        int brc = eng_guest_add_bind_owned(g, sysdirs[i], sysdirs[i], 0, 0);
        if (brc) ENG_WARN("android bind %s: %s", sysdirs[i], strerror(-brc));
    }
#endif
    return rc;
}

/* ---- binfmt rules ------------------------------------------------------------ */

static int unescape(const char *in, unsigned char *out, unsigned cap, unsigned *len) {
    unsigned n = 0;
    for (const char *p = in; *p; p++) {
        unsigned c = (unsigned char)*p;
        if (c == '\\' && p[1] == 'x' && p[2] && p[3]) {
            char hx[3] = {p[2], p[3], 0};
            char *end;
            c = (unsigned)strtoul(hx, &end, 16);
            if (*end) return -EINVAL;
            p += 3;
        } else if (c == '\\' && p[1] == '\\') {
            p++;
        }
        if (n >= cap) return -EINVAL;
        out[n++] = (unsigned char)c;
    }
    *len = n;
    return 0;
}

int eng_guest_add_binfmt(eng_guest *g, const char *rule) {
    if (g->nbinfmt >= ENG_MAX_BINFMT || !rule[0]) return -EINVAL;
    char buf[PATH_MAX * 2];
    snprintf(buf, sizeof buf, "%s", rule);
    size_t bl = strlen(buf);
    while (bl && (buf[bl - 1] == '\n' || buf[bl - 1] == '\r' || buf[bl - 1] == ' ')) buf[--bl] = 0;
    char del = buf[0];
    char *f[7] = {0};
    char *p = buf + 1;
    for (int i = 0; i < 7; i++) {
        f[i] = p;
        char *q = strchr(p, del);
        if (!q) { if (i < 5) return -EINVAL; for (int k = i + 1; k < 7; k++) f[k] = p + strlen(p); break; }
        *q = 0;
        p = q + 1;
    }
    eng_binfmt b;
    memset(&b, 0, sizeof b);
    snprintf(b.name, sizeof b.name, "%s", f[0]);
    if (!b.name[0] || strlen(f[1]) != 1 || (f[1][0] != 'M' && f[1][0] != 'E')) return -EINVAL;
    b.type = f[1][0];
    if (f[5][0] != '/') return -EINVAL;
    snprintf(b.interp, sizeof b.interp, "%s", f[5]);
    for (const char *fl = f[6]; *fl; fl++) {
        if (*fl == 'P') b.preserve_argv0 = 1;
        else if (*fl != 'O' && *fl != 'C' && *fl != 'F') return -EINVAL;   /* O/C/F accepted, see engine.md */
    }
    if (b.type == 'E') {
        if (!f[3][0] || strchr(f[3], '/')) return -EINVAL;
        snprintf(b.ext, sizeof b.ext, "%s", f[3]);
    } else {
        b.offset = f[2][0] ? (unsigned)strtoul(f[2], NULL, 10) : 0;
        if (unescape(f[3], b.magic, sizeof b.magic, &b.len) || !b.len) return -EINVAL;
        unsigned ml = 0;
        if (f[4][0]) {
            if (unescape(f[4], b.mask, sizeof b.mask, &ml) || ml != b.len) return -EINVAL;
        } else {
            memset(b.mask, 0xff, b.len);
        }
        if (b.offset + b.len > 256) return -EINVAL;   /* BINPRM_BUF_SIZE */
        for (unsigned i = 0; i < b.len; i++) b.magic[i] &= b.mask[i];
    }
    for (int i = 0; i < g->nbinfmt; i++)
        if (!strcmp(g->binfmt[i].name, b.name)) { g->binfmt[i] = b; return 0; }
    g->binfmt[g->nbinfmt++] = b;
    return 0;
}

int eng_guest_load_binfmt_file(eng_guest *g, const char *host_path) {
    FILE *f = fopen(host_path, "re");
    if (!f) return -errno;
    char line[PATH_MAX * 2];
    int n = 0;
    while (fgets(line, sizeof line, f)) {
        if (line[0] == '#' || line[0] == ';' || line[0] == '\n' || !line[0]) continue;
        if (eng_guest_add_binfmt(g, line) == 0) n++;
        else ENG_WARN("binfmt: ignoring invalid rule in %s: %.80s", host_path, line);
    }
    fclose(f);
    return n;
}

static int cmp_str(const void *a, const void *b) { return strcmp(*(char *const *)a, *(char *const *)b); }

int eng_guest_load_binfmt_dirs(eng_guest *g) {
    static const char *dirs[] = {"/usr/lib/binfmt.d", "/etc/binfmt.d"};
    char *names[128];
    char *paths[128];
    int n = 0;
    for (size_t d = 0; d < sizeof dirs / sizeof dirs[0]; d++) {
        char host[PATH_MAX];
        if (eng_guest_to_host(g, dirs[d], host, sizeof host)) continue;
        DIR *dp = opendir(host);
        if (!dp) continue;
        struct dirent *e;
        while ((e = readdir(dp)) && n < 128) {
            size_t l = strlen(e->d_name);
            if (l < 6 || strcmp(e->d_name + l - 5, ".conf")) continue;
            int dup = -1;
            for (int i = 0; i < n; i++) if (!strcmp(names[i], e->d_name)) dup = i;
            char p[PATH_MAX + 256];
            snprintf(p, sizeof p, "%s/%s", host, e->d_name);
            if (dup >= 0) { free(paths[dup]); paths[dup] = strdup(p); continue; }   /* /etc wins */
            names[n] = strdup(e->d_name);
            paths[n] = strdup(p);
            n++;
        }
        closedir(dp);
    }
    /* sort by file name, keep path pairing */
    for (int i = 1; i < n; i++)
        for (int j = i; j > 0 && strcmp(names[j - 1], names[j]) > 0; j--) {
            char *t = names[j]; names[j] = names[j - 1]; names[j - 1] = t;
            t = paths[j]; paths[j] = paths[j - 1]; paths[j - 1] = t;
        }
    (void)cmp_str;
    int rules = 0;
    for (int i = 0; i < n; i++) {
        int r = eng_guest_load_binfmt_file(g, paths[i]);
        if (r > 0) rules += r;
        free(names[i]);
        free(paths[i]);
    }
    return rules;
}

void eng_guest_make_mountpoint(eng_guest *g, const char *guest, int is_dir) {
    char buf[PATH_MAX];
    snprintf(buf, sizeof buf, "%s", guest);
    int fd = dup(g->rootfd);
    int rfd = open(g->root, O_RDONLY | O_DIRECTORY | O_CLOEXEC);
    if (fd >= 0) close(fd);
    fd = rfd;
    char *save = NULL;
    char *c = strtok_r(buf, "/", &save);
    while (c && fd >= 0) {
        char *next = strtok_r(NULL, "/", &save);
        int last = next == NULL;
        struct stat st;
        if (fstatat(fd, c, &st, AT_SYMLINK_NOFOLLOW) != 0) {
            eng_meta m = {.uid = 0, .gid = 0, .present = 1};
            if (!last || is_dir) {
                if (mkdirat(fd, c, 0700) != 0) break;
                m.mode = S_IFDIR | 0755;
                int d = openat(fd, c, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
                if (d >= 0) {
                    char v[64];
                    int n = snprintf(v, sizeof v, "1 0 0 %o 0 0,0", m.mode);
                    fsetxattr(d, ENG_META_XATTR, v, (size_t)n, 0);
                    close(d);
                }
            } else {
                int f = openat(fd, c, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0600);
                if (f >= 0) {
                    char v[64];
                    int n = snprintf(v, sizeof v, "1 0 0 %o 0 0,0", (unsigned)(S_IFREG | 0644));
                    fsetxattr(f, ENG_META_XATTR, v, (size_t)n, 0);
                    close(f);
                }
                break;
            }
        } else if (!S_ISDIR(st.st_mode)) {
            break;   /* a symlink or file on the way: leave the rootfs as it is */
        }
        if (last) break;
        int nfd = openat(fd, c, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
        close(fd);
        fd = nfd;
        c = next;
    }
    if (fd >= 0) close(fd);
}
