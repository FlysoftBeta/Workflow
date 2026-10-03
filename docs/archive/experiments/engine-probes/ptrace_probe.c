/* Independent host capability probe. Not a guest loader, sandbox or engine. */
#define _GNU_SOURCE
#include <errno.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ptrace.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

static volatile sig_atomic_t delivered;
static void on_signal(int signal_number) { delivered = signal_number; }
static void die(const char *message) { perror(message); exit(2); }

int main(int argc, char **argv) {
    if (argc == 2 && !strcmp(argv[1], "--tracee-exec")) return 23;
    pid_t root = fork();
    if (root < 0) die("fork");
    if (!root) {
        struct sigaction action = {.sa_handler = on_signal};
        sigemptyset(&action.sa_mask);
        if (sigaction(SIGUSR1, &action, NULL) < 0) _exit(100);
        if (ptrace(PTRACE_TRACEME, 0, NULL, NULL) < 0) _exit(101);
        raise(SIGSTOP);
        raise(SIGUSR1);
        if (delivered != SIGUSR1) _exit(102);
        pid_t child = fork();
        if (child < 0) _exit(103);
        if (!child) {
            execl("/proc/self/exe", "ptrace-probe", "--tracee-exec", (char *)NULL);
            _exit(104);
        }
        int status;
        if (waitpid(child, &status, 0) != child || !WIFEXITED(status) || WEXITSTATUS(status) != 23) _exit(105);
        _exit(0);
    }
    int status;
    if (waitpid(root, &status, 0) != root || !WIFSTOPPED(status)) die("initial trace stop");
    long options = PTRACE_O_TRACESYSGOOD | PTRACE_O_TRACEFORK | PTRACE_O_TRACEEXEC | PTRACE_O_TRACEEXIT | PTRACE_O_EXITKILL;
    if (ptrace(PTRACE_SETOPTIONS, root, NULL, (void *)options) < 0) die("PTRACE_SETOPTIONS");
    if (ptrace(PTRACE_SYSCALL, root, NULL, NULL) < 0) die("initial PTRACE_SYSCALL");
    unsigned long syscall_stops = 0, syscall_info_stops = 0, forks = 0, execs = 0, user_signals = 0, exits = 0;
    int root_exit = -1, child_exit = -1, info_errno = 0;
    for (;;) {
        pid_t tid = waitpid(-1, &status, __WALL);
        if (tid < 0) {
            if (errno == EINTR) continue;
            if (errno == ECHILD) break;
            die("waitpid");
        }
        if (WIFEXITED(status) || WIFSIGNALED(status)) {
            int result = WIFEXITED(status) ? WEXITSTATUS(status) : 128 + WTERMSIG(status);
            if (tid == root) root_exit = result; else child_exit = result;
            ++exits;
            continue;
        }
        if (!WIFSTOPPED(status)) continue;
        int sig = WSTOPSIG(status), deliver = 0;
        unsigned int event = (unsigned int)status >> 16;
        if (event) {
            if (event == PTRACE_EVENT_FORK) ++forks;
            if (event == PTRACE_EVENT_EXEC) ++execs;
        } else if (sig == (SIGTRAP | 0x80)) {
            ++syscall_stops;
            /* This only detects support; it deliberately does not guess entry/exit
             * from a global toggle or change any syscall/registers. */
#ifdef PTRACE_GET_SYSCALL_INFO
            unsigned char info[128] = {0};
            long size = ptrace(PTRACE_GET_SYSCALL_INFO, tid, (void *)sizeof(info), info);
            if (size >= 0 && info[0] != 0) ++syscall_info_stops;
            else if (size < 0) info_errno = errno;
#else
            info_errno = ENOSYS;
#endif
        } else if (sig != SIGSTOP) {
            deliver = sig;
            if (sig == SIGUSR1) ++user_signals;
        }
        if (ptrace(PTRACE_SYSCALL, tid, NULL, (void *)(intptr_t)deliver) < 0 && errno != ESRCH) die("PTRACE_SYSCALL");
    }
    int passed = root_exit == 0 && child_exit == 23 && forks == 1 && execs == 1 && user_signals == 1 && exits == 2 && syscall_stops > 0;
    printf("{\"scope\":\"host-only\",\"passed\":%s,\"syscallStops\":%lu,\"syscallInfoStops\":%lu,\"syscallInfoErrno\":%d,\"forkEvents\":%lu,\"execEvents\":%lu,\"deliveredUserSignals\":%lu,\"exits\":%lu,\"rootExit\":%d,\"childExit\":%d}\n", passed ? "true" : "false", syscall_stops, syscall_info_stops, info_errno, forks, execs, user_signals, exits, root_exit, child_exit);
    return passed ? 0 : 1;
}
