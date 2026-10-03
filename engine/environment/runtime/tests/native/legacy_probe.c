/* legacy_probe.c — calls every legacy x86_64 syscall the engine rewrites
 * (open, stat, dup2, pipe, poll, select, getdents, alarm, time, ...) through
 * raw syscall(2) and checks the results.  Built static; prints one line per
 * check and exits 0 only if all pass.  With --filter it first installs a
 * seccomp RET_TRAP filter for exactly the calls Android's x86_64 app filter
 * blocks (measured on API 28), like a guest started by the real app;
 * `--exec PROG ARGS...` installs the whole measured app filter (include/engine/
 * appfilter.h: legacy calls, newer calls, the set*id family) and executes PROG.
 * x86_64 only (aarch64 has no legacy calls). */
#define _GNU_SOURCE
#include <errno.h>
#include <dirent.h>
#include <fcntl.h>
#include <linux/filter.h>
#include <linux/seccomp.h>
#include <poll.h>
#include <signal.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/prctl.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/time.h>
#include <time.h>
#include <unistd.h>

#include "engine/appfilter.h"

static int fails;
static void check(const char *name, int ok, long v) {
    printf("%-14s %s", name, ok ? "ok" : "FAIL");
    if (!ok) printf(" (%ld, errno %d %s)", v, errno, strerror(errno));
    printf("\n");
    if (!ok) fails++;
}

static const int legacy_set[] = {ENG_APPFILTER_LEGACY};
static const int app_set[] = {ENG_APPFILTER_LEGACY, ENG_APPFILTER_OTHER};

static void install_filter(const int *trapped, int cnt) {
    struct sock_filter f[160];
    int n = 0;
    f[n++] = (struct sock_filter)BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, nr));
    for (int i = 0; i < cnt; i++)
        f[n++] = (struct sock_filter)BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, (unsigned)trapped[i], (unsigned char)(cnt - i), 0);
    f[n++] = (struct sock_filter)BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ALLOW);
    f[n++] = (struct sock_filter)BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_TRAP);
    struct sock_fprog prog = {(unsigned short)n, f};
    if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) || prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &prog)) {
        perror("seccomp");
        exit(2);
    }
}

struct old_dirent { unsigned long ino, off; unsigned short reclen; char name[]; };

