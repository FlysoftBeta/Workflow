# Workflow-owned root proxy

`runtime/local/LocalProxyManager.kt` owns the local Mihomo integration. It does not call the Termux/Python companion. The configuration and process audit data are in the same Workflow workspace used by the editor and local runtime:

- `.workflow/proxy/config.yaml`: the user's unmodified configuration bytes.
- `.workflow/proxy/.process.json`: verified child/guardian PID, kernel start ticks, executable, workspace path and run UUID. This is an audit record, not authority to kill a PID.
- `.workflow/proxy/runtime.log` and `runtime.previous.log`: bounded, rotated kernel diagnostics.

Mihomo and the guardian are PackageManager-managed executables named `libmihomo.so` and `libworkflow_proxy_guard.so` under `ApplicationInfo.nativeLibraryDir`. They are not copied into a writable directory and then executed. The guardian's NDK build is reproducible through `app/src/main/cpp/proxy_guard/build.sh`; see its adjacent README for the complete control protocol and tests.

## User configuration

Import and editing require explicit user actions. The manager does not merge overrides, inject TUN settings, replace DNS servers, install certificates or alter global Android proxy/DNS/CA settings. An optional starter template is created only when no configuration exists; it has `tun.enable: false` and never starts itself. Existing configurations are not replaced by the template action.

The kernel checks the actual file through its `-t` mode before a start. The manager keeps a configuration digest across validation and root authorization so a changed file must be checked again. Mode and proxy-group selection use Mihomo's live REST API; their success is read back from the controller. They do not rewrite the YAML.

The controller address and secret are read from YAML using a safe constructor with input-size, alias and nesting bounds. The only accepted controller destinations are device loopback. Wildcard binds are contacted over loopback instead of their wildcard address. A nonlocal address is rejected for controller access, and the secret is never sent to that host. No remote subscription, proxy-group or rule payload can change this destination policy. A running configuration without a usable local controller is reported separately from a reachable controller; its existence is not fabricated into group/mode data.

## Root process ownership

Only explicit start/stop operations may request `su`. Opening the page, refreshing, reading the version, checking configuration, or loading existing groups does not trigger a root prompt. Start verifies a real UID 0 result and then invokes the guardian with separately quoted arguments. Both guardian control stdout and kernel stderr are continuously drained and kept separate.

The guardian forks its own Mihomo child, changes into the configuration directory, gives the child `/dev/null` stdin, and verifies exec completion and `/proc` identity before emitting `started`. It holds the direct child unreaped until cleanup is complete. Normal stop travels over the original private stdin pipe and must match the guardian's own UUID, PID and start ticks. The guardian rechecks identity and sends TERM, then KILL if required, before actually reaping. This ownership avoids the old-kernel race of reading a PID file and later calling kill on a reused PID. There is no arbitrary-PID stop command.

EOF on the app-owned control pipe, guardian TERM/HUP/INT, or explicit stop cleans the owned child. Parent-death signaling is a fallback for guardian death. Forced KILL cannot promise that Mihomo completed graceful route restoration; forced or indeterminate shutdown remains visible. A stored identity whose live guardian channel is gone is only inspected, never used as an unqualified kill target. The manager refuses to launch a duplicate process when the previous matching kernel is still present.

Active runs retain the independent `LocalRuntimeService` foreground service. It is not tied to a settings page, an Activity or the floating overlay. No boot-start policy, root auto-grant or battery-policy change is installed.

## Interface

```kotlin
val proxy = LocalProxyManager.get(context)
val state: StateFlow<LocalProxyState> = proxy.state
proxy.version()                  // Result<String>, no root
proxy.checkConfig()              // Result<ProxyConfigCheck(valid, output)>, no root
proxy.refresh()                  // Result<LocalProxyState>, no root
proxy.readConfig()               // Result<String>
proxy.importConfig(bytes)        // explicit replacement; Result<Unit>
proxy.writeConfig(text)          // explicit replacement; Result<Unit>
proxy.createTemplate()           // refuses an existing config; Result<File>
proxy.start()                    // explicit root operation
proxy.stop()                     // stops the owned guardian session
proxy.groups()                   // Result<JSONObject>, real /proxies response
proxy.setMode("rule")            // rule / global / direct, confirmed readback
proxy.select(group, proxyName)   // selection confirmed by that group response
```

`LocalProxyState` separates running, busy, configuration/kernel presence, last root result, controller reachability, PID, version, actual mode and diagnostics. `started` is proof of an executed root child; it is not by itself evidence of working TUN, routing, DNS or internet access.

## Verification boundary

The native guardian passes real host-process tests covering stream separation, ordinary and forced stop, EOF, guardian death, invalid controls, executable changes, exec failure and start-time parsing. Those tests do not establish Android root/TUN behavior. Device acceptance must first check packaged kernel version and configuration without changing routes. Actual root proxy/TUN start, route changes and shutdown require an explicitly coordinated device/network save point. Do not use a host loopback tunnel or a successful `su` binary lookup as proof that the device proxy is working.

### Physical-device read-only result · 2026-09-26

`LocalProxyExecutableTest` passed on the authorized Android 9 arm64 device as part of the final **9/9** headless suite (`artifacts/physical-review/headless-runtime-tests.log`). In Workflow's own UID, it verified the packaged Mihomo **1.19.31** executable, validated an isolated profile with TUN disabled, and confirmed that the production guardian rejects a non-root caller. The fixture was isolated from the user's profile and removed afterward.

That initial headless pass did not start a root process. The subsequent 0.2.1 opt-in `LocalProxyRootFixtureTest` passed on the same physical device after Workflow's previously denied Magisk permission was explicitly enabled through its UI. It exercised the real manager and root guardian, authenticated loopback REST, global/direct/rule mode readback, the QA selector's DIRECT/REJECT behavior against a local echo origin, stop, restart with a new run identity, port closure, foreground lease release and fixture removal. The production profile remained unchanged.

The fixture disabled TUN, DNS and transparent ports. The device already had an unrelated active `Meta` interface and table 2022. Full before/after `ip link`, `ip rule` and route outputs were identical, so that existing TUN is not counted as a Workflow result. TUN forwarding and route installation/restoration by Workflow remain a separate unverified boundary. Evidence: `artifacts/physical-review/0.2.1-root-proxy-final.log`, `0.2.1-root-proxy-final-result.json` and `network-*-root-*.txt`.

The manager preserves an explicit `stopUnconfirmed` state if the guardian channel dies without a verified child-exit record. It retains audit identity and does not claim the proxy is stopped merely because a wrapper process disappeared. There is also a remaining configuration concurrency limit: an external edit after the final digest check but before Mihomo opens the profile can change the bytes it reads. This version does not claim an immutable per-run profile snapshot.
