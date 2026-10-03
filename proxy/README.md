# Proxy service library

`:proxy` is a pure Kotlin/JVM library for Mihomo configuration, controller access and owned-process
supervision. It does not depend on Android, `:agent` or `:core`. Android supplies the platform ports;
the Engine supplies the canonical workspace configuration, provider assets and published service
state. The library's local files are disposable executor staging and bounded logs.

The source root is `src/main/kotlin/top/flysoftbeta/workflow/proxy/`. `config/` inspects YAML and
controller/TUN settings; `controller/` implements the local HTTP/WebSocket API; `guardian/` handles
the identity and control protocol; `network/` detects interface and route conflicts; `redact/`
removes secrets from diagnostics; and `io/` bounds executor IO. `runtime/ProxyRuntime.kt` owns the
kernel lifecycle through injected ports. `runtime/WorkspaceProxyApi.kt` synchronizes the executor
with Engine-owned documents and reports.

A requested setting is not proof that root, controller or TUN capabilities work. The runtime
publishes observed results and keeps uncertain stops visible. It signals only the process owned
by its guardian channel and never takes over another app's VPN/TUN. The guardian's native
implementation and wire protocol are documented in
[`native/proxy-guard/`](../native/proxy-guard/README.md).

`src/test/` uses isolated processes, local controllers and fake platform ports. Run it from the
repository root:

```sh
flock artifacts/.gradle.lock ./gradlew :proxy:test
```

These tests do not replace Android root/TUN acceptance. Those cases live in `app/src/androidTest/`
and run only on an explicitly gated isolated emulator. See the
[proxy guide](../docs/implementation/proxy.md) and [testing guide](../docs/development/testing.md).
Generated classes and reports belong in the ignored `build/`.
