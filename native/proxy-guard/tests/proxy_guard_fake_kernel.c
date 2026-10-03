#define _GNU_SOURCE
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/prctl.h>
#include <unistd.h>

static volatile sig_atomic_t replace_requested;
static void on_replace(int unused) { (void)unused; replace_requested = 1; }

int main(int argc, char **argv) {
    if (argc != 5 || strcmp(argv[1], "-d") || strcmp(argv[3], "-f")) return 80;
    char cwd[4096];
    if (!getcwd(cwd, sizeof(cwd)) || strcmp(cwd, argv[2])) return 81;
    FILE *config = fopen(argv[4], "r");
    char mode[64] = {0};
    if (!config || !fgets(mode, sizeof(mode), config)) return 82;
    fclose(config);
    if (strcmp(mode, "ignore") == 0) signal(SIGTERM, SIG_IGN);
    if (strcmp(mode, "replace") == 0) signal(SIGUSR1, on_replace);
    if (strcmp(mode, "comm") == 0 && prctl(PR_SET_NAME, "odd ) (x\nname") < 0) return 83;
    char input;
    if (read(STDIN_FILENO, &input, 1) != 0) return 84;
    if (getsid(0) != getpid() || getpgrp() != getpid()) return 85;
    printf("fake stdout on log channel\n");
    fflush(stdout);
    fprintf(stderr, "FAKE_READY\n");
    if (strcmp(mode, "exit") == 0) { usleep(250000); return 37; }
    for (;;) {
        pause();
        if (replace_requested) {
            char *replacement[] = {"/bin/sleep", "30", NULL};
            execv(replacement[0], replacement);
            return 86;
        }
    }
}
