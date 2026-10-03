/* loader.c — freestanding guest ELF loader (libworkflow-loader.so).
 *
 * Built -nostdlib -static-pie -ffreestanding; no libc, no relocations (the
 * build verifies there are none).  The tracer rewrites every guest execve into
 * an execve of this file (nativeLibraryDir => permitted on API 29+), leaving
 * argv/envp untouched, so the kernel performs the real exec.  We then:
 *
 *   1. ask the tracer for the load plan (OP_QUERY marker, see loader_proto.h);
 *   2. map the guest ELF and its PT_INTERP: file-backed MAP_PRIVATE first
 *      (shares page cache — node is ~100 MB), falling back to anonymous
 *      memory + pread + mprotect when the platform refuses PROT_EXEC file
 *      mappings (EACCES/EPERM) or the segments are not congruent with the
 *      runtime page size;
 *   3. mmap the per-process scratch region the tracer writes translated paths
 *      into, and report it (OP_DONE marker);
 *   4. build the guest's initial stack on the REAL process stack just below
 *      the kernel-built one (keeps [stack], RLIMIT_STACK growth and the
 *      original argv/envp strings, so /proc/self/cmdline stays correct), and
 *      jump to the interpreter (or static entry).
 *
 * Page size always comes from AT_PAGESZ; nothing assumes 4096.
 */
#include <asm/unistd.h>
#include <elf.h>
#include <stddef.h>
#include <stdint.h>

#include "engine/loader_proto.h"

/* ---- raw syscalls -------------------------------------------------------- */
#if defined(__x86_64__)
static inline long sc6(long n, long a, long b, long c, long d, long e, long f) {
    register long r10 __asm__("r10") = d;
    register long r8 __asm__("r8") = e;
    register long r9 __asm__("r9") = f;
    long ret;
    __asm__ volatile("syscall"
                     : "=a"(ret)
                     : "a"(n), "D"(a), "S"(b), "d"(c), "r"(r10), "r"(r8), "r"(r9)
                     : "rcx", "r11", "memory");
    return ret;
}
#define EM_SELF EM_X86_64
#define PLATFORM_MACHINE "x86_64"
#elif defined(__aarch64__)
static inline long sc6(long n, long a, long b, long c, long d, long e, long f) {
    register long x8 __asm__("x8") = n;
    register long x0 __asm__("x0") = a;
    register long x1 __asm__("x1") = b;
    register long x2 __asm__("x2") = c;
    register long x3 __asm__("x3") = d;
    register long x4 __asm__("x4") = e;
    register long x5 __asm__("x5") = f;
    __asm__ volatile("svc #0"
                     : "+r"(x0)
                     : "r"(x8), "r"(x1), "r"(x2), "r"(x3), "r"(x4), "r"(x5)
                     : "memory");
    return x0;
}
#define EM_SELF EM_AARCH64
#else
#error unsupported arch
#endif

static long sys1(long n, long a) { return sc6(n, a, 0, 0, 0, 0, 0); }
static long sys3(long n, long a, long b, long c) { return sc6(n, a, b, c, 0, 0, 0); }
static long sys4(long n, long a, long b, long c, long d) { return sc6(n, a, b, c, d, 0, 0); }

#define L_PROT_READ 1
#define L_PROT_WRITE 2
#define L_PROT_EXEC 4
#define L_MAP_PRIVATE 0x02
#define L_MAP_FIXED 0x10
#define L_MAP_ANONYMOUS 0x20
#define L_MAP_NORESERVE 0x4000
#define L_MAP_FIXED_NOREPLACE 0x100000
#define L_O_RDONLY 0
#define L_O_CLOEXEC 02000000
#define L_AT_FDCWD (-100)
#define L_EACCES 13
#define L_EPERM 1
#define L_EINVAL 22

static int is_err(long r) { return (unsigned long)r > (unsigned long)-4096; }

/* ---- mini libc ------------------------------------------------------------ */
static size_t l_strlen(const char *s) { size_t n = 0; while (s[n]) n++; return n; }
static void l_memcpy(void *d, const void *s, size_t n) {
    unsigned char *dd = d; const unsigned char *ss = s;
    while (n--) *dd++ = *ss++;
}
static void l_memset(void *d, int c, size_t n) {
    unsigned char *dd = d;
    while (n--) *dd++ = (unsigned char)c;
}
/* Compilers may emit calls to these even with -ffreestanding. */
void *memcpy(void *d, const void *s, size_t n) { l_memcpy(d, s, n); return d; }
void *memset(void *d, int c, size_t n) { l_memset(d, c, n); return d; }

