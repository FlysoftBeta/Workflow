# Android native adapters

This directory contains the Android PTY/JNI adapter and the root proxy guardian. The production
container is Rust under [`engine/environment/runtime/`](../../engine/environment/runtime/README.md); no archived C container
source is a build input here.

`pty/pty_process.c` implements process, PTY and lifetime primitives. `pty/pty_jni.cpp` exposes them
to Android, and `pty/CMakeLists.txt` builds `libworkflow_pty.so` with 16 KiB-compatible load segments.
Its host tests live in `pty/tests/`. The JNI library is distinct from the executable guardian:
`proxy-guard/proxy_guard.c` builds an executable named `libworkflow_proxy_guard.so` so Android can
extract it into `nativeLibraryDir`. It must be executed, never loaded as a JNI library.

The [guardian README](proxy-guard/README.md) defines its launch, identity, control and shutdown
protocol. `proxy-guard/tests/` contains parser tests, a fake kernel and the supervisor test driver.
The host-test compile flag belongs only to those test binaries; the production guardian requires
effective UID 0 before forking.

The lightweight host suite uses the system C compiler and needs no device or root access:

```sh
app/native/test-host.sh
```

By default it writes into ignored `artifacts/native-host/`; `NATIVE_HOST_BUILD_DIR` selects an
isolated output directory. Android builds use the pinned NDK through the app's build tasks.
The [testing guide](../../docs/development/testing.md) distinguishes these host checks from isolated
Android PTY and root/TUN acceptance.
