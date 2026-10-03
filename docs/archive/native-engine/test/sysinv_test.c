/* sysinv_test.c -- checks for the generated syscall inventory (src/sysinv.c).
 *
 *   sysinv_test          run the checks for the table this build selected
 *   sysinv_test --dump   print "abi", "max" and one "nr<TAB>name<TAB>class" line per number
 *                        in [0, max]; gen/gensysinv.py --verify-dump compares it to the tables
 *
 * Built natively (x86_64 host, or aarch64 with the NDK) the table is also cross-checked
 * against the toolchain's <sys/syscall.h>.  gen/check.sh additionally builds it on the host
 * with -DENG_SYSINV_FORCE_ABI=2 to exercise the aarch64 table without an arm64 machine.
 */
#define _GNU_SOURCE
#include <limits.h>
#include <stdio.h>
#include <string.h>

#include "engine/sysinv.h"

#if !defined(ENG_SYSINV_FORCE_ABI)
#include <sys/syscall.h>
#endif

static int fails, checks;

#define CHECK(cond, ...) do {                                           \
        checks++;                                                       \
        if (!(cond)) {                                                  \
            fails++;                                                    \
            fprintf(stderr, "FAIL %s:%d: ", __FILE__, __LINE__);        \
            fprintf(stderr, __VA_ARGS__);                               \
            fputc('\n', stderr);                                        \
        }                                                               \
    } while (0)

typedef struct { long nr; const char *name; eng_sc_class cls; } known;
typedef struct { long lo, hi; } gap;

static const known X86_64[] = {
    {0, "read", ENG_SC_PASS},          {2, "open", ENG_SC_PATH},
    {3, "close", ENG_SC_FD},           {4, "stat", ENG_SC_PATH},
    {5, "fstat", ENG_SC_META},         {42, "connect", ENG_SC_SOCK},
    {56, "clone", ENG_SC_PROC},        {57, "fork", ENG_SC_PROC},
    {59, "execve", ENG_SC_EXEC},       {101, "ptrace", ENG_SC_EPERM},
    {102, "getuid", ENG_SC_ID},        {134, "uselib", ENG_SC_ENOSYS},
    {156, "_sysctl", ENG_SC_ENOSYS},   {165, "mount", ENG_SC_EPERM},
    {205, "set_thread_area", ENG_SC_ENOSYS},
    {217, "getdents64", ENG_SC_META},  {257, "openat", ENG_SC_PATH},
    {262, "newfstatat", ENG_SC_PATH},  {322, "execveat", ENG_SC_EXEC},
    {332, "statx", ENG_SC_PATH},       {334, "rseq", ENG_SC_PASS},
    {335, "uretprobe", ENG_SC_PASS},   {336, "uprobe", ENG_SC_PASS},
    {425, "io_uring_setup", ENG_SC_ENOSYS},
    {435, "clone3", ENG_SC_ENOSYS},    {437, "openat2", ENG_SC_ENOSYS},
    {439, "faccessat2", ENG_SC_PATH},  {444, "landlock_create_ruleset", ENG_SC_ENOSYS},
    {452, "fchmodat2", ENG_SC_PATH},   {457, "statmount", ENG_SC_ENOSYS},
    {463, "setxattrat", ENG_SC_PATH},  {471, "rseq_slice_yield", ENG_SC_PASS},
};
static const gap X86_64_GAPS[] = {{337, 423}};

