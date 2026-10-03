/* install.h — image installer and generation lifecycle (docs/environment.md §2). */
#ifndef WORKFLOW_ENGINE_INSTALL_H
#define WORKFLOW_ENGINE_INSTALL_H

typedef struct {
    const char *image;      /* image.tar.zst path, "-" = stdin */
    const char *sha256;     /* expected sha256 of the whole stream (hex) */
    const char *index;      /* image.json: sha256 (+ size) taken from it */
    const char *target;     /* generation directory to create (must not exist) */
    const char *profile;    /* "workspace" (default), "base" or "any" */
    int quiet;
    int keep_partial;       /* debugging: keep TARGET on failure */
} eng_install_opts;

/* Exit codes (sysexits): 0 ok, 2 usage, 65 bad image data, 66 input missing,
 * 73 cannot create target, 74 I/O error, 75 not enough space.  Prints a JSON
 * summary on stdout on success, progress/errors on stderr. */
int eng_install(const eng_install_opts *o);
int eng_clone(const char *src_generation, const char *dst_generation, int quiet);
int eng_verify(const char *generation, int quiet);
/* rm -r without following symlinks.  0 or -errno. */
int eng_remove_tree(const char *path);

#endif
