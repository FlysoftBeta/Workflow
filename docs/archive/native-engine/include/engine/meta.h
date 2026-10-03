/* meta.h — virtual file metadata and hardlink emulation (M3).
 *
 * On-disk format (docs/engine.md):
 *   xattr user.workflow.meta = "1 UID GID MODE NLINK MAJ,MIN"  (MODE octal incl. type bits)
 *   on regular files, directories and virtual special files.  Symlinks carry
 *   none (user.* xattrs are refused on symlinks): they present as owned by
 *   their directory's owner, mode 0777.
 *   Hardlinks: content lives in <root>/.workflow-engine/links/<id>; every name
 *   is a symlink whose text is "/.workflow-engine/links/<id>" (a "stub");
 *   NLINK in the object's xattr counts the names.  Stubs behave as regular
 *   files towards the guest.
 * Special files (char/block devices, FIFOs, sockets made by mknod) are regular
 * host placeholders whose xattr carries the type: the kernel refuses user.*
 * xattrs on real FIFOs/sockets.  Opening a device placeholder opens the
 * whitelisted host device; opening a FIFO placeholder opens a real FIFO kept
 * at <root>/.workflow-engine/fifo/<dev>-<ino> (created on demand: a FIFO has no
 * content, only the placeholder's identity matters).
 * Files without the xattr get a default: owner 0:0 inside the rootfs, the
 * configured bind owner (work, 1000:1000) inside binds; mode from the host.
 */
#ifndef WORKFLOW_ENGINE_META_H
#define WORKFLOW_ENGINE_META_H

#include <stdint.h>
#include <sys/stat.h>

#include "engine/tracer.h"

#define ENG_META_XATTR   "user.workflow.meta"
#define ENG_META_PREFIX  "user.workflow."
#define ENG_STORE_GUEST  "/.workflow-engine"
#define ENG_LINK_PREFIX  "/.workflow-engine/links/"

typedef struct eng_meta {
    uint32_t uid, gid;
    uint32_t mode;       /* st_mode, type bits included */
    uint32_t nlink;      /* names of a hardlink object; 0 = not an object */
    uint32_t major, minor;
    int      present;    /* read from the xattr (1) or defaulted (0) */
} eng_meta;

/* Metadata of a host object.  `host` is used for the xattr lookup (following
 * symlinks unless nofollow; "/proc/<tid>/fd/<n>" works for fds).  `st` is the
 * host (l)stat of the same object.  `in_rootfs`: default owner selection. */
int eng_meta_read(struct eng_guest *g, const char *host, int nofollow, const struct stat *st, eng_meta *m);
int eng_meta_write(const char *host, int nofollow, const eng_meta *m);
/* Same as eng_meta_read (kept as the call sites' name; no cache). */
int eng_meta_get(struct eng_guest *g, const char *host, int nofollow, const struct stat *st, eng_meta *m);
/* 1 if metadata of this host object is kept in the store (rootfs outside every
 * bind; fd paths are followed).  Binds keep none: owner = bind owner, mode =
 * host bits. */
int eng_meta_in_store(struct eng_guest *g, const char *host);
/* Serialise read-modify-write metadata updates (chmod/chown) across engine
 * instances sharing the rootfs.  Returns a token for eng_meta_unlock. */
int  eng_meta_lock(struct eng_guest *g);
void eng_meta_unlock(int token);

/* 1 if the virtual type is a special file kept as a regular placeholder. */
int eng_meta_is_placeholder(const eng_meta *m, mode_t host_mode);
/* Host path of the real FIFO behind a FIFO placeholder (created if missing). */
int eng_meta_fifo_path(struct eng_guest *g, const struct stat *placeholder, char *out, size_t cap);

/* Overlay virtual metadata onto a kernel-filled struct stat / struct statx. */
void eng_meta_apply_stat(const eng_meta *m, struct stat *st);
void eng_meta_apply_statx(const eng_meta *m, void *stx);

/* Permission check with the task's fs credentials: mask of R_OK/W_OK/X_OK.
 * Returns 0 or -EACCES.  `use_real`: access(2) semantics (real ids). */
int eng_meta_permission(eng_task *t, const eng_meta *m, int mask, int use_real);
int eng_meta_may_exec(eng_task *t, const char *host, const struct stat *st);
int eng_in_group(const eng_task *t, uint32_t gid, int use_real);

/* Hardlink store (rootfs only) ------------------------------------------------ */
/* 1 if symlink text denotes a hardlink stub; copies the object id. */
int eng_link_is_stub_text(const char *text, char *id, size_t cap);
/* Host path of an object in the rootfs store. */
int eng_link_object_path(struct eng_guest *g, const char *id, char *out, size_t cap);
/* link(old_entry -> new_entry) inside the rootfs.  old_entry is the host path
 * of an existing regular file or stub.  Returns 0 or -errno. */
int eng_link_create(struct eng_guest *g, const char *old_entry, const char *new_entry);
/* One name of object `id` disappeared: nlink--, delete at 0. */
void eng_link_drop(struct eng_guest *g, const char *id);
/* Finish or undo interrupted operations (run at engine start). */
int eng_link_recover(struct eng_guest *g);

/* Consistency check of the hardlink store: every stub's object exists, every
 * object's NLINK equals its number of names, no journal left.  With `repair`
 * (only when no other instance runs: see eng_instance_lock) counts are fixed
 * and orphan objects removed.  Prints one line per finding to `out`.  Returns
 * the number of problems left (0 = consistent), or -errno. */
int eng_link_fsck(struct eng_guest *g, int repair, void *out /* FILE* */);

/* Instance registry: every running engine holds a shared lock on
 * <root>/.workflow-engine/instances.lock; `exclusive` (non-blocking) succeeds
 * only when no other instance uses the rootfs.  Returns fd or -errno
 * (-EWOULDBLOCK when busy). */
int eng_instance_lock(struct eng_guest *g, int exclusive);

#endif
