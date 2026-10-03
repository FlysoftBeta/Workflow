/* main.c — CLI for libworkflow-engine.so (a PIE executable packaged as .so so
 * PackageManager extracts it into nativeLibraryDir).  Contract: docs/engine.md.
 *
 *   run --root DIR [options] -- CMD [ARGS...]     run CMD inside the guest
 *   run -- CMD [ARGS...]                          host pass-through (tests)
 *   probe                                         capability JSON
 *
 * Exit status of `run`: the guest's (0..255, 128+signal); 125 engine error;
 * 126 command not executable; 127 command not found.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <libgen.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "engine/guest.h"
#include "engine/install.h"
#include "engine/log.h"
#include "engine/meta.h"
#include "engine/path.h"
#include "engine/tracer.h"

extern char **environ;

static void usage(void) {
    fprintf(stderr,
        "usage: workflow-engine run [--root DIR] [--cwd GUESTDIR] [--bind HOST:GUEST]...\n"
        "                           [--hide GUESTPATH]... [--loader FILE] [--user work|root]\n"
        "                           [--uid N] [--gid N] [--no-default-binds] [--no-filemap]\n"
        "                           [--wait-all] [-v] -- CMD [ARGS...]\n"
        "       workflow-engine probe\n"
        "       workflow-engine fsck --root DIR [--repair]\n"
        "       workflow-engine install --image FILE|- (--index image.json | --sha256 HEX) --target GEN\n"
        "                               [--profile workspace|base|any] [--quiet]\n"
        "       workflow-engine clone --from GEN --to GEN\n"
        "       workflow-engine verify --generation GEN\n"
        "       workflow-engine remove --generation GEN\n");
}

static int default_loader(char *out, size_t cap) {
    const char *env = getenv("WORKFLOW_ENGINE_LOADER");
    if (env && *env) { snprintf(out, cap, "%s", env); return 0; }
    char self[PATH_MAX];
    ssize_t n = readlink("/proc/self/exe", self, sizeof self - 1);
    if (n <= 0) return -1;
    self[n] = 0;
    char *dir = dirname(self);
    snprintf(out, cap, "%s/libworkflow-loader.so", dir);
    if (access(out, X_OK) == 0) return 0;
    snprintf(out, cap, "%s/loader", dir);            /* host build layout */
    return access(out, X_OK) == 0 ? 0 : -1;
}

/* Search the guest PATH for a bare command name; returns 0 with a guest path. */
static int guest_path_search(eng_guest *g, const char *name, char **envp, char *out, size_t cap) {
    const char *path = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";
    for (char **e = envp; e && *e; e++)
        if (!strncmp(*e, "PATH=", 5)) { path = *e + 5; break; }
    char buf[4096];
    snprintf(buf, sizeof buf, "%s", path);
    for (char *save = NULL, *dir = strtok_r(buf, ":", &save); dir; dir = strtok_r(NULL, ":", &save)) {
        if (dir[0] != '/') continue;
        char gp[PATH_MAX], hp[PATH_MAX];
        snprintf(gp, sizeof gp, "%s/%s", dir, name);
        if (eng_guest_to_host(g, gp, hp, sizeof hp)) continue;
        struct stat st;
        /* existence check follows host symlinks, good enough for the search
         * (the exec planner re-resolves with guest semantics) */
        if (stat(hp, &st) == 0 && S_ISREG(st.st_mode)) { snprintf(out, cap, "%s", gp); return 0; }
        if (lstat(hp, &st) == 0 && S_ISLNK(st.st_mode)) { snprintf(out, cap, "%s", gp); return 0; }
    }
    return -1;
}

static int cmd_probe(void);

/* fsck --root DIR [--repair]: 0 consistent, 1 problems left, 3 busy (repair
 * needs every other instance stopped), 125 error */
