/* fork_stress.c — fork/exec/vfork/thread stress for the tracer's task table
 * and event accounting.  Run under the engine; must exit 0 with no orphaned or
 * lost children (the tracer's waitpid loop reaps everything). */
#define _GNU_SOURCE
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/wait.h>
#include <unistd.h>

static void *worker(void *arg) {
    long n = (long)arg;
    for (int i = 0; i < 5; i++) (void)getpid();
    return (void *)(n + 1);
}

int main(void) {
    /* fork + exec many short children. */
    for (int i = 0; i < 60; i++) {
        pid_t p = fork();
        if (p < 0) return 2;
        if (p == 0) {
            char *const argv[] = {(char *)"/bin/true", NULL};
            execv("/bin/true", argv);
            _exit(3);
        }
        int st;
        if (waitpid(p, &st, 0) != p || !WIFEXITED(st) || WEXITSTATUS(st) != 0) return 4;
    }
    /* vfork user. */
    pid_t v = vfork();
    if (v == 0) _exit(0);
    int st;
    waitpid(v, &st, 0);
    /* threads (clone with CLONE_THREAD). */
    pthread_t th[4];
    for (long i = 0; i < 4; i++)
        if (pthread_create(&th[i], NULL, worker, (void *)i) != 0) return 5;
    for (int i = 0; i < 4; i++) pthread_join(th[i], NULL);
    return 0;
}
