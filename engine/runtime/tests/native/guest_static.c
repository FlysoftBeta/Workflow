/* guest_static.c — a static, no-libc guest ELF used to test the loader in
 * isolation (no dynamic linker, no library opens, so no path translation
 * needed).  It reads argc/argv/envp/auxv straight off the stack the loader
 * built and validates them, then exits with a status the host test checks.
 *
 * Build: cc -static -no-pie -nostdlib -ffreestanding -fno-stack-protector
 * exit codes:
 *   0  = argv[0]=="/guest_static", >=1 env var seen, AT_PAGESZ in {4096,16384},
 *        AT_RANDOM present and non-zero
 *   71 = bad argv    72 = bad pagesz    73 = missing AT_RANDOM   74 = bad envp
 *   75 = initialised data / bss wrong (segments sharing a runtime page were
 *        clobbered)   76 = program headers (AT_PHDR) unreadable or wrong
 */
#include <sys/syscall.h>
#include <stdint.h>

#if defined(__x86_64__)
static long syscall3(long n, long a, long b, long c) {
    long r;
    __asm__ volatile("syscall" : "=a"(r) : "a"(n), "D"(a), "S"(b), "d"(c)
                     : "rcx", "r11", "memory");
    return r;
}
__asm__(".globl _start\n_start:\n\txor %rbp,%rbp\n\tmov %rsp,%rdi\n\tand $-16,%rsp\n\tcall real_start\n\thlt\n");
#elif defined(__aarch64__)
static long syscall3(long n, long a, long b, long c) {
    register long x8 __asm__("x8") = n;
    register long x0 __asm__("x0") = a;
    register long x1 __asm__("x1") = b;
    register long x2 __asm__("x2") = c;
    __asm__ volatile("svc #0" : "+r"(x0) : "r"(x8), "r"(x1), "r"(x2) : "memory");
    return x0;
}
__asm__(".globl _start\n_start:\n\tmov x0, sp\n\tbl real_start\n\tbrk #0\n");
#endif

static volatile uint64_t data_v[4] = {0x1122334455667788ull, 2, 3, 4};
static volatile uint64_t bss_v[64];

static int streq(const char *a, const char *b) {
    while (*a && *a == *b) { a++; b++; }
    return *a == *b;
}

__attribute__((used)) void real_start(uint64_t *sp) {
    long argc = (long)sp[0];
    char **argv = (char **)(sp + 1);
    char **envp = argv + argc + 1;
    int envc = 0;
    while (envp[envc]) envc++;
    uint64_t *auxv = (uint64_t *)(envp + envc + 1);

    long pagesz = 0;
    uint64_t random_ptr = 0, phdr = 0, phnum = 0;
    for (uint64_t *a = auxv; a[0]; a += 2) {
        if (a[0] == 6 /*AT_PAGESZ*/) pagesz = (long)a[1];
        if (a[0] == 25 /*AT_RANDOM*/) random_ptr = a[1];
        if (a[0] == 3 /*AT_PHDR*/) phdr = a[1];
        if (a[0] == 5 /*AT_PHNUM*/) phnum = a[1];
    }
    int data_ok = data_v[0] == 0x1122334455667788ull && data_v[3] == 4;
    for (int i = 0; i < 64; i++) data_ok = data_ok && bss_v[i] == 0;
    data_v[1] = 7; bss_v[5] = 9;                 /* both must be writable */
    data_ok = data_ok && data_v[1] == 7 && bss_v[5] == 9;
    int phdr_ok = phdr && phnum >= 1 && phnum < 32;
    for (uint64_t i = 0; phdr_ok && i < phnum; i++) {
        uint32_t type = *(const uint32_t *)(phdr + i * 56);
        if (type != 1 && type != 6 && type != 4 && type != 0x6474e551 && type != 0x6474e552 &&
            type != 0x6474e553 && type != 7 && type != 0)
            phdr_ok = 0;
    }

    int code = 0;
    if (argc < 1 || !streq(argv[0], "/guest_static")) code = 71;
    else if (pagesz != 4096 && pagesz != 16384) code = 72;
    else if (!random_ptr || (*(uint64_t *)random_ptr == 0)) code = 73;
    else if (envc < 1) code = 74;
    else if (!data_ok) code = 75;
    else if (!phdr_ok) code = 76;

    if (code == 0) {
        const char msg[] = "guest-static-ok\n";
        syscall3(__NR_write, 1, (long)msg, sizeof(msg) - 1);
    }
    syscall3(__NR_exit_group, code, 0, 0);
    for (;;) {}
}