static const known AARCH64[] = {
    {0, "io_setup", ENG_SC_PASS},      {17, "getcwd", ENG_SC_PATH},
    {18, "lookup_dcookie", ENG_SC_ENOSYS},
    {23, "dup", ENG_SC_FD},            {25, "fcntl", ENG_SC_FD},
    {29, "ioctl", ENG_SC_FD},          {38, "renameat", ENG_SC_PATH},
    {39, "umount2", ENG_SC_EPERM},     {40, "mount", ENG_SC_EPERM},
    {42, "nfsservctl", ENG_SC_ENOSYS}, {56, "openat", ENG_SC_PATH},
    {61, "getdents64", ENG_SC_META},   {63, "read", ENG_SC_PASS},
    {79, "newfstatat", ENG_SC_PATH},   {80, "fstat", ENG_SC_META},
    {117, "ptrace", ENG_SC_EPERM},     {163, "getrlimit", ENG_SC_PASS},
    {164, "setrlimit", ENG_SC_PASS},   {174, "getuid", ENG_SC_ID},
    {203, "connect", ENG_SC_SOCK},     {220, "clone", ENG_SC_PROC},
    {221, "execve", ENG_SC_EXEC},      {222, "mmap", ENG_SC_PASS},
    {243, "recvmmsg", ENG_SC_SOCK},    {260, "wait4", ENG_SC_PROC},
    {281, "execveat", ENG_SC_EXEC},    {291, "statx", ENG_SC_PATH},
    {294, "kexec_file_load", ENG_SC_EPERM},
    {425, "io_uring_setup", ENG_SC_ENOSYS},
    {435, "clone3", ENG_SC_ENOSYS},    {437, "openat2", ENG_SC_ENOSYS},
    {447, "memfd_secret", ENG_SC_PASS}, {471, "rseq_slice_yield", ENG_SC_PASS},
};
static const gap AARCH64_GAPS[] = {{244, 259}, {295, 423}};

/* Names that exist on one ABI only must be absent from the other table. */
static const char *const X86_ONLY[] = {"open", "stat", "fork", "vfork", "getdents", "arch_prctl",
                                       "iopl", "uselib", "time", "uprobe"};

#define N(a) (sizeof(a) / sizeof((a)[0]))

static long find_name(const char *name) {
    for (long nr = 0; nr <= eng_sysinv_max(); nr++) {
        const char *n = eng_sysinv_name(nr);
        if (n && !strcmp(n, name)) return nr;
    }
    return -1;
}

static void check_known(const known *k, size_t nk, const gap *g, size_t ng) {
    for (size_t i = 0; i < nk; i++) {
        const char *n = eng_sysinv_name(k[i].nr);
        CHECK(n && !strcmp(n, k[i].name), "nr %ld: name %s, want %s", k[i].nr, n ? n : "(null)", k[i].name);
        CHECK(eng_sysinv_class(k[i].nr) == k[i].cls, "%s: class %s, want %s", k[i].name,
              eng_sysinv_class_name(eng_sysinv_class(k[i].nr)), eng_sysinv_class_name(k[i].cls));
    }
    for (size_t i = 0; i < ng; i++)
        for (long nr = g[i].lo; nr <= g[i].hi; nr++) {
            CHECK(eng_sysinv_name(nr) == NULL, "gap nr %ld has name %s", nr, eng_sysinv_name(nr));
            CHECK(eng_sysinv_class(nr) == ENG_SC_UNKNOWN, "gap nr %ld is classified", nr);
        }
}

/* Cross-check against the toolchain's own __NR_* macros (native builds only). */
static void check_toolchain_headers(void) {
#if !defined(ENG_SYSINV_FORCE_ABI)
    static const struct { const char *name; long nr; } h[] = {
#define H(x) {#x, __NR_##x}
        H(read), H(write), H(close), H(openat), H(newfstatat), H(fstat), H(statx), H(execve),
        H(execveat), H(clone), H(wait4), H(kill), H(getuid), H(setresuid), H(capget), H(bind),
        H(connect), H(recvmsg), H(mount), H(umount2), H(ptrace), H(dup3), H(fcntl), H(ioctl),
        H(getdents64), H(renameat2), H(readlinkat), H(getcwd), H(pidfd_open), H(close_range),
        H(io_uring_setup), H(clone3), H(openat2), H(faccessat2), H(rseq), H(process_vm_readv),
#ifdef __NR_open
        H(open), H(stat), H(lstat), H(access), H(fork), H(vfork), H(getdents), H(readlink),
#endif
#ifdef __NR_renameat
        H(renameat),
#endif
#ifdef __NR_fchmodat2
        H(fchmodat2),
#endif
#undef H
    };
    for (size_t i = 0; i < N(h); i++) {
        const char *n = eng_sysinv_name(h[i].nr);
        CHECK(n && !strcmp(n, h[i].name), "__NR_%s = %ld but table says %s", h[i].name, h[i].nr,
              n ? n : "(null)");
    }
#endif
}

