# Client service and resource boundary

Android's local services execute device operations; the Workspace Engine owns their durable
configuration and published workspace state. There is no migration or local workspace repository fallback.

## Implemented boundary

- `WorkspaceDocuments` reads/writes opaque Engine documents with safe namespace/key identifiers and
  an expected document revision. Overlay position uses `overlay/position`; grayscale restoration uses
  `device-controls/grayscale-restore`. Restoration is committed before the explicit root action and
  cleared only after actual settings readback succeeds. Overlay writes are serialized, and failures
  remain visible in the settings service-error state. An overlay stops when its connection changes.
- `ProxyService` binds one executor to one connection token. `WorkspaceProxyApi` fetches canonical
  configuration before refresh/validation/start, creates/imports through Engine CAS, and stages only
  Engine-returned bytes under `cache/proxy-executors/<profile>/`. Unchanged configuration does not
  rewrite staging or invalidate the last-live-config stamp. A failed Engine read cannot start from cache.
  File-provider assets are bounded Engine reads before validation/start/explicit provider refresh
  (16 MiB per file, 64 MiB total, 256 files), with safe relative paths and no stale cache fallback.
  Generic service paths support safe nested asset directories; documents keys remain flat.
- Generic service document mapping: `namespace=services.proxy,key=config.yaml` is the same
  `.workspace/services/proxy/config.yaml` that the editor opens; `key=runtime.log` maps the bounded,
  redacted log copy. The Rust implementation stores hash/revision metadata rather than a second copy.
  Editor saves invalidate document CAS. No new proxy-specific Engine method is needed.
- Measured proxy state crosses `services.report {serviceId:proxy,state}` before UI publication. The
  client verifies the returned service ID and state. It does not report config credentials, subscriptions
  or test URLs. Failed acknowledgement disables start/controller UI and labels the state unconfirmed.
  Local controller/root/TUN checks remain in the existing runtime. Config imports never imply a running kernel.
- Disconnect closes only the owned guardian pipe, preventing later starts; no disconnect root probe or
  PID signal is added. Existing post-root `ip -o link show`, foreign-rule checks and native owned-rule
  cleanup are unchanged. The runtime remains alive long enough to observe guardian exit and release its lease.
- `WorkspaceResourceCache` reads Engine `files.read` chunks of at most 64 KiB into transient cache,
  validating offsets, progress, EOF, stable size and per-file limits. Image previews cap input at 32 MiB;
  open/share caps at 64 MiB. The cache has a 256 MiB total budget and reclaims one-hour-old copies;
  recent shared copies are retained and new copies fail if the budget cannot hold them. Failure or
  cancellation removes incomplete copies. Image decode remains off-main and downsampled to 4096 px;
  its temporary copy is deleted immediately after decoding.
- `WorkspaceIntents.openWith/share` are suspend functions. Call sites in ImagePanel,
  TextEditorController and FilesExplorer launch them from their panel scope. FileProvider now exposes
  only `cache/workspace-resources` plus the existing capture cache; it cannot expose the workspace root.
  `WorkspaceLocation.root` is connection bootstrap identity only, never a feature data-access API.

## Integration hooks

Production callers keep `ProxyService.get(context)` and existing `ProxyApi` methods. The service
requires a connected `WorkspaceConnectionManager` session. `ProxyApi.configFile` names the disposable
kernel staging file; editor navigation must continue using `ProxyService.CONFIG_PATH`/`LOG_PATH`.
No changes are needed to AppGraph beyond its existing connected-session gating.

`WorkspaceDocuments(session.rpc)` is the shared generic document helper. No additional local config
cache was added; the connection manager remains the only owner of permitted Engine-delivered config
caching. All service state IO uses RPC, with no direct `<workspace-root>` File access.

## Verification

- Initial `:proxy:test :app:compileEmulatorDebugKotlin` passed (`artifacts/client-services-first-build.log`).
- First full app unit run passed the six new resource cache tests and all proxy tests, while three
  existing rewrite path/index assertions failed outside this workstream: AgentPathsTest and two
  ConversationIndexFileTest cases (`artifacts/client-services-tests.log`). These were handed to integration.
- Final serialized `:proxy:test` plus focused `:app:testEmulatorDebugUnitTest` passed:
  **83/83 proxy tests** (including 8 workspace-boundary, 2 asset-path and 2 owned-channel disconnect
  cases) and **11/11 app tests** (6 resource-cache, 4 Engine RPC/report, 1 overlay clamp), zero skipped.
  Production Kotlin also compiled in this run. Evidence: `artifacts/client-services-verified.log` and
  the module JUnit XMLs. Report tests cover field-order/integer-width echo validation, mismatched
  receipts, generic service namespace/CAS and exclusion of config credentials/subscription/test URLs.
- `:app:compileEmulatorDebugAndroidTestKotlin` reached the existing platform/engine
  `EngineIntegrationTest` references to removed `EngineCli`, `GuestAgents` and `ClaudeCodeInstaller`;
  that separate integration fixture must be rewritten before a full instrumentation APK can compile
  (`artifacts/client-services-final-app.log`). Owned proxy/overlay test sources had no compiler errors.

Device acceptance is separate. This change did not touch the daily tablet, host routes/DNS or Android
system settings. Prior guardian/TUN device evidence remains in `docs/report/initial/w9-proxy.md`; the
new Engine/service UI wiring requires the integrated disposable-emulator acceptance run.
