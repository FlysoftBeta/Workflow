# Engine-owned local-service acceptance

The proxy instrumentation fixture now explicitly connects a new embedded workspace with
`WorkspaceConnectionManager`. It starts the packaged Rust `libworkflow-engine.so`, then wraps the
real disposable `ProxyRuntime` with production `EngineProxyWorkspace` and `WorkspaceProxyApi`.
Only root process transport is injected through the emulator-only `ShellRoot` bridge. The panels use
`RemoteWorkspaceStore`; no reference store or raw runtime bypass supplies their workspace state.

Canonical proxy YAML is written through document CAS and compared with Engine `files.read`;
provider assets are created with chunked Engine upload. Editor saves update the same canonical
configuration/provider files. Tests check stale document CAS rejection, fresh executor staging,
provider refresh, redacted Engine log readback, and published `services.status` measurements.
No assertion prints configuration bytes or controller secrets.

The existing root test retains its three gates (explicit instrumentation argument,
ranchu/goldfish hardware, shell UID 0). It checks actual TUN capture, controller operations,
occupied-table rejection, foreign-TUN rejection, explicit stop, EOF, kernel death, owned-rule cleanup,
foreign-rule preservation, and foreground lease release. It additionally disconnects the actual
Engine session while the proxy runs, checks owned process/TUN/rule cleanup, and rejects a subsequent
start from cached configuration. A separate host observer exercises actual app force-stop.

Device-control tests explicitly connect the packaged Engine before actions. Overlay dragging reads
its position from Engine documents, restores it across service and Engine process restarts, and
checks that connection loss stops the actual overlay service. Root availability and grayscale success
remain based on observed device outcomes; emulator `adb root` does not grant app `su` access.

## Evidence

Acceptance build and emulator results are recorded under `artifacts/service-acceptance/`.
The isolated API 28 x86_64 AVD uses `tools/with-emulator.sh`, port 5896, 1536 MiB, disk-backed
`ANDROID_TMP`, and explicit serial selection for every root/device command. No daily tablet is used.

Run on 2026-10-03 (Asia/Hong_Kong):

- `build-v1.log`: `:app:assembleEmulatorDebug :app:assembleEmulatorDebugAndroidTest` succeeded.
  Both APK copies were made under the same `artifacts/.gradle.lock` lease. Exact SHA-256 values
  are in `apk/SHA256SUMS` and were checked after execution.
- `instrumentation-v1.log`: **15/15 passed, zero failures, zero skips**, 73.193 seconds:
  `ProxyExecutableTest` 2, `ProxyEmulatorRootTest` 1, `ProxyPanelEmulatorTest` 4,
  `DeviceControlsEmulatorTest` 4, `OverlayEmulatorTest` 4.
- `root-result-v1.json`: canonical YAML/document revision 1, Engine provider upload and refresh,
  `services.status` confirmed running/controller/TUN routing, real TEST-NET-2 reject capture,
  both providers refreshed, and empty owned rule/interface checks after stop, EOF, kernel death,
  restart and Engine disconnect. The root scenario asserted its complete routing snapshot restored.
- `app-death-result.txt`: separate host observer passed after actual `am force-stop`.
  App PID 4665, guardian PID 4846 and Mihomo PID 4850 disappeared; `wf-qa-tun` and owned
  IPv4/IPv6 rules disappeared; foreign sentinel rules at 9505 and 9700 survived and were then
  explicitly removed by the isolated test harness. The interrupted instrumentation process is an
  intentional part of this observer scenario, not an additional normally completed JUnit test.
- `screenshots/proxy/`: 16 actual API 28 screenshots; `screenshots/device/`: four overlay
  screenshots and measured device-results JSON. Visual inspection of latency colours/timeouts,
  connections, dark solo proxy, inline lock denial and rotated overlay found readable controls and
  no clipping. Overlay screenshots retain Android's Home-app chooser behind the window.