static void die(const char *what, const char *arg, long err) __attribute__((noreturn));
static void wr(const char *s) { sys3(__NR_write, 2, (long)s, (long)l_strlen(s)); }
static void die(const char *what, const char *arg, long err) {
    char num[24]; int i = 23; num[i] = 0;
    unsigned long v = (unsigned long)(err < 0 ? -err : err);
    do { num[--i] = (char)('0' + v % 10); v /= 10; } while (v && i > 0);
    wr("workflow-loader: "); wr(what);
    if (arg) { wr(" "); wr(arg); }
    wr(": errno "); wr(&num[i]); wr("\n");
    sys1(__NR_exit_group, 127);
    for (;;) {}
}

/* ---- state ---------------------------------------------------------------- */
static uint8_t plan_buf[ENG_PLAN_MAX] __attribute__((aligned(16)));

typedef struct {
    uintptr_t bias, entry, phdr;
    uint16_t phnum, phent;
    int is_dyn;
} img;

static uintptr_t pgsz;
static uintptr_t pdown(uintptr_t x) { return x & ~(pgsz - 1); }
static uintptr_t pup(uintptr_t x) { return (x + pgsz - 1) & ~(pgsz - 1); }

static long pread_full(int fd, void *buf, size_t n, long off) {
    size_t done = 0;
    while (done < n) {
        long r = sc6(__NR_pread64, fd, (long)((char *)buf + done), (long)(n - done), off + (long)done, 0, 0);
        if (is_err(r)) { if (r == -4 /*EINTR*/) continue; return r; }
        if (r == 0) break;
        done += (size_t)r;
    }
    return (long)done;
}

static int prot_of(uint32_t f) {
    return ((f & PF_R) ? L_PROT_READ : 0) | ((f & PF_W) ? L_PROT_WRITE : 0) | ((f & PF_X) ? L_PROT_EXEC : 0);
}

/* Map one segment from the file.  Returns 0, or -errno. */
static long map_seg_file(int fd, const Elf64_Phdr *p, uintptr_t bias) {
    int prot = prot_of(p->p_flags);
    uintptr_t va = bias + p->p_vaddr;
    uintptr_t start = pdown(va);
    uintptr_t fend = pup(va + p->p_filesz);
    uintptr_t mend = pup(va + p->p_memsz);
    if (p->p_filesz) {
        long r = sc6(__NR_mmap, (long)start, (long)(fend - start), prot,
                     L_MAP_PRIVATE | L_MAP_FIXED, fd, (long)pdown(p->p_offset));
        if (is_err(r)) return r;
    }
    if (p->p_memsz > p->p_filesz) {
        /* zero the tail of the last file page */
        uintptr_t zstart = va + p->p_filesz;
        uintptr_t zend = p->p_filesz ? fend : start;
        if (zend > zstart && p->p_filesz) {
            if (!(prot & L_PROT_WRITE))
                sys3(__NR_mprotect, (long)pdown(zstart), (long)pgsz, prot | L_PROT_WRITE);
            l_memset((void *)zstart, 0, zend - zstart);
            if (!(prot & L_PROT_WRITE))
                sys3(__NR_mprotect, (long)pdown(zstart), (long)pgsz, prot);
        }
        if (mend > zend) {
            long r = sc6(__NR_mmap, (long)zend, (long)(mend - zend), prot,
                         L_MAP_PRIVATE | L_MAP_FIXED | L_MAP_ANONYMOUS, -1, 0);
            if (is_err(r)) return r;
        }
    }
    return 0;
}

/* Map all PT_LOAD segments as anonymous memory + copy.  Segments of an ELF
 * linked for a smaller page size share runtime pages (16 KiB kernel, 4 KiB
 * binary), so the union is mapped once, every segment is copied into it, and
 * each page gets the union of the protections of the segments touching it. */
