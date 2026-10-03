/* json.c — small strict JSON reader (RFC 8259) for metadata.json/image.json.
 * Builds a tree; depth-limited; rejects trailing garbage. */
#include "engine/json.h"

#include <stdlib.h>
#include <string.h>

typedef struct { const char *p, *end; int depth; } rd;

static void ws(rd *r) { while (r->p < r->end && (*r->p == ' ' || *r->p == '\t' || *r->p == '\n' || *r->p == '\r')) r->p++; }

static eng_json *node(eng_jtype t) {
    eng_json *n = calloc(1, sizeof *n);
    if (n) n->t = t;
    return n;
}

static int put_utf8(char **o, unsigned cp) {
    char *q = *o;
    if (cp < 0x80) *q++ = (char)cp;
    else if (cp < 0x800) { *q++ = (char)(0xc0 | cp >> 6); *q++ = (char)(0x80 | (cp & 0x3f)); }
    else if (cp < 0x10000) { *q++ = (char)(0xe0 | cp >> 12); *q++ = (char)(0x80 | ((cp >> 6) & 0x3f)); *q++ = (char)(0x80 | (cp & 0x3f)); }
    else { *q++ = (char)(0xf0 | cp >> 18); *q++ = (char)(0x80 | ((cp >> 12) & 0x3f)); *q++ = (char)(0x80 | ((cp >> 6) & 0x3f)); *q++ = (char)(0x80 | (cp & 0x3f)); }
    *o = q;
    return 0;
}

static int hex4(const char *p, unsigned *v) {
    *v = 0;
    for (int i = 0; i < 4; i++) {
        char c = p[i];
        *v <<= 4;
        if (c >= '0' && c <= '9') *v |= (unsigned)(c - '0');
        else if (c >= 'a' && c <= 'f') *v |= (unsigned)(c - 'a' + 10);
        else if (c >= 'A' && c <= 'F') *v |= (unsigned)(c - 'A' + 10);
        else return -1;
    }
    return 0;
}

static char *str(rd *r) {
    if (r->p >= r->end || *r->p != '"') return NULL;
    r->p++;
    const char *s = r->p;
    size_t cap = 0;
    while (r->p < r->end && *r->p != '"') { if (*r->p == '\\') r->p++; r->p++; cap++; }
    if (r->p >= r->end) return NULL;
    char *out = malloc(cap * 4 + 1), *o = out;
    if (!out) return NULL;
    for (const char *q = s; q < r->p; q++) {
        unsigned char c = (unsigned char)*q;
        if (c < 0x20) { free(out); return NULL; }
        if (c != '\\') { *o++ = (char)c; continue; }
        q++;
        switch (*q) {
            case '"': *o++ = '"'; break;
            case '\\': *o++ = '\\'; break;
            case '/': *o++ = '/'; break;
            case 'b': *o++ = '\b'; break;
            case 'f': *o++ = '\f'; break;
            case 'n': *o++ = '\n'; break;
            case 'r': *o++ = '\r'; break;
            case 't': *o++ = '\t'; break;
            case 'u': {
                unsigned cp;
                if (r->p - q < 5 || hex4(q + 1, &cp)) { free(out); return NULL; }
                q += 4;
                if (cp >= 0xd800 && cp < 0xdc00) {
                    unsigned lo;
                    if (r->p - q < 7 || q[1] != '\\' || q[2] != 'u' || hex4(q + 3, &lo) || lo < 0xdc00 || lo > 0xdfff) {
                        free(out);
                        return NULL;
                    }
                    cp = 0x10000 + ((cp - 0xd800) << 10) + (lo - 0xdc00);
                    q += 6;
                } else if (cp >= 0xdc00 && cp < 0xe000) { free(out); return NULL; }
                put_utf8(&o, cp);
                break;
            }
            default: free(out); return NULL;
        }
    }
    *o = 0;
    r->p++;
    return out;
}

static eng_json *value(rd *r);

static eng_json *container(rd *r, int obj) {
    if (++r->depth > 64) return NULL;
    eng_json *n = node(obj ? ENG_J_OBJ : ENG_J_ARR), *last = NULL;
    if (!n) return NULL;
    r->p++;
    ws(r);
    if (r->p < r->end && *r->p == (obj ? '}' : ']')) { r->p++; r->depth--; return n; }
    for (;;) {
        ws(r);
        char *key = NULL;
        if (obj) {
            if (!(key = str(r))) goto bad;
            ws(r);
            if (r->p >= r->end || *r->p != ':') { free(key); goto bad; }
            r->p++;
        }
        eng_json *v = value(r);
        if (!v) { free(key); goto bad; }
        v->key = key;
        if (last) last->next = v; else n->child = v;
        last = v;
        ws(r);
        if (r->p < r->end && *r->p == ',') { r->p++; continue; }
        if (r->p < r->end && *r->p == (obj ? '}' : ']')) { r->p++; break; }
        goto bad;
    }
    r->depth--;
    return n;
bad:
    eng_json_free(n);
    return NULL;
}

static eng_json *value(rd *r) {
    ws(r);
    if (r->p >= r->end) return NULL;
    char c = *r->p;
    if (c == '{') return container(r, 1);
    if (c == '[') return container(r, 0);
    if (c == '"') {
        char *s = str(r);
        if (!s) return NULL;
        eng_json *n = node(ENG_J_STR);
        if (!n) { free(s); return NULL; }
        n->s = s;
        return n;
    }
    if (r->end - r->p >= 4 && !memcmp(r->p, "true", 4)) { r->p += 4; eng_json *n = node(ENG_J_BOOL); if (n) n->b = 1; return n; }
    if (r->end - r->p >= 5 && !memcmp(r->p, "false", 5)) { r->p += 5; return node(ENG_J_BOOL); }
    if (r->end - r->p >= 4 && !memcmp(r->p, "null", 4)) { r->p += 4; return node(ENG_J_NULL); }
    if (c == '-' || (c >= '0' && c <= '9')) {
        char buf[64];
        size_t k = 0;
        while (r->p < r->end && k < sizeof buf - 1 && strchr("-+.eE0123456789", *r->p)) buf[k++] = *r->p++;
        buf[k] = 0;
        char *e;
        double d = strtod(buf, &e);
        if (*e || !k) return NULL;
        eng_json *n = node(ENG_J_NUM);
        if (n) n->n = d;
        return n;
    }
    return NULL;
}

eng_json *eng_json_parse(const char *text, size_t len) {
    rd r = {text, text + len, 0};
    eng_json *v = value(&r);
    if (!v) return NULL;
    ws(&r);
    if (r.p != r.end) { eng_json_free(v); return NULL; }
    return v;
}

void eng_json_free(eng_json *n) {
    while (n) {
        eng_json *next = n->next;
        eng_json_free(n->child);
        free(n->key);
        free(n->s);
        free(n);
        n = next;
    }
}

const eng_json *eng_json_get(const eng_json *obj, const char *key) {
    if (!obj || obj->t != ENG_J_OBJ) return NULL;
    for (const eng_json *c = obj->child; c; c = c->next)
        if (c->key && !strcmp(c->key, key)) return c;
    return NULL;
}

const char *eng_json_str(const eng_json *obj, const char *key) {
    const eng_json *v = eng_json_get(obj, key);
    return v && v->t == ENG_J_STR ? v->s : NULL;
}

int eng_json_int(const eng_json *obj, const char *key, long long *out) {
    const eng_json *v = eng_json_get(obj, key);
    if (!v || v->t != ENG_J_NUM || v->n != (double)(long long)v->n) return -1;
    *out = (long long)v->n;
    return 0;
}
