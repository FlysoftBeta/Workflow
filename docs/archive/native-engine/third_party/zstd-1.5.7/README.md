# Zstandard 1.5.7 — single-file decoder (vendored)

Used by the engine installer (`workflow-engine install`) to read `image.tar.zst`.

| item | value |
| --- | --- |
| upstream | https://github.com/facebook/zstd, release v1.5.7 |
| archive | `zstd-1.5.7.tar.gz`, sha256 `eb33e51f49a15e023950cd7825ca74a4a2b43db8354825ac24fc1b7ee09e6fa3` (matches the release's `.sha256`) |
| license | BSD (`LICENSE`) or GPLv2 (`COPYING`), at our option: **BSD** |
| files | `zstddeclib.c` generated, `zstd.h`, `zstd_errors.h`, `LICENSE`, `COPYING` copied verbatim |

Regenerate `zstddeclib.c` (decoder only, no legacy formats, no asm, error strings stripped):

```sh
tar xzf zstd-1.5.7.tar.gz && cd zstd-1.5.7/build/single_file_libs
python3 combine.py -r ../../lib -x legacy/zstd_legacy.h -o zstddeclib.c zstddeclib-in.c
```