static uintptr_t cuts[2 * 256 + 2];
static long map_all_anon(int fd, const Elf64_Phdr *ph, int n, uintptr_t bias, uintptr_t lo, uintptr_t hi) {
    uintptr_t start = pdown(bias + lo), end = pup(bias + hi);
    long r = sc6(__NR_mmap, (long)start, (long)(end - start), L_PROT_READ | L_PROT_WRITE,
                 L_MAP_PRIVATE | L_MAP_FIXED | L_MAP_ANONYMOUS, -1, 0);
    if (is_err(r)) return r;
    int nc = 0;
    cuts[nc++] = start;
    cuts[nc++] = end;
    for (int i = 0; i < n; i++) {
        if (ph[i].p_type != PT_LOAD) continue;
        uintptr_t va = bias + ph[i].p_vaddr;
        if (ph[i].p_filesz) {
            r = pread_full(fd, (void *)va, ph[i].p_filesz, (long)ph[i].p_offset);
            if (is_err(r)) return r;
        }
        cuts[nc++] = pdown(va);
        cuts[nc++] = pup(va + ph[i].p_memsz);
    }
    for (int i = 1; i < nc; i++)            /* insertion sort, n is tiny */
        for (int j = i; j > 0 && cuts[j - 1] > cuts[j]; j--) {
            uintptr_t x = cuts[j]; cuts[j] = cuts[j - 1]; cuts[j - 1] = x;
        }
    for (int i = 0; i + 1 < nc; i++) {
        uintptr_t a = cuts[i], b = cuts[i + 1];
        if (a >= b) continue;
        int prot = 0;
        for (int k = 0; k < n; k++) {
            if (ph[k].p_type != PT_LOAD) continue;
            uintptr_t s = pdown(bias + ph[k].p_vaddr), e = pup(bias + ph[k].p_vaddr + ph[k].p_memsz);
            if (s < b && e > a) prot |= prot_of(ph[k].p_flags);
        }
        r = sys3(__NR_mprotect, (long)a, (long)(b - a), prot);
        if (is_err(r)) return r;
    }
    return 0;
}

static Elf64_Phdr phbuf[2][256];

