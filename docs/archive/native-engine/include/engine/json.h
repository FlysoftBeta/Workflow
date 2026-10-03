/* json.h — small strict JSON reader. */
#ifndef WORKFLOW_ENGINE_JSON_H
#define WORKFLOW_ENGINE_JSON_H

#include <stddef.h>

typedef enum { ENG_J_NULL, ENG_J_BOOL, ENG_J_NUM, ENG_J_STR, ENG_J_ARR, ENG_J_OBJ } eng_jtype;

typedef struct eng_json {
    eng_jtype t;
    char *key;                  /* member name inside an object */
    char *s;                    /* ENG_J_STR */
    double n;                   /* ENG_J_NUM */
    int b;                      /* ENG_J_BOOL */
    struct eng_json *child;     /* first element / member */
    struct eng_json *next;
} eng_json;

eng_json *eng_json_parse(const char *text, size_t len);   /* NULL on any syntax error */
void eng_json_free(eng_json *n);
const eng_json *eng_json_get(const eng_json *obj, const char *key);
const char *eng_json_str(const eng_json *obj, const char *key);
int eng_json_int(const eng_json *obj, const char *key, long long *out);   /* 0, or -1 if absent/not integral */

#endif
