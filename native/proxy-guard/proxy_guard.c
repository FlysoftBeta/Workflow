#define _GNU_SOURCE
#include <ctype.h>
#include <errno.h>
#include <fcntl.h>
#include <inttypes.h>
#include <limits.h>
#include <linux/fib_rules.h>
#include <linux/netlink.h>
#include <linux/rtnetlink.h>
#include <net/if.h>
#include <sys/socket.h>
#include <poll.h>
#include <signal.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/prctl.h>
#include <sys/resource.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

/* This opt-in exists only for unprivileged Linux host tests. Android may never
 * compile a guardian which permits a non-root child. */
#if defined(__ANDROID__) && defined(WORKFLOW_PROXY_GUARD_HOST_TEST)
#error WORKFLOW_PROXY_GUARD_HOST_TEST must never be enabled for Android
#endif

#define STOP_GRACE_MS 2000
#define EXEC_TIMEOUT_MS 5000
#define CONTROL_CAPACITY 256

extern char **environ;
static volatile sig_atomic_t requested_signal;

static void on_signal(int value) { requested_signal = value; }

static int64_t now_ms(void) {
    struct timespec value;
    if (clock_gettime(CLOCK_MONOTONIC, &value) < 0) return 0;
    return (int64_t)value.tv_sec * 1000 + value.tv_nsec / 1000000;
}

static bool write_all(int fd, const void *data, size_t size) {
    const char *bytes = data;
    while (size) {
        ssize_t count = write(fd, bytes, size);
        if (count < 0 && errno == EINTR) continue;
        if (count <= 0) return false;
        bytes += count;
        size -= (size_t)count;
    }
    return true;
}

/* All error descriptions are fixed internal strings; no path, config, or
 * secret from the caller is reflected onto the control channel. */
static bool emit_error(const char *message, bool fatal) {
    char line[512];
    int size = snprintf(line, sizeof(line),
        "{\"event\":\"error\",\"message\":\"%s\",\"fatal\":%s}\n",
        message, fatal ? "true" : "false");
    return size > 0 && (size_t)size < sizeof(line) &&
        write_all(STDOUT_FILENO, line, (size_t)size);
}

/* TUN policy-rule cleanup is explicitly scoped by the validated launch arguments. The
 * guardian refuses any existing rule in that scope, then deletes exact matching
 * rule records after its owned child has been reaped. No shell, flush, guessed PID,
 * Android/netd table or other interface is ever a deletion target. Routes disappear
 * with the non-persistent TUN when the final child descriptor closes. */
#define RULE_LIMIT 128
#define RULE_BYTES 1024
static struct {
    bool enabled;
    uint32_t table, priority;
    char device[IF_NAMESIZE];
} cleanup;
typedef struct { size_t count; unsigned char rules[RULE_LIMIT][RULE_BYTES]; } rule_list;

static bool uint_arg(const char *text, uint32_t *value) {
    if (!text[0]) return false;
    for (const char *p = text; *p; ++p) if (!isdigit((unsigned char)*p)) return false;
    char *end;
    errno = 0;
    unsigned long n = strtoul(text, &end, 10);
    if (errno || *end || n > UINT32_MAX || n == 0) return false;
    *value = (uint32_t)n;
    return true;
}

/* Return 0 outside the reserved scope, 1 for an exact Mihomo rule in scope,
 * 2 for an unfamiliar rule in scope (never delete), and -1 for malformed data. */
