/* ident.h — virtual credentials (M4).  The guest sees uid/gid/groups and
 * capability sets kept by the engine; the real app UID never changes.  The
 * kernel's rules are emulated: setuid-family permission checks and the
 * capability transitions of commoncap (setxuid, setfsuid, exec), KEEPCAPS,
 * the bounding and ambient sets, no_new_privs for set-id execs. */
#ifndef WORKFLOW_ENGINE_IDENT_H
#define WORKFLOW_ENGINE_IDENT_H

#include "engine/tracer.h"

struct eng_guest;

#define ENG_CAP_CHOWN            0
#define ENG_CAP_DAC_OVERRIDE     1
#define ENG_CAP_DAC_READ_SEARCH  2
#define ENG_CAP_FOWNER           3
#define ENG_CAP_FSETID           4
#define ENG_CAP_KILL             5
#define ENG_CAP_SETGID           6
#define ENG_CAP_SETUID           7
#define ENG_CAP_SETPCAP          8
#define ENG_CAP_MKNOD           27
#define ENG_CAP_LAST            40      /* CAP_CHECKPOINT_RESTORE */
#define ENG_CAP_FULL  ((1ull << (ENG_CAP_LAST + 1)) - 1)

static inline int eng_capable(const eng_task *t, int cap) { return (int)((t->cr.cap_eff >> cap) & 1); }

/* Identity syscalls (uid/gid/groups/caps, credential prctls, umask).
 * Returns 1/0 (handled, regs-changed flag as for eng_sys_entry) or -1 if the
 * syscall is not an identity call. */
int eng_ident_entry(eng_task *t, eng_regs *r);
/* Exit-side part (umask result).  Returns 1 if regs changed. */
int eng_ident_exit(eng_task *t, eng_regs *r);

/* Initial credentials: uid/gid plus supplementary groups looked up in the
 * guest's /etc/passwd and /etc/group (like initgroups); full capabilities for
 * uid 0; umask 022. */
void eng_ident_init(struct eng_guest *g, eng_task *t, uint32_t uid, uint32_t gid);

/* Apply exec-time credential transitions (set-id, saved ids, capabilities). */
void eng_ident_exec(eng_task *t);

/* Effective ids a successful exec of an object with virtual `mode`/owner
 * would produce (no_new_privs honoured).  Returns 1 if set-id applies. */
int eng_ident_setid(const eng_task *t, uint32_t mode, uint32_t uid, uint32_t gid, uint32_t *neuid, uint32_t *negid);

#endif
