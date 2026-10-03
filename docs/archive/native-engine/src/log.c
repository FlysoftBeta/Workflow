#define _GNU_SOURCE
#include "engine/log.h"

#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

static eng_log_level g_level = ENG_LOG_WARN;

void eng_log_init(int verbosity) {
    const char *e = getenv("WORKFLOW_ENGINE_LOG");
    int env = -1;
    if (e && *e) env = atoi(e);
    int v = verbosity > env ? verbosity : env;
    if (v < 0) v = ENG_LOG_WARN;
    if (v > ENG_LOG_TRACE) v = ENG_LOG_TRACE;
    g_level = (eng_log_level)v;
}

int eng_log_enabled(eng_log_level lv) { return lv <= g_level; }

void eng_logf(eng_log_level lv, const char *fmt, ...) {
    if (lv > g_level) return;
    static const char *tag[] = {"E", "W", "I", "D", "T"};
    flockfile(stderr);
    fprintf(stderr, "[engine:%s:%d] ", tag[lv], (int)getpid());
    va_list ap;
    va_start(ap, fmt);
    vfprintf(stderr, fmt, ap);
    va_end(ap);
    fputc('\n', stderr);
    funlockfile(stderr);
}