static void map_image(const char *path, img *out, int which, int force_anon) {
    long fd = sys4(__NR_openat, L_AT_FDCWD, (long)path, L_O_RDONLY | L_O_CLOEXEC, 0);
    if (is_err(fd)) die("open", path, fd);
    Elf64_Ehdr eh;
    long r = pread_full((int)fd, &eh, sizeof eh, 0);
    if (r != (long)sizeof eh) die("read ELF header", path, is_err(r) ? r : -L_EINVAL);
    if (eh.e_ident[0] != 0x7f || eh.e_ident[1] != 'E' || eh.e_ident[2] != 'L' || eh.e_ident[3] != 'F' ||
        eh.e_ident[EI_CLASS] != ELFCLASS64 || eh.e_machine != EM_SELF ||
        (eh.e_type != ET_DYN && eh.e_type != ET_EXEC) ||
        eh.e_phentsize != sizeof(Elf64_Phdr) || eh.e_phnum == 0 || eh.e_phnum > 256)
        die("not a loadable ELF for this ABI", path, -8 /*ENOEXEC*/);
    Elf64_Phdr *ph = phbuf[which];
    r = pread_full((int)fd, ph, (size_t)eh.e_phnum * sizeof(Elf64_Phdr), (long)eh.e_phoff);
    if (r != (long)(eh.e_phnum * sizeof(Elf64_Phdr))) die("read program headers", path, -L_EINVAL);

    uintptr_t lo = ~(uintptr_t)0, hi = 0, align = pgsz, phdr_va = 0;
    int have_phdr = 0, congruent = 1;
    for (int i = 0; i < eh.e_phnum; i++) {
        if (ph[i].p_type == PT_PHDR) { phdr_va = ph[i].p_vaddr; have_phdr = 1; }
        if (ph[i].p_type != PT_LOAD) continue;
        if (ph[i].p_vaddr < lo) lo = ph[i].p_vaddr;
        if (ph[i].p_vaddr + ph[i].p_memsz > hi) hi = ph[i].p_vaddr + ph[i].p_memsz;
        if (ph[i].p_align > align && (ph[i].p_align & (ph[i].p_align - 1)) == 0 && ph[i].p_align <= 0x10000)
            align = ph[i].p_align;
        if (((ph[i].p_vaddr - ph[i].p_offset) & (pgsz - 1)) != 0) congruent = 0;
    }
    if (hi <= lo) die("no PT_LOAD", path, -8);
    /* file-backed maps need every segment on pages of its own: with a runtime
     * page larger than the link-time one, neighbours share a page and the
     * later MAP_FIXED would replace the earlier one's bytes/protection */
    for (int i = 0; i < eh.e_phnum && congruent; i++) {
        if (ph[i].p_type != PT_LOAD) continue;
        for (int j = 0; j < eh.e_phnum; j++) {
            if (j == i || ph[j].p_type != PT_LOAD) continue;
            uintptr_t si = pdown(ph[i].p_vaddr), ei = pup(ph[i].p_vaddr + ph[i].p_memsz);
            uintptr_t sj = pdown(ph[j].p_vaddr), ej = pup(ph[j].p_vaddr + ph[j].p_memsz);
            if (si < ej && sj < ei) { congruent = 0; break; }
        }
    }

    int anon = force_anon || !congruent;
    for (int attempt = 0; attempt < 2; attempt++) {
        uintptr_t bias = 0;
        uintptr_t span = pup(hi) - pdown(lo);
        long base = 0;
        if (eh.e_type == ET_DYN) {
            base = sc6(__NR_mmap, 0, (long)(span + align), 0 /*PROT_NONE*/,
                       L_MAP_PRIVATE | L_MAP_ANONYMOUS, -1, 0);
            if (is_err(base)) die("reserve address space", path, base);
            uintptr_t aligned = ((uintptr_t)base + align - 1) & ~(align - 1);
            /* trim the unused head/tail of the reservation */
            if (aligned > (uintptr_t)base) sys3(__NR_munmap, base, (long)(aligned - (uintptr_t)base), 0);
            uintptr_t tail = (uintptr_t)base + span + align - (aligned + span);
            if (tail) sys3(__NR_munmap, (long)(aligned + span), (long)tail, 0);
            bias = aligned - pdown(lo);
        } else {
            /* ET_EXEC: fixed addresses; refuse to clobber existing mappings. */
            long got = sc6(__NR_mmap, (long)pdown(lo), (long)span, 0,
                           L_MAP_PRIVATE | L_MAP_ANONYMOUS | L_MAP_FIXED_NOREPLACE, -1, 0);
            if (is_err(got) || (uintptr_t)got != pdown(lo)) {
                if (!is_err(got)) sys3(__NR_munmap, got, (long)span, 0);
                die("fixed-address ELF overlaps an existing mapping", path, -12);
            }
        }
        long err = 0;
        if (anon) err = map_all_anon((int)fd, ph, eh.e_phnum, bias, lo, hi);
        else
            for (int i = 0; i < eh.e_phnum && !err; i++)
                if (ph[i].p_type == PT_LOAD) err = map_seg_file((int)fd, &ph[i], bias);
        if (!err) {
            out->bias = bias;
            out->entry = bias + eh.e_entry;
            out->phnum = eh.e_phnum;
            out->phent = eh.e_phentsize;
            out->is_dyn = eh.e_type == ET_DYN;
            out->phdr = 0;
            if (have_phdr) out->phdr = bias + phdr_va;
            else
                for (int i = 0; i < eh.e_phnum; i++)
                    if (ph[i].p_type == PT_LOAD && eh.e_phoff >= ph[i].p_offset &&
                        eh.e_phoff < ph[i].p_offset + ph[i].p_filesz) {
                        out->phdr = bias + ph[i].p_vaddr + (eh.e_phoff - ph[i].p_offset);
                        break;
                    }
            sys1(__NR_close, fd);
            return;
        }
        /* Platform refused file-backed PROT_EXEC: retry anonymously. */
        sys3(__NR_munmap, (long)(bias + pdown(lo)), (long)span, 0);
        if (anon || (err != -L_EACCES && err != -L_EPERM)) die("map segment", path, err);
        anon = 1;
    }
    die("map", path, -L_EINVAL);
}

/* ---- final hand-off (asm) --------------------------------------------------
 * wf_finish(final_sp, block, len, entry): switch to final_sp, copy len bytes
 * (multiple of 16) from block to it, munmap(block, len), zero registers and
 * jump.  Never touches the stack after switching. */
extern void wf_finish(uintptr_t sp, const void *block, size_t len, uintptr_t entry) __attribute__((noreturn));

