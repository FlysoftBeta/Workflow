# native/engine

Container engine for the workspace Debian environment: `libworkflow-engine.so` (tracer, dispatcher,
installer, CLI) and `libworkflow-loader.so` (freestanding guest ELF loader), both packaged in
`nativeLibraryDir`. Design, CLI contract, data layout and limits: [docs/engine.md](../../docs/engine.md).
Progress, evidence and the device matrix: `docs/report/initial/w2-engine.md`.

| Path | Content |
| --- | --- |
| `src/`, `include/engine/`, `loader/` | engine and loader sources (C17) |
| `third_party/zstd-1.5.7/` | vendored zstd single-file decoder (BSD) for the installer |
| `gen/` | syscall inventory generator (Linux v7.2.8 tables) |
| `test/` | host tests: `../test-host.sh` suites, oracles in `test/guest/`, `accept-host.sh` acceptance |
| `device/`, `android-harness/` | on-device harness (real app process), AVD scripts, case files |
| `build-android.sh` | NDK cross-build for arm64-v8a and x86_64 |

```sh
native/engine/test-host.sh                 # host suites (needs artifacts/engine/rootfs-amd64 and the amd64 image)
native/engine/test/accept-host.sh          # acceptance with the real workspace image (network)
native/engine/gen/check.sh                 # syscall inventory
native/engine/build-android.sh             # -> artifacts/engine/android/<abi>/
```