static int scoped_rule(const struct nlmsghdr *header) {
    if (header->nlmsg_len < NLMSG_LENGTH(sizeof(struct fib_rule_hdr))) return -1;
    const struct fib_rule_hdr *rule = NLMSG_DATA(header);
    uint32_t priority = 0, table = rule->table, target = 0;
    bool input_ok = true;
    int length = (int)header->nlmsg_len - (int)NLMSG_LENGTH(sizeof(*rule));
    const struct rtattr *attr = (const struct rtattr *)((const char *)rule + NLMSG_ALIGN(sizeof(*rule)));
    for (; RTA_OK(attr, length); attr = RTA_NEXT(attr, length)) {
        if (attr->rta_type == FRA_PRIORITY || attr->rta_type == FRA_TABLE || attr->rta_type == FRA_GOTO) {
            if (RTA_PAYLOAD(attr) != sizeof(uint32_t)) return -1;
            uint32_t n; memcpy(&n, RTA_DATA(attr), sizeof(n));
            if (attr->rta_type == FRA_PRIORITY) priority = n;
            else if (attr->rta_type == FRA_TABLE) table = n;
            else target = n;
        } else if (attr->rta_type == FRA_IIFNAME) {
            size_t size = RTA_PAYLOAD(attr);
            const char *name = RTA_DATA(attr);
            if (!size || size > IF_NAMESIZE || name[size - 1] != 0) return -1;
            input_ok = strcmp(name, "lo") == 0 || strcmp(name, cleanup.device) == 0;
        } else if (attr->rta_type == FRA_OIFNAME) input_ok = false;
    }
    if (length != 0) return -1;
    bool in_range = priority >= cleanup.priority && priority <= cleanup.priority + 10;
    if (!in_range && table != cleanup.table) return 0;
    bool own_target = (rule->action == FR_ACT_TO_TBL && table == cleanup.table) ||
        (rule->action == FR_ACT_GOTO && target >= cleanup.priority && target <= cleanup.priority + 10) ||
        rule->action == FR_ACT_NOP;
    return in_range && own_target && input_ok ? 1 : 2;
}

static int rule_socket(void) {
    int fd = socket(AF_NETLINK, SOCK_RAW | SOCK_CLOEXEC, NETLINK_ROUTE);
    if (fd < 0) return -1;
    struct sockaddr_nl address = {.nl_family = AF_NETLINK};
    if (bind(fd, (struct sockaddr *)&address, sizeof(address)) < 0) { close(fd); return -1; }
    return fd;
}

static bool netlink_send(int fd, const void *message, size_t length) {
    struct sockaddr_nl kernel = {.nl_family = AF_NETLINK};
    return sendto(fd, message, length, 0, (struct sockaddr *)&kernel, sizeof(kernel)) == (ssize_t)length;
}

static ssize_t netlink_receive(int fd, void *buffer, size_t size) {
    struct pollfd pending = {.fd = fd, .events = POLLIN};
    int status;
    do { status = poll(&pending, 1, 1500); } while (status < 0 && errno == EINTR);
    if (status <= 0) return -1;
    struct sockaddr_nl sender = {0};
    struct iovec iov = {.iov_base = buffer, .iov_len = size};
    struct msghdr message = {.msg_name = &sender, .msg_namelen = sizeof(sender), .msg_iov = &iov, .msg_iovlen = 1};
    ssize_t count = recvmsg(fd, &message, 0);
    return count > 0 && sender.nl_pid == 0 && !(message.msg_flags & MSG_TRUNC) ? count : -1;
}

static bool scan_rules(int fd, int family, rule_list *found, bool *occupied, bool *foreign) {
    struct { struct nlmsghdr header; struct fib_rule_hdr rule; } request = {
        .header = {.nlmsg_len = NLMSG_LENGTH(sizeof(struct fib_rule_hdr)), .nlmsg_type = RTM_GETRULE,
                   .nlmsg_flags = NLM_F_REQUEST | NLM_F_DUMP, .nlmsg_seq = 1},
        .rule = {.family = (unsigned char)family},
    };
    if (!netlink_send(fd, &request, request.header.nlmsg_len)) return false;
    unsigned char bytes[32768];
    for (int batch = 0; batch < 256; ++batch) {
        ssize_t count = netlink_receive(fd, bytes, sizeof(bytes));
        if (count < 0) return false;
        unsigned int remaining = (unsigned int)count;
        for (struct nlmsghdr *h = (struct nlmsghdr *)bytes; NLMSG_OK(h, remaining); h = NLMSG_NEXT(h, remaining)) {
            if (h->nlmsg_seq != 1 || h->nlmsg_flags & NLM_F_DUMP_INTR) return false;
            if (h->nlmsg_type == NLMSG_DONE) return true;
            if (h->nlmsg_type != RTM_NEWRULE) return false;
            int match = scoped_rule(h);
            if (match < 0) return false;
            if (match) *occupied = true;
            if (match == 2 && foreign) *foreign = true;
            if (match == 1 && found) {
                if (h->nlmsg_len > RULE_BYTES || found->count == RULE_LIMIT) return false;
                memcpy(found->rules[found->count++], h, h->nlmsg_len);
            }
        }
        if (remaining) return false;
    }
    return false;
}

