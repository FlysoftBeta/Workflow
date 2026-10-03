/* install.c — image installer and generation lifecycle (docs/environment.md §2,
 * docs/engine.md "数据布局").
 *
 *   install  image.tar.zst (formatVersion 2) -> TARGET/rootfs (+ metadata store)
 *            and TARGET/seeds/<store>/ for rows below metadata.stores prefixes
 *   clone    SRC generation -> DST (rootfs, xattrs, hardlink store)
 *   verify   metadata presence + hardlink store consistency of a generation
 *   remove   delete a generation tree (never follows symlinks)
 *
 * The installer validates metadata.json and attributes.tsv before extracting
 * anything, streams the zstd/tar image once, works only relative to directory
 * fds it created (O_NOFOLLOW/O_EXCL, never following archive symlinks), never
 * creates device nodes/FIFOs or host set-id bits, and only reports success
 * after the whole stream's sha256 matched and the target was synced.  Any
 * failure removes TARGET. */
#define _GNU_SOURCE
#include "engine/install.h"

#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/stat.h>
#include <sys/statvfs.h>
#include <sys/time.h>
#include <sys/xattr.h>
#include <time.h>
#include <unistd.h>

#include "engine/guest.h"
#include "engine/json.h"
#include "engine/meta.h"
#include "engine/sha256.h"
#include "../third_party/zstd-1.5.7/zstd.h"

#if defined(__aarch64__)
#define HOST_DEB_ARCH "arm64"
#else
#define HOST_DEB_ARCH "amd64"
#endif

#define STORE_DIR  ".workflow-engine"
#define LINKS_DIR  ".workflow-engine/links"

static const char *const IMPLEMENTED[] = {"virtual-ownership", "virtual-mode", "hardlink-emulation",
                                          "virtual-special-files", "store-seeds"};

/* ---- errors --------------------------------------------------------------------- */

static int g_quiet;
static char g_err[512];
static int fail(int code, const char *fmt, ...) __attribute__((format(printf, 2, 3)));
static int fail(int code, const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(g_err, sizeof g_err, fmt, ap);
    va_end(ap);
    return code;
}
static void note(const char *fmt, ...) __attribute__((format(printf, 1, 2)));
static void note(const char *fmt, ...) {
    if (g_quiet) return;
    va_list ap;
    va_start(ap, fmt);
    fprintf(stderr, "install: ");
    vfprintf(stderr, fmt, ap);
    fputc('\n', stderr);
    va_end(ap);
}

/* ---- decompressing, hashing byte source ------------------------------------------ */

typedef struct {
    int fd;
    ZSTD_DCtx *dctx;
    eng_sha256 sha;
    uint64_t in_bytes;
    unsigned char in[1 << 17];
    ZSTD_inBuffer zin;
    int in_eof;
    unsigned char out[1 << 17];
    size_t out_pos, out_len;
    size_t last_ret;          /* ZSTD_decompressStream result: 0 = frame complete */
} src_t;

/* Fill out[] with more decompressed data.  Returns bytes available, 0 at end of
 * input, or -1 on a zstd/IO error. */
static ssize_t src_fill(src_t *s) {
    s->out_pos = s->out_len = 0;
    for (;;) {
        if (s->zin.pos == s->zin.size && !s->in_eof) {
            ssize_t n;
            do n = read(s->fd, s->in, sizeof s->in); while (n < 0 && errno == EINTR);
            if (n < 0) return -1;
            if (n == 0) s->in_eof = 1;
            else {
                eng_sha256_update(&s->sha, s->in, (size_t)n);
                s->in_bytes += (uint64_t)n;
                s->zin.src = s->in;
                s->zin.size = (size_t)n;
                s->zin.pos = 0;
            }
        }
        if (s->zin.pos == s->zin.size && s->in_eof) return 0;
        ZSTD_outBuffer zo = {s->out, sizeof s->out, 0};
        size_t r = ZSTD_decompressStream(s->dctx, &zo, &s->zin);
        if (ZSTD_isError(r)) { errno = EBADMSG; return -1; }
        s->last_ret = r;
        if (zo.pos) { s->out_len = zo.pos; return (ssize_t)zo.pos; }
    }
}

/* Read exactly n bytes (buf may be NULL to skip).  0 ok, -1 error/short. */
static int src_read(src_t *s, void *buf, size_t n) {
    unsigned char *b = buf;
    while (n) {
        if (s->out_pos == s->out_len) {
            ssize_t k = src_fill(s);
            if (k <= 0) { if (k == 0) errno = EPIPE; return -1; }
        }
        size_t take = s->out_len - s->out_pos < n ? s->out_len - s->out_pos : n;
        if (b) { memcpy(b, s->out + s->out_pos, take); b += take; }
        s->out_pos += take;
        n -= take;
    }
    return 0;
}

/* ---- tar ----------------------------------------------------------------------------- */

typedef struct {
    char path[4096];
    char link[4096];
    char type;
    uint64_t size;
    int64_t mtime;
} member_t;

static uint64_t octal(const char *p, size_t n, int *ok) {
    uint64_t v = 0;
    size_t i = 0;
    while (i < n && (p[i] == ' ' || p[i] == 0)) i++;
    for (; i < n && p[i] >= '0' && p[i] <= '7'; i++) v = v * 8 + (uint64_t)(p[i] - '0');
    for (; i < n; i++) if (p[i] != ' ' && p[i] != 0) *ok = 0;
    return v;
}

static int valid_utf8(const unsigned char *s, size_t n) {
    size_t i = 0;
    while (i < n) {
        unsigned char c = s[i];
        if (c < 0x80) { i++; continue; }
        size_t k;
        unsigned cp;
        if ((c & 0xe0) == 0xc0) { k = 1; cp = c & 0x1f; }
        else if ((c & 0xf0) == 0xe0) { k = 2; cp = c & 0x0f; }
        else if ((c & 0xf8) == 0xf0) { k = 3; cp = c & 0x07; }
        else return 0;
        if (i + k >= n) return 0;
        for (size_t j = 1; j <= k; j++) {
            if ((s[i + j] & 0xc0) != 0x80) return 0;
            cp = cp << 6 | (s[i + j] & 0x3f);
        }
        if ((k == 1 && cp < 0x80) || (k == 2 && cp < 0x800) || (k == 3 && (cp < 0x10000 || cp > 0x10ffff)) ||
            (cp >= 0xd800 && cp <= 0xdfff))
            return 0;
        i += k + 1;
    }
    return 1;
}

/* Parse PAX records into m (path, linkpath, size, mtime only). */
static int pax_apply(char *d, size_t n, member_t *m, int *has_size) {
    size_t i = 0;
    while (i < n) {
        char *end;
        unsigned long len = strtoul(d + i, &end, 10);
        if (end == d + i || *end != ' ' || len < 5 || i + len > n || d[i + len - 1] != '\n') return -1;
        char *kv = end + 1, *rec_end = d + i + len - 1;
        char *eq = memchr(kv, '=', (size_t)(rec_end - kv));
        if (!eq) return -1;
        *eq = 0;
        *rec_end = 0;
        const char *val = eq + 1;
        size_t vl = (size_t)(rec_end - val);
        if (!strcmp(kv, "path")) {
            if (vl >= sizeof m->path || !valid_utf8((const unsigned char *)val, vl)) return -1;
            memcpy(m->path, val, vl + 1);
        } else if (!strcmp(kv, "linkpath")) {
            if (vl >= sizeof m->link || !valid_utf8((const unsigned char *)val, vl)) return -1;
            memcpy(m->link, val, vl + 1);
        } else if (!strcmp(kv, "size")) {
            m->size = strtoull(val, NULL, 10);
            *has_size = 1;
        } else if (!strcmp(kv, "mtime")) {
            m->mtime = strtoll(val, NULL, 10);
        } else {
            return -1;   /* only path, linkpath, size, mtime are allowed (§2.2) */
        }
        i += len;
    }
    return 0;
}