static void check_all(void) {
    long max = eng_sysinv_max();
    CHECK(max > 400 && max < 1024, "implausible max %ld", max);
    CHECK(eng_sysinv_name(max) != NULL, "max %ld is not assigned", max);
    const long bad[] = {-1, -2, LONG_MIN, max + 1, max + 1000, LONG_MAX,
                        0x40000000L | 257 /* x32 openat */, 0x40000000L};
    for (size_t i = 0; i < N(bad); i++) {
        CHECK(eng_sysinv_name(bad[i]) == NULL, "nr %ld has a name", bad[i]);
        CHECK(eng_sysinv_class(bad[i]) == ENG_SC_UNKNOWN, "nr %ld is classified", bad[i]);
    }
    int per_class[ENG_SC_COUNT] = {0}, assigned = 0;
    for (long nr = 0; nr <= max; nr++) {
        const char *n = eng_sysinv_name(nr);
        eng_sc_class c = eng_sysinv_class(nr);
        CHECK((n != NULL) == (c != ENG_SC_UNKNOWN), "nr %ld: name %s but class %s", nr,
              n ? n : "(null)", eng_sysinv_class_name(c));
        CHECK((unsigned)c < ENG_SC_COUNT, "nr %ld: class %d out of range", nr, (int)c);
        if (!n) continue;
        assigned++;
        per_class[c]++;
        CHECK(n[0] && strspn(n, "abcdefghijklmnopqrstuvwxyz0123456789_") == strlen(n), "nr %ld: bad name '%s'", nr, n);
        CHECK(find_name(n) == nr, "name %s appears twice (first at %ld, again at %ld)", n, find_name(n), nr);
    }
    CHECK(per_class[ENG_SC_UNKNOWN] == 0, "assigned numbers classified UNKNOWN");
    CHECK(per_class[ENG_SC_EXEC] == 2, "want exactly execve+execveat as EXEC, got %d", per_class[ENG_SC_EXEC]);
    for (int c = 0; c < ENG_SC_COUNT; c++)
        for (int d = c + 1; d < ENG_SC_COUNT; d++)
            CHECK(strcmp(eng_sysinv_class_name((eng_sc_class)c), eng_sysinv_class_name((eng_sc_class)d)),
                  "class names %d and %d collide", c, d);
    CHECK(!strcmp(eng_sysinv_class_name(ENG_SC_PATH), "PATH"), "class name PATH");
    CHECK(!strcmp(eng_sysinv_class_name((eng_sc_class)ENG_SC_COUNT), "INVALID"), "class name out of range");
    CHECK(!strcmp(eng_sysinv_class_name((eng_sc_class)-1), "INVALID"), "class name negative");

    const char *abi = eng_sysinv_abi();
    if (!strcmp(abi, "x86_64")) {
        check_known(X86_64, N(X86_64), X86_64_GAPS, N(X86_64_GAPS));
        CHECK(assigned == 385, "x86_64: %d syscalls, want 385 (Linux %s)", assigned, ENG_SYSINV_LINUX);
    } else if (!strcmp(abi, "aarch64")) {
        check_known(AARCH64, N(AARCH64), AARCH64_GAPS, N(AARCH64_GAPS));
        CHECK(assigned == 327, "aarch64: %d syscalls, want 327 (Linux %s)", assigned, ENG_SYSINV_LINUX);
        for (size_t i = 0; i < N(X86_ONLY); i++)
            CHECK(find_name(X86_ONLY[i]) < 0, "aarch64 table has x86_64-only %s", X86_ONLY[i]);
    } else {
        CHECK(0, "unknown abi %s", abi);
    }
    check_toolchain_headers();

    printf("sysinv %s (Linux %s): %d syscalls, max %ld;", abi, ENG_SYSINV_LINUX, assigned, max);
    for (int c = 1; c < ENG_SC_COUNT; c++) printf(" %s=%d", eng_sysinv_class_name((eng_sc_class)c), per_class[c]);
    printf("\n");
}

static void dump(void) {
    long max = eng_sysinv_max();
    printf("abi\t%s\nmax\t%ld\n", eng_sysinv_abi(), max);
    for (long nr = 0; nr <= max; nr++) {
        const char *n = eng_sysinv_name(nr);
        printf("%ld\t%s\t%s\n", nr, n ? n : "-", eng_sysinv_class_name(eng_sysinv_class(nr)));
    }
}

int main(int argc, char **argv) {
    if (argc > 1 && !strcmp(argv[1], "--dump")) {
        dump();
        return 0;
    }
    check_all();
    printf("sysinv_test: %d checks, %d failed\n", checks, fails);
    return fails ? 1 : 0;
}