/* A table can contain another owner's routes even when it has no policy rule yet. */
static bool route_table_empty(int fd, int family) {
    struct { struct nlmsghdr header; struct rtmsg route; } request = {
        .header = {.nlmsg_len = NLMSG_LENGTH(sizeof(struct rtmsg)), .nlmsg_type = RTM_GETROUTE,
                   .nlmsg_flags = NLM_F_REQUEST | NLM_F_DUMP, .nlmsg_seq = 3},
        .route = {.rtm_family = (unsigned char)family},
    };
    if (!netlink_send(fd, &request, request.header.nlmsg_len)) return false;
    unsigned char bytes[32768];
    for (int batch = 0; batch < 256; ++batch) {
        ssize_t count = netlink_receive(fd, bytes, sizeof(bytes));
        if (count < 0) return false;
        unsigned int remaining = (unsigned int)count;
        for (struct nlmsghdr *h = (struct nlmsghdr *)bytes; NLMSG_OK(h, remaining); h = NLMSG_NEXT(h, remaining)) {
            if (h->nlmsg_seq != 3 || h->nlmsg_flags & NLM_F_DUMP_INTR) return false;
            if (h->nlmsg_type == NLMSG_DONE) return true;
            if (h->nlmsg_type != RTM_NEWROUTE || h->nlmsg_len < NLMSG_LENGTH(sizeof(struct rtmsg))) return false;
            const struct rtmsg *route = NLMSG_DATA(h);
            uint32_t table = route->rtm_table;
            int length = (int)h->nlmsg_len - (int)NLMSG_LENGTH(sizeof(*route));
            const struct rtattr *attr = RTM_RTA(route);
            for (; RTA_OK(attr, length); attr = RTA_NEXT(attr, length)) {
                if (attr->rta_type == RTA_TABLE) {
                    if (RTA_PAYLOAD(attr) != sizeof(uint32_t)) return false;
                    memcpy(&table, RTA_DATA(attr), sizeof(table));
                }
            }
            if (length != 0 || table == cleanup.table) return false;
        }
        if (remaining) return false;
    }
    return false;
}

static bool cleanup_preflight(void) {
    if (!cleanup.enabled) return true;
    if (if_nametoindex(cleanup.device)) return false;
    int fd = rule_socket();
    if (fd < 0) return false;
    bool occupied = false;
    bool valid = scan_rules(fd, AF_INET, NULL, &occupied, NULL) && scan_rules(fd, AF_INET6, NULL, &occupied, NULL) &&
        route_table_empty(fd, AF_INET) && route_table_empty(fd, AF_INET6);
    close(fd);
    return valid && !occupied;
}

/* Mihomo's graceful teardown may sweep a whole priority range. If a foreign rule
 * appeared after our empty-range preflight, it must not run that sweep. Kill only
 * our pinned child and let cleanup_rules delete the exact owned records instead.
 * An unreadable routing state cannot authorize the broader kernel cleanup. */
static bool graceful_cleanup_safe(void) {
    if (!cleanup.enabled) return true;
    int fd = rule_socket();
    if (fd < 0) return false;
    bool occupied = false, foreign = false;
    bool valid = scan_rules(fd, AF_INET, NULL, &occupied, &foreign) &&
        scan_rules(fd, AF_INET6, NULL, &occupied, &foreign);
    close(fd);
    return valid && !foreign;
}

static bool cleanup_rules(void) {
    if (!cleanup.enabled) return true;
    int fd = rule_socket();
    if (fd < 0) return false;
    rule_list *found = calloc(1, sizeof(*found));
    bool occupied = false;
    bool valid = found && scan_rules(fd, AF_INET, found, &occupied, NULL) && scan_rules(fd, AF_INET6, found, &occupied, NULL);
    if (valid) for (size_t i = 0; i < found->count; ++i) {
        struct nlmsghdr *h = (struct nlmsghdr *)found->rules[i];
        h->nlmsg_type = RTM_DELRULE;
        h->nlmsg_flags = NLM_F_REQUEST | NLM_F_ACK;
        h->nlmsg_seq = 2;
        h->nlmsg_pid = 0;
        unsigned char response[4096];
        if (!netlink_send(fd, h, h->nlmsg_len)) { valid = false; break; }
        ssize_t count = netlink_receive(fd, response, sizeof(response));
        struct nlmsghdr *ack = (struct nlmsghdr *)response;
        if (count < (ssize_t)NLMSG_LENGTH(sizeof(struct nlmsgerr)) || ack->nlmsg_type != NLMSG_ERROR || ack->nlmsg_seq != 2) { valid = false; break; }
        int error = ((struct nlmsgerr *)NLMSG_DATA(ack))->error;
        if (error && error != -ENOENT) { valid = false; break; }
    }
    free(found);
    close(fd);
    return valid;
}

