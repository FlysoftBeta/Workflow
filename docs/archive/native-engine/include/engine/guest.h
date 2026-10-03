/* guest.h — guest environment context: rootfs, bind table, hidden paths.
 *
 * Binds are user-space guest→host prefix mappings (not mounts).  The rootfs
 * is the implicit bind for "/".  Resolution always works on canonical GUEST
 * paths and maps the result to a host path by longest-prefix match, so ".."
 * from a bind root goes to the guest parent, never the host parent. */
#ifndef WORKFLOW_ENGINE_GUEST_H
#define WORKFLOW_ENGINE_GUEST_H

#include <limits.h>
#include <stddef.h>

#define ENG_MAX_BINDS 32
#define ENG_MAX_HIDES 16
#define ENG_MAX_BINFMT 32
#define ENG_BINFMT_MAGIC 128

/* A binfmt_misc-style rule (the engine's own table: the host's binfmt_misc is
 * never used).  Register-string syntax :name:type:offset:magic:mask:interp:flags */
typedef struct eng_binfmt {
    char     name[64];
    char     type;                       /* 'M' magic, 'E' extension */
    unsigned offset;
    unsigned char magic[ENG_BINFMT_MAGIC], mask[ENG_BINFMT_MAGIC];
    unsigned len;                        /* magic length, or extension length */
    char     ext[64];
    char     interp[PATH_MAX];           /* guest path */
    int      preserve_argv0;             /* flag P */
} eng_binfmt;

typedef struct eng_bind {
    char   guest[PATH_MAX];
    char   host[PATH_MAX];
    size_t glen, hlen;
    unsigned uid, gid;           /* owner presented for objects without metadata */
} eng_bind;

typedef struct eng_guest {
    char   root[PATH_MAX];       /* canonical host path of the rootfs */
    size_t rootlen;
    int    rootfd;
    eng_bind binds[ENG_MAX_BINDS];   /* sorted by guest length, longest first */
    int    nbinds;
    char   hides[ENG_MAX_HIDES][PATH_MAX];  /* guest paths that do not exist */
    int    nhides;
    eng_binfmt binfmt[ENG_MAX_BINFMT];
    int    nbinfmt;
    char   loader[PATH_MAX];     /* host path of libworkflow-loader.so */
    char   sockdir[PATH_MAX];    /* short host dir for AF_UNIX path aliases (sun_path is 108 bytes) */
    int    no_filemap;           /* force anonymous segment copies */
    unsigned test_pagesz;        /* test hook: loader behaves as with this page size */
} eng_guest;

eng_guest *eng_guest_open(const char *root);
void       eng_guest_close(eng_guest *g);

/* Add a bind (host must exist).  Returns 0 or -errno.  Objects without
 * metadata inside it are presented as owned by work (1000:1000). */
int eng_guest_add_bind(eng_guest *g, const char *host, const char *guest);
/* Same, with an explicit default owner (system binds use root). */
int eng_guest_add_bind_owned(eng_guest *g, const char *host, const char *guest, unsigned uid, unsigned gid);
/* Where a host path lives: ENG_LOC_ROOTFS (the rootfs, outside every bind:
 * the metadata store applies), a bind index >= 0, or ENG_LOC_NONE. */
#define ENG_LOC_ROOTFS (-1)
#define ENG_LOC_NONE   (-2)
int eng_guest_locate(const eng_guest *g, const char *host);
/* Default owner of a host object without metadata: the containing bind's, or
 * 0:0 inside the rootfs. */
void eng_guest_default_owner(const eng_guest *g, const char *host, unsigned *uid, unsigned *gid);
/* Standard runtime binds: /proc, /sys, /dev whitelist.  Creates placeholder
 * entries in the rootfs /dev so listings show them. */
int eng_guest_default_binds(eng_guest *g);
int eng_guest_add_hide(eng_guest *g, const char *guest);
/* Create the bind's mount point in the rootfs (missing parents as root-owned
 * 0755 directories, the last one as a directory or an empty file), like a
 * mount point that must exist.  Existing entries are left alone. */
void eng_guest_make_mountpoint(eng_guest *g, const char *guest, int is_dir);
/* Parse and add one register string.  Returns 0 or -EINVAL. */
int eng_guest_add_binfmt(eng_guest *g, const char *rule);
/* Add every rule of a file (one per line, '#'/';' comments), host path. */
int eng_guest_load_binfmt_file(eng_guest *g, const char *host_path);
/* Rules from the guest's /usr/lib/binfmt.d and /etc/binfmt.d (*.conf, sorted,
 * same name in /etc wins), like systemd-binfmt.  Returns the rule count. */
int eng_guest_load_binfmt_dirs(eng_guest *g);

/* Map a canonical guest path to its host path.  Returns 0 or -ENAMETOOLONG. */
int eng_guest_to_host(const eng_guest *g, const char *guest, char *host, size_t cap);
/* Map a host path back to a guest path.  Returns 0, or -1 when the host path
 * is outside every mapping. */
int eng_host_to_guest(const eng_guest *g, const char *host, char *guest, size_t cap);
/* 1 if the canonical guest path is hidden (or inside a hidden directory). */
int eng_guest_is_hidden(const eng_guest *g, const char *guest);

#endif
