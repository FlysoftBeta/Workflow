#define _GNU_SOURCE
#include "engine/mem.h"

#include <errno.h>
#include <string.h>
#include <sys/ptrace.h>
#include <sys/uio.h>

ssize_t eng_mem_read(pid_t tid, uintptr_t addr, void *buf, size_t n) {
    struct iovec local = {buf, n};
    struct iovec remote = {(void *)addr, n};
    ssize_t r = process_vm_readv(tid, &local, 1, &remote, 1, 0);
    if (r >= 0) return r;
    /* PEEKDATA fallback (word-at-a-time). */
    size_t done = 0;
    while (done < n) {
        errno = 0;
        long word = ptrace(PTRACE_PEEKDATA, tid, (void *)(addr + done), 0);
        if (word == -1 && errno) return done ? (ssize_t)done : -1;
        size_t chunk = n - done < sizeof(long) ? n - done : sizeof(long);
        memcpy((char *)buf + done, &word, chunk);
        done += chunk;
    }
    return (ssize_t)done;
}

ssize_t eng_mem_write(pid_t tid, uintptr_t addr, const void *buf, size_t n) {
    struct iovec local = {(void *)buf, n};
    struct iovec remote = {(void *)addr, n};
    ssize_t r = process_vm_writev(tid, &local, 1, &remote, 1, 0);
    if (r >= 0) return r;
    /* POKEDATA fallback: must read-modify-write partial trailing word. */
    size_t done = 0;
    while (done < n) {
        size_t chunk = n - done < sizeof(long) ? n - done : sizeof(long);
        long word = 0;
        if (chunk < sizeof(long)) {
            errno = 0;
            word = ptrace(PTRACE_PEEKDATA, tid, (void *)(addr + done), 0);
            if (word == -1 && errno) return done ? (ssize_t)done : -1;
        }
        memcpy(&word, (const char *)buf + done, chunk);
        if (ptrace(PTRACE_POKEDATA, tid, (void *)(addr + done), (void *)word) != 0)
            return done ? (ssize_t)done : -1;
        done += chunk;
    }
    return (ssize_t)done;
}

ssize_t eng_mem_read_cstr(pid_t tid, uintptr_t addr, char *buf, size_t cap) {
    if (cap == 0) return -1;
    size_t got = 0;
    while (got < cap - 1) {
        char chunk[256];
        size_t want = cap - 1 - got;
        if (want > sizeof(chunk)) want = sizeof(chunk);
        ssize_t r = eng_mem_read(tid, addr + got, chunk, want);
        if (r <= 0) {
            if (got == 0) return -1;
            break;
        }
        for (ssize_t i = 0; i < r; i++) {
            if (chunk[i] == '\0') {
                memcpy(buf + got, chunk, (size_t)i);
                buf[got + i] = '\0';
                return (ssize_t)(got + i);
            }
        }
        memcpy(buf + got, chunk, (size_t)r);
        got += (size_t)r;
    }
    buf[got] = '\0';
    return (ssize_t)got;
}
