#define _GNU_SOURCE
#include "pty_process.h"
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <pthread.h>
#include <signal.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/prctl.h>
#include <sys/resource.h>
#include <sys/wait.h>
#include <termios.h>
#include <time.h>
#include <unistd.h>

struct workflow_pty {
    pthread_mutex_t lock;
    pthread_cond_t exited_condition;
    int master;
    pid_t pid;
    int waiting;
    int exited;
    int wait_done;
    int exit_code;
};

static int64_t monotonic_ms(void) {
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return (int64_t)t.tv_sec * 1000 + t.tv_nsec / 1000000;
}

static int duplicate_master(workflow_pty *s, int require_running) {
    pthread_mutex_lock(&s->lock);
    int fd = s->master < 0 || (require_running && s->exited)
        ? -1 : fcntl(s->master, F_DUPFD_CLOEXEC, 3);
    if (fd < 0 && (s->master < 0 || (require_running && s->exited))) errno = EPIPE;
    pthread_mutex_unlock(&s->lock);
    return fd;
}

/* Called only in the fork child: write + _exit are async-signal-safe. */
static void child_failure(int error_fd, int stage) {
    workflow_pty_error message = {stage, errno};
    const char *cursor = (const char *)&message;
    size_t left = sizeof(message);
    while (left) {
        ssize_t count = write(error_fd, cursor, left);
        if (count < 0 && errno == EINTR) continue;
        if (count <= 0) break;
        cursor += count;
        left -= (size_t)count;
    }
    _exit(127);
}

static int ensure_nonstdio(int fd) {
    if (fd < 0 || fd >= 3) return fd;
    int moved = fcntl(fd, F_DUPFD_CLOEXEC, 3);
    int saved = errno;
    close(fd);
    errno = saved;
    return moved;
}

