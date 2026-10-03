/* log.h — leveled logging to stderr, gated by WORKFLOW_ENGINE_LOG env or -v. */
#ifndef WORKFLOW_ENGINE_LOG_H
#define WORKFLOW_ENGINE_LOG_H

typedef enum { ENG_LOG_ERROR = 0, ENG_LOG_WARN, ENG_LOG_INFO, ENG_LOG_DEBUG, ENG_LOG_TRACE } eng_log_level;

void eng_log_init(int verbosity);      /* 0..4; also reads WORKFLOW_ENGINE_LOG */
int  eng_log_enabled(eng_log_level lv);
void eng_logf(eng_log_level lv, const char *fmt, ...) __attribute__((format(printf, 2, 3)));

#define ENG_ERR(...)   eng_logf(ENG_LOG_ERROR, __VA_ARGS__)
#define ENG_WARN(...)  eng_logf(ENG_LOG_WARN, __VA_ARGS__)
#define ENG_INFO(...)  eng_logf(ENG_LOG_INFO, __VA_ARGS__)
#define ENG_DBG(...)   eng_logf(ENG_LOG_DEBUG, __VA_ARGS__)
#define ENG_TRACE(...) eng_logf(ENG_LOG_TRACE, __VA_ARGS__)

#endif
