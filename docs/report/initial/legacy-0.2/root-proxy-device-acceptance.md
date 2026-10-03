# Root proxy acceptance without touching the user's VPN/profile

Status: **physical device fixture passed on 2026-09-26, Workflow 0.2.1**. The opt-in fixture ran the actual manager under app UID 10087 and confirmed two owned root runs, mode/group controls, DIRECT/REJECT loopback forwarding and complete stop/port/foreground-owner cleanup. The production configuration was unchanged and the isolated directory removed. See `artifacts/physical-review/0.2.1-root-proxy-final-result.json`.

Only the coordinated QA operator runs these steps. This pass is deliberately TUN-disabled and uses loopback traffic exclusively. It must not be reported as TUN/routing acceptance. The physical device already has an unrelated active `Meta` TUN and policy table 2022; full interface/rule/route baselines were unchanged and are not evidence for Workflow. Background Home retention was not exercised by this bounded fixture and remains a separate UI check.

## Isolation prerequisite

The production `LocalProxyManager.get(context)` is fixed to `.workspace/.workflow/proxy`. Do not back up, replace, test, and restore the user's profile there: restoration can lose concurrent edits or disturb an existing run.

`LocalProxyManager.isolatedForTesting()` exercises the actual manager in a new canonical `.workspace/.state/runtime/proxy-qa/<UUID>` child. The production factory remains unchanged. It rejects an existing directory or a symlink escaping that subtree. `LocalProxyRootFixtureTest` uses this seam rather than substituting a raw guardian process.

Root-starting instrumentation requires `-e rootProxy true`, and otherwise skips. It is not an automatically starting member of the ordinary headless suite. The test writes a redacted result to `.workspace/.state/runtime/root-proxy-fixture-result.json`; its generated controller secret is excluded.

## Before the run

1. Coordinate a save point with the UI operator; do not restart/force-stop Workflow while it owns user terminals or Codex work.
2. Record the existing VPN owner/interface and read-only network baseline: IPv4/IPv6 rules, routes across all tables, interface addresses, Android global proxy, and private DNS settings. Keep an existing VPN running. Do not dump authentication material into the evidence.
3. Record SHA-256 and existence of the production proxy profile; do not read or print its secret. Record existing production manager state if present.
4. Allocate distinct unused high loopback ports inside the app for controller, mixed proxy and a tiny HTTP echo origin. Close only the allocation sockets that the kernel will need to bind. Keep the origin open. An occupied port or other preflight failure aborts the fixture; do not stop the conflicting owner.
5. Generate a fresh controller secret in memory. Do not reuse a user's profile/secret.

## Isolated profile

Use the allocated ports in this literal fixture; no subscriptions, providers, DNS interception, automatic routes or geodata rules are needed:

```yaml
mixed-port: <mixedPort>
port: 0
socks-port: 0
redir-port: 0
tproxy-port: 0
bind-address: 127.0.0.1
allow-lan: false
external-controller: 127.0.0.1:<controllerPort>
secret: "<fresh random secret>"
mode: rule
log-level: warning
find-process-mode: off
tun:
  enable: false
dns:
  enable: false
proxies: []
proxy-groups:
  - name: QA
    type: select
    proxies: [DIRECT, REJECT]
rules:
  - MATCH,QA
```

First call `version()` and `checkConfig()` as the app UID. The exact fixture hash must remain unchanged. Neither operation should invoke `su`.

## Positive manager/guardian pass

- Explicitly call `start()` from a user/QA foreground action. The QA operator may grant the root prompt under the user's existing authorization.
- Require a verified guardian `started` event with UID 0, matching UUID, child/guardian PID and start ticks. Confirm the child executable points to the PM-managed Mihomo binary. A PID file alone is insufficient.
- Require the foreground service's proxy owner count to increase. Do not infer successful routing from that notification.
- Wait for the authenticated loopback controller, then confirm `/version`, `/configs` and the `QA` group's current selection. A wrong secret should be rejected; never send the secret to a nonloopback address.
- For rule/global/direct, call `setMode` and confirm the controller's returned mode. These are changes to this isolated proxy, not to Android's global proxy.
- Return to rule mode. With `QA=DIRECT`, send one HTTP request through the fixture mixed port to the app's separate loopback echo origin; check a unique response marker. With `QA=REJECT`, the same request must not reach the origin or return its marker. Restore `DIRECT` and verify again. No public internet request is needed.
- Send Workflow to the background through the coordinated UI action. After a short interval, return and confirm the same run UUID/PID/start time is still owned; do not count a replacement run as continuity.
- Call `stop()`. Require the guardian's validated exit record and reap, closed listener ports, removed matching audit record, and proxy foreground-owner count restored to its prior value. Other terminal/Codex owners may legitimately keep the service alive.
- Start and stop once more; require a new run UUID. A reused numeric PID must still have the new start time. Never signal an old recorded PID to "clean up" the test.

## Cleanup and failure checks

Run cleanup in a noncancellable `finally` block. Stop only the fixture's owned guardian. Delete its directory only after confirmed exit; if shutdown is indeterminate, preserve the identity/log record and report `stopUnconfirmed`.

A separate controlled guardian fixture can test stdin EOF, malformed/stale stop controls, and a forced-stop timeout with a test child. Do not SIGKILL a real TUN process to infer graceful route restoration, and do not use `pkill mihomo`, kill-by-name, or unconditional kill of a saved PID.

Afterward compare the production profile hash and the semantic network baseline. Existing VPN interfaces/owner, routes/rules, proxy and DNS policy must remain intact. Ignore merely expiring cache/lease counters when comparing snapshots; explain any actual interface/policy change before continuing.

## Remaining boundaries

This verifies root process ownership, loopback REST controls, controlled proxy forwarding, background lifetime and cleanup. Actual TUN/interface routing is a separate coordinated pass after assessing the current VPN/network. The manager still reads the original configuration path at kernel startup; an edit after its last digest check is not covered by an immutable per-run snapshot.