static bool valid_uuid(const char *value) {
    if (strlen(value) != 36) return false;
    for (size_t index = 0; index < 36; ++index) {
        if (index == 8 || index == 13 || index == 18 || index == 23) {
            if (value[index] != '-') return false;
        } else if (!isxdigit((unsigned char)value[index])) return false;
    }
    return true;
}

/* Field 2 (comm) may contain spaces, ')' and even newlines. Its final ')' is
 * the boundary; all following fields are whitespace-delimited numerics except
 * state. This is deliberately not a scanf("%s") parser. */
static bool parse_start_time(char *stat, uint64_t *value) {
    char *cursor = strrchr(stat, ')');
    if (!cursor || cursor[1] != ' ') return false;
    cursor += 2;
    for (int field = 3; field <= 22; ++field) {
        while (*cursor == ' ' || *cursor == '\t' || *cursor == '\n') ++cursor;
        char *end = cursor;
        while (*end && *end != ' ' && *end != '\t' && *end != '\n') ++end;
        if (end == cursor) return false;
        if (field == 22) {
            char saved = *end;
            *end = '\0';
            char *number_end;
            errno = 0;
            unsigned long long parsed = strtoull(cursor, &number_end, 10);
            bool valid = !errno && number_end == end && *cursor >= '0' && *cursor <= '9';
            *end = saved;
            if (!valid) return false;
            *value = (uint64_t)parsed;
            return true;
        }
        cursor = end;
    }
    return false;
}

static bool read_start_time(pid_t pid, uint64_t *value) {
    char path[64], data[4096];
    snprintf(path, sizeof(path), "/proc/%ld/stat", (long)pid);
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return false;
    ssize_t count;
    do { count = read(fd, data, sizeof(data) - 1); } while (count < 0 && errno == EINTR);
    close(fd);
    if (count <= 0 || (size_t)count >= sizeof(data) - 1) return false;
    data[count] = '\0';
    return parse_start_time(data, value);
}

static bool executable_matches(pid_t pid, const char *expected) {
    char path[64], target[PATH_MAX];
    snprintf(path, sizeof(path), "/proc/%ld/exe", (long)pid);
    ssize_t length = readlink(path, target, sizeof(target) - 1);
    if (length <= 0 || (size_t)length >= sizeof(target) - 1) return false;
    target[length] = '\0';
    return strcmp(target, expected) == 0;
}

/* WNOWAIT is the lifetime guarantee: this is our forked child and remains
 * unreaped while any process/group signal can still be sent. Its PID therefore
 * cannot be recycled between inspecting /proc and kill(), including API 28
 * kernels that have no pidfd. No arbitrary/persisted PID can enter this path. */
static int child_exited(pid_t child) {
    siginfo_t info;
    memset(&info, 0, sizeof(info));
    int result;
    do { result = waitid(P_PID, (id_t)child, &info, WEXITED | WNOHANG | WNOWAIT); }
    while (result < 0 && errno == EINTR);
    if (result < 0) return -1;
    return info.si_pid == child ? 1 : 0;
}

static void signal_owned_child(pid_t child, int signal_number) {
    /* Also address the child itself when failure occurred before setsid().
     * Neither positive nor negative PID is ever sourced from stdin. */
    kill(-child, signal_number);
    kill(child, signal_number);
}

static int reap_child(pid_t child) {
    int status;
    pid_t result;
    do { result = waitpid(child, &status, 0); } while (result < 0 && errno == EINTR);
    if (result != child) return 125;
    if (WIFEXITED(status)) return WEXITSTATUS(status);
    if (WIFSIGNALED(status)) return 128 + WTERMSIG(status);
    return 125;
}

typedef struct { int stage; int number; } exec_error;

