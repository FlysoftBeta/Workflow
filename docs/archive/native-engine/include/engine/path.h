/* path.h — guest path resolution (M2). */
#ifndef WORKFLOW_ENGINE_PATH_H
#define WORKFLOW_ENGINE_PATH_H

#include <limits.h>

#include "engine/tracer.h"

#define ENG_RES_FOLLOW     0x1   /* follow a symlink in the final component */
#define ENG_RES_MISSING_OK 0x2   /* final component may be absent (creation) */
#define ENG_RES_DIR_ONLY   0x4   /* final must be a directory (chdir) */

typedef struct eng_resolved {
    char guest[PATH_MAX];      /* canonical guest path */
    char host[PATH_MAX];       /* path to give the kernel */
    int  verbatim;             /* host is a /proc magic path left to the kernel */
    int  magic;                /* final component is an unfollowed /proc magic link */
    char magic_text[PATH_MAX]; /* its guest-visible link text */
    int  exists;
    int  stub;                 /* final component is a hardlink stub */
    char stub_id[64];
    char entry[PATH_MAX];      /* host path of the directory entry (== host unless stub) */
} eng_resolved;

/* Resolve `path` (guest) relative to `dirfd` (guest fd or AT_FDCWD) for task
 * `t`.  Returns 0 or -errno (ENOENT, ENOTDIR, ELOOP, ENAMETOOLONG, EACCES). */
int eng_resolve(eng_task *t, int dirfd, const char *path, int flags, eng_resolved *out);

/* Guest-visible cwd / fd target of a task.  Return 0, or -errno; -ENOENT
 * with `out` set to the raw kernel text when the target is not a path in the
 * guest (pipe:[..], socket:[..], a host path outside every mapping). */
int eng_task_cwd(eng_task *t, char *out, size_t cap);
int eng_task_fd_path(eng_task *t, pid_t pid, int fd, char *out, size_t cap);

#endif