static int cmd_fsck(int argc, char **argv) {
    const char *root = NULL;
    int repair = 0;
    for (int i = 2; i < argc; i++) {
        if (!strcmp(argv[i], "--root") && i + 1 < argc) root = argv[++i];
        else if (!strcmp(argv[i], "--repair")) repair = 1;
        else { usage(); return 2; }
    }
    if (!root) { usage(); return 2; }
    eng_guest *g = eng_guest_open(root);
    if (!g) { fprintf(stderr, "fsck: cannot open %s: %s\n", root, strerror(errno)); return 125; }
    int lk = eng_instance_lock(g, repair);
    if (lk < 0) {
        fprintf(stderr, "fsck: %s\n", lk == -EWOULDBLOCK ? "rootfs in use by a running engine" : strerror(-lk));
        return lk == -EWOULDBLOCK ? 3 : 125;
    }
    int rc = eng_link_fsck(g, repair, stdout);
    close(lk);
    return rc < 0 ? 125 : rc > 0 ? 1 : 0;
}

/* install/clone/verify/remove (docs/engine.md "CLI") */
static int cmd_lifecycle(int argc, char **argv) {
    eng_install_opts o = {.profile = "workspace"};
    const char *from = NULL, *gen = NULL;
    for (int i = 2; i < argc; i++) {
        const char *a = argv[i];
        int more = i + 1 < argc;
        if (!strcmp(a, "--image") && more) o.image = argv[++i];
        else if (!strcmp(a, "--sha256") && more) o.sha256 = argv[++i];
        else if (!strcmp(a, "--index") && more) o.index = argv[++i];
        else if ((!strcmp(a, "--target") || !strcmp(a, "--to")) && more) o.target = argv[++i];
        else if (!strcmp(a, "--profile") && more) o.profile = argv[++i];
        else if (!strcmp(a, "--from") && more) from = argv[++i];
        else if (!strcmp(a, "--generation") && more) gen = argv[++i];
        else if (!strcmp(a, "--quiet")) o.quiet = 1;
        else if (!strcmp(a, "--keep-partial")) o.keep_partial = 1;
        else { fprintf(stderr, "%s: unknown option %s\n", argv[1], a); usage(); return 2; }
    }
    if (!strcmp(argv[1], "install")) {
        if (!o.image || !o.target) { usage(); return 2; }
        return eng_install(&o);
    }
    if (!strcmp(argv[1], "clone")) {
        if (!from || !o.target) { usage(); return 2; }
        return eng_clone(from, o.target, o.quiet);
    }
    if (!gen) { usage(); return 2; }
    if (!strcmp(argv[1], "verify")) return eng_verify(gen, o.quiet);
    /* remove: refuses a generation an engine instance is running on */
    char root[PATH_MAX];
    snprintf(root, sizeof root, "%s/rootfs", gen);
    eng_guest *g = eng_guest_open(root);
    if (g) {
        int lk = eng_instance_lock(g, 1);
        eng_guest_close(g);
        if (lk == -EWOULDBLOCK) { fprintf(stderr, "remove: generation in use by a running engine\n"); return 3; }
        if (lk >= 0) close(lk);
    }
    int rc = eng_remove_tree(gen);
    if (rc && rc != -ENOENT) { fprintf(stderr, "remove: %s\n", strerror(-rc)); return 74; }
    return 0;
}