/* Called only in the fork child: fixed-stack data plus syscall/libc syscall
 * wrappers; no stdio, allocation, directory traversal, or formatting. */
static void child_fail(int fd, int stage) {
    exec_error error = {.stage = stage, .number = errno};
    write_all(fd, &error, sizeof(error));
    _exit(126);
}

static void close_extra_fds(int highest) {
#ifdef __NR_close_range
    if (syscall(__NR_close_range, 4U, ~0U, 0U) == 0) return;
#endif
    for (int fd = 4; fd < highest; ++fd) close(fd);
}

static void run_child(pid_t parent, int status_fd, int null_fd,
                      int highest_fd, const char *kernel, const char *directory,
                      char *const arguments[]) {
    if (prctl(PR_SET_PDEATHSIG, SIGKILL) < 0) child_fail(status_fd, 1);
    if (getppid() != parent) _exit(126);
    struct sigaction reset = {.sa_handler = SIG_DFL};
    sigemptyset(&reset.sa_mask);
    if (sigaction(SIGTERM, &reset, NULL) < 0 ||
        sigaction(SIGHUP, &reset, NULL) < 0 ||
        sigaction(SIGINT, &reset, NULL) < 0 ||
        sigaction(SIGPIPE, &reset, NULL) < 0) child_fail(status_fd, 1);
    sigset_t empty_mask;
    sigemptyset(&empty_mask);
    if (sigprocmask(SIG_SETMASK, &empty_mask, NULL) < 0) child_fail(status_fd, 1);
    if (setsid() < 0) child_fail(status_fd, 2);
    if (chdir(directory) < 0) child_fail(status_fd, 3);
    if (dup2(null_fd, STDIN_FILENO) < 0 || dup2(STDERR_FILENO, STDOUT_FILENO) < 0)
        child_fail(status_fd, 4);
    if (status_fd != 3 && dup2(status_fd, 3) < 0) child_fail(status_fd, 4);
    if (fcntl(3, F_SETFD, FD_CLOEXEC) < 0) child_fail(3, 4);
    close_extra_fds(highest_fd);
    execve(kernel, arguments, environ);
    child_fail(3, 5);
}

static bool read_exec_result(int status_fd) {
    struct pollfd pending = {.fd = status_fd, .events = POLLIN | POLLHUP};
    int64_t deadline = now_ms() + EXEC_TIMEOUT_MS;
    for (;;) {
        int64_t remaining = deadline - now_ms();
        if (remaining <= 0 || requested_signal) return false;
        int result = poll(&pending, 1, (int)remaining);
        if (result < 0 && errno == EINTR) continue;
        if (result <= 0) return false;
        exec_error error;
        ssize_t count;
        do { count = read(status_fd, &error, sizeof(error)); } while (count < 0 && errno == EINTR);
        return count == 0; /* CLOEXEC EOF is the only success result. */
    }
}

static bool control_matches(const char *line, const char *run_id,
                            pid_t child, uint64_t start_time) {
    char expected[CONTROL_CAPACITY];
    int count = snprintf(expected, sizeof(expected), "stop %s %ld %" PRIu64,
                         run_id, (long)child, start_time);
    return count > 0 && (size_t)count < sizeof(expected) && strcmp(line, expected) == 0;
}

static void begin_stop(pid_t child, bool *forced, int64_t *deadline) {
    *deadline = now_ms() + STOP_GRACE_MS;
    *forced = !graceful_cleanup_safe();
    signal_owned_child(child, *forced ? SIGKILL : SIGTERM);
}

