# Runtime platform extraction audit

The retained baseline is `a2cf9908e6d4f82315bf741d5698a6898281cd25`, with 69,977 Rust
source lines. The shared tree contains 26,013 lines, a reduction of 43,964 lines (62.8%). The extraction changes source organization, not supported behavior. Dependencies,
the syscall inventory and exported C entry points retain their existing contracts.

## Equivalence method

`python3 tools/check-platform-merge.py` asks the pinned Rust compiler to expand each module for
Linux x86_64, Android x86_64 and Android aarch64. It compares the resulting items with the baseline
in Git. It performs no linking, target execution or Cargo build and needs no third-party Python
packages. Compilation and device tests remain separate requirements.

The comparison ignores declaration order, foreign parameter names, comments, whitespace and
optional trailing commas. It resolves scalar aliases and the equivalent `core::ffi` scalar types
on these three LP64 targets. It deliberately retains `c_char`, whose signedness depends on the
target, as well as pointer mutability, signatures, aggregate fields and their order. Header-only
transparent integer enum wrappers become scalar constants with the same representation and values;
ordinary runtime newtypes remain intact. Compiler-generated derive implementations follow the
compared fields; they are excluded from the item comparison.

The Rust-local `errno` declaration retains the original symbol through `link_name`. Named file-mode
flags and their expanded integer values are compared by value. Explicit const-pointer coercions
on C string-search results can become implicit coercions in typed bindings or null checks; the
different `memchr` void-to-char conversion stays explicit. Private `platform_empty_*` functions
return the original aggregate literal. The checker substitutes those constructors at call sites,
so every initialized field and value is still compared. These helpers contain no calls or effects.
No runtime algorithm is regenerated from archived C, and no source generator is needed to build.

## Divergence decisions

No behavioral accidental drift was found in any of the fourteen merged modules. The non-behavioral drift
consists of C header parameter names/order, scalar aliases, expanded file-mode macros, transparent
header enum names, and redundant pointer casts. Those spellings are normalized as described above;
neither target's behavior takes precedence.

| Shared module | Intentional differences retained with cfg |
| --- | --- |
| `json`, `sha256` | None; Android copies were identical, host differed only in declaration spellings. |
| `mem` | glibc/bionic errno symbols and ptrace request types; native iovec length representation. |
| `scratch` | Register layout and x86_64's 128-byte red zone versus aarch64's zero red zone. |
| `ident` | Register/stdio declarations, architecture syscall numbers and umask dispatch. |
| `exec` | libc stat/stdio declarations, target aggregate initialization and native ELF aliases. |
| `path` | libc stat layout and aggregate initialization; path semantics are shared. |
| `cli` | libc stdio/getopt declarations, stat initialization, architecture name and sysconf key. |

| `arch` | Native iovec and ptrace signatures; register get/set/access, aarch64 NT_ARM_SYSTEM_CALL writes, argument restoration, instruction size and syscall-name numbers. |
| `guest` | Android system/linkerconfig binds; glibc device-number functions versus bionic bit expansions; libc stat/stdio/directory layouts. |
| `install` | glibc unsigned-long versus bionic unsigned-int ioctl requests; native stat/statvfs/stdio/directory layouts. Archive validation, installation and copy fallback are shared. |
| `meta` | Native stat/statx and directory/stdio declarations; device-number encoding; aarch64 nlink width; libc directory-type constants. Permissions, hardlink journal/recovery and fsck remain shared. |
| `sys` | ABI syscall numbers, socket/signal/stat layouts and native device encoding; x86 legacy conversion/exit helpers, getdents/time path table, entry rewrite, dispatch and SIGSYS reissue; aarch64 direct syscall path. |
| `tracer` | Native sigaction/sigset/siginfo layouts and field access, ptrace request/event/option declarations, argv pointer signatures and wait-status macro expressions; architecture audit ID, x32 filter, legacy deny list and synthetic x86 test filter (ARM still returns ENOSYS). |

Every differing declaration belongs to the selected libc/kernel ABI or its header spelling;
every differing executable fragment belongs to the functions and mechanisms listed above.
The exact target-selected comparison covers declarations as well as all function bodies, rather
than treating a successful host build as evidence about bionic or ARM register semantics.
No full per-target file remains. The small remaining per-target fragments deliberately retain
original behavior where choosing a generic implementation would add risk without device proof.

The expression helpers have one scalar or pointer argument and contain only the original field
access or predicate. The checker substitutes their bodies and arguments at the call site and
compares the resulting expression. It also removes scope-free single-assignment blocks required
by Rust's cfg syntax. Blocks with local bindings retain their scope in the comparison. Eleven
negative-control tests exercise width/signedness, layouts, strings, execution order, mode values,
constructor fields, helper predicates, errno symbols and real compiler target selection.

The syscall inventory, CLI options, exported entry points and dependency manifests are unchanged.
Unsupported targets are rejected explicitly, rather than compiling a CLI with no selected runtime.
ARM64 device acceptance is unavailable; the daily tablet is excluded from this task. JSON and
SHA-256 library adoption, further consolidation of repeated ABI declarations between different
modules, and replacement of remaining C-port idioms are follow-ups, not prerequisites for building
this shared tree. Check and device run directories and the final source identity are recorded in
the task handoff; outputs remain outside tracked source.