- `rules-after.txt` and `links-after.txt`: no test TUN or test-owned rule range remained after the
  suite. Android's own radio rules changed during the longer UI/device suite, so this evidence does
  not claim byte-for-byte equality of the whole system route snapshot across all 15 tests.

The device-control result is precise: brightness permission denial and granted value 91 were read
back; device-admin lock was observed noninteractive and its grant removed. App `su` was unavailable,
so grayscale failed with the physical system value unchanged and the revoked-lock test displayed its
inline failure. Successful OEM `su` authorization/grayscale restoration is still a separate device
matrix item. These API 28 x86_64 results do not establish ARM64, API 29+, 16 KiB-page devices,
remote transport or real provider-account acceptance. No production service implementation was
changed in this acceptance work.

## Overlay rotation acceptance follow-up

The integration matrix later exposed a failure in the overlay restart position assertion. This was
investigated separately from the Engine's eager environment-monitor extraction problem. The same
failure reproduced on a fresh AVD with the lazy-monitor APK and ample free storage, so storage
pressure alone does not explain it.

The diagnostic run in `artifacts/service-acceptance/overlay-regression/final3-v1/` recorded the test's
node bounds before restart as `Rect(-36, 864 - 42, 942)`, and after restart as
`Rect(-36, 1475 - 42, 1553)`. Simultaneous WindowManager snapshots show the original portrait window
`113f3bf` already at `[-36,1475][42,1553]` throughout samples 2–7, and the recreated window `bdbe09e`
at the identical frame in samples 8–11. The saved Engine document retained the same left edge and
vertical fraction `0.8488121032714844`. The actual portrait screenshot also shows the lower position.
Thus the physical window did not move on restart; the API 28 accessibility node location remained
stale at its landscape position. Waiting for accessibility idle and refreshing the node did not
correct that location, as the subsequent diagnostic runs demonstrate.

`OverlayEmulatorTest` now samples the actual overlay `AccessibilityWindowInfo` bounds, matched by
window ID, and waits for them to settle. It retains the same left-edge and less-than-eight-pixel
restart assertions, plus Engine document checks and Engine reconnect. Failure messages now include
both rectangles, the nonsecret saved position, and any service failure. No production overlay geometry
was changed. The separate reconnect guard in `restoreIfEnabled` was implemented by integration and
is included in the final acceptance APK.

Final follow-up evidence is in `artifacts/service-acceptance/overlay-regression/final6-v1/`:

- Current source APK pair: `overlay-regression/apk-v6/`, built and copied under the shared lock;
  `apk-v6/SHA256SUMS` records both artifacts. `build-v6.log` succeeded.
- `instrumentation-v1.log`: **15/15 passed, zero skips**, 70 seconds, after the same display override
  sequence as the full matrix: 1920×1080/306 dpi → 1920×1200/261 dpi → 800×1280/320 dpi →
  1920×1200/261 dpi.
- `repeat-1.log` through `repeat-5.log`: **five consecutive passes** of the formerly failing
  rotation/service-restart/Engine-reconnect test on the same disposable AVD; zero skips.
  `acceptance.json` records all 20 successful JUnit executions.
- The actual app force-stop observer passed again: app PID 4785, guardian PID 4964 and kernel
  PID 4968 exited; the TUN and owned IPv4/IPv6 rules disappeared; both foreign sentinels survived.
- `/data` retained 4.2 GiB free after the full service suite. The daily tablet remained untouched.

This replaces the earlier failed integration overlay result with measured acceptance of the corrected
instrumentation. The test now checks the visible window itself; no tolerance was widened and no
proxy, Engine-state or overlay persistence assertion was removed.

Both final acceptance APK files are byte-identical to integration's
`artifacts/rewrite-integration/apk-final4/{app.apk,test.apk}`. App SHA-256:
`06a87289ce1c45be189af170f27c57fa77aa342f87ae1a48d043d533802af00a`;
instrumentation SHA-256:
`39289f5a73f012e58b1f566dd1c96ed33d4e7de9b8018a05bb82c869a85cd1e4`.