static int supervise(pid_t child, const char *kernel, const char *run_id,
                     uint64_t start_time, bool initial_stop) {
    char input[CONTROL_CAPACITY];
    size_t used = 0;
    bool discard_line = false, stopping = initial_stop, forced = false;
    int64_t deadline = 0;
    if (stopping) begin_stop(child, &forced, &deadline);
    for (;;) {
        int exited = child_exited(child);
        if (exited != 0) {
            if (exited < 0) {
                /* ECHILD would break the pin: never signal after this point. */
                emit_error("Lost ownership of supervised child", true);
                return 125;
            }
            /* Keep the session leader unreaped until remaining group members
             * have been told to exit. This cannot target a recycled PGID. */
            kill(-child, SIGKILL);
            int exit_code = reap_child(child);
            if (!cleanup_rules()) emit_error("Owned TUN policy-rule cleanup could not be confirmed", false);
            char line[160];
            int count = snprintf(line, sizeof(line),
                "{\"event\":\"exit\",\"pid\":%ld,\"exitCode\":%d,\"forced\":%s}\n",
                (long)child, exit_code, forced ? "true" : "false");
            write_all(STDOUT_FILENO, line, (size_t)count);
            return exit_code;
        }
        if (requested_signal && !stopping) {
            stopping = true;
            begin_stop(child, &forced, &deadline);
        }
        if (stopping && !forced && now_ms() >= deadline) {
            forced = true;
            signal_owned_child(child, SIGKILL);
        }
        struct pollfd pending = {.fd = stopping ? -1 : STDIN_FILENO, .events = POLLIN | POLLHUP};
        int result = poll(&pending, 1, 50);
        if (result < 0 && errno == EINTR) continue;
        if (result < 0 || (result > 0 && (pending.revents & (POLLERR | POLLNVAL)))) {
            if (!stopping) {
                stopping = true;
                begin_stop(child, &forced, &deadline);
            }
            continue;
        }
        if (result == 0 || stopping) continue;
        char bytes[512];
        ssize_t length = read(STDIN_FILENO, bytes, sizeof(bytes));
        if (length < 0 && (errno == EINTR || errno == EAGAIN)) continue;
        if (length <= 0) {
            stopping = true; /* App/su stdin owner died: do not leave root TUN. */
            begin_stop(child, &forced, &deadline);
            continue;
        }
        for (ssize_t index = 0; index < length && !stopping; ++index) {
            char byte = bytes[index];
            if (byte != '\n') {
                if (byte == '\0' || used + 1 >= sizeof(input)) discard_line = true;
                if (!discard_line) input[used++] = byte;
                continue;
            }
            input[used] = '\0';
            uint64_t actual_start;
            bool valid = !discard_line && control_matches(input, run_id, child, start_time);
            if (valid) valid = child_exited(child) == 0 &&
                read_start_time(child, &actual_start) && actual_start == start_time &&
                executable_matches(child, kernel);
            used = 0;
            discard_line = false;
            if (!valid) {
                if (emit_error("Stop rejected: control or owned child identity mismatch", false)) continue;
            }
            stopping = true;
            begin_stop(child, &forced, &deadline);
        }
    }
}

