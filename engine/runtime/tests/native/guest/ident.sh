#!/bin/bash
# Identity / exec probe (M4).  Runs as `work` in the workspace image: by the
# engine (virtual ids, virtual set-id, emulated exec) and by podman on the same
# image imported as a real rootfs (real kernel); outputs must match.
set -u
# The image build date changes /etc/shadow's last-change field. Pin only this
# disposable guest account's fixture input so chage output tests identity/access
# semantics against the unchanged real-kernel golden, not the build clock.
sudo -n chage -d 2026-09-27 work || exit 91
T=/tmp/wf-ident
rm -rf "$T"; mkdir -p "$T"; cd "$T" || exit 90
r() { local n=$1; shift; local o; o=$("$@" 2>&1); local rc=$?; o=${o//$'\n'/|}; printf '%s rc=%d %s\n' "$n" "$rc" "$o"; }

r ids          bash -c 'id; id -un; id -gn'
r setid_bits   stat -c '%n %a %U %G' /usr/bin/sudo /usr/bin/su /usr/bin/passwd /usr/bin/chage /usr/sbin/unix_chkpwd
r sudo_id      sudo -n id
r sudo_su      sudo -n su -c 'id -u; id -un; echo $HOME; cd && pwd'
r sudo_login   sudo -n -i pwd
r sudo_nobody  sudo -n -u nobody id
r sudo_back    sudo -n su work -c 'id; echo $HOME'
r sudo_env     sudo -n sh -c 'echo "$SUDO_USER $SUDO_UID $USER $LOGNAME"'
r su_denied    bash -c 'su -c true root < /dev/null'
r chage        chage -l work
r shadow_read  cat /etc/shadow
r root_shadow  sudo -n sh -c 'grep -c "^work:" /etc/shadow'
r perms_root   sudo -n sh -c 'touch /etc/wf-root && stat -c "%U %G %a" /etc/wf-root && rm /etc/wf-root'

# exec: shebang chains (the kernel allows 5 interpreter levels), execveat
for i in 1 2 3 4 5 6; do printf '#!%s/s%d\n' "$T" $((i + 1)) > s$i; chmod +x s$i; done
printf '#!/bin/sh\necho "chain $0 $*"\n' > s7; chmod +x s7
r chain5       ./s3 a
r chain6       ./s2 a
r chain7       ./s1 a
cat > ex.c <<'EOF'
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <unistd.h>
static char *env[] = {"PATH=/usr/bin:/bin", NULL};
static void run(const char *what, int dfd, const char *p, int fl) {
    char *av[] = {"prog", "arg1", NULL};
    fflush(stdout);
    pid_t c = fork();
    if (c == 0) {
        syscall(SYS_execveat, dfd, p, av, env, fl);
        printf("%s: errno %d (%s)\n", what, errno, strerror(errno));
        fflush(stdout);
        _exit(111);
    }
    int st; waitpid(c, &st, 0);
    printf("%s: exit %d\n", what, WIFEXITED(st) ? WEXITSTATUS(st) : 128 + WTERMSIG(st));
}
int main(void) {
    int bin = open("/usr/bin", O_RDONLY | O_DIRECTORY);
    run("dirfd-relative", bin, "true", 0);
    run("dirfd-missing", bin, "no-such", 0);
    int f = open("/usr/bin/echo", O_RDONLY | O_CLOEXEC);
    run("empty-path", f, "", AT_EMPTY_PATH);
    run("empty-path-without-flag", f, "", 0);
    int opath = open("/usr/bin/false", O_PATH | O_CLOEXEC);
    run("o-path", opath, "", AT_EMPTY_PATH);
    symlink("/usr/bin/true", "/tmp/wf-ident/lnk");
    run("nofollow-symlink", AT_FDCWD, "/tmp/wf-ident/lnk", AT_SYMLINK_NOFOLLOW);
    run("follow-symlink", AT_FDCWD, "/tmp/wf-ident/lnk", 0);
    int sc = open("/tmp/wf-ident/s7", O_RDONLY);             /* script by fd, not CLOEXEC */
    run("script-fd", sc, "", AT_EMPTY_PATH);
    int scx = open("/tmp/wf-ident/s7", O_RDONLY | O_CLOEXEC);  /* the interpreter could not open it */
    run("script-fd-cloexec", scx, "", AT_EMPTY_PATH);
    run("dir", AT_FDCWD, "/usr", 0);
    run("noexec-file", AT_FDCWD, "/etc/passwd", 0);
    return 0;
}
EOF
r gcc          gcc -O1 -o ex ex.c
r execveat     ./ex
cat > auxv.c <<'EOF'
#include <stdio.h>
#include <sys/auxv.h>
#include <unistd.h>
int main(void) {
    char comm[32] = "";
    FILE *f = fopen("/proc/self/comm", "r");
    if (f) { if (!fgets(comm, sizeof comm, f)) comm[0] = 0; fclose(f); }
    printf("secure=%lu uid=%d euid=%d comm=%s", getauxval(AT_SECURE), getuid(), geteuid(), comm);
    return 0;
}
EOF
r suid_auxv    bash -c 'gcc -o auxv auxv.c && ./auxv && sudo -n install -o root -m 4755 auxv auxv-suid && ./auxv-suid &&
                        sudo -n install -o root -g shadow -m 2755 auxv auxv-sgid && ./auxv-sgid'
r cleanup      bash -c 'cd / && sudo -n rm -rf /tmp/wf-ident && echo gone'
# last: sudo must refuse a world-writable sudoers (virtual mode is what it checks)
r sudoers_mode bash -c 'sudo -n chmod 0666 /etc/sudoers; sudo -n true; echo "rc=$?"'