#if defined(__x86_64__)
__asm__(".text\n.globl wf_finish\n.type wf_finish,@function\nwf_finish:\n"
        "\tmov %rdi, %rsp\n"
        "\tmov %rcx, %r8\n"          /* entry */
        "\tmov %rsi, %r9\n"          /* block (for munmap) */
        "\tmov %rdx, %r10\n"         /* len */
        "\tmov %rdx, %rcx\n\tshr $3, %rcx\n"
        "\tcld\n\trep movsq\n"       /* rdi=sp(dst) rsi=block(src) */
        "\tmov %r9, %rdi\n\tmov %r10, %rsi\n\tmov $" "11" ", %eax\n\tsyscall\n"  /* munmap */
        "\txor %eax,%eax\n\txor %ebx,%ebx\n\txor %ecx,%ecx\n\txor %edx,%edx\n"
        "\txor %esi,%esi\n\txor %edi,%edi\n\txor %ebp,%ebp\n\txor %r9d,%r9d\n"
        "\txor %r10d,%r10d\n\txor %r11d,%r11d\n\txor %r12d,%r12d\n\txor %r13d,%r13d\n"
        "\txor %r14d,%r14d\n\txor %r15d,%r15d\n"
        "\tjmp *%r8\n");
__asm__(".globl _start\n.type _start,@function\n_start:\n"
        "\txor %rbp,%rbp\n\tmov %rsp,%rdi\n\tand $-16,%rsp\n\tcall loader_main\n\tud2\n");
#elif defined(__aarch64__)
__asm__(".text\n.globl wf_finish\n.type wf_finish,%function\nwf_finish:\n"
        "\tmov sp, x0\n"
        "\tmov x4, x0\n\tmov x5, x1\n\tlsr x6, x2, #3\n"
        "1:\tcbz x6, 2f\n\tldr x7, [x5], #8\n\tstr x7, [x4], #8\n\tsub x6, x6, #1\n\tb 1b\n"
        "2:\tmov x0, x1\n\tmov x1, x2\n\tmov x8, #215\n\tsvc #0\n"   /* munmap */
        "\tmov x0, #0\n\tmov x1, #0\n\tmov x2, #0\n\tmov x4, #0\n\tmov x5, #0\n"
        "\tmov x6, #0\n\tmov x7, #0\n\tmov x8, #0\n\tmov x29, #0\n\tmov x30, #0\n"
        "\tbr x3\n");
__asm__(".globl _start\n.type _start,%function\n_start:\n"
        "\tmov x29, #0\n\tmov x30, #0\n\tmov x0, sp\n\tbl loader_main\n\tbrk #0\n");
#endif

static int streq(const char *a, const char *b) { while (*a && *a == *b) { a++; b++; } return *a == *b; }