int main(int argc, char **argv) {
    if (argc < 2) { usage(); return 2; }
    if (!strcmp(argv[1], "probe")) { eng_log_init(0); return cmd_probe(); }
    if (!strcmp(argv[1], "fsck")) { eng_log_init(0); return cmd_fsck(argc, argv); }
    if (!strcmp(argv[1], "install") || !strcmp(argv[1], "clone") || !strcmp(argv[1], "verify") ||
        !strcmp(argv[1], "remove")) {
        eng_log_init(0);
        return cmd_lifecycle(argc, argv);
    }
    if (strcmp(argv[1], "run")) { usage(); return 2; }

    const char *root = NULL, *cwd = NULL, *loader = NULL;
    const char *binds[ENG_MAX_BINDS], *hides[ENG_MAX_HIDES];
    int nb = 0, nh = 0, verbosity = 0, default_binds = 1, no_filemap = 0, seccomp = 1;
    unsigned test_pagesz = 0;
    int test_app_filter = 0, wait_all = 0;
    const char *binfmts[ENG_MAX_BINFMT];
    int nbf = 0, no_guest_binfmt = 0;
    const char *sockdir = NULL;
    long uid = 1000, gid = 1000;
    int i = 2;
    for (; i < argc; i++) {
        const char *a = argv[i];
        if (!strcmp(a, "--")) { i++; break; }
        else if (!strcmp(a, "--root") && i + 1 < argc) root = argv[++i];
        else if (!strcmp(a, "--cwd") && i + 1 < argc) cwd = argv[++i];
        else if (!strcmp(a, "--loader") && i + 1 < argc) loader = argv[++i];
        else if (!strcmp(a, "--bind") && i + 1 < argc && nb < ENG_MAX_BINDS) binds[nb++] = argv[++i];
        else if (!strcmp(a, "--hide") && i + 1 < argc && nh < ENG_MAX_HIDES) hides[nh++] = argv[++i];
        else if (!strcmp(a, "--uid") && i + 1 < argc) uid = strtol(argv[++i], NULL, 10);
        else if (!strcmp(a, "--gid") && i + 1 < argc) gid = strtol(argv[++i], NULL, 10);
        else if (!strcmp(a, "--user") && i + 1 < argc) {
            const char *u = argv[++i];
            if (!strcmp(u, "root")) uid = gid = 0;
            else if (!strcmp(u, "work")) uid = gid = 1000;
            else { fprintf(stderr, "run: unknown user %s\n", u); return 2; }
        }
        else if (!strcmp(a, "--no-default-binds")) default_binds = 0;
        else if (!strcmp(a, "--binfmt") && i + 1 < argc && nbf < ENG_MAX_BINFMT) binfmts[nbf++] = argv[++i];
        else if (!strcmp(a, "--no-guest-binfmt")) no_guest_binfmt = 1;
        else if (!strcmp(a, "--socket-dir") && i + 1 < argc) sockdir = argv[++i];
        else if (!strcmp(a, "--no-filemap")) no_filemap = 1;
        else if (!strcmp(a, "--test-page-size") && i + 1 < argc) test_pagesz = (unsigned)strtoul(argv[++i], NULL, 0);
        else if (!strcmp(a, "--seccomp")) seccomp = 1;
        else if (!strcmp(a, "--no-seccomp")) seccomp = 0;
        else if (!strcmp(a, "--test-app-filter")) test_app_filter = 1;
        else if (!strcmp(a, "--wait-all")) wait_all = 1;
        else if (!strcmp(a, "-v")) verbosity++;
        else if (!strcmp(a, "-vv")) verbosity += 2;
        else { fprintf(stderr, "run: unknown option %s\n", a); usage(); return 2; }
    }
    if (i >= argc) { fprintf(stderr, "run: missing command\n"); return 2; }
    eng_log_init(verbosity);

    eng_guest *guest = NULL;
    char cmd[PATH_MAX];
    char **gargv = &argv[i];
    if (root) {
        guest = eng_guest_open(root);
        if (!guest) { fprintf(stderr, "run: cannot open guest root %s: %s\n", root, strerror(errno)); return 125; }
        guest->no_filemap = no_filemap;
        guest->test_pagesz = test_pagesz;
        if (sockdir) snprintf(guest->sockdir, sizeof guest->sockdir, "%s", sockdir);
        else {
            const char *td = getenv("TMPDIR");
            snprintf(guest->sockdir, sizeof guest->sockdir, "%s/wfs-%d", td && *td ? td : "/tmp", (int)getuid());
        }
        if (loader) snprintf(guest->loader, sizeof guest->loader, "%s", loader);
        else if (default_loader(guest->loader, sizeof guest->loader)) {
            fprintf(stderr, "run: loader not found (set --loader or WORKFLOW_ENGINE_LOADER)\n");
            return 125;
        }
        {
            char real[PATH_MAX];
            if (!realpath(guest->loader, real) || access(real, X_OK) != 0) {
                fprintf(stderr, "run: loader %s not executable\n", guest->loader);
                return 125;
            }
            snprintf(guest->loader, sizeof guest->loader, "%s", real);
        }
        if (eng_instance_lock(guest, 0) < 0) {   /* held (shared) until exit */
            fprintf(stderr, "run: cannot register with %s: %s\n", root, strerror(errno));
            return 125;
        }
        if (default_binds && eng_guest_default_binds(guest)) {
            fprintf(stderr, "run: default binds failed\n");
            return 125;
        }
        for (int k = 0; k < nb; k++) {
            char spec[2 * PATH_MAX];
            snprintf(spec, sizeof spec, "%s", binds[k]);
            char *colon = strrchr(spec, ':');
            if (!colon) { fprintf(stderr, "run: --bind needs HOST:GUEST\n"); return 2; }
            *colon = 0;
            int rc = eng_guest_add_bind(guest, spec, colon + 1);
            if (rc) { fprintf(stderr, "run: bind %s: %s\n", binds[k], strerror(-rc)); return 125; }
            struct stat bst;
            eng_guest_make_mountpoint(guest, colon + 1, stat(spec, &bst) != 0 || S_ISDIR(bst.st_mode));
        }
        for (int k = 0; k < nh; k++) eng_guest_add_hide(guest, hides[k]);
        if (!no_guest_binfmt) eng_guest_load_binfmt_dirs(guest);
        for (int k = 0; k < nbf; k++) {
            int rc = binfmts[k][0] == '@' ? eng_guest_load_binfmt_file(guest, binfmts[k] + 1)
                                          : eng_guest_add_binfmt(guest, binfmts[k]);
            if (rc < 0) { fprintf(stderr, "run: bad --binfmt %s\n", binfmts[k]); return 2; }
        }
        if (!strchr(gargv[0], '/')) {
            if (guest_path_search(guest, gargv[0], environ, cmd, sizeof cmd)) {
                fprintf(stderr, "workflow-engine: %s: command not found\n", gargv[0]);
                return 127;
            }
        } else if (gargv[0][0] != '/') {
            snprintf(cmd, sizeof cmd, "%s/%s", cwd ? cwd : "", gargv[0]);
        } else {
            snprintf(cmd, sizeof cmd, "%s", gargv[0]);
        }
        /* argv[0] stays as the user typed it; the plan uses the resolved path */
    }

    {
        const char *se = getenv("WORKFLOW_ENGINE_SECCOMP");
        if (se && *se == '0') seccomp = 0;
    }
    /* the guest does not inherit the engine's own settings */
    static char *genv[4096];
    int ne = 0;
    for (char **e = environ; *e && ne < 4095; e++)
        if (strncmp(*e, "WORKFLOW_ENGINE_", 16)) genv[ne++] = *e;
    genv[ne] = NULL;
    if (guest && eng_log_enabled(ENG_LOG_DEBUG))
        for (int k = 0; k < guest->nbinds; k++)
            ENG_DBG("bind %s -> %s (owner %u:%u)", guest->binds[k].guest, guest->binds[k].host,
                    guest->binds[k].uid, guest->binds[k].gid);
    eng_run_cfg cfg = {
        .argv = gargv,
        .envp = root ? genv : environ,
        .cwd = cwd,
        .guest = guest,
        .use_seccomp_fastpath = seccomp,
        .test_app_filter = test_app_filter,
        .wait_all = wait_all,
        .uid = (uint32_t)uid,
        .gid = (uint32_t)gid,
        .exe = root ? cmd : NULL,
    };
    int rc = eng_tracer_run(&cfg);
    return rc < 0 ? 125 : rc;
}

int cmd_probe(void) {
    printf("{\"arch\":\"%s\",\"pageSize\":%ld}\n", ENG_ARCH_NAME, sysconf(_SC_PAGESIZE));
    return 0;
}
