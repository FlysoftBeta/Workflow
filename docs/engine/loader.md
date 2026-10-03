# Guest loader

`engine/environment/loader` is package `workflow-loader`, a Rust `no_std` freestanding ELF loader used by [Runtime](runtime.md). Android packages it as `libworkflow-loader.so`; the Server and isolation runtime are distinct executables. Environment supplies the loader path to every guest run through its typed runtime API.

The loader is a static PIE with no libc, dynamic interpreter or runtime relocations. Assembly is limited to syscall entry and the final transfer to the guest stack. Segments are aligned to 16 KiB, and the loader reads the actual page size from `AT_PAGESZ`. It retains file-mapping and anonymous-copy paths for guest ELF segments.

Every guest exec actually executes this loader. Runtime prepares the virtual UID/GID, `AT_SECURE`, shebang/binfmt handling and argv; loader transfer must preserve close-on-exec behavior, signal reset and thread reaping. The runtime and guest ABI must match. The loader does not provide CPU emulation or authorize a product host-execution fallback.

The specialized `build.sh` preserves its known static-PIE and Android linker flags. A Cargo package or source relocation does not replace those build constraints. `engine/build-android.sh` builds Server, runtime and loader for the selected ABIs; runtime build and acceptance entry points are documented in [Runtime](runtime.md#build-and-acceptance-entry-points).

The archived C loader is historical source, never a production dependency. Host or static ELF checks cannot establish all Android behavior. Both Android cross-builds and isolated API 28 runtime tests are required for loader changes; physical ARM64 and real 16 KiB glibc workloads need separate evidence. The [runtime report](../archive/implementation-1.0.0/reports/rewrite/rust-runtime.md) and [status](../status.md) retain those limits.
