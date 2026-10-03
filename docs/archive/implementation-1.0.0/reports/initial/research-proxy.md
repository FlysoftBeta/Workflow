# Proxy feature research — Mihomo/CMFA-equivalent for Workflow

Scope per brief: root-mode Mihomo (Clash.Meta) integration providing Rule/Global/Direct
modes, proxy-group node selection with latency test, and related visibility — explicitly
**without** a config-override feature (the user edits `config.yaml` themselves). All
findings below are grounded in the vendored Mihomo v1.19.31 source tree
(`runtime/vendor/mihomo/mihomo-source-v1.19.31.tar.gz`), the current app code, and one
GitHub API query (read-only, `api.github.com`, no repo access needed). No device
network-changing commands were run; the tablet route/rule dumps referenced below are the
pre-existing `artifacts/physical-review/network-*-root-*.txt` files from the 2026-09-26
fixture pass.

## 1. Mihomo kernel version

`runtime/vendor/mihomo/manifest.json` pins **v1.19.31**. `GET
https://api.github.com/repos/MetaCubeX/mihomo/releases/latest` returns tag `v1.19.31`,
published 2026-09-14T13:34:02Z — **this is already the latest stable release**, no update
needed. Asset identity was verified byte-for-byte:

| asset | vendored sha256 | GitHub sha256 | size |
|---|---|---|---|
| `mihomo-android-arm64-v8-v1.19.31.gz` | `de00bc53…7904f6` | `de00bc53…7904f6` | 21,808,349 |
| `mihomo-android-amd64-v1.19.31.gz` | `6f6ebcb3…4492f99` | `6f6ebcb3…4492f99` | 23,407,524 |

License is GPL-3.0 (`runtime/vendor/mihomo/LICENSE`, full GPLv3 text present); the
matching source archive (`mihomo-source-v1.19.31.tar.gz`, sha256 also verified against the
manifest) is vendored, which satisfies GPL-3 corresponding-source obligations since the
app execs Mihomo as a separate process rather than linking it. No action item here beyond
re-checking `releases/latest` periodically (`tools/vendor-editor-grammars.py`-style script
does not yet cover Mihomo; there is no `update-mihomo.py`, only the Codex-oriented
`tools/update-protocol.py` — worth a small follow-up script, not urgent).

## 2. CMFA feature inventory vs. scope

Source: DeepWiki `MetaCubeX/ClashMetaForAndroid` settings page + general knowledge,
cross-checked against Mihomo's own `hub/route/*.go` (the REST surface both CMFA and
Workflow ultimately depend on).

| CMFA feature | REST evidence (mihomo source) | In scope for Workflow | Notes |
|---|---|---|---|
| Rule/Global/Direct mode | `PATCH /configs {mode}` | **Yes** | Already implemented. |
| Script mode | `mode: script` | No | Config-only edge case; not a UI concern. |
| Proxy-group list + node select | `GET /proxies`, `PUT /proxies/{name}` | **Yes** | Already implemented. |
| Latency test (single node / group url-test) | `GET /proxies/{name}/delay`, `GET /group/{name}/delay` | **Yes — missing today** | Biggest concrete gap; endpoints exist, UI/manager code does not call them. |
| Traffic/speed indicator | `GET /traffic` (streaming) | **Yes, minimal** | One line of up/down throughput; not a full graph. |
| Connections list (kill connection) | `GET/DELETE /connections` (WS) | Optional/stretch | Useful for debugging; not essential for "similar to CMFA minus override." Recommend a simple list + close, no filters UI. |
| Logs viewer | `GET /logs` (streaming) + on-disk `runtime.log` | **Yes, minimal** | `runtime.log` already exists on disk (rotated); just needs a viewer. Live `/logs` stream is a nice-to-have once controller is reachable. |
| Proxy/rule provider update | `PUT /providers/proxies/{name}`, `.../healthcheck`, `PUT /providers/rules/{name}` | **Yes** | User's config defines providers; Workflow only needs to trigger refresh + show last result, never edit the provider list itself. |
| Profile/subscription manager (add/switch multiple profiles, auto-update) | N/A (CMFA-side feature, not Mihomo) | **No** | Out of scope by design — "single config from `.workspace`," no profile switcher, no subscription fetching. This *is* the config-override-adjacent territory the brief excludes. |
| DNS/fake-ip config | `config.yaml` `dns:` block | **No UI**, doc note only | Config-only; Workflow should not add a DNS settings screen, just document fake-ip implications for the TUN section (see §4). |
| Per-app proxy (include/exclude) | `tun.include-package/exclude-package/include-uid-range/…` (see §4) | **Yes, config-only** | Mihomo's own Android build already resolves package↔uid; no custom code needed, just documentation of the YAML keys. |
| System-proxy-less TUN | `tun.enable/auto-route/auto-redirect` | **Yes** | Core of §4; this is the "run TUN under root" requirement. |
| Config override (static host/key mapping) | CMFA `override` | **Explicitly excluded** | Per requirement. |
| Geo-database import, Age-encrypted config, dark mode, launcher icon toggle, notification style | — | **No** | Cosmetic/CMFA-specific, not part of "similar to CMFA minus override." |