int main(int argc, char **argv) {
    struct sigaction action = {.sa_handler = on_signal};
    sigemptyset(&action.sa_mask);
    struct sigaction ignore = {.sa_handler = SIG_IGN};
    sigemptyset(&ignore.sa_mask);
    /* Reset SIGCHLD even if a launcher inherited SIG_IGN/SA_NOCLDWAIT. */
    struct sigaction child_action = {.sa_handler = SIG_DFL};
    sigemptyset(&child_action.sa_mask);
    if (sigaction(SIGTERM, &action, NULL) < 0 || sigaction(SIGHUP, &action, NULL) < 0 ||
        sigaction(SIGINT, &action, NULL) < 0 || sigaction(SIGPIPE, &ignore, NULL) < 0 ||
        sigaction(SIGCHLD, &child_action, NULL) < 0) return 125;
    sigset_t mask;
    sigemptyset(&mask);
    if (sigprocmask(SIG_SETMASK, &mask, NULL) < 0) return 125;
    /* Never let a full/dead control consumer prevent EOF/signal cleanup.
     * Every JSON record is smaller than PIPE_BUF and written atomically. */
    int stdout_flags = fcntl(STDOUT_FILENO, F_GETFL);
    if (fcntl(STDIN_FILENO, F_GETFD) < 0 || fcntl(STDERR_FILENO, F_GETFD) < 0 ||
        stdout_flags < 0 || fcntl(STDOUT_FILENO, F_SETFL, stdout_flags | O_NONBLOCK) < 0)
        return 125;
#ifndef WORKFLOW_PROXY_GUARD_HOST_TEST
    if (geteuid() != 0) {
        emit_error("Root uid is required", true);
        return 126;
    }
#endif
    if ((argc != 6 && argc != 10) || strcmp(argv[1], "supervise") != 0 || !valid_uuid(argv[5])) {
        emit_error("Expected supervise kernelAbs directoryAbs configAbs runUuid", true);
        return 126;
    }
    if (argc == 10) {
        if (strcmp(argv[6], "--tun-cleanup") || !uint_arg(argv[7], &cleanup.table) ||
            !uint_arg(argv[8], &cleanup.priority) || cleanup.table < 256 || cleanup.table > 0x7fffffff ||
            cleanup.priority >= 9990 || strlen(argv[9]) >= IF_NAMESIZE || !argv[9][0] ||
            strcmp(argv[9], ".") == 0 || strcmp(argv[9], "..") == 0) {
            emit_error("Invalid TUN cleanup scope", true); return 126;
        }
        for (const char *p = argv[9]; *p; ++p) if (!isalnum((unsigned char)*p) && *p != '-' && *p != '_' && *p != '.') {
            emit_error("Invalid TUN cleanup device", true); return 126;
        }
        strcpy(cleanup.device, argv[9]);
        cleanup.enabled = true;
        if (!cleanup_preflight()) { emit_error("TUN cleanup scope is occupied or cannot be verified", true); return 126; }
    }
    char kernel[PATH_MAX], directory[PATH_MAX], config[PATH_MAX];
    if (argv[2][0] != '/' || argv[3][0] != '/' || argv[4][0] != '/' ||
        !realpath(argv[2], kernel) || !realpath(argv[3], directory) || !realpath(argv[4], config)) {
        emit_error("Absolute existing kernel, directory and config paths are required", true);
        return 126;
    }
    struct stat kernel_stat, directory_stat, config_stat;
    if (stat(kernel, &kernel_stat) < 0 || !S_ISREG(kernel_stat.st_mode) || access(kernel, X_OK) < 0 ||
        stat(directory, &directory_stat) < 0 || !S_ISDIR(directory_stat.st_mode) ||
        stat(config, &config_stat) < 0 || !S_ISREG(config_stat.st_mode)) {
        emit_error("Invalid kernel, directory or configuration file", true);
        return 126;
    }
    int status_pipe[2];
    if (pipe2(status_pipe, O_CLOEXEC) < 0) {
        emit_error("Unable to create exec status pipe", true);
        return 125;
    }
    int null_fd = open("/dev/null", O_RDONLY | O_CLOEXEC);
    if (null_fd < 0) {
        close(status_pipe[0]); close(status_pipe[1]);
        emit_error("Unable to open child input", true);
        return 125;
    }
    struct rlimit limits;
    int highest_fd = getrlimit(RLIMIT_NOFILE, &limits) == 0 && limits.rlim_cur < INT_MAX
        ? (int)limits.rlim_cur : 1048576;
    char *arguments[] = {kernel, "-d", directory, "-f", config, NULL};
    pid_t parent = getpid();
    pid_t child = fork();
    if (child == 0) {
        close(status_pipe[0]);
        run_child(parent, status_pipe[1], null_fd, highest_fd, kernel, directory, arguments);
        _exit(126);
    }
    close(status_pipe[1]); close(null_fd);
    if (child < 0) {
        close(status_pipe[0]);
        emit_error("Unable to fork kernel", true);
        return 125;
    }
    bool executed = read_exec_result(status_pipe[0]);
    close(status_pipe[0]);
    uint64_t start_time, guard_start_time;
    int initial_exit = child_exited(child);
    if (!executed || !read_start_time(child, &start_time) ||
        !read_start_time(parent, &guard_start_time) || initial_exit != 0 ||
        !executable_matches(child, kernel)) {
        emit_error("Kernel exec or initial process identity verification failed", true);
        if (initial_exit >= 0) {
            signal_owned_child(child, SIGKILL);
            reap_child(child);
            if (!cleanup_rules()) emit_error("Owned TUN policy-rule cleanup could not be confirmed", false);
        }
        return 126;
    }
    char line[256];
    int size = snprintf(line, sizeof(line),
        "{\"event\":\"started\",\"uid\":%ld,\"pid\":%ld,\"startTime\":%" PRIu64
        ",\"guardPid\":%ld,\"guardStartTime\":%" PRIu64 ",\"runId\":\"%s\"}\n",
        (long)geteuid(), (long)child, start_time, (long)parent, guard_start_time, argv[5]);
    bool announced = size > 0 && (size_t)size < sizeof(line) &&
        write_all(STDOUT_FILENO, line, (size_t)size);
    return supervise(child, kernel, argv[5], start_time, !announced);
}
