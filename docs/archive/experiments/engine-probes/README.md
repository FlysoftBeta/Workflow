# Host engine probes

Run `python3 tools/engine-probes/run.py` from the repository. Requires Linux,
Python 3 and a C compiler (`cc`); no Gradle, Android SDK or root is used.
Binaries, temporary files and JSON evidence stay in ignored `artifacts/engine-probes/`.

The C probe only traces its own children: syscall stops, a fork, a normal kernel
exec and one delivered SIGUSR1. PTRACE_GET_SYSCALL_INFO support is reported rather
than required. EXITKILL is required by this **host probe** for bounded cleanup;
its absence is a probe failure, not a conclusion that a future engine cannot work.
The Python runner gives the tracer ten seconds. ELF inspection reads the probe's
own ELF64 headers; it does not implement or validate a guest loader. The xattr
probe uses only newly created temporary files and reports unsupported attributes.

Neither tool is a sandbox or native engine. Android PackageManager execution,
SELinux policy, guest ELF mappings, syscall emulation, job control and the device
matrix remain untested. See [the design and acceptance gates](../../docs/engine.md).