## 3. Audit of existing implementation

| Component | Verdict | Findings |
|---|---|---|
| `app/src/main/cpp/proxy_guard/proxy_guard.c` + `build.sh` + `README.md` | **Keep** | Well-hardened: `waitid(WNOWAIT)` PID-reuse-proof ownership, `PR_SET_PDEATHSIG`, exec-result verification via a CLOEXEC status pipe, fixed-string JSON protocol immune to path/secret injection, explicit stop-identity matching (run id + pid + start ticks). No bugs found. Host test suite (`runtime/android/tests/proxy_guard_run_tests.sh`) covers the relevant edge cases. One tuning note for TUN: `STOP_GRACE_MS` is hardcoded to 2000 ms; once TUN/`auto-redirect` (which sets up nftables rules) is actually exercised, re-measure whether 2 s is enough for Mihomo's own graceful teardown before the guardian escalates to `SIGKILL` — a `SIGKILL` before teardown finishes can leave `ip rule`/nftables state behind (guardian's own README already documents this limitation honestly). |
| `LocalProxyManager.kt` | **Keep, extend** | Core lifecycle/identity logic is sound (digest-checked config, no PID-file trust, foreground-service refcounting, redacted logs/errors). Concrete gap, not a bug: it never inspects the `tun:` block of the user's config at all — no TUN-aware warnings, no coexistence check, no distinct handling before `start()`. This is the main code addition needed for §4, not a rewrite. |
| `LocalProxyConfig.kt` | **Keep, extend** | `SafeConstructor`-based YAML parsing with size/alias/nesting limits is correct and matches Mihomo's own YAML surface. Only reads `external-controller`/`secret` today; needs a parallel, equally defensive read of `tun.enable` (and optionally `iproute2-table-index`/`iproute2-rule-index`/`device`) for the same "read-only, never rewrite" pattern — no architectural change. |
| `ui/ProxyScreen.kt` | **Keep, extend** | Clean single-screen Material 3 layout already close to the target UI. Missing, relative to CMFA-equivalent scope: delay/url-test button per node and per group, traffic/speed readout, a way to view `runtime.log`, and a provider-refresh action. No per-app-proxy picker is needed (config-only, see §2). |
| `tools/run-mihomo.sh`, `tools/package-proxy.sh`, `runtime/proxy/install-proxy-termux.sh`, `runtime/workflow_daemon/proxy.py` | **Keep as optional compat, do not extend** | Confirmed these are the "historical Termux/HTTP tooling" AGENTS.md calls optional. `run-mihomo.sh` invokes `su -c "$kernel_command"` **directly, with no guardian** — no WNOWAIT PID pinning, no stop protocol, no identity verification. That is an acceptable trade-off *only* because it is explicitly advanced/manual tooling; it must never be surfaced as a default path in the app UI, and its own header comment should say so (currently it doesn't — a documentation nit worth a follow-up). |
| `runtime/proxy/config.example.yaml` | **Fix (concrete risk)** | This legacy example — still referenced by `install-proxy-termux.sh` and the "advanced compatibility" docs — defaults to `tun.enable: true`, `auto-route: true`, `auto-detect-interface: true`, and `dns-hijack: [any:53]`, i.e. it silently claims default routing and DNS the moment it's started. This is inconsistent with, and materially less safe than, the in-app `LocalProxyManager.createTemplate()` default (`tun.enable: false`, never self-starts). Recommend flipping this example to `tun.enable: false`/`auto-route: false` to match the in-app template's safe-by-default posture — a one-line-per-key fix, flagged here rather than changed, since editing it is outside this research task's remit. |
| `docs/android-local-proxy.md`, `docs/root-proxy-device-acceptance.md` | **Keep** | Read closely against the actual code; both are accurate — no claims found that outrun real behavior. Neither yet documents TUN coexistence (expected, since it isn't built); §4 below is meant to become that section once implemented. |

## 4. Root TUN design for Android 9 (kernel 4.14)

### What Mihomo's TUN actually does, read from `listener/sing_tun/*.go` and
`listener/config/tun.go` in the vendored source

- The **Android build actually used here is compiled with the `android` tag and *without*
  `cmfa`** (`server_android.go`: `//go:build android && !cmfa`). That build already wires
  `tun.PackageManager` (`getPackageManager()` → `sing-tun`'s own Android package-manager
  binding) into `BuildAndroidRules`, and registers
  `process.DefaultPackageNameResolver = findPackageName` (uid→package name via `pm % 100000`).
  In plain terms: **per-app proxy is a native Mihomo/sing-tun capability on this exact
  binary**, driven entirely by config keys already present in `listener/config/tun.go`:
  `include-package` / `exclude-package` / `include-android-user` / `include-uid[-range]` /
  `exclude-uid[-range]`. Workflow needs **zero custom per-app-routing code** — just
  documentation of these keys, since the user edits the YAML themselves (no override
  feature needed here at all).
- `var InterfaceName = "Meta"` in `listener/sing_tun/server.go` is the **literal default
  TUN device name Mihomo picks**, auto-incremented (`Meta`, `Meta1`, `Meta2`, …) via
  `CalculateInterfaceName` if the name is taken. The tablet's current live
  `ip link`/`ip route` dump (`artifacts/physical-review/network-final-root-link.txt`,
  `…-route.txt`) shows an interface literally named `Meta` already up, with `default dev
  Meta table 2022` and rule priorities `9000/9001/9010`. This is almost certainly CMFA's
  own Mihomo-derived TUN using its own defaults. **Name collision is self-avoiding**
  (auto-increment); **table/priority collision is not** — if Workflow's own config also
  leaves `iproute2-table-index`/`iproute2-rule-index` at Mihomo's defaults, both kernels
  could contend for the same policy-routing table/rule range.
- `listener/config/tun.go` exposes exactly the knobs needed to avoid that: `iproute2-table-index`,
  `iproute2-rule-index`, `auto-redirect-input-mark`, `auto-redirect-output-mark`,
  `auto-redirect-iproute2-fallback-rule-index`. `redirect_linux.go` confirms
  `auto-redirect` (nftables TCP redirect + TUN for UDP) is compiled in for Linux/Android.

### Design

1. **Never touch CMFA.** No stop/route/iptables/DNS change to the existing `Meta`
   interface, ever, under any code path — this matches the hard safety rule already in
   force for this research session and should carry into the shipped feature.
2. **Detection is root-free and passive.** Before offering to start a TUN-enabled config,
   enumerate `java.net.NetworkInterface.getNetworkInterfaces()` (no permission, no `su`)
   and check for an existing interface whose name matches Mihomo/sing-tun's own naming
   convention (`Meta`, `Meta\d+`, `tun\d+`) that Workflow did not itself create this run.
   If found, surface a **visible warning banner** in `ProxyScreen` ("检测到疑似其他 VPN/TUN
   接口 `Meta` 正在运行；Workflow 不会覆盖其路由") rather than silently proceeding — this
   satisfies "detect and warn; never hijack silently" without needing root just to check.
   A supplementary root, read-only `ip rule show` / `ip route show table all` scan (only
   run at the moment the user already grants root for their own start) can additionally
   confirm the specific table/priority numbers in use, to pick a template default that
   provably avoids them (see below).
3. **Ship a safe, non-default, collision-avoiding template.** `LocalProxyManager.createTemplate()`
   already sets `tun.enable: false`; when a user turns TUN on themselves, the *documentation*
   (not an override) should recommend explicit `iproute2-table-index`/`iproute2-rule-index`
   values clearly outside the ranges observed on this device class (CMFA/Android's own
   VPN plumbing was seen using tables `2022`, `1002`, `1015`, `99`, `97`, `98` and rule
   priorities `0, 9000, 9001, 9010, 10000, 10500, 13000, 14000, 15000, 16000, 17000,
   19000, 22000, 32000`) — e.g. table `9601`, rule index `21000`, and a distinct
   `auto-redirect-*-mark` fwmark (observed fwmarks in use: `0xc0000/0xd0000`, `0x10063`,
   `0x1006c`, `0x6c`). This is pure documentation/config guidance, consistent with "the
   user edits the config file himself."
4. **Scope by UID when the goal is "just Workflow's own traffic."** Since the vendored
   binary already resolves package names to uids, a config using
   `include-package: [top.flysoftbeta.workflow]` (plus optionally the terminal/runtime's
   own child processes, which inherit the app's uid) is the sensible default scope for
   "per-app bypass" mentioned in the brief — no custom `ip rule` code needed, just a
   documented recommended snippet.
5. **Cleanup guarantees, re-examined against the actual guardian code:** `run_child()` sets
   `PR_SET_PDEATHSIG(SIGKILL)` tied to the **guardian's** pid, not the app's — so if the
   *app* process dies (not the guardian), the guardian's stdin pipe gets EOF, which the
   guardian already treats as a stop request (`supervise()`: `length <= 0 → stopping =
   true`), giving Mihomo a **graceful SIGTERM** first (2 s grace) — i.e. TUN/route
   teardown still gets a chance to run even if Workflow itself crashes, as long as the
   guardian process (a separate `su`-spawned session leader) survives. The one real residual
   gap is **guardian death itself** (already called out in the existing README): Mihomo
   then gets an immediate `SIGKILL` via `PDEATHSIG` with no graceful route/nftables
   cleanup chance. Mitigation, consistent with the existing "never blind-kill by PID"
   philosophy: on the *next* `start()`, before adding new state, the manager should
   reconcile only the exact `iproute2-table-index`/`iproute2-rule-index`/fwmark values it
   itself is configured to use (read-only `ip rule show`/`ip route show table <n>` to
   confirm, then let Mihomo's own startup — which already replaces/creates its known
   tables — take over) — never a generic `ip rule flush`.
6. **External-controller secret / bind:** already correctly loopback-only and
   safely parsed (`LocalProxyConfig.controller`); nothing to change here.
7. **State/log layout:** stays exactly `.workspace/.workflow/proxy/{config.yaml,
   .process.json, runtime.log, runtime.previous.log}` per the existing architecture
   contract; no new top-level paths are needed for TUN — it's config-driven, not a
   separate subsystem.
8. **Android 9 / kernel 4.14 specifics:** `redirect_linux.go` hardcodes
   `supportRedirect = true` for Linux builds (no kernel-version gating in Go); the real
   constraint is whether the 4.14 kernel's nftables support is sufficient for
   `auto-redirect` — this can only be confirmed by an actual on-device TUN-enabled run
   (out of scope for this read-only research pass, but should be the first thing the
   coordinated device fixture measures once implemented, per `docs/root-proxy-device-acceptance.md`'s
   own stated boundary that its current pass is TUN-disabled).

## 5. UI spec (single screen, Material 3, 1920×1200 tablet)

Keep `ProxyScreen.kt`'s existing structure (status card → config section → mode segmented
row → group list) and extend rather than redesign:

- **Status card** (unchanged shape): add a one-line traffic readout (`↑12 KB/s ↓340 KB/s`)
  next to the version text once `controllerReachable`, sourced from a single lightweight
  poll of `GET /traffic`'s first frame rather than keeping a permanent streaming
  connection open (simpler lifecycle, matches the rest of the manager's poll-based
  `refresh()` pattern). If interface enumeration (§4.2) finds a foreign TUN interface,
  show a dismissible warning row directly under the status card, not a blocking dialog.
- **Mode row:** unchanged (already correct).
- **Node rows:** add a small trailing latency chip per group (`128 ms` / `超时`, tap to
  re-test just that group via `GET /group/{name}/delay`); inside the node picker sheet,
  add a per-node delay chip using `GET /proxies/{name}/delay`, and a "全部测速" action at
  the top of the sheet that fires the group url-test. On a 1920×1200 tablet the existing
  `LazyColumn` with 24 dp side padding has ample width — the node picker `ModalBottomSheet`
  should switch to a fixed side sheet or two-column grid above ~840 dp width so latency
  chips don't get cramped; this is a layout-breakpoint tweak, not a new component.
- **New "运行日志" row** under the config section (only when `configExists`): opens a
  bottom sheet tailing `runtime.log` (already on disk, just needs a read + `SelectionContainer`,
  mirroring the existing "检查配置" sheet pattern).
- **New "更新订阅/规则" action** (only shown if the config actually defines
  `proxy-providers`/`rule-providers` — detected the same safe-YAML-read way as
  `external-controller`): triggers `PUT /providers/proxies/{name}` and
  `/providers/rules/{name}` for each, shows a snackbar with per-provider success/failure.
- **Connections (optional/stretch):** a single flat list (name, host, rule, up/down
  bytes) behind a "连接" row, with a close (×) affordance per row calling `DELETE
  /connections/{id}`; skip filtering/sorting UI — CMFA's connections screen is more
  elaborate than this scope needs.
- No DNS, no profile switcher, no override editor, no per-app picker UI — all
  intentionally left to the YAML file per the brief.

### Controller API surface needed (all through the existing `request()` helper in
`LocalProxyManager`, same auth/loopback pattern already in place)

```
GET  /traffic                          → one JSON frame {up, down}, short-poll
GET  /proxies/{name}/delay?timeout&url → per-node latency
GET  /group/{name}/delay?timeout&url   → per-group url-test (all members)
GET  /providers/proxies                → list configured providers (names only, to decide whether to show the update row)
PUT  /providers/proxies/{name}         → trigger provider refresh
GET  /providers/proxies/{name}/healthcheck → post-refresh latency summary
PUT  /providers/rules/{name}           → trigger rule-provider refresh
GET  /connections                      → list (optional/stretch)
DELETE /connections/{id}               → close one (optional/stretch)
```

All of the above are already implemented server-side in the vendored kernel
(`hub/route/{proxies,groups,provider,connections}.go`); nothing needs to be added to
Mihomo itself, only to `LocalProxyManager`'s Kotlin wrapper and `ProxyScreen`.