int workflow_pty_spawn(const char *cwd, char *const argv[], char *const envp[],
                       unsigned short rows, unsigned short columns,
                       workflow_pty **result, workflow_pty_error *error) {
    *result = NULL;
    error->stage = WORKFLOW_PTY_OPEN;
    error->error_number = 0;
    if (!cwd || !argv || !argv[0] || !envp || !rows || !columns) { errno = EINVAL; goto early_error; }
    int master = ensure_nonstdio(posix_openpt(O_RDWR | O_NOCTTY | O_CLOEXEC | O_NONBLOCK));
    if (master < 0) goto early_error;
    if (grantpt(master) < 0 || unlockpt(master) < 0) goto master_error;
    char slave_name[128];
    int pts_error = ptsname_r(master, slave_name, sizeof(slave_name));
    if (pts_error) { errno = pts_error; goto master_error; }
    int slave = ensure_nonstdio(open(slave_name, O_RDWR | O_NOCTTY | O_CLOEXEC));
    if (slave < 0) goto master_error;
    struct winsize size = {.ws_row = rows, .ws_col = columns, .ws_xpixel = 0, .ws_ypixel = 0};
    if (ioctl(slave, TIOCSWINSZ, &size) < 0) goto slave_error;
    struct termios attributes;
    if (tcgetattr(slave, &attributes) < 0) goto slave_error;
#ifdef IUTF8
    attributes.c_iflag |= IUTF8;
#endif
    if (tcsetattr(slave, TCSANOW, &attributes) < 0) goto slave_error;
    int errors[2];
    if (pipe2(errors, O_CLOEXEC) < 0) goto slave_error;
    errors[0] = ensure_nonstdio(errors[0]);
    errors[1] = ensure_nonstdio(errors[1]);
    if (errors[0] < 0 || errors[1] < 0) goto pipe_error;

    workflow_pty *s = calloc(1, sizeof(*s));
    if (!s) goto pipe_error;
    int rc = pthread_mutex_init(&s->lock, NULL);
    if (rc) { free(s); errno = rc; goto pipe_error; }
    rc = pthread_cond_init(&s->exited_condition, NULL);
    if (rc) { pthread_mutex_destroy(&s->lock); free(s); errno = rc; goto pipe_error; }
    s->master = master;
    s->exit_code = -1;

    /* Precompute everything requiring libc allocation or Java interaction. */
    struct rlimit limit;
    unsigned long max_fd = getrlimit(RLIMIT_NOFILE, &limit) == 0 ? limit.rlim_cur : 65536;
    if (max_fd == RLIM_INFINITY || max_fd > 1048576) max_fd = 1048576;
    pid_t expected_parent = getpid();
    sigset_t signals;
    sigemptyset(&signals);
    struct sigaction default_signal;
    memset(&default_signal, 0, sizeof(default_signal));
    default_signal.sa_handler = SIG_DFL;
    sigemptyset(&default_signal.sa_mask);

    error->stage = WORKFLOW_PTY_FORK;
    pid_t child = fork();
    if (child == 0) {
        /* No allocation, JNI, setenv, stdio, or logging in this branch. */
        close(errors[0]);
        close(master);
        if (prctl(PR_SET_PDEATHSIG, SIGKILL) < 0) child_failure(errors[1], WORKFLOW_PTY_SESSION);
        if (getppid() != expected_parent) _exit(127);
        if (sigprocmask(SIG_SETMASK, &signals, NULL) < 0) child_failure(errors[1], WORKFLOW_PTY_SESSION);
        for (int signal_number = 1; signal_number < NSIG; ++signal_number) {
            if (signal_number != SIGKILL && signal_number != SIGSTOP) sigaction(signal_number, &default_signal, NULL);
        }
        if (setsid() < 0) child_failure(errors[1], WORKFLOW_PTY_SESSION);
        if (ioctl(slave, TIOCSCTTY, 0) < 0) child_failure(errors[1], WORKFLOW_PTY_CTTY);
        if (dup2(slave, STDIN_FILENO) < 0 || dup2(slave, STDOUT_FILENO) < 0 || dup2(slave, STDERR_FILENO) < 0)
            child_failure(errors[1], WORKFLOW_PTY_STDIO);
        if (chdir(cwd) < 0) child_failure(errors[1], WORKFLOW_PTY_CWD);
        for (unsigned long fd = 3; fd < max_fd; ++fd) {
            if ((int)fd != errors[1]) close((int)fd);
        }
        execve(argv[0], argv, envp);
        child_failure(errors[1], WORKFLOW_PTY_EXEC);
    }
    int fork_errno = errno;
    close(slave); slave = -1;
    close(errors[1]); errors[1] = -1;
    if (child < 0) {
        pthread_cond_destroy(&s->exited_condition); pthread_mutex_destroy(&s->lock); free(s);
        errno = fork_errno; goto pipe_error;
    }
    s->pid = child;
    error->stage = WORKFLOW_PTY_HANDSHAKE;
    workflow_pty_error failure;
    size_t received = 0;
    const int64_t deadline = monotonic_ms() + 5000;
    int successful_exec = 0;
    while (monotonic_ms() < deadline) {
        int64_t remaining = deadline - monotonic_ms();
        if (remaining <= 0) { errno = ETIMEDOUT; break; }
        struct pollfd pending = {.fd = errors[0], .events = POLLIN};
        int polled = poll(&pending, 1, (int)remaining);
        if (polled < 0 && errno == EINTR) continue;
        if (polled <= 0) { if (polled == 0) errno = ETIMEDOUT; break; }
        ssize_t count = read(errors[0], ((char *)&failure) + received, sizeof(failure) - received);
        if (count < 0 && errno == EINTR) continue;
        if (count == 0) { successful_exec = received == 0; if (!successful_exec) errno = EIO; break; }
        if (count < 0) break;
        received += (size_t)count;
        if (received == sizeof(failure)) { *error = failure; errno = failure.error_number; break; }
    }
    close(errors[0]); errors[0] = -1;
    if (!successful_exec) {
        int saved = errno ? errno : ETIMEDOUT;
        kill(child, SIGKILL);
        while (waitpid(child, NULL, 0) < 0 && errno == EINTR) {}
        close(master);
        pthread_cond_destroy(&s->exited_condition); pthread_mutex_destroy(&s->lock); free(s);
        error->error_number = saved; errno = saved;
        return -1;
    }
    *result = s;
    return 0;

pipe_error: {
    int saved = errno;
    if (errors[0] >= 0) close(errors[0]);
    if (errors[1] >= 0) close(errors[1]);
    errno = saved;
}
slave_error: {
    int saved = errno;
    if (slave >= 0) close(slave);
    errno = saved;
}
master_error: {
    int saved = errno;
    close(master);
    errno = saved;
}
early_error:
    error->error_number = errno;
    return -1;
}

pid_t workflow_pty_pid(workflow_pty *s) { return s->pid; }

ssize_t workflow_pty_read(workflow_pty *s, void *buffer, size_t length, int timeout_ms) {
    int fd = duplicate_master(s, 0);
    if (fd < 0) return -1;
    struct pollfd pending = {.fd = fd, .events = POLLIN};
    int rc;
    do { rc = poll(&pending, 1, timeout_ms); } while (rc < 0 && errno == EINTR);
    ssize_t count = -1;
    if (rc > 0) {
        do { count = read(fd, buffer, length); } while (count < 0 && errno == EINTR);
        if (count < 0 && errno == EIO) count = 0; /* PTY slave closed. */
    } else if (rc == 0) errno = EAGAIN;
    int saved = errno;
    close(fd);
    errno = saved;
    return count;
}

