/* scratch.c — per-thread scratch in the tracee for translated arguments.
 * Slots come from the per-mm region the loader mapped (4 MiB NORESERVE,
 * 16 KiB per thread), so threads sharing an mm never overwrite each other's
 * pending path.  If no slot is free, fall back to the thread's own stack
 * below sp and the red zone (safe for the duration of one syscall). */
#define _GNU_SOURCE
#include <stdint.h>
#include <string.h>

#include "engine/loader_proto.h"
#include "engine/log.h"
#include "engine/mem.h"
#include "engine/tracer.h"

#if defined(__x86_64__)
#define RED_ZONE 128
#else
#define RED_ZONE 0
#endif

uint64_t eng_scratch_alloc(eng_task *t, const eng_regs *r, size_t n) {
    n = (n + 15) & ~(size_t)15;
    eng_mm *mm = t->mm;
    if (mm && mm->base && n <= ENG_SLOT_SIZE) {
        if (t->slot < 0) {
            for (uint32_t i = 0; i < mm->nslots; i++)
                if (!mm->used[i]) { mm->used[i] = 1; t->slot = (int)i; break; }
        }
        if (t->slot >= 0 && t->slot_off + n <= ENG_SLOT_SIZE) {
            uint64_t a = mm->base + (uint64_t)t->slot * ENG_SLOT_SIZE + t->slot_off;
            t->slot_off += (uint32_t)n;
            return a;
        }
    }
    if (!t->stack_scratch) t->stack_scratch = (eng_sp(r) - RED_ZONE - 64) & ~(uint64_t)15;
    t->stack_scratch = (t->stack_scratch - n) & ~(uint64_t)15;
    static int warned;
    if (!warned++) ENG_DBG("scratch: using stack fallback for tid %d", t->tid);
    return t->stack_scratch;
}

uint64_t eng_scratch_put_str(eng_task *t, const eng_regs *r, const char *s) {
    size_t n = strlen(s) + 1;
    uint64_t a = eng_scratch_alloc(t, r, n);
    if (!a) return 0;
    if (eng_mem_write(t->tid, a, s, n) != (ssize_t)n) return 0;
    return a;
}

void eng_scratch_release(eng_task *t) {
    if (t->mm && t->slot >= 0 && (uint32_t)t->slot < t->mm->nslots) t->mm->used[t->slot] = 0;
    t->slot = -1;
    t->slot_off = 0;
}
