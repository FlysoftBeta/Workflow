/* mem.h — tracee memory access.
 * process_vm_readv/writev with a PTRACE_PEEKDATA/POKEDATA fallback (both
 * measured working across the matrix).  Includes bounded C-string reads. */
#ifndef WORKFLOW_ENGINE_MEM_H
#define WORKFLOW_ENGINE_MEM_H

#include <stddef.h>
#include <stdint.h>
#include <sys/types.h>

/* Raw block copy.  Return bytes transferred, or -1 (errno). */
ssize_t eng_mem_read(pid_t tid, uintptr_t addr, void *buf, size_t n);
ssize_t eng_mem_write(pid_t tid, uintptr_t addr, const void *buf, size_t n);

/* Read a NUL-terminated string from the tracee into buf (always NUL-
 * terminated on success).  Returns string length (excluding NUL), or -1.
 * Stops at cap-1 bytes. */
ssize_t eng_mem_read_cstr(pid_t tid, uintptr_t addr, char *buf, size_t cap);

#endif