__attribute__((used, noreturn)) void loader_main(uintptr_t *sp0) {
    long argc = (long)sp0[0];
    char **argv = (char **)(sp0 + 1);
    char **envp = argv + argc + 1;
    long envc = 0;
    while (envp[envc]) envc++;
    uint64_t *auxv = (uint64_t *)(envp + envc + 1);
    long nauxv = 0;
    pgsz = 4096;
    for (uint64_t *a = auxv; a[0] != AT_NULL; a += 2, nauxv++)
        if (a[0] == AT_PAGESZ) pgsz = a[1];

    /* 1. load plan */
    long r = sc6(__NR_getpid, (long)ENG_MARK_A, (long)ENG_MARK_B, ENG_MARK_OP_QUERY,
                 (long)plan_buf, (long)sizeof plan_buf, 0);
    eng_load_plan *pl = (eng_load_plan *)plan_buf;
    if (pl->magic != ENG_PLAN_MAGIC || pl->version != ENG_PLAN_VERSION || pl->size > sizeof plan_buf ||
        !pl->exe_off || pl->exe_off >= pl->size)
        die("no load plan from the engine (not started by workflow-engine?)", 0, is_err(r) ? r : -L_EINVAL);
    const char *base = (const char *)plan_buf;
    const char *exe = base + pl->exe_off;
    const char *interp = pl->interp_off ? base + pl->interp_off : 0;
    const char *execfn = pl->execfn_off ? base + pl->execfn_off : exe;
    int force_anon = (pl->flags & ENG_PLAN_NO_FILEMAP) != 0;
    /* test hook: behave as on a kernel with a larger page size (anonymous
     * copies only: a file map would fault past EOF inside the fake page) */
    if ((pl->flags & ENG_PLAN_PAGESZ) && pl->pagesz > pgsz && (pl->pagesz & (pl->pagesz - 1)) == 0) {
        pgsz = pl->pagesz;
        force_anon = 1;
    }

    /* 2. map */
    img ex, in;
    map_image(exe, &ex, 0, force_anon);
    uintptr_t jump = ex.entry, at_base = 0;
    if (interp) { map_image(interp, &in, 1, force_anon); jump = in.entry; at_base = in.bias; }

    /* 3. scratch */
    long scratch = sc6(__NR_mmap, 0, ENG_SCRATCH_SIZE, L_PROT_READ | L_PROT_WRITE,
                       L_MAP_PRIVATE | L_MAP_ANONYMOUS | L_MAP_NORESERVE, -1, 0);
    if (is_err(scratch)) die("mmap scratch", 0, scratch);

    /* 4. build the new initial stack image in a temporary buffer */
    uint32_t skip = pl->argv_skip > (uint32_t)argc ? (uint32_t)argc : pl->argv_skip;
    long nargc = (long)pl->n_prepend + argc - (long)skip;
    size_t strbytes = l_strlen(execfn) + 1;
    const char *pp = pl->prepend_off ? base + pl->prepend_off : 0;
    {
        const char *q = pp;
        for (uint32_t i = 0; i < pl->n_prepend; i++) { size_t l = l_strlen(q) + 1; strbytes += l; q += l; }
    }
    size_t nwords = 1 + (size_t)nargc + 1 + (size_t)envc + 1 + 2 * ((size_t)nauxv + 1);
    size_t len = (nwords * 8 + strbytes + 15) & ~(size_t)15;
    uintptr_t final_sp = ((uintptr_t)sp0 - len) & ~(uintptr_t)15;
    long blk = sc6(__NR_mmap, 0, (long)len, L_PROT_READ | L_PROT_WRITE, L_MAP_PRIVATE | L_MAP_ANONYMOUS, -1, 0);
    if (is_err(blk)) die("mmap stack image", 0, blk);
    uint64_t *w = (uint64_t *)blk;
    char *sdst = (char *)blk + nwords * 8;               /* strings, in the buffer */
    uintptr_t sfinal = final_sp + nwords * 8;            /* same strings, final addr */

    *w++ = (uint64_t)nargc;
    const char *q = pp;
    for (uint32_t i = 0; i < pl->n_prepend; i++) {
        size_t l = l_strlen(q) + 1;
        l_memcpy(sdst, q, l);
        *w++ = sfinal; sdst += l; sfinal += l; q += l;
    }
    for (long i = skip; i < argc; i++) *w++ = (uint64_t)argv[i];
    *w++ = 0;
    for (long i = 0; i < envc; i++) *w++ = (uint64_t)envp[i];
    *w++ = 0;
    size_t efl = l_strlen(execfn) + 1;
    l_memcpy(sdst, execfn, efl);
    uintptr_t execfn_addr = sfinal;
    for (uint64_t *a = auxv; a[0] != AT_NULL; a += 2) {
        uint64_t v = a[1];
        switch (a[0]) {
            case AT_PHDR: v = ex.phdr; break;
            case AT_PAGESZ: v = pgsz; break;
            case AT_PHENT: v = ex.phent; break;
            case AT_PHNUM: v = ex.phnum; break;
            case AT_ENTRY: v = ex.entry; break;
            case AT_BASE: v = at_base; break;
            case AT_EXECFN: v = execfn_addr; break;
            case AT_SECURE: v = (pl->flags & ENG_PLAN_SECURE) ? 1 : 0; break;
            case AT_UID: v = pl->uid; break;
            case AT_EUID: v = pl->euid; break;
            case AT_GID: v = pl->gid; break;
            case AT_EGID: v = pl->egid; break;
            default: break;
        }
        *w++ = a[0];
        *w++ = v;
    }
    *w++ = AT_NULL;
    *w++ = 0;
    (void)streq;

    /* 5. the task name the kernel would have given the guest (ps, pkill) */
    if (pl->comm_off && pl->comm_off < pl->size)
        sc6(__NR_prctl, 15 /* PR_SET_NAME */, (long)(base + pl->comm_off), 0, 0, 0, 0);

    /* 6. hand over to the tracer's guest phase, then jump */
    sc6(__NR_getpid, (long)ENG_MARK_A, (long)ENG_MARK_B, ENG_MARK_OP_DONE, scratch, ENG_SCRATCH_SIZE, 0);
    wf_finish(final_sp, (const void *)blk, len, jump);
}
