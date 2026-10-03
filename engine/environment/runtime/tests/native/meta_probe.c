/* meta_probe.c — engine-private xattrs are invisible and protected.
 *   meta_probe FILE : listxattr shows no user.workflow.*, getxattr of it is
 *   ENODATA, setxattr/removexattr of it are EPERM, a user.test attribute round-trips.
 * Built static (runs as a guest without libc path translation concerns). */
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/xattr.h>

int main(int argc, char **argv) {
    if (argc != 2) return 2;
    const char *f = argv[1];
    int fails = 0;
    char buf[4096];
    ssize_t n = listxattr(f, buf, sizeof buf);
    for (ssize_t i = 0; n > 0 && i < n; i += (ssize_t)strlen(buf + i) + 1)
        if (!strncmp(buf + i, "user.workflow.", 14)) { printf("list shows %s\n", buf + i); fails++; }
    if (getxattr(f, "user.workflow.meta", buf, sizeof buf) >= 0 || errno != ENODATA) { printf("get visible\n"); fails++; }
    if (setxattr(f, "user.workflow.meta", "1 0 0 100777 0 0,0", 18, 0) == 0 || errno != EPERM) { printf("set allowed\n"); fails++; }
    if (removexattr(f, "user.workflow.meta") == 0 || errno != EPERM) { printf("remove allowed\n"); fails++; }
    if (setxattr(f, "user.test", "v1", 2, 0) != 0) { printf("user.test set: %s\n", strerror(errno)); fails++; }
    else if (getxattr(f, "user.test", buf, sizeof buf) != 2 || memcmp(buf, "v1", 2)) { printf("user.test get\n"); fails++; }
    if (setxattr(f, "security.capability", "\1\0\0\2", 4, 0) == 0) { printf("file caps accepted\n"); fails++; }
    printf("meta-probe %s\n", fails ? "FAILED" : "passed");
    return fails != 0;
}
