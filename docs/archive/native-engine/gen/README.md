# native/engine/gen — syscall inventory

Generates a versioned, classified list of every syscall on the engine's two guest ABIs
(x86_64 and aarch64; guest arch == host arch, one ABI per build, no 32-bit/x32). The engine
uses it to refuse ("default deny") any number that is not in the table or has not been classified.

| file | role |
| --- | --- |
| `src/linux-v7.2.8/…` | verbatim upstream kernel files (pinned below; do not edit) |
| `syscalls.tsv` | **hand-maintained** classification: `name<TAB>class<TAB>note`, one row per name |
| `gensysinv.py` | generator + validator (python3 stdlib only) |
| `coverage.md` | generated table: name, x86_64 nr, aarch64 nr, class, note |
| `check.sh` | regenerate, verify determinism, cross-check headers, build and run the tests |
| `../include/engine/sysinv.h`, `../src/sysinv.c` | generated C module (both ABIs, `#if __x86_64__ / __aarch64__`) |
| `../test/sysinv_test.c` | host test (native + forced aarch64 table) and `--dump` mode |

## Sources (Linux v7.2.8, latest stable on 2026-09-27)

linux-stable tag `v7.2.8` → commit `9a66fdc0d7fd55f54235524a73435af99051e46f`
(tag object `c4589b1a32fcdcc949d038fb662bf41b15403e68`). Files fetched from
`https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git/plain/<path>?h=v7.2.8`;
`syscall_64.tbl` and `scripts/syscall.tbl` were also byte-identical from the
`github.com/gregkh/linux` mirror at the same tag.

| path under `src/linux-v7.2.8/` | sha256 |
| --- | --- |
| `arch/x86/entry/syscalls/syscall_64.tbl` | `93e42d351de2002418bf499b9a538f189fd683e8ecd52b4adc8a0bbe59130455` |
| `scripts/syscall.tbl` | `222c40f91975eb2860bf4d334863005ef084fee39dd031d252ad2f2f2342d485` |
| `arch/arm64/kernel/Makefile.syscalls` | `27e95d7bd10c3ab67c8a3f16c7dfe5da30c586c2fda3993c42025109a22098c5` |

The hashes are also pinned in `gensysinv.py`; it refuses to run if a file changed.
The files keep their upstream SPDX headers (`GPL-2.0 WITH Linux-syscall-note` for the tables,
`GPL-2.0` for the Makefile fragment).

ABI selection:

- **x86_64**: rows with ABI `common` or `64` (x32 rows, 512–547, excluded).
- **aarch64**: in v7.2.8 `arch/arm64/tools/syscall_64.tbl` is a symlink to `scripts/syscall.tbl`;
  rows with ABI `common`, `64` and arm64's `syscall_abis_64` from `Makefile.syscalls`
  (`renameat rlimit memfd_secret`). `32`, `time32`, `stat64` and other arches' rows are excluded.
- Name aliases: none needed at this tag — the generic table's 64-bit rows already say
  `newfstatat` (79) and `fstat` (80), matching x86_64 and the uapi headers. `HEADER_ALIASES`
  in the generator is the place to add one if a future table diverges.
- Rows without a kernel entry point (or `sys_ni_syscall`) are kept (the number is assigned),
  and the generator requires them to be classed `ENOSYS`.

Header cross-check (`gensysinv.py --check-headers`, run by `check.sh`): NDK r30
(`30.0.15729638`) aarch64 and x86_64 `asm/unistd_64.h`, host `/usr/include/asm/unistd_64.h`
(kernel-headers 7.2.4). Result at v7.2.8: no number mismatches, no header-only names; both NDK
headers lack only `rseq_slice_yield` (471), which is newer than NDK r30.

## Classes

`EXEC > ENOSYS > EPERM > PATH > ID > META > SOCK > PROC > FD > PASS` — first matching rule wins.
`UNKNOWN` (0) is never written in the TSV; it is what a number outside the table maps to.
See the header of `syscalls.tsv` and `coverage.md` for the per-syscall decisions.

## Updating to a new kernel

1. Fetch the three files at the new tag into `src/linux-<tag>/` (same relative paths), delete
   the old directory, update `LINUX_TAG`, `LINUX_COMMIT` and `SOURCES` in `gensysinv.py` and
   the table above.
2. Run `check.sh`: it names every syscall that lacks a row. Classify each one in
   `syscalls.tsv` (never PASS without checking it carries no path and no identity semantics).
3. Update the expected totals in `test/sysinv_test.c` and re-run `check.sh`.

## Commands

```sh
native/engine/gen/check.sh                     # everything; build output in artifacts/engine/sysinv-check/
python3 native/engine/gen/gensysinv.py         # regenerate in place only
python3 native/engine/gen/gensysinv.py --check-headers
```
