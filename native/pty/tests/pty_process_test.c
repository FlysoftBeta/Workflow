#define _GNU_SOURCE
#include "pty_process.h"
#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <pthread.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/prctl.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

typedef struct { workflow_pty *pty; int code; int result; } waiter;
static void *wait_main(void *argument) {
    waiter *w = argument;
    w->result = workflow_pty_wait(w->pty, &w->code);
    return NULL;
}
static void drain(workflow_pty *pty, char *output, size_t capacity) {
    size_t used = strlen(output);
    for (int tries = 0; tries < 100; ++tries) {
        ssize_t count = workflow_pty_read(pty, output + used, capacity - used - 1, 100);
        if (count == 0) { output[used] = 0; return; }
        if (count < 0) { assert(errno == EAGAIN || errno == EWOULDBLOCK); continue; }
        used += (size_t)count;
        output[used] = 0;
        assert(used < capacity - 1);
    }
    fprintf(stderr, "PTY did not reach EOF; output: %s\n", output);
    abort();
}
static void wait_marker(workflow_pty *pty, char *output, size_t capacity, const char *marker) {
    size_t used = 0;
    output[0] = 0;
    for (int tries = 0; tries < 50 && !strstr(output, marker); ++tries) {
        ssize_t count = workflow_pty_read(pty, output + used, capacity - used - 1, 100);
        if (count < 0) { assert(errno == EAGAIN || errno == EWOULDBLOCK); continue; }
        assert(count > 0);
        used += (size_t)count;
        output[used] = 0;
    }
    assert(strstr(output, marker));
}
static workflow_pty *start(const char *cwd, const char *script) {
    char *arguments[] = {"/bin/sh", "-c", (char *)script, NULL};
    char *environment[] = {"PATH=/usr/bin:/bin", "TERM=xterm-256color", "TEST_VALUE=shared-root", NULL};
    workflow_pty *pty = NULL;
    workflow_pty_error error;
    if (workflow_pty_spawn(cwd, arguments, environment, 24, 80, &pty, &error) < 0) {
        fprintf(stderr, "spawn stage=%d errno=%d\n", error.stage, error.error_number);
        abort();
    }
    return pty;
}
static int finish(workflow_pty *pty, char *output, size_t capacity) {
    waiter wait = {.pty = pty};
    pthread_t thread;
    assert(pthread_create(&thread, NULL, wait_main, &wait) == 0);
    drain(pty, output, capacity);
    assert(pthread_join(thread, NULL) == 0);
    assert(wait.result == 0);
    workflow_pty_destroy(pty);
    return wait.code;
}

