# Local proxy

The Rust Workspace Engine owns proxy configuration, template secrets, desired state, operation tickets and acknowledged measurements. Android's `ProxyService` executes local operations for the current connection and exposes the JVM `:proxy` module's `ProxyApi` through `AppGraph.proxy`. Mihomo runs as a separate Root process on the device, outside the development container. Opening a panel, reading configuration, validating it, or inspecting status does not request Root. Only an explicit start or confirmed stop may use Root.

## Configuration and panels

The single configuration file is `.workspace/services/proxy/config.yaml` beneath the workspace root. Reads use Engine documents with namespace `services.proxy` and key `config.yaml`; initial creation and import use lease-protected Engine service commands, and the editor opens that same file. A stale revision fails explicitly. Android stages the returned bytes under `cache/proxy-executors/<connection>/` for Mihomo's working directory, but this cache is disposable and cannot rescue a failed authoritative read.

Relative file-provider paths resolve within the workspace proxy service directory. Before validation, start, or explicit provider update, the executor reads their bounded contents through the Engine and stages them. Missing or unreadable providers cannot be replaced with stale cache contents. Users edit YAML directly; the application does not rewrite their configuration, manage subscription accounts, or migrate historical proxy settings.

The first edit action asks Engine to create a safe template once: TUN disabled, a loopback controller, a random secret, independent ports, and a dedicated TUN routing range. Import performs an explicitly confirmed atomic replacement. A running kernel uses new configuration only after restart.

The overview provides start/stop, rule/global/direct modes, group/node selection, latency checks, and live transfer rates. When stopped it shows disabled modes and nodes from the configuration, optionally retaining the same configuration's last measured selection for reference. Logs and connections open separate panels in the current stack; closing those panels does not stop the kernel. Logs support selection and copying, while read-only connections display destination, routing chain, traffic, and age with one-second updates. Provider refresh uses the configuration's existing file/HTTP providers, reports failures per provider, and reloads nodes afterwards.

## Measured state and connection lifecycle

Starting first runs unprivileged `mihomo -t`, then checks local ports, other VPN/TUN owners, and configuration changes. Root is available only after an actual UID-0 result, not because `su` exists. An executable on disk does not establish a running kernel or functioning TUN. Reports distinguish controller reachability, TUN interface appearance, and routing-rule checks.

The executor registers a connection-instance ID and receives a boot-bound epoch. Repeated registration by the same active instance is idempotent; another instance or a new Engine process retires the old epoch and interrupts pending work. This is stale-instance fencing, not authentication.

Every device operation first requests an Engine ticket through `services.command`. Engine commits desired running state, mode or node selection and supplies canonical configuration when needed. The Android executor performs local root, TUN, kernel and controller operations, then submits `services.complete` with the pending ID, epoch and measured state. Successful start, stop, mode and node selection require corresponding measured fields. Failures retain desired intent and record the actual failure; unknown outcomes are never automatically replayed.

`services.report` can publish additional measurements but cannot complete a pending ticket. The UI uses acknowledged state and `control.proxy`, which separates desired state, operation status, active executor epoch and measured confirmation. Reports omit controller secrets, subscription addresses and latency-test URLs. A connection failure means unconfirmed state; it cannot appear as a confirmed stop or permit another start. Disconnect retires its lease when possible and closes only its own guardian pipe, without probing Root again or signaling a historical PID. Emergency cleanup of the currently owned process remains possible after connection failure, but cannot manufacture an Engine receipt.

The foreground-service lease follows the lifetime of the guardian for that run. A UUID, PID, and `/proc` start time identify control messages. The cache's `.process.json` is diagnostic metadata, not permission to kill a process later by a remembered PID.

## Guardian ownership and cleanup

The native guardian supervises only the Mihomo child it forked. `/proc` identity checks, `waitid(WNOWAIT)`, and retention of the unreaped process-group leader prevent PID reuse from redirecting a signal to an unrelated process. On application death, pipe EOF normally sends SIGTERM to that owned group, escalating to SIGKILL after two seconds if it remains alive.

For an explicitly named TUN device with `auto-route`, the guardian receives the run's routing table and rule-priority range. It checks through netlink that the scope is empty before launch. After kernel exit it removes only complete rule records matching that run's table/device scope. It never flushes a table, removes another interface, or changes system DNS. Mihomo's normal exit handles its remaining resources; a nonpersistent TUN disappears when its file descriptors close.

If a conflicting target or interface appears in the owned rule range before stopping, the guardian kills its own kernel directly and performs exact cleanup. This avoids Mihomo's graceful shutdown removing later rules merely because their priorities overlap. If rules cannot be read, scoped rule cleanup is not permitted.

Killing the guardian itself with SIGKILL triggers the child's parent-death signal, but the guardian can no longer execute cleanup. Residual rules in the reserved range block a later start; the application does not guess that they are safe to reclaim. Automatically named devices, auto-redirect/nftables, and external Root managers killing process trees do not have the same forced-exit cleanup guarantees. The template uses an explicit device name and disables `auto-redirect`. Real-device Root-manager behavior still requires separate acceptance evidence.

## Coexistence and redaction

An existing VPN or TUN owner prevents TUN startup, with no force-start option. The executor repeats conflict checks after Root authorization. Existing use of the target table or priority range blocks startup, as does an inability to inspect it. The template uses device `workflow-tun`, table 9500, and priority 9500 with the following ten priorities reserved. Package inclusion/exclusion, UID ranges, and DNS policy remain Mihomo YAML settings; the application never automatically changes another proxy.

A point-to-point TUN without carrier still counts as an owner even when Android temporarily reports it down. The Root recheck uses `ip -o link show` to include addressless interfaces omitted by Java enumeration. Failure, empty output, or unparseable output blocks startup.

Local `runtime.log` rotates to `runtime.previous.log` after 1 MiB. The log panel submits a bounded redacted copy through the epoch-protected `publishLog` service command with the canonical document revision. It updates `.workspace/services/proxy/runtime.log`, sharing its service file with the editor. Configuration reads, diagnostics, live logs, and provider errors redact controller secrets, node authentication, and subscription URLs, including encoded values and values split across read chunks. Configuration contents and full controller requests/responses are not logged or copied into backups and test evidence.

## Verification boundary

`third_party/mihomo/manifest.json` pins the kernel and source provenance. `:proxy:test` exercises ticket sequencing, stale-epoch failures, configuration staging and JVM executor behavior; Rust local-service tests cover desired-state persistence and measured completion, and `native/test-host.sh proxy-guard` exercises guardian mechanics. Android tests include the packaged executable, panel, Root TUN, provider updates, and cleanup. Root/panel acceptance requires explicit instrumentation arguments, an isolated ranchu/goldfish emulator, and `adb root`; it is not a test for the user's daily tablet.

The [initial proxy report](../archive/implementation-1.0.0/reports/initial/w9-proxy.md) records the original module's evidence. The [client-services report](../archive/implementation-1.0.0/reports/rewrite/client-services.md) and [service acceptance report](../archive/implementation-1.0.0/reports/rewrite/service-acceptance.md) record Engine ownership and later integration checks, including actual application force-stop and preservation of external rules. None establishes arbitrary OEM Root-manager behavior. [Status](../status.md) states the remaining device matrix.
