# Runtime platform extraction audit

The retained baseline is `a2cf9908e6d4f82315bf741d5698a6898281cd25`, with 69,977 Rust
source lines. The extraction changes source organization, not supported behavior. Dependencies,
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

No behavioral accidental drift has been found in the completed modules. The non-behavioral drift
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

Register operations, guest setup, installation, metadata, syscall interception and tracing are
pending the next extraction step. ARM64 device acceptance is unavailable; the daily tablet is
excluded from this task. JSON and SHA-256 library adoption is deferred because dependency and
lockfile changes belong to a separate task.
