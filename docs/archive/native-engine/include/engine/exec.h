/* exec.h — guest execve/execveat planning (M2 core, M4 extends). */
#ifndef WORKFLOW_ENGINE_EXEC_H
#define WORKFLOW_ENGINE_EXEC_H

#include "engine/tracer.h"

/* Build the load plan for an exec by task `t`.  `path` is the guest string
 * passed to execve (already read from the tracee).  at_flags: AT_EMPTY_PATH /
 * AT_SYMLINK_NOFOLLOW from execveat.  On success stores the plan in t->plan
 * and returns 0; otherwise returns -errno exactly as execve would report. */
int eng_exec_prepare(eng_task *t, int dirfd, const char *path, int at_flags);

/* Build a plan for the initial command (no tracee memory involved). */
int eng_exec_prepare_initial(eng_task *t, const char *path);

void eng_exec_discard(eng_task *t);

#endif
