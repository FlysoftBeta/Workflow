#ifndef WORKFLOW_PTY_PROCESS_H
#define WORKFLOW_PTY_PROCESS_H
#include <stddef.h>
#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct workflow_pty workflow_pty;
typedef struct { int stage; int error_number; } workflow_pty_error;
enum {
    WORKFLOW_PTY_OPEN = 1, WORKFLOW_PTY_FORK, WORKFLOW_PTY_SESSION,
    WORKFLOW_PTY_CTTY, WORKFLOW_PTY_STDIO, WORKFLOW_PTY_CWD,
    WORKFLOW_PTY_EXEC, WORKFLOW_PTY_HANDSHAKE
};

/* argv/envp must be complete, null-terminated arrays prepared BEFORE fork. */
int workflow_pty_spawn(const char *cwd, char *const argv[], char *const envp[],
                       unsigned short rows, unsigned short columns,
                       workflow_pty **result, workflow_pty_error *error);
pid_t workflow_pty_pid(workflow_pty *session);
/* 0 = EOF; -1/EAGAIN = timeout; -1/other errno = failure. */
ssize_t workflow_pty_read(workflow_pty *session, void *buffer, size_t length, int timeout_ms);
ssize_t workflow_pty_write(workflow_pty *session, const void *buffer, size_t length, int timeout_ms);
int workflow_pty_resize(workflow_pty *session, unsigned short rows, unsigned short columns);
/* Wait observes with WNOWAIT before reaping, so stop never signals a reused PID. */
int workflow_pty_wait(workflow_pty *session, int *exit_code);
int workflow_pty_stop(workflow_pty *session, int grace_ms);
/* Caller must have joined read/wait/write operations before destroying. */
void workflow_pty_destroy(workflow_pty *session);

#ifdef __cplusplus
}
#endif
#endif