int main(void) {
    assert(prctl(PR_SET_CHILD_SUBREAPER, 1) == 0);
    char workspace[] = "/tmp/workflow-pty-test-XXXXXX";
    assert(mkdtemp(workspace));
    char output[32768] = "";
    workflow_pty *pty = start(workspace,
        "test -t 0 && test -t 1 && test -t 2 && printf 'TTY_OK\\n'; "
        "pwd; printf '%s\\n' \"$TEST_VALUE\"; printf from-terminal > shared.txt; "
        "printf '\\360\\237\\214\\237\\344\\270\\255\\346\\226\\207\\n'");
    assert(finish(pty, output, sizeof(output)) == 0);
    assert(strstr(output, "TTY_OK"));
    assert(strstr(output, workspace));
    assert(strstr(output, "shared-root"));
    assert(strstr(output, "🌟中文"));
    char filename[1024];
    snprintf(filename, sizeof(filename), "%s/shared.txt", workspace);
    FILE *shared = fopen(filename, "rb");
    assert(shared);
    char content[32] = {0};
    assert(fread(content, 1, sizeof(content) - 1, shared) == strlen("from-terminal"));
    fclose(shared);
    assert(strcmp(content, "from-terminal") == 0);
    puts("PASS: actual PTY, shared cwd/file write, env, Unicode bytes");

    pty = start(workspace, "printf 'READY\\n'; read value; printf 'read=%s\\n' \"$value\"; stty size; exit 7");
    wait_marker(pty, output, sizeof(output), "READY");
    assert(workflow_pty_resize(pty, 37, 111) == 0);
    assert(workflow_pty_write(pty, "hello\n", 6, 2000) == 6);
    assert(finish(pty, output, sizeof(output)) == 7);
    assert(strstr(output, "read=hello"));
    assert(strstr(output, "37 111"));
    puts("PASS: input, resize visible to stty, real exit status");

    pty = start(workspace, "trap 'printf INTERRUPTED; exit 23' INT; printf 'READY\\n'; sleep 30");
    wait_marker(pty, output, sizeof(output), "READY");
    const char ctrl_c = 3;
    assert(workflow_pty_write(pty, &ctrl_c, 1, 2000) == 1);
    assert(finish(pty, output, sizeof(output)) == 23);
    assert(strstr(output, "INTERRUPTED"));
    puts("PASS: terminal Ctrl-C delivers SIGINT to actual foreground job");

    pty = start(workspace, "trap '' HUP; printf 'READY\\n'; while :; do sleep 1; done");
    wait_marker(pty, output, sizeof(output), "READY");
    waiter stopped = {.pty = pty};
    pthread_t wait_thread;
    assert(pthread_create(&wait_thread, NULL, wait_main, &stopped) == 0);
    assert(workflow_pty_stop(pty, 100) == 0);
    assert(pthread_join(wait_thread, NULL) == 0);
    assert(stopped.result == 0 && stopped.code == 137);
    pid_t pid = workflow_pty_pid(pty);
    assert(kill(pid, 0) < 0 && errno == ESRCH);
    workflow_pty_destroy(pty);
    puts("PASS: HUP grace timeout escalates, shell is reaped");

    char *interactive_argv[] = {"/bin/sh", "-i", NULL};
    char *interactive_environment[] = {"PATH=/usr/bin:/bin", "TERM=xterm-256color", "PS1=ready$ ", NULL};
    workflow_pty_error interactive_error;
    assert(workflow_pty_spawn(workspace, interactive_argv, interactive_environment, 24, 80, &pty, &interactive_error) == 0);
    wait_marker(pty, output, sizeof(output), "ready$ ");
    const char *foreground_command = "/bin/sh -c 'echo $$ > foreground.pid; exec sleep 30'\n";
    assert(workflow_pty_write(pty, foreground_command, strlen(foreground_command), 2000) == (ssize_t)strlen(foreground_command));
    char foreground_file[1024];
    snprintf(foreground_file, sizeof(foreground_file), "%s/foreground.pid", workspace);
    pid_t foreground_pid = -1;
    for (int retry = 0; retry < 100 && foreground_pid < 1; ++retry) {
        FILE *pid_file = fopen(foreground_file, "r");
        if (pid_file) { if (fscanf(pid_file, "%d", &foreground_pid) != 1) foreground_pid = -1; fclose(pid_file); }
        if (foreground_pid < 1) usleep(20000);
    }
    assert(foreground_pid > 1);
    assert(getpgid(foreground_pid) != workflow_pty_pid(pty));
    assert(getsid(foreground_pid) == workflow_pty_pid(pty));
    waiter interactive_wait = {.pty = pty};
    assert(pthread_create(&wait_thread, NULL, wait_main, &interactive_wait) == 0);
    assert(workflow_pty_stop(pty, 300) == 0);
    assert(pthread_join(wait_thread, NULL) == 0);
    assert(interactive_wait.result == 0);
    int foreground_reaped = 0;
    for (int retry = 0; retry < 100 && !foreground_reaped; ++retry) {
        int foreground_status;
        pid_t child_result = waitpid(foreground_pid, &foreground_status, WNOHANG);
        if (child_result == foreground_pid) { assert(WIFSIGNALED(foreground_status)); foreground_reaped = 1; }
        else if (child_result < 0 && errno == ECHILD) { assert(kill(foreground_pid, 0) < 0 && errno == ESRCH); foreground_reaped = 1; }
        else usleep(20000);
    }
    assert(foreground_reaped);
    workflow_pty_destroy(pty);
    unlink(foreground_file);
    puts("PASS: interactive shell foreground job has a distinct PGID and is stopped/reaped");

    char *bad_argv[] = {"/definitely/not/a/program", NULL};
    char *environment[] = {"PATH=/usr/bin:/bin", NULL};
    workflow_pty_error error;
    assert(workflow_pty_spawn(workspace, bad_argv, environment, 24, 80, &pty, &error) < 0);
    assert(pty == NULL && error.stage == WORKFLOW_PTY_EXEC && error.error_number == ENOENT);
    char *good_argv[] = {"/bin/sh", "-c", "exit 0", NULL};
    assert(workflow_pty_spawn("/definitely/missing/cwd", good_argv, environment, 24, 80, &pty, &error) < 0);
    assert(pty == NULL && error.stage == WORKFLOW_PTY_CWD && error.error_number == ENOENT);
    puts("PASS: exec/cwd failures returned synchronously with real errno");

    int inherited[2];
    assert(pipe(inherited) == 0);
    char command[256];
    snprintf(command, sizeof(command), "test ! -e /proc/self/fd/%d && test ! -e /proc/self/fd/%d", inherited[0], inherited[1]);
    pty = start(workspace, command);
    output[0] = 0;
    assert(finish(pty, output, sizeof(output)) == 0);
    close(inherited[0]); close(inherited[1]);
    puts("PASS: non-CLOEXEC parent descriptors do not leak into shell");
    unlink(filename);
    rmdir(workspace);
    return 0;
}