ssize_t workflow_pty_write(workflow_pty *s, const void *buffer, size_t length, int timeout_ms) {
    int fd = duplicate_master(s, 1);
    if (fd < 0) return -1;
    size_t written = 0;
    const int64_t deadline = monotonic_ms() + timeout_ms;
    while (written < length) {
        ssize_t count = write(fd, ((const char *)buffer) + written, length - written);
        if (count > 0) { written += (size_t)count; continue; }
        if (count < 0 && errno == EINTR) continue;
        if (count < 0 && errno != EAGAIN && errno != EWOULDBLOCK) break;
        int64_t remaining = deadline - monotonic_ms();
        if (remaining <= 0) { errno = ETIMEDOUT; break; }
        struct pollfd pending = {.fd = fd, .events = POLLOUT};
        int rc = poll(&pending, 1, (int)remaining);
        if (rc < 0 && errno == EINTR) continue;
        if (rc <= 0) { if (rc == 0) errno = ETIMEDOUT; break; }
        if (pending.revents & (POLLERR | POLLHUP | POLLNVAL)) { errno = EPIPE; break; }
    }
    int saved = errno;
    close(fd);
    errno = saved;
    return written ? (ssize_t)written : (length ? -1 : 0);
}

int workflow_pty_resize(workflow_pty *s, unsigned short rows, unsigned short columns) {
    if (!rows || !columns) { errno = EINVAL; return -1; }
    int fd = duplicate_master(s, 1);
    if (fd < 0) return -1;
    struct winsize size = {.ws_row = rows, .ws_col = columns, .ws_xpixel = 0, .ws_ypixel = 0};
    int rc = ioctl(fd, TIOCSWINSZ, &size);
    int saved = errno;
    close(fd);
    errno = saved;
    return rc;
}

/* Must hold s->lock. The child remains unreaped here, so pid/pgid cannot be reused. */
static void signal_groups(workflow_pty *s, int signal_number) {
    pid_t foreground = s->master >= 0 ? tcgetpgrp(s->master) : -1;
    if (foreground > 1 && foreground != s->pid && foreground != getpgrp() && getsid(foreground) == s->pid) {
        kill(-foreground, SIGCONT);
        kill(-foreground, signal_number);
    }
    kill(-s->pid, SIGCONT);
    kill(-s->pid, signal_number);
    kill(s->pid, signal_number);
}

int workflow_pty_wait(workflow_pty *s, int *exit_code) {
    pthread_mutex_lock(&s->lock);
    if (s->waiting) {
        while (!s->wait_done) pthread_cond_wait(&s->exited_condition, &s->lock);
        *exit_code = s->exit_code;
        pthread_mutex_unlock(&s->lock);
        return 0;
    }
    s->waiting = 1;
    pthread_mutex_unlock(&s->lock);
    siginfo_t info;
    int rc;
    do { memset(&info, 0, sizeof(info)); rc = waitid(P_PID, (id_t)s->pid, &info, WEXITED | WNOWAIT); }
    while (rc < 0 && errno == EINTR);
    int wait_error = errno;
    pthread_mutex_lock(&s->lock);
    if (rc == 0) signal_groups(s, SIGHUP);
    s->exited = 1;
    pthread_mutex_unlock(&s->lock);
    int status = 0;
    if (rc == 0) {
        pid_t reaped;
        do { reaped = waitpid(s->pid, &status, 0); } while (reaped < 0 && errno == EINTR);
        if (reaped < 0) { rc = -1; wait_error = errno; }
    }
    pthread_mutex_lock(&s->lock);
    s->exit_code = rc < 0 ? 255 : WIFEXITED(status) ? WEXITSTATUS(status) : WIFSIGNALED(status) ? 128 + WTERMSIG(status) : 255;
    s->wait_done = 1;
    *exit_code = s->exit_code;
    pthread_cond_broadcast(&s->exited_condition);
    pthread_mutex_unlock(&s->lock);
    if (rc < 0) errno = wait_error;
    return rc;
}

int workflow_pty_stop(workflow_pty *s, int grace_ms) {
    pthread_mutex_lock(&s->lock);
    if (!s->exited) signal_groups(s, SIGHUP);
    pthread_mutex_unlock(&s->lock);
    const int64_t deadline = monotonic_ms() + grace_ms;
    while (monotonic_ms() < deadline) {
        pthread_mutex_lock(&s->lock);
        int exited = s->exited;
        pthread_mutex_unlock(&s->lock);
        if (exited) return 0;
        struct timespec pause = {.tv_sec = 0, .tv_nsec = 20000000};
        nanosleep(&pause, NULL);
    }
    pthread_mutex_lock(&s->lock);
    if (!s->exited) signal_groups(s, SIGKILL);
    pthread_mutex_unlock(&s->lock);
    return 0;
}

void workflow_pty_destroy(workflow_pty *s) {
    if (!s) return;
    if (s->master >= 0) close(s->master);
    pthread_cond_destroy(&s->exited_condition);
    pthread_mutex_destroy(&s->lock);
    free(s);
}