/* Next member header.  1 = member, 0 = end of archive, <0 = error code. */
static int tar_next(src_t *s, member_t *m) {
    char pax_path[4096] = "", pax_link[4096] = "";
    uint64_t pax_size = 0;
    int64_t pax_mtime = -1;
    int have_pax = 0, pax_has_size = 0;
    for (;;) {
        unsigned char h[512];
        if (src_read(s, h, 512)) return -1;
        int zero = 1;
        for (int i = 0; i < 512; i++) if (h[i]) { zero = 0; break; }
        if (zero) {
            /* end of archive: a second zero block, then only zeros */
            if (src_read(s, h, 512)) return -1;
            for (int i = 0; i < 512; i++) if (h[i]) return -5;
            for (;;) {
                if (s->out_pos == s->out_len) {
                    ssize_t k = src_fill(s);
                    if (k < 0) return -1;
                    if (k == 0) break;
                }
                for (; s->out_pos < s->out_len; s->out_pos++) if (s->out[s->out_pos]) return -5;
            }
            return 0;
        }
        unsigned sum = 0;
        for (int i = 0; i < 512; i++) sum += (i >= 148 && i < 156) ? ' ' : h[i];
        int ok = 1;
        unsigned hsum = (unsigned)octal((char *)h + 148, 8, &ok);
        if (!ok || hsum != sum) return -2;
        if (memcmp(h + 257, "ustar\0" "00", 8)) return -2;
        memset(m, 0, sizeof *m);
        m->type = (char)h[156];
        m->size = octal((char *)h + 124, 12, &ok);
        m->mtime = (int64_t)octal((char *)h + 136, 12, &ok);
        if (!ok) return -2;
        char name[101], prefix[156];
        memcpy(name, h, 100); name[100] = 0;
        memcpy(prefix, h + 345, 155); prefix[155] = 0;
        if (prefix[0]) snprintf(m->path, sizeof m->path, "%s/%s", prefix, name);
        else snprintf(m->path, sizeof m->path, "%s", name);
        memcpy(m->link, h + 157, 100); m->link[100] = 0;
        if (m->type == 'x') {
            if (have_pax || m->size > (1 << 20)) return -2;
            size_t n = (size_t)m->size;
            char *d = malloc(n + 1);
            if (!d || src_read(s, d, n) || src_read(s, NULL, (512 - n % 512) % 512)) { free(d); return -1; }
            member_t px;
            memset(&px, 0, sizeof px);
            px.mtime = -1;
            int rc = pax_apply(d, n, &px, &pax_has_size);
            free(d);
            if (rc) return -3;
            snprintf(pax_path, sizeof pax_path, "%s", px.path);
            snprintf(pax_link, sizeof pax_link, "%s", px.link);
            pax_size = px.size;
            pax_mtime = px.mtime;
            have_pax = 1;
            continue;
        }
        if (m->type != '0' && m->type != 0 && m->type != '5' && m->type != '2') return -4;
        if (m->type == 0) m->type = '0';
        if (have_pax) {
            if (pax_path[0]) snprintf(m->path, sizeof m->path, "%s", pax_path);
            if (pax_link[0]) snprintf(m->link, sizeof m->link, "%s", pax_link);
            if (pax_has_size) m->size = pax_size;
            if (pax_mtime >= 0) m->mtime = pax_mtime;
        }
        size_t pl = strlen(m->path);
        if (m->type == '5' && pl > 1 && m->path[pl - 1] == '/') m->path[pl - 1] = 0;
        if (!valid_utf8((unsigned char *)m->path, strlen(m->path))) return -3;
        if (m->type != '0' && m->size) return -2;
        return 1;
    }
}

static int read_member_data(src_t *s, const member_t *m, char **out, size_t max) {
    if (m->size > max) return -1;
    size_t n = (size_t)m->size;
    char *d = malloc(n + 1);
    if (!d) return -1;
    if (src_read(s, d, n) || src_read(s, NULL, (512 - n % 512) % 512)) { free(d); return -1; }
    d[n] = 0;
    *out = d;
    return 0;
}

/* ---- attributes.tsv ------------------------------------------------------------------- */

typedef struct {
    char *path;          /* decoded guest path */
    size_t plen;
    char type;           /* d f l h c b p */
    uint32_t uid, gid, mode;
    char *extra;         /* h: decoded primary path; c/b: "maj,min" */
    uint32_t major, minor;
    int store;           /* index into stores, -1 if not below a store prefix */
    int primary;         /* h: row index of the primary */
} row_t;

typedef struct { char guest[1024]; char store[256]; size_t glen; } store_t;

typedef struct {
    row_t *rows;
    size_t n;
    int *hash;           /* open addressing over path -> row index */
    size_t hcap;
    store_t stores[8];
    int nstores;
    uint32_t user_uid, user_gid;
} attrs_t;

static uint64_t hstr(const char *s, size_t n) {
    uint64_t h = 1469598103934665603ull;
    for (size_t i = 0; i < n; i++) { h ^= (unsigned char)s[i]; h *= 1099511628211ull; }
    return h;
}

static int find_row(const attrs_t *a, const char *p, size_t n) {
    for (size_t i = hstr(p, n) & (a->hcap - 1);; i = (i + 1) & (a->hcap - 1)) {
        int r = a->hash[i];
        if (r < 0) return -1;
        if (a->rows[r].plen == n && !memcmp(a->rows[r].path, p, n)) return r;
    }
}

static void add_hash(attrs_t *a, int r) {
    for (size_t i = hstr(a->rows[r].path, a->rows[r].plen) & (a->hcap - 1);; i = (i + 1) & (a->hcap - 1))
        if (a->hash[i] < 0) { a->hash[i] = r; return; }
}

static int unpct(const char *in, size_t n, char **out, size_t *olen) {
    char *o = malloc(n + 1);
    if (!o) return -1;
    size_t k = 0;
    for (size_t i = 0; i < n; i++) {
        unsigned char c = (unsigned char)in[i];
        if (c == '%') {
            if (i + 2 >= n) { free(o); return -1; }
            int v = 0;
            for (int j = 1; j <= 2; j++) {
                char h = in[i + j];
                v <<= 4;
                if (h >= '0' && h <= '9') v |= h - '0';
                else if (h >= 'A' && h <= 'F') v |= h - 'A' + 10;
                else { free(o); return -1; }
            }
            o[k++] = (char)v;
            i += 2;
        } else if (c >= 0x21 && c <= 0x7e) {
            o[k++] = (char)c;
        } else {
            free(o);
            return -1;
        }
    }
    o[k] = 0;
    *out = o;
    *olen = k;
    return 0;
}

static int path_ok(const char *p, size_t n) {
    if (n == 1 && p[0] == '/') return 1;
    if (!n || p[0] != '/' || p[n - 1] == '/' || memchr(p, 0, n)) return 0;
    if (!valid_utf8((const unsigned char *)p, n)) return 0;
    for (size_t i = 1; i <= n;) {
        size_t j = i;
        while (j < n && p[j] != '/') j++;
        size_t cl = j - i;
        if (cl == 0 || (cl == 1 && p[i] == '.') || (cl == 2 && p[i] == '.' && p[i + 1] == '.')) return 0;
        i = j + 1;
    }
    return 1;
}

