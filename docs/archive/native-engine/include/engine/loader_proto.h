/* loader_proto.h — protocol between the tracer (libworkflow-engine.so) and the
 * freestanding loader (libworkflow-loader.so).  Shared, dependency-free.
 *
 * Every guest execve is rewritten by the tracer into a real kernel execve of
 * the loader (which lives in nativeLibraryDir, so exec is permitted on API 29+)
 * with the guest's argv/envp unchanged.  The kernel therefore performs the
 * genuine exec semantics (new mm, CLOEXEC, thread-group teardown, signal
 * reset).  The loader then asks the tracer what to load:
 *
 *   getpid(MAGIC_A, MAGIC_B, OP_QUERY, buf, bufsize)
 *       tracer writes an eng_load_plan into buf; result = plan length or -errno
 *   getpid(MAGIC_A, MAGIC_B, OP_DONE, scratch, scratch_size)
 *       loader finished mapping; tracer records the scratch region and switches
 *       the task to guest phase; result = 0
 *
 * getpid ignores its arguments, so if no tracer intercepts (e.g. loader run by
 * hand) the markers are harmless and the loader reports "no plan".  The tracer
 * voids markers by rewriting them to getpid anyway (see arch: void policy).
 */
#ifndef WORKFLOW_ENGINE_LOADER_PROTO_H
#define WORKFLOW_ENGINE_LOADER_PROTO_H

#include <stdint.h>

#define ENG_MARK_A 0x574f524b464c4f57ull   /* "WOWKFLOW" */
#define ENG_MARK_B 0x4c4f414445523031ull   /* "LOADER01" */

#define ENG_MARK_OP_QUERY 1u
#define ENG_MARK_OP_DONE  2u

#define ENG_PLAN_MAGIC   0x4e414c50u        /* "PLAN" */
#define ENG_PLAN_VERSION 2u
#define ENG_PLAN_MAX     (64u * 1024u)
#define ENG_SCRATCH_SIZE (4u * 1024u * 1024u)   /* virtual, MAP_NORESERVE */
#define ENG_SLOT_SIZE    (16u * 1024u)          /* one per thread */

/* Plan flags */
#define ENG_PLAN_SECURE      0x1u   /* AT_SECURE=1 (virtual set-id exec) */
#define ENG_PLAN_NO_FILEMAP  0x2u   /* force anonymous-copy segment mapping */
#define ENG_PLAN_PAGESZ      0x4u   /* test hook: use `pagesz` instead of AT_PAGESZ */

/* All *_off fields are byte offsets from the start of the plan to NUL-
 * terminated strings inside the plan; 0 means absent. */
typedef struct eng_load_plan {
    uint32_t magic;
    uint32_t version;
    uint32_t size;          /* total bytes including strings */
    uint32_t flags;
    uint32_t exe_off;       /* host path of the ELF to map */
    uint32_t interp_off;    /* host path of PT_INTERP, 0 = static */
    uint32_t execfn_off;    /* guest path for AT_EXECFN */
    uint32_t argv_skip;     /* drop this many original argv entries */
    uint32_t n_prepend;     /* prepend this many strings (consecutive) */
    uint32_t prepend_off;   /* first prepend string; the rest follow NUL-separated */
    uint32_t uid, euid, gid, egid;   /* virtual credentials for auxv */
    uint32_t pagesz;        /* with ENG_PLAN_PAGESZ */
    uint32_t comm_off;      /* task name to set (basename of the executed path), 0 = keep */
    uint32_t reserved[2];
} eng_load_plan;

#endif
