/* sha256.h — streaming SHA-256. */
#ifndef WORKFLOW_ENGINE_SHA256_H
#define WORKFLOW_ENGINE_SHA256_H

#include <stddef.h>
#include <stdint.h>

typedef struct { uint32_t h[8]; uint64_t len; uint8_t buf[64]; size_t n; } eng_sha256;

void eng_sha256_init(eng_sha256 *c);
void eng_sha256_update(eng_sha256 *c, const void *data, size_t n);
void eng_sha256_final(eng_sha256 *c, uint8_t out[32]);
void eng_sha256_hex(const uint8_t d[32], char out[65]);

#endif