static int parse_u32(const char *s, uint32_t max, uint32_t *out) {
    if (!*s) return -1;
    uint64_t v = 0;
    for (; *s; s++) {
        if (*s < '0' || *s > '9') return -1;
        v = v * 10 + (uint64_t)(*s - '0');
        if (v > max) return -1;
    }
    *out = (uint32_t)v;
    return 0;
}

/* Parse and check invariants 1, 2, 4, 6 (5 is checked by the caller). */
static int parse_attrs(char *text, size_t len, attrs_t *a) {
    const char hdr[] = "#workflow-attributes 1\n";
    if (len < sizeof hdr - 1 || memcmp(text, hdr, sizeof hdr - 1)) return fail(65, "attributes.tsv: bad header");
    size_t cap = 0;
    for (size_t i = 0; i < len; i++) if (text[i] == '\n') cap++;
    a->rows = calloc(cap + 1, sizeof *a->rows);
    a->hcap = 1;
    while (a->hcap < cap * 2 + 2) a->hcap <<= 1;
    a->hash = malloc(a->hcap * sizeof *a->hash);
    if (!a->rows || !a->hash) return fail(74, "out of memory");
    for (size_t i = 0; i < a->hcap; i++) a->hash[i] = -1;
    char *p = text + sizeof hdr - 1, *end = text + len;
    if (len && text[len - 1] != '\n') return fail(65, "attributes.tsv: missing final newline");
    while (p < end) {
        char *nl = memchr(p, '\n', (size_t)(end - p));
        *nl = 0;
        char *f[6];
        int nf = 0;
        for (char *q = p; nf < 6;) {
            f[nf++] = q;
            char *tab = strchr(q, '\t');
            if (!tab) break;
            *tab = 0;
            q = tab + 1;
        }
        size_t idx = a->n;
        if (nf != 6 || strchr(f[5], '\t')) return fail(65, "attributes.tsv row %zu: need 6 fields", idx + 1);
        row_t *r = &a->rows[idx];
        if (unpct(f[0], strlen(f[0]), &r->path, &r->plen) || !path_ok(r->path, r->plen))
            return fail(65, "attributes.tsv row %zu: bad path", idx + 1);
        if (strlen(f[1]) != 1 || !strchr("dflhcbp", f[1][0])) return fail(65, "row %zu: bad type", idx + 1);
        r->type = f[1][0];
        if (parse_u32(f[2], 4294967294u, &r->uid) || parse_u32(f[3], 4294967294u, &r->gid))
            return fail(65, "row %zu: bad uid/gid", idx + 1);
        if (strlen(f[4]) != 4 || strspn(f[4], "01234567") != 4) return fail(65, "row %zu: bad mode", idx + 1);
        r->mode = (uint32_t)strtoul(f[4], NULL, 8);
        if (r->type == 'l' && r->mode != 0777) return fail(65, "row %zu: symlink mode must be 0777", idx + 1);
        r->store = -1;
        r->primary = -1;
        if (r->type == 'h') {
            size_t el;
            if (unpct(f[5], strlen(f[5]), &r->extra, &el) || !path_ok(r->extra, el))
                return fail(65, "row %zu: bad hardlink primary", idx + 1);
        } else if (r->type == 'c' || r->type == 'b') {
            if (sscanf(f[5], "%u,%u", &r->major, &r->minor) != 2) return fail(65, "row %zu: bad device", idx + 1);
        } else if (strcmp(f[5], "-")) {
            return fail(65, "row %zu: extra must be '-'", idx + 1);
        }
        /* invariant 1: strictly increasing, first row is / (d) */
        if (idx == 0) {
            if (strcmp(r->path, "/") || r->type != 'd') return fail(65, "attributes.tsv: first row must be / (d)");
        } else {
            const row_t *pr = &a->rows[idx - 1];
            size_t m = pr->plen < r->plen ? pr->plen : r->plen;
            int c = memcmp(pr->path, r->path, m);
            if (c > 0 || (c == 0 && pr->plen >= r->plen)) return fail(65, "row %zu: rows not strictly sorted", idx + 1);
            /* invariant 2: parent is an earlier d row */
            const char *sl = strrchr(r->path, '/');
            size_t plen = sl == r->path ? 1 : (size_t)(sl - r->path);
            int pi = find_row(a, r->path, plen);
            if (pi < 0 || a->rows[pi].type != 'd') return fail(65, "row %zu: parent is not a directory row", idx + 1);
        }
        add_hash(a, (int)idx);
        a->n++;
        p = nl + 1;
    }
    /* invariant 4: hardlinks */
    for (size_t i = 0; i < a->n; i++) {
        row_t *r = &a->rows[i];
        if (r->type != 'h') continue;
        int pi = find_row(a, r->extra, strlen(r->extra));
        if (pi < 0 || (size_t)pi >= i || a->rows[pi].type != 'f')
            return fail(65, "row %zu: hardlink primary must be an earlier f row", i + 1);
        const row_t *pr = &a->rows[pi];
        if (pr->uid != r->uid || pr->gid != r->gid || pr->mode != r->mode)
            return fail(65, "row %zu: hardlink attributes differ from the primary", i + 1);
        r->primary = pi;
    }
    /* invariant 6: stores */
    for (int s = 0; s < a->nstores; s++) {
        store_t *st = &a->stores[s];
        int pi = find_row(a, st->guest, st->glen);
        if (pi < 0 || a->rows[pi].type != 'd') return fail(65, "store prefix %s is not a directory row", st->guest);
        for (size_t i = 0; i < a->n; i++) {
            row_t *r = &a->rows[i];
            int below = r->plen > st->glen && !memcmp(r->path, st->guest, st->glen) && r->path[st->glen] == '/';
            int to_store = r->type == 'h' && strlen(r->extra) > st->glen && !memcmp(r->extra, st->guest, st->glen) &&
                           r->extra[st->glen] == '/';
            if (to_store) return fail(65, "row %zu: hardlink into store %s", i + 1, st->guest);
            if (!below) continue;
            if (!strchr("dfl", r->type) || r->uid != a->user_uid || r->gid != a->user_gid || (r->mode & 07000))
                return fail(65, "row %zu: store entries must be d/f/l owned by the user without set-id", i + 1);
            r->store = s;
        }
    }
    return 0;
}

/* ---- metadata.json ----------------------------------------------------------------------- */

typedef struct {
    long long rows, size, entries, members, regular_bytes;
    char attr_sha[65];
    unsigned requires;        /* REQ_* */
} meta_t;

enum { REQ_OWNERSHIP = 1, REQ_MODE = 2, REQ_HARDLINK = 4, REQ_SPECIAL = 8, REQ_SEEDS = 16 };