int main(int argc, char **argv) {
    if (argc > 2 && !strcmp(argv[1], "--exec")) {      /* run a program behind the filter */
        install_filter(app_set, (int)(sizeof app_set / sizeof app_set[0]));
        execv(argv[2], argv + 2);
        perror("execv");
        return 127;
    }
    int filter = argc > 1 && !strcmp(argv[1], "--filter");
    if (filter) install_filter(legacy_set, (int)(sizeof legacy_set / sizeof legacy_set[0]));
    char dir[] = "/tmp/wf-legacy-XXXXXX";
    if (!mkdtemp(dir) || chdir(dir)) { perror("tmpdir"); return 2; }
    long r;
    struct stat st;

    r = syscall(__NR_creat, "c", 0640);           check("creat", r >= 0, r); close((int)r);
    r = syscall(__NR_open, "c", O_RDWR);          check("open", r >= 0, r);
    int fd = (int)r;
    r = syscall(__NR_access, "c", R_OK | W_OK);   check("access", r == 0, r);
    r = syscall(__NR_access, "nope", F_OK);       check("access-enoent", r == -1 && errno == ENOENT, r);
    r = syscall(__NR_stat, "c", &st);             check("stat", r == 0 && S_ISREG(st.st_mode), r);
    r = syscall(__NR_symlink, "c", "s");          check("symlink", r == 0, r);
    r = syscall(__NR_lstat, "s", &st);            check("lstat", r == 0 && S_ISLNK(st.st_mode), r);
    char buf[256];
    r = syscall(__NR_readlink, "s", buf, sizeof buf); check("readlink", r == 1 && buf[0] == 'c', r);
    r = syscall(__NR_mkdir, "d", 0755);           check("mkdir", r == 0, r);
    r = syscall(__NR_rename, "c", "d/c2");        check("rename", r == 0, r);
    r = syscall(__NR_link, "d/c2", "d/c3");       check("link", r == 0, r);
    r = syscall(__NR_stat, "d/c3", &st);          check("link-nlink", r == 0 && st.st_nlink == 2, (long)st.st_nlink);
    r = syscall(__NR_chmod, "d/c2", 0600);        check("chmod", r == 0, r);
    r = syscall(__NR_stat, "d/c2", &st);          check("chmod-stat", r == 0 && (st.st_mode & 0777) == 0600, (long)st.st_mode);
    r = syscall(__NR_chown, "d/c2", getuid(), getgid()); check("chown", r == 0, r);
    r = syscall(__NR_lchown, "s", getuid(), getgid());   check("lchown", r == 0, r);
    r = syscall(__NR_mknod, "fifo", S_IFIFO | 0600, 0);  check("mknod-fifo", r == 0, r);
    struct timeval tv[2] = {{1000000000, 0}, {1000000000, 500000}};
    r = syscall(__NR_utimes, "d/c2", tv);         check("utimes", r == 0, r);
    r = syscall(__NR_stat, "d/c2", &st);
    check("utimes-mtime", r == 0 && st.st_mtim.tv_sec == 1000000000 && st.st_mtim.tv_nsec == 500000000, (long)st.st_mtim.tv_nsec);
    tv[1].tv_sec = 1100000000; tv[1].tv_usec = 0;
    r = syscall(__NR_futimesat, AT_FDCWD, "d/c2", tv); check("futimesat", r == 0, r);
    struct { long a, m; } ub = {1200000000, 1200000000};
    r = syscall(__NR_utime, "d/c2", &ub);         check("utime", r == 0, r);
    r = syscall(__NR_stat, "d/c2", &st);          check("utime-mtime", r == 0 && st.st_mtim.tv_sec == 1200000000, r);

    r = syscall(__NR_dup2, fd, 50);               check("dup2", r == 50, r);
    r = syscall(__NR_dup2, 50, 50);               check("dup2-same", r == 50, r);
    r = syscall(__NR_dup2, 51, 51);               check("dup2-same-bad", r == -1 && errno == EBADF, r);
    int p[2];
    r = syscall(__NR_pipe, p);                    check("pipe", r == 0 && p[0] >= 0 && p[1] >= 0, r);
    struct pollfd pf = {p[0], POLLIN, 0};
    r = syscall(__NR_poll, &pf, 1, 10);           check("poll-timeout", r == 0, r);
    if (write(p[1], "x", 1) != 1) fails++;
    r = syscall(__NR_poll, &pf, 1, -1);           check("poll-ready", r == 1 && (pf.revents & POLLIN), r);
    fd_set rs; FD_ZERO(&rs); FD_SET(p[0], &rs);
    struct timeval to = {5, 0};
    r = syscall(__NR_select, p[0] + 1, &rs, NULL, NULL, &to);
    check("select-ready", r == 1 && FD_ISSET(p[0], &rs), r);
    check("select-tv-updated", to.tv_sec <= 5 && to.tv_sec >= 4, to.tv_sec);
    char c; if (read(p[0], &c, 1) != 1) fails++;
    FD_ZERO(&rs); FD_SET(p[0], &rs); to.tv_sec = 0; to.tv_usec = 20000;
    r = syscall(__NR_select, p[0] + 1, &rs, NULL, NULL, &to);
    check("select-timeout", r == 0 && to.tv_sec == 0 && to.tv_usec == 0, r);

    int dfd = open("d", O_RDONLY | O_DIRECTORY);
    char db[4096];
    r = syscall(__NR_getdents, dfd, db, sizeof db);
    int seen = 0, types_ok = 1;
    for (long i = 0; i < r;) {
        struct old_dirent *e = (struct old_dirent *)(db + i);
        char type = db[i + e->reclen - 1];
        if (!strcmp(e->name, "c2") || !strcmp(e->name, "c3")) { seen++; if (type != DT_REG) types_ok = 0; }
        if (!strcmp(e->name, ".") && type != DT_DIR) types_ok = 0;
        i += e->reclen;
    }
    check("getdents", r > 0 && seen == 2 && types_ok, r);
    close(dfd);

    r = syscall(__NR_getpgrp);                    check("getpgrp", r == getpgid(0), r);
    r = syscall(__NR_epoll_create, 0);            check("epoll_create0", r == -1 && errno == EINVAL, r);
    r = syscall(__NR_epoll_create, 1);            check("epoll_create", r >= 0, r);
    struct { unsigned ev; unsigned long data; } __attribute__((packed)) evs[4];
    r = syscall(__NR_epoll_wait, (int)r, evs, 4, 0); check("epoll_wait", r == 0, r);
    r = syscall(__NR_inotify_init);               check("inotify_init", r >= 0, r);
    r = syscall(__NR_eventfd, 3);                 check("eventfd", r >= 0, r);
    unsigned long long ev = 0;
    check("eventfd-read", r >= 0 && read((int)r, &ev, 8) == 8 && ev == 3, (long)ev);
    sigset_t ss; sigemptyset(&ss); sigaddset(&ss, SIGUSR1);
    r = syscall(__NR_signalfd, -1, &ss, 8);       check("signalfd", r >= 0, r);
    r = syscall(__NR_alarm, 100);                 check("alarm-set", r == 0, r);
    r = syscall(__NR_alarm, 0);                   check("alarm-old", r == 100 || r == 99, r);
    time_t tt = 0;
    r = syscall(__NR_time, &tt);                  check("time", r > 1700000000 && tt == r, r);

    r = syscall(__NR_unlink, "d/c3");             check("unlink", r == 0, r);
    r = syscall(__NR_stat, "d/c2", &st);          check("unlink-nlink", r == 0 && st.st_nlink == 1, (long)st.st_nlink);
    unlink("d/c2"); unlink("s"); unlink("fifo");
    r = syscall(__NR_rmdir, "d");                 check("rmdir", r == 0, r);
    if (chdir("/") == 0) rmdir(dir);
    printf("legacy-probe %s\n", fails ? "FAILED" : "passed");
    return fails ? 1 : 0;
}