static int check_metadata(const char *text, size_t len, const char *profile, meta_t *mt, attrs_t *a) {
    eng_json *j = eng_json_parse(text, len);
    if (!j || j->t != ENG_J_OBJ) { eng_json_free(j); return fail(65, "metadata.json: not a JSON object"); }
    int rc = 0;
    const char *s;
    long long v;
    if (!(s = eng_json_str(j, "format")) || strcmp(s, "workflow-image")) { rc = fail(65, "metadata: format is not workflow-image"); goto out; }
    if (eng_json_int(j, "formatVersion", &v) || v != 2) { rc = fail(65, "metadata: formatVersion must be 2"); goto out; }
    if (!(s = eng_json_str(j, "type")) || strcmp(s, "debian-trixie")) { rc = fail(65, "metadata: unknown image type"); goto out; }
    if (eng_json_int(j, "typeVersion", &v) || v != 1) { rc = fail(65, "metadata: unsupported typeVersion"); goto out; }
    s = eng_json_str(j, "profile");
    if (!s || (strcmp(s, "workspace") && strcmp(s, "base"))) { rc = fail(65, "metadata: bad profile"); goto out; }
    if (strcmp(profile, "any") && strcmp(profile, s)) { rc = fail(65, "metadata: profile %s, expected %s", s, profile); goto out; }
    if (!(s = eng_json_str(j, "architecture")) || strcmp(s, HOST_DEB_ARCH)) {
        rc = fail(65, "metadata: architecture %s does not match this device (%s)", s ? s : "?", HOST_DEB_ARCH);
        goto out;
    }
    const eng_json *req = eng_json_get(j, "requires");
    if (!req || req->t != ENG_J_ARR) { rc = fail(65, "metadata: requires missing"); goto out; }
    for (const eng_json *e = req->child; e; e = e->next) {
        int known = 0;
        for (size_t i = 0; e->t == ENG_J_STR && i < sizeof IMPLEMENTED / sizeof IMPLEMENTED[0]; i++)
            if (!strcmp(e->s, IMPLEMENTED[i])) { known = 1; mt->requires |= 1u << i; }
        if (!known) { rc = fail(65, "metadata: required capability %s is not implemented", e->t == ENG_J_STR ? e->s : "?"); goto out; }
    }
    if ((mt->requires & (REQ_OWNERSHIP | REQ_MODE)) != (REQ_OWNERSHIP | REQ_MODE)) {
        rc = fail(65, "metadata: requires must list virtual-ownership and virtual-mode");
        goto out;
    }
    const eng_json *at = eng_json_get(j, "attributes");
    if (!at || !(s = eng_json_str(at, "path")) || strcmp(s, "attributes.tsv") || eng_json_int(at, "version", &v) || v != 1 ||
        eng_json_int(at, "rows", &mt->rows) || eng_json_int(at, "size", &mt->size) || !(s = eng_json_str(at, "sha256")) ||
        strlen(s) != 64) {
        rc = fail(65, "metadata: bad attributes descriptor");
        goto out;
    }
    snprintf(mt->attr_sha, sizeof mt->attr_sha, "%s", s);
    const eng_json *rf = eng_json_get(j, "rootfs");
    if (!rf || eng_json_int(rf, "entries", &mt->entries) || eng_json_int(rf, "members", &mt->members) ||
        eng_json_int(rf, "regularBytes", &mt->regular_bytes)) {
        rc = fail(65, "metadata: bad rootfs descriptor");
        goto out;
    }
    const eng_json *user = eng_json_get(j, "user");
    long long uu = 1000, ug = 1000;
    if (user) { eng_json_int(user, "uid", &uu); eng_json_int(user, "gid", &ug); }
    a->user_uid = (uint32_t)uu;
    a->user_gid = (uint32_t)ug;
    const eng_json *st = eng_json_get(j, "stores");
    if (st) {
        if (st->t != ENG_J_OBJ) { rc = fail(65, "metadata: stores must be an object"); goto out; }
        for (const eng_json *e = st->child; e; e = e->next) {
            const char *name = eng_json_str(e, "store"), *seed = eng_json_str(e, "seed");
            if (a->nstores >= 8 || !name || !seed || (strcmp(seed, "if-absent") && strcmp(seed, "merge")) ||
                !path_ok(e->key, strlen(e->key)) || !strcmp(e->key, "/") || name[0] == '/' || strstr(name, "..") ||
                strlen(e->key) >= sizeof a->stores[0].guest || strlen(name) >= sizeof a->stores[0].store) {
                rc = fail(65, "metadata: bad store %s", e->key ? e->key : "?");
                goto out;
            }
            store_t *sd = &a->stores[a->nstores++];
            snprintf(sd->guest, sizeof sd->guest, "%s", e->key);
            snprintf(sd->store, sizeof sd->store, "%s", name);
            sd->glen = strlen(sd->guest);
        }
    }
out:
    eng_json_free(j);
    return rc;
}

/* ---- extraction helpers ---------------------------------------------------------------- */

typedef struct {
    int base_root;       /* TARGET/rootfs */
    int base_seeds;      /* TARGET/seeds */
    char cache_dir[4096];
    int cache_fd;
    int cache_base;
} ex_t;

/* Open (and cache) the directory `dir` (relative, no leading '/') under `base`
 * with O_NOFOLLOW at every component. */
static int open_dir(ex_t *x, int base, const char *dir) {
    if (x->cache_fd >= 0 && x->cache_base == base && !strcmp(x->cache_dir, dir)) return x->cache_fd;
    if (x->cache_fd >= 0) close(x->cache_fd);
    x->cache_fd = -1;
    int fd = dup(base);
    if (fd < 0) return -1;
    char buf[4096];
    snprintf(buf, sizeof buf, "%s", dir);
    for (char *save = NULL, *c = strtok_r(buf, "/", &save); c; c = strtok_r(NULL, "/", &save)) {
        int n = openat(fd, c, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
        close(fd);
        if (n < 0) return -1;
        fd = n;
    }
    snprintf(x->cache_dir, sizeof x->cache_dir, "%s", dir);
    x->cache_fd = fd;
    x->cache_base = base;
    return fd;
}

static int fset_meta(int fd, uint32_t uid, uint32_t gid, uint32_t mode, uint32_t nlink, uint32_t maj, uint32_t min) {
    char v[128];
    int n = snprintf(v, sizeof v, "1 %u %u %o %u %u,%u", uid, gid, mode, nlink, maj, min);
    return fsetxattr(fd, ENG_META_XATTR, v, (size_t)n, 0);
}

/* Base fd and parent directory (relative) of a row: the rootfs, or its seed
 * directory seeds/<store>/... for rows below a store prefix. */
static void locate(const ex_t *x, const attrs_t *a, const row_t *r, int *base, char *dir, size_t dcap) {
    char tmp[4200];
    if (r->store >= 0) {
        const store_t *st = &a->stores[r->store];
        snprintf(tmp, sizeof tmp, "%s%s", st->store, r->path + st->glen);
        *base = x->base_seeds;
    } else {
        snprintf(tmp, sizeof tmp, "%s", r->path + 1);
        *base = x->base_root;
    }
    char *sl = strrchr(tmp, '/');
    if (sl) *sl = 0; else tmp[0] = 0;
    snprintf(dir, dcap, "%s", tmp);
}

static int mkdirs_rel(int base, const char *rel) {
    char buf[4096];
    snprintf(buf, sizeof buf, "%s", rel);
    int fd = dup(base);
    for (char *save = NULL, *c = strtok_r(buf, "/", &save); c && fd >= 0; c = strtok_r(NULL, "/", &save)) {
        if (mkdirat(fd, c, 0700) != 0 && errno != EEXIST) { close(fd); return -1; }
        int n = openat(fd, c, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
        close(fd);
        fd = n;
    }
    if (fd < 0) return -1;
    close(fd);
    return 0;
}

typedef struct { int row; int64_t mtime; } dtime_t;   /* directory mtimes, applied last */
/* ---- remove ------------------------------------------------------------------------------- */

static int rm_tree_at(int dfd, const char *name, int depth) {
    if (depth > 512) return -ELOOP;
    if (unlinkat(dfd, name, 0) == 0 || errno == ENOENT) return 0;
    if (errno != EISDIR && errno != EPERM) return -errno;
    int fd = openat(dfd, name, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if (fd < 0) return -errno;
    fchmod(fd, 0700);
    DIR *d = fdopendir(fd);
    if (!d) { close(fd); return -errno; }
    struct dirent *e;
    int rc = 0;
    while ((e = readdir(d))) {
        if (!strcmp(e->d_name, ".") || !strcmp(e->d_name, "..")) continue;
        int r = rm_tree_at(dirfd(d), e->d_name, depth + 1);
        if (r && !rc) rc = r;
    }
    closedir(d);
    if (unlinkat(dfd, name, AT_REMOVEDIR) != 0 && !rc) rc = -errno;
    return rc;
}

int eng_remove_tree(const char *path) {
    char buf[4096];
    snprintf(buf, sizeof buf, "%s", path);
    size_t n = strlen(buf);
    while (n > 1 && buf[n - 1] == '/') buf[--n] = 0;
    char *sl = strrchr(buf, '/');
    int dfd;
    const char *name;
    if (!sl) { dfd = AT_FDCWD; name = buf; }
    else {
        *sl = 0;
        dfd = open(sl == buf ? "/" : buf, O_RDONLY | O_DIRECTORY | O_CLOEXEC);
        if (dfd < 0) return -errno;
        name = sl + 1;
    }
    int rc = rm_tree_at(dfd, name, 0);
    if (dfd != AT_FDCWD) close(dfd);
    return rc;
}

/* ---- install --------------------------------------------------------------------------------- */

static double now_s(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + ts.tv_nsec / 1e9;
}

static int do_install(const eng_install_opts *o, src_t *s, int tfd, meta_t *mt, attrs_t *a, long long *stats) {
    member_t m;
    char *meta_text = NULL, *attr_text = NULL;
    int rc;
    /* metadata.json, attributes.tsv */
    if ((rc = tar_next(s, &m)) != 1 || m.type != '0' || strcmp(m.path, "metadata.json"))
        return fail(65, "image: first member must be metadata.json");
    if (read_member_data(s, &m, &meta_text, 64 << 10)) return fail(65, "image: metadata.json too large or truncated");
    rc = check_metadata(meta_text, (size_t)m.size, o->profile ? o->profile : "workspace", mt, a);
    free(meta_text);
    if (rc) return rc;
    if (tar_next(s, &m) != 1 || m.type != '0' || strcmp(m.path, "attributes.tsv"))
        return fail(65, "image: second member must be attributes.tsv");
    if ((long long)m.size != mt->size) return fail(65, "attributes.tsv: size differs from metadata");
    if (read_member_data(s, &m, &attr_text, 64 << 20)) return fail(65, "image: attributes.tsv too large or truncated");
    {
        eng_sha256 c;
        uint8_t d[32];
        char hex[65];
        eng_sha256_init(&c);
        eng_sha256_update(&c, attr_text, (size_t)m.size);
        eng_sha256_final(&c, d);
        eng_sha256_hex(d, hex);
        if (strcmp(hex, mt->attr_sha)) { free(attr_text); return fail(65, "attributes.tsv: sha256 mismatch"); }
    }
    rc = parse_attrs(attr_text, (size_t)m.size, a);
    free(attr_text);
    if (rc) return rc;
    if ((long long)a->n != mt->rows || (long long)a->n != mt->entries) return fail(65, "attributes.tsv: row count differs from metadata");
    {
        long long mem = 0, hard = 0, special = 0;
        for (size_t i = 0; i < a->n; i++) {
            char t = a->rows[i].type;
            if (t == 'd' || t == 'f' || t == 'l') mem++;
            else if (t == 'h') hard++;
            else special++;
        }
        if (mem != mt->members) return fail(65, "attributes.tsv: %lld member rows, metadata says %lld", mem, mt->members);
        if (hard && !(mt->requires & REQ_HARDLINK)) return fail(65, "metadata: hardlink rows without hardlink-emulation");
        if (special && !(mt->requires & REQ_SPECIAL)) return fail(65, "metadata: special rows without virtual-special-files");
        if (a->nstores && !(mt->requires & REQ_SEEDS)) return fail(65, "metadata: stores without store-seeds");
    }

    /* space: regular bytes plus a block per entry, 5% margin */
    struct statvfs vfs;
    if (fstatvfs(tfd, &vfs) == 0) {
        unsigned long long need = (unsigned long long)mt->regular_bytes + (unsigned long long)mt->entries * 4096ull;
        need += need / 20;
        unsigned long long have = (unsigned long long)vfs.f_bavail * vfs.f_frsize;
        if (have < need) return fail(75, "not enough space: need %llu MB, have %llu MB", need >> 20, have >> 20);
    }
    note("image valid: %zu rows, %lld members, %lld MB", a->n, mt->members, mt->regular_bytes >> 20);

    ex_t x = {.cache_fd = -1};
    if (mkdirat(tfd, "rootfs", 0700) || (x.base_root = openat(tfd, "rootfs", O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC)) < 0)
        return fail(74, "create rootfs: %s", strerror(errno));
    x.base_seeds = -1;
    if (a->nstores) {
        if (mkdirat(tfd, "seeds", 0700) || (x.base_seeds = openat(tfd, "seeds", O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC)) < 0)
            return fail(74, "create seeds: %s", strerror(errno));
        for (int i = 0; i < a->nstores; i++)   /* seeds/<store>/ (e.g. seeds/home/work) */
            if (mkdirs_rel(x.base_seeds, a->stores[i].store)) return fail(74, "create seeds: %s", strerror(errno));
    }
    dtime_t *dts = calloc(a->n, sizeof *dts);
    size_t ndt = 0;
    if (!dts) return fail(74, "out of memory");
    size_t ri = 0;
    long long members = 0, seeds = 0, regular = 0;
    char *buf = malloc(1 << 20);
    if (!buf) { free(dts); return fail(74, "out of memory"); }
    double t0 = now_s(), last = t0;
    rc = 0;
    for (;;) {
        int k = tar_next(s, &m);
        if (k == 0) break;
        if (k < 0) {
            rc = fail(65, "image: %s after %lld members",
                      k == -4 ? "forbidden tar member type" : k == -3 ? "bad PAX header or non-UTF-8 name" :
                      k == -5 ? "data after the end of the archive" : k == -1 ? "truncated or corrupt stream" : "bad tar header",
                      members);
            break;
        }
        /* invariant 3: next d/f/l row, same path and type */
        while (ri < a->n && !strchr("dfl", a->rows[ri].type)) ri++;
        if (ri >= a->n) { rc = fail(65, "image: member %s has no attributes row", m.path); break; }
        row_t *r = &a->rows[ri++];
        char gp[4200];
        if (!strcmp(m.path, "rootfs")) snprintf(gp, sizeof gp, "/");
        else if (!strncmp(m.path, "rootfs/", 7)) snprintf(gp, sizeof gp, "/%s", m.path + 7);
        else { rc = fail(65, "image: member outside rootfs: %s", m.path); break; }
        char mt_ = m.type == '5' ? 'd' : m.type == '2' ? 'l' : 'f';
        if (strcmp(gp, r->path) || mt_ != r->type) { rc = fail(65, "image: member %s does not match row %s (%c)", gp, r->path, r->type); break; }
        members++;
        int is_seed = r->store >= 0;
        if (!strcmp(r->path, "/")) {
            fchmod(x.base_root, (r->mode & 0777) | 0700);
            if (fset_meta(x.base_root, r->uid, r->gid, S_IFDIR | r->mode, 0, 0, 0)) { rc = fail(74, "xattr on /: %s", strerror(errno)); break; }
            dts[ndt].row = (int)(r - a->rows); dts[ndt].mtime = m.mtime; ndt++;
            continue;
        }
        int base;
        char dir[4096];
        locate(&x, a, r, &base, dir, sizeof dir);
        const char *nm = strrchr(r->path, '/') + 1;
        int pfd = open_dir(&x, base, dir);
        if (pfd < 0) { rc = fail(74, "open parent of %s: %s", r->path, strerror(errno)); break; }
        if (r->type == 'd') {
            if (mkdirat(pfd, nm, 0700)) { rc = fail(74, "mkdir %s: %s", r->path, strerror(errno)); break; }
            int dfd = openat(pfd, nm, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
            if (dfd < 0) { rc = fail(74, "open %s: %s", r->path, strerror(errno)); break; }
            fchmod(dfd, (r->mode & 0777) | 0700);
            int e = is_seed ? 0 : fset_meta(dfd, r->uid, r->gid, S_IFDIR | r->mode, 0, 0, 0);
            close(dfd);
            if (e) { rc = fail(74, "xattr %s: %s", r->path, strerror(errno)); break; }
            dts[ndt].row = (int)(r - a->rows);
            dts[ndt].mtime = m.mtime;
            ndt++;
        } else if (r->type == 'l') {
            if (symlinkat(m.link, pfd, nm)) { rc = fail(74, "symlink %s: %s", r->path, strerror(errno)); break; }
            struct timespec ts[2] = {{m.mtime, 0}, {m.mtime, 0}};
            utimensat(pfd, nm, ts, AT_SYMLINK_NOFOLLOW);
        } else {
            int fd = openat(pfd, nm, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0600);
            if (fd < 0) { rc = fail(74, "create %s: %s", r->path, strerror(errno)); break; }
            regular += (long long)m.size;
            uint64_t left = m.size;
            while (left && !rc) {
                size_t chunk = left < (1 << 20) ? (size_t)left : (1 << 20);
                if (src_read(s, buf, chunk)) { rc = fail(65, "image: truncated data of %s", r->path); break; }
                for (size_t off = 0; off < chunk;) {
                    ssize_t w = write(fd, buf + off, chunk - off);
                    if (w < 0) { if (errno == EINTR) continue; rc = fail(74, "write %s: %s", r->path, strerror(errno)); break; }
                    off += (size_t)w;
                }
                left -= chunk;
            }
            if (!rc && src_read(s, NULL, (512 - m.size % 512) % 512)) rc = fail(65, "image: truncated");
            if (!rc) {
                fchmod(fd, is_seed ? (r->mode & 0777) : ((r->mode & 0777) | 0600));
                if (!is_seed && fset_meta(fd, r->uid, r->gid, S_IFREG | r->mode, 0, 0, 0)) rc = fail(74, "xattr %s: %s", r->path, strerror(errno));
                struct timespec ts[2] = {{m.mtime, 0}, {m.mtime, 0}};
                futimens(fd, ts);
            }
            close(fd);
            if (rc) break;
        }
        if (is_seed) seeds++;
        if (!g_quiet && now_s() - last > 2.0) {
            last = now_s();
            note("%lld/%lld members", members, mt->members);
        }
    }
    free(buf);
    if (!rc && members != mt->members) rc = fail(65, "image: %lld members, metadata says %lld", members, mt->members);
    if (!rc && regular != mt->regular_bytes) rc = fail(65, "image: %lld regular bytes, metadata says %lld", regular, mt->regular_bytes);
    if (!rc) { while (ri < a->n && !strchr("dfl", a->rows[ri].type)) ri++; if (ri < a->n) rc = fail(65, "image: member for %s missing", a->rows[ri].path); }

    /* hardlinks and special files (rows without members) */
    long long hard = 0, special = 0, objects = 0;
    int *obj_of = rc ? NULL : calloc(a->n, sizeof *obj_of);
    if (!rc && !obj_of) rc = fail(74, "out of memory");
    if (!rc) {
        int have_links = 0;
        for (size_t i = 0; i < a->n; i++) if (a->rows[i].type == 'h') have_links = 1;
        int lfd = -1;
        if (have_links) {
            mkdirat(x.base_root, STORE_DIR, 0700);
            mkdirat(x.base_root, LINKS_DIR, 0700);
            lfd = openat(x.base_root, LINKS_DIR, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
            if (lfd < 0) rc = fail(74, "create hardlink store: %s", strerror(errno));
        }
        for (size_t i = 0; i < a->n && !rc; i++) {
            row_t *r = &a->rows[i];
            if (!strchr("hcbp", r->type)) continue;
            char dir[4096];
            int base;
            locate(&x, a, r, &base, dir, sizeof dir);
            const char *nm = strrchr(r->path, '/') + 1;
            int pfd = open_dir(&x, base, dir);
            if (pfd < 0) { rc = fail(74, "open parent of %s: %s", r->path, strerror(errno)); break; }
            if (r->type == 'h') {
                row_t *pr = &a->rows[r->primary];
                char id[32], stub[64];
                if (!obj_of[r->primary]) {
                    /* first link of this group: the primary's content becomes the object */
                    obj_of[r->primary] = (int)++objects;
                    snprintf(id, sizeof id, "%016llx", 0x1000000000000000ull + (unsigned long long)objects);
                    char pdir[4096];
                    int pbase;
                    locate(&x, a, pr, &pbase, pdir, sizeof pdir);
                    const char *pnm = strrchr(pr->path, '/') + 1;
                    int ppfd = open_dir(&x, pbase, pdir);
                    if (ppfd < 0 || renameat(ppfd, pnm, lfd, id)) { rc = fail(74, "hardlink %s: %s", pr->path, strerror(errno)); break; }
                    snprintf(stub, sizeof stub, "/%s/%s", LINKS_DIR, id);
                    if (symlinkat(stub, ppfd, pnm)) { rc = fail(74, "hardlink %s: %s", pr->path, strerror(errno)); break; }
                    pfd = open_dir(&x, base, dir);
                    if (pfd < 0) { rc = fail(74, "open parent of %s: %s", r->path, strerror(errno)); break; }
                }
                snprintf(id, sizeof id, "%016llx", 0x1000000000000000ull + (unsigned long long)obj_of[r->primary]);
                snprintf(stub, sizeof stub, "/%s/%s", LINKS_DIR, id);
                if (symlinkat(stub, pfd, nm)) { rc = fail(74, "hardlink %s: %s", r->path, strerror(errno)); break; }
                obj_of[i] = obj_of[r->primary];
                hard++;
            } else {
                int fd = openat(pfd, nm, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0600);
                if (fd < 0) { rc = fail(74, "create %s: %s", r->path, strerror(errno)); break; }
                uint32_t type = r->type == 'c' ? S_IFCHR : r->type == 'b' ? S_IFBLK : S_IFIFO;
                if (fset_meta(fd, r->uid, r->gid, type | r->mode, 0, r->major, r->minor)) rc = fail(74, "xattr %s: %s", r->path, strerror(errno));
                close(fd);
                special++;
            }
        }
        /* NLINK of each object: its primary plus its h rows */
        for (long long ob = 1; ob <= objects && !rc; ob++) {
            unsigned names = 0;
            size_t prim = 0;
            for (size_t i = 0; i < a->n; i++)
                if (obj_of[i] == ob) { names++; if (a->rows[i].type == 'f') prim = i; }
            char id[32];
            snprintf(id, sizeof id, "%016llx", 0x1000000000000000ull + (unsigned long long)ob);
            int fd = openat(lfd, id, O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
            const row_t *pr = &a->rows[prim];
            if (fd < 0 || fset_meta(fd, pr->uid, pr->gid, S_IFREG | pr->mode, names, 0, 0)) rc = fail(74, "hardlink count: %s", strerror(errno));
            if (fd >= 0) close(fd);
        }
        if (lfd >= 0) close(lfd);
    }
    free(obj_of);
    /* directory mtimes last (children changed them), deepest first */
    for (size_t i = ndt; i-- > 0 && !rc;) {
        const row_t *r = &a->rows[dts[i].row];
        struct timespec ts[2] = {{dts[i].mtime, 0}, {dts[i].mtime, 0}};
        if (!strcmp(r->path, "/")) { futimens(x.base_root, ts); continue; }
        int base;
        char dir[4096];
        locate(&x, a, r, &base, dir, sizeof dir);
        int pfd = open_dir(&x, base, dir);
        if (pfd >= 0) utimensat(pfd, strrchr(r->path, '/') + 1, ts, AT_SYMLINK_NOFOLLOW);
    }
    free(dts);
    if (x.cache_fd >= 0) close(x.cache_fd);
    if (!rc) {
        /* finish the stream: trailing padding, then the zstd frame must end */
        unsigned char tmp[4096];
        while (!rc) {
            ssize_t k = s->out_pos < s->out_len ? (ssize_t)(s->out_len - s->out_pos) : src_fill(s);
            if (k < 0) { rc = fail(65, "image: zstd stream error"); break; }
            if (k == 0) break;
            s->out_pos = s->out_len;
            (void)tmp;
        }
        if (!rc && s->last_ret != 0) rc = fail(65, "image: zstd frame incomplete");
    }
    if (!rc) syncfs(x.base_root);
    if (x.base_root >= 0) close(x.base_root);
    if (x.base_seeds >= 0) close(x.base_seeds);
    stats[0] = members; stats[1] = hard; stats[2] = special; stats[3] = seeds; stats[4] = objects;
    (void)t0;
    return rc;
}

int eng_install(const eng_install_opts *o) {
    g_quiet = o->quiet;
    g_err[0] = 0;
    char want[65] = "";
    long long want_size = -1;
    if (o->sha256) snprintf(want, sizeof want, "%s", o->sha256);
    if (o->index) {
        FILE *f = fopen(o->index, "re");
        if (!f) { fprintf(stderr, "install: cannot read %s: %s\n", o->index, strerror(errno)); return 66; }
        char buf[1 << 16];
        size_t n = fread(buf, 1, sizeof buf, f);
        fclose(f);
        eng_json *j = eng_json_parse(buf, n);
        const char *s = j ? eng_json_str(j, "sha256") : NULL;
        if (!s || strlen(s) != 64) { eng_json_free(j); fprintf(stderr, "install: bad index %s\n", o->index); return 65; }
        if (want[0] && strcmp(want, s)) { eng_json_free(j); fprintf(stderr, "install: --sha256 differs from the index\n"); return 65; }
        snprintf(want, sizeof want, "%s", s);
        eng_json_int(j, "size", &want_size);
        eng_json_free(j);
    }
    if (!want[0]) { fprintf(stderr, "install: the expected sha256 is required (--sha256 or --index)\n"); return 2; }

    int in = !strcmp(o->image, "-") ? dup(0) : open(o->image, O_RDONLY | O_CLOEXEC);
    if (in < 0) { fprintf(stderr, "install: cannot open %s: %s\n", o->image, strerror(errno)); return 66; }
    if (mkdir(o->target, 0700) != 0) {
        fprintf(stderr, "install: cannot create %s: %s\n", o->target, strerror(errno));
        close(in);
        return 73;
    }
    int tfd = open(o->target, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    src_t *s = calloc(1, sizeof *s);
    attrs_t a;
    memset(&a, 0, sizeof a);
    meta_t mt;
    memset(&mt, 0, sizeof mt);
    long long stats[5] = {0};
    int rc;
    double t0 = now_s();
    if (tfd < 0 || !s) rc = fail(74, "open target: %s", strerror(errno));
    else {
        s->fd = in;
        s->dctx = ZSTD_createDCtx();
        eng_sha256_init(&s->sha);
        rc = s->dctx ? do_install(o, s, tfd, &mt, &a, stats) : fail(74, "zstd context");
    }
    if (!rc) {
        /* the whole stream must match image.json before anything is committed */
        unsigned char tmp[1 << 16];
        ssize_t k;
        while ((k = read(in, tmp, sizeof tmp)) > 0) { eng_sha256_update(&s->sha, tmp, (size_t)k); s->in_bytes += (uint64_t)k; }
        uint8_t d[32];
        char hex[65];
        eng_sha256_final(&s->sha, d);
        eng_sha256_hex(d, hex);
        if (strcmp(hex, want)) rc = fail(65, "image sha256 %s does not match the expected %s", hex, want);
        else if (want_size >= 0 && (long long)s->in_bytes != want_size) rc = fail(65, "image size %llu, index says %lld", (unsigned long long)s->in_bytes, want_size);
    }
    if (s && s->dctx) ZSTD_freeDCtx(s->dctx);
    for (size_t i = 0; i < a.n; i++) { free(a.rows[i].path); free(a.rows[i].extra); }
    free(a.rows);
    free(a.hash);
    close(in);
    if (tfd >= 0) { if (!rc) fsync(tfd); close(tfd); }
    if (rc) {
        fprintf(stderr, "install: %s\n", g_err[0] ? g_err : "failed");
        if (!o->keep_partial) eng_remove_tree(o->target);
        free(s);
        return rc;
    }
    printf("{\"rows\":%zu,\"members\":%lld,\"hardlinks\":%lld,\"objects\":%lld,\"special\":%lld,\"seedEntries\":%lld,"
           "\"bytes\":%llu,\"sha256\":\"%s\",\"seconds\":%.1f}\n",
           (size_t)mt.rows, stats[0], stats[1], stats[4], stats[2], stats[3],
           (unsigned long long)s->in_bytes, want, now_s() - t0);
    free(s);
    return 0;
}

/* ---- clone ------------------------------------------------------------------------------------ */

#ifndef FICLONE
#define FICLONE _IOW(0x94, 9, int)
#endif

static int copy_file(int sfd, int dfd) {
    if (ioctl(dfd, FICLONE, sfd) == 0) return 0;   /* reflink where the fs can */
    char buf[1 << 16];
    for (;;) {
        ssize_t n = read(sfd, buf, sizeof buf);
        if (n < 0) { if (errno == EINTR) continue; return -errno; }
        if (n == 0) return 0;
        for (ssize_t off = 0; off < n;) {
            ssize_t w = write(dfd, buf + off, (size_t)(n - off));
            if (w < 0) { if (errno == EINTR) continue; return -errno; }
            off += w;
        }
    }
}

static int copy_xattrs(int sfd, int dfd) {
    char names[4096];
    ssize_t n = flistxattr(sfd, names, sizeof names);
    for (ssize_t i = 0; n > 0 && i < n; i += (ssize_t)strlen(names + i) + 1) {
        if (strncmp(names + i, "user.", 5)) continue;
        char v[4096];
        ssize_t vl = fgetxattr(sfd, names + i, v, sizeof v);
        if (vl >= 0 && fsetxattr(dfd, names + i, v, (size_t)vl, 0) != 0) return -errno;
    }
    return 0;
}

static int clone_dir(int sdir, int ddir, int depth, long long *count) {
    if (depth > 512) return -ELOOP;
    int sfd2 = dup(sdir);
    DIR *d = fdopendir(sfd2);
    if (!d) { close(sfd2); return -errno; }
    struct dirent *e;
    int rc = 0;
    while (!rc && (e = readdir(d))) {
        const char *n = e->d_name;
        if (!strcmp(n, ".") || !strcmp(n, "..")) continue;
        if (depth == 0 && !strcmp(n, STORE_DIR)) {
            /* engine store: links/ only (locks, fifo objects, journals are per-instance) */
            int ss = openat(sdir, n, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
            if (ss < 0) continue;
            mkdirat(ddir, n, 0700);
            int dd = openat(ddir, n, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
            int sl = openat(ss, "links", O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
            if (sl >= 0 && dd >= 0 && mkdirat(dd, "links", 0700) == 0) {
                int dl = openat(dd, "links", O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
                if (dl >= 0) {
                    DIR *ld = fdopendir(sl);
                    sl = -1;
                    struct dirent *le;
                    while (!rc && ld && (le = readdir(ld))) {
                        if (le->d_name[0] == '.' || !strncmp(le->d_name, "journal-", 8)) continue;
                        int a = openat(dirfd(ld), le->d_name, O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
                        int b = a < 0 ? -1 : openat(dl, le->d_name, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0600);
                        struct stat st;
                        if (a < 0 || b < 0 || fstat(a, &st)) rc = -errno;
                        else if (!(rc = copy_file(a, b)) && !(rc = copy_xattrs(a, b))) {
                            fchmod(b, st.st_mode & 07777);
                            struct timespec ts[2] = {st.st_atim, st.st_mtim};
                            futimens(b, ts);
                            (*count)++;
                        }
                        if (a >= 0) close(a);
                        if (b >= 0) close(b);
                    }
                    if (ld) closedir(ld);
                    close(dl);
                }
            }
            if (sl >= 0) close(sl);
            if (dd >= 0) close(dd);
            close(ss);
            continue;
        }
        struct stat st;
        if (fstatat(dirfd(d), n, &st, AT_SYMLINK_NOFOLLOW)) { rc = -errno; break; }
        struct timespec ts[2] = {st.st_atim, st.st_mtim};
        if (S_ISDIR(st.st_mode)) {
            if (mkdirat(ddir, n, 0700)) { rc = -errno; break; }
            int a = openat(dirfd(d), n, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
            int b = openat(ddir, n, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
            if (a < 0 || b < 0) rc = -errno;
            else if (!(rc = copy_xattrs(a, b)) && !(rc = clone_dir(a, b, depth + 1, count))) {
                fchmod(b, st.st_mode & 07777);
                futimens(b, ts);
            }
            if (a >= 0) close(a);
            if (b >= 0) close(b);
        } else if (S_ISLNK(st.st_mode)) {
            char t[4096];
            ssize_t k = readlinkat(dirfd(d), n, t, sizeof t - 1);
            if (k < 0) { rc = -errno; break; }
            t[k] = 0;
            if (symlinkat(t, ddir, n)) { rc = -errno; break; }
            utimensat(ddir, n, ts, AT_SYMLINK_NOFOLLOW);
        } else if (S_ISREG(st.st_mode)) {
            int a = openat(dirfd(d), n, O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
            int b = a < 0 ? -1 : openat(ddir, n, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0600);
            if (a < 0 || b < 0) rc = -errno;
            else if (!(rc = copy_file(a, b)) && !(rc = copy_xattrs(a, b))) {
                fchmod(b, st.st_mode & 07777);
                futimens(b, ts);
            }
            if (a >= 0) close(a);
            if (b >= 0) close(b);
        } else {
            continue;   /* sockets/FIFOs made at runtime are not part of a generation */
        }
        (*count)++;
    }
    closedir(d);
    return rc;
}

int eng_clone(const char *src, const char *dst, int quiet) {
    char sr[4096];
    snprintf(sr, sizeof sr, "%s/rootfs", src);
    int s = open(sr, O_RDONLY | O_DIRECTORY | O_CLOEXEC);
    if (s < 0) { fprintf(stderr, "clone: %s: %s\n", sr, strerror(errno)); return 66; }
    if (mkdir(dst, 0700)) { fprintf(stderr, "clone: cannot create %s: %s\n", dst, strerror(errno)); close(s); return 73; }
    int t = open(dst, O_RDONLY | O_DIRECTORY | O_CLOEXEC);
    int d = -1;
    if (t >= 0 && mkdirat(t, "rootfs", 0700) == 0) d = openat(t, "rootfs", O_RDONLY | O_DIRECTORY | O_CLOEXEC);
    double t0 = now_s();
    long long count = 0;
    int rc = d < 0 ? -errno : 0;
    if (!rc) {
        struct stat st;
        fstat(s, &st);
        if (!(rc = copy_xattrs(s, d)) && !(rc = clone_dir(s, d, 0, &count))) {
            fchmod(d, st.st_mode & 07777);
            struct timespec ts[2] = {st.st_atim, st.st_mtim};
            futimens(d, ts);
            syncfs(d);
        }
    }
    close(s);
    if (d >= 0) close(d);
    if (t >= 0) { fsync(t); close(t); }
    if (rc) {
        fprintf(stderr, "clone: %s\n", strerror(-rc));
        eng_remove_tree(dst);
        return 74;
    }
    if (!quiet) fprintf(stderr, "clone: %lld entries\n", count);
    printf("{\"entries\":%lld,\"seconds\":%.1f}\n", count, now_s() - t0);
    return 0;
}

/* ---- verify ------------------------------------------------------------------------------------ */

static void count_meta(int dfd, int depth, long long *entries, long long *missing, FILE *out) {
    DIR *d = fdopendir(dfd);
    if (!d) { close(dfd); return; }
    struct dirent *e;
    while ((e = readdir(d))) {
        const char *n = e->d_name;
        if (!strcmp(n, ".") || !strcmp(n, "..") || (depth == 0 && !strcmp(n, STORE_DIR))) continue;
        struct stat st;
        if (fstatat(dirfd(d), n, &st, AT_SYMLINK_NOFOLLOW)) continue;
        (*entries)++;
        if (S_ISLNK(st.st_mode)) continue;
        int fd = openat(dirfd(d), n, O_RDONLY | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK | (S_ISDIR(st.st_mode) ? O_DIRECTORY : 0));
        if (fd < 0) continue;
        char v[128];
        if (fgetxattr(fd, ENG_META_XATTR, v, sizeof v) <= 0) {
            (*missing)++;
            if (*missing <= 10) fprintf(out, "no metadata: %s (depth %d)\n", n, depth);
        }
        if (S_ISDIR(st.st_mode)) count_meta(fd, depth + 1, entries, missing, out);
        else close(fd);
    }
    closedir(d);
}

int eng_verify(const char *generation, int quiet) {
    char r[4096];
    snprintf(r, sizeof r, "%s/rootfs", generation);
    eng_guest *g = eng_guest_open(r);
    if (!g) { fprintf(stderr, "verify: %s: %s\n", r, strerror(errno)); return 66; }
    int fd = open(r, O_RDONLY | O_DIRECTORY | O_CLOEXEC);
    long long entries = 0, missing = 0;
    if (fd >= 0) count_meta(fd, 0, &entries, &missing, stderr);
    FILE *devnull = quiet ? fopen("/dev/null", "we") : NULL;
    int problems = eng_link_fsck(g, 0, devnull ? devnull : stderr);
    if (devnull) fclose(devnull);
    eng_guest_close(g);
    printf("{\"entries\":%lld,\"withoutMetadata\":%lld,\"hardlinkProblems\":%d}\n", entries, missing, problems);
    return (problems == 0 && missing == 0) ? 0 : 1;
}
