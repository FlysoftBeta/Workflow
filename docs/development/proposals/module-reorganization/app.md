# App layout, configuration and documentation

Status: decided; App relocation and documentation are round-2 work, with integrated acceptance pending. Updated: 2026-10-03.

The [target architecture](../module-reorganization.md) leaves Android responsible for connections, local Engine hosting, configuration synchronization, the Workbench and local capabilities. The module move preserves Kotlin package and wire identities. The Kotlin Chat service remains the production path while Rust parity and isolated device acceptance are incomplete.

## Target modules and synchronization

Use `app/android/` for the Android application, Compose features, platform adapters and instrumentation; `app/client/` for pure JVM typed protocol bindings, JSON-RPC transport, connection abstractions, disposable projections and configuration synchronization; and `app/proxy/` for the pure JVM proxy executor model and controller. App-owned native PTY fixtures and the proxy guardian move to `app/native/`; offline xterm sources/tests move to `app/web/`, with packaged assets remaining in the Android application. `core` client contracts move to `app/client`; obsolete reference persistence stays test-only or is archived. `agent` and the Engine JVM service are deleted only after Rust Chat parity and device acceptance. `agent-model` presentation types move to the typed client, then its reducer is replaced by projection changes in the coordinated Chat follow-up. Feature packages continue to communicate through App interfaces.

Connection profiles remain the only client-authored durable source. Workspace-delivered appearance, fonts, terminal preferences and service configuration may be cached by connection/workspace identity and revision. Initial connection loads `hello`, the authoritative snapshot and `client.config`; reconnect replaces stale projections. Writes carry the last observed revision. A conflict first updates the projection, then reapplies the user's intended transformation; it never retries an unchanged stale whole document. Proxy canonical configuration is `.workspace/proxy/`, retaining the `services.proxy` document namespace with no migration; it is accessed through revisioned document APIs; boot-bound executor epochs and operation receipts prevent replaying an unknown local action. Credentials are not logged or copied into fixtures. Engine-private state never enters Android's durable cache.

Environment owns the configuration and service domain types and behavior, including appearance, overlay/client settings and proxy intent/receipts. Chat owns agent/backend defaults; Terminal owns terminal configuration. Server composes their existing serde shapes into the wire contract. App synchronization mirrors these owners rather than depending on Server-only domain state.

The final Gradle projects are `:app:android`, `:app:client` and `:app:proxy`. `core` and `agent-model` are merged into the client project, while their Kotlin package names remain unchanged initially. The temporary JVM `:agent` and `:engine-chat` projects that depended on `:app:client` were deleted on 2026-10-04; Android production never depended on vendor execution.

## Terminal repairs

The verified baseline has these mechanisms but still has reported usability defects. Device evidence is needed to determine all failure conditions; this table distinguishes concrete code weaknesses from hypotheses.

| Defect | Existing mechanism and likely failure boundary | Required client change |
| --- | --- | --- |
| Open links | `terminal.js` computes buffer-cell hit regions and invokes `TerminalWebView.openUrl`; `TerminalPanel` permits HTTP(S) and launches an intent. Touch position, wrapped lines, asynchronous validation and viewport movement can invalidate the hit target. | Test tap-versus-scroll arbitration, refreshed wrapped-line geometry, URL schemes, disabled bridges and missing handlers on API 28; retain offline assets. |
| Open files | `TerminalPanel.resolve` parses the output, computes candidates from cached cwd and searches parent directory listings client-side. That duplicates Engine path policy and can use stale cwd. | Call typed Engine resolution with terminal generation; open returned workspace paths/cursors or reveal a directory. Invalidate link results on generation/cwd changes. |
| Scrollbar | Styling hides the horizontal scrollbar but does not supply a large touch thumb or explicit drag-to-row behavior. Desktop xterm scrollbar geometry is insufficient evidence of touch usability. | Add a minimum touch target with drag capture, visible large thumb, proportional row mapping, fast scrolling and streaming viewport preservation. |
| Text selection | Long press calls xterm selection and a Compose copy/paste popup; native textarea selection is suppressed. There are no visible independently draggable selection handles. | Provide two handles, correct buffer/cell mapping, edge autoscroll, cancel/selection state arbitration and stable copy text across wrapped/Unicode rows. |

No replacement terminal renderer or host PTY backend is introduced. Link hit testing, selection, accessibility, keyboard input and scroll dragging must be tested together rather than declaring success from isolated JavaScript tests.

## Full documentation relocation map

The maintained documentation move is included in round 2. The [JSON relocation manifest](../../../archive/module-reorganization-map.json) retains the repository's existing map format and identifies preserved originals and current destinations. Historical text is never rewritten as current guidance.

| Retired or retained location | Destination/action |
| --- | --- |
| `docs/product/**`, `docs/ux/**` | Keep, update source links and clarified ownership |
| `implementation/architecture.md` | `docs/engine/README.md` and `docs/app/README.md`, split shared boundary narrative |
| `implementation/workspace-engine.md` | `docs/engine/server.md`, transaction portions linked to domain pages |
| `implementation/protocol.md` | `docs/engine/protocol.md`, links to generated schema/goldens |
| `implementation/workspace.md` | `docs/engine/workspace.md` and `docs/engine/filework.md` |
| `implementation/environment.md` | `docs/engine/environment.md` |
| `implementation/container-runtime.md` | `docs/engine/runtime.md` and `docs/engine/loader.md` |
| `implementation/image-format.md` | `docs/engine/image-format.md`, linked from Environment |
| `implementation/agents.md` | `docs/engine/chat.md`; App presentation text to `docs/app/workbench.md` |
| Terminal implementation sections | `docs/engine/terminal.md` and `docs/app/workbench.md` |
| `implementation/android-client.md` | `docs/app/connection.md`, `docs/app/configuration.md`, `docs/app/workbench.md` |
| `implementation/proxy.md` | `docs/app/proxy.md`, Engine desired-state contract linked to protocol |
| `implementation/dependencies.md` | `docs/development/dependencies.md`, grouped by actual Engine/App owner |
| `implementation/README.md` | Retire after inbound links point at the two new indexes |
| `docs/development/**`, `docs/status.md`, `docs/report/**` | Keep their roles; reports record exact source/artifacts and acceptance limits |
| `docs/archive/**` | Preserve; add `docs/archive/module-reorganization-map.json` with old-to-new paths and a short Markdown catalog |
| Top-level stubs `agents`, `architecture`, `container-runtime`, `dependencies`, `engine`, `environment`, `product`, `protocol`, `proxy`, `testing`, `ui`, `workspace-engine`, `workspace` | Update all inbound links, record destinations in the relocation map, then remove stubs |
| Root README, AGENTS, module READMEs and proposal links | Update navigation to final Engine/App/development indexes in the same documentation change |

## Later rounds: delivery tasks

| Task | Owned paths | Dependencies | Checks |
| --- | --- | --- | --- |
| R2 App module move | `app/**`, `core/**`, `proxy/**`, `agent-model/**`, root Gradle includes/catalog, `native/**`, `web/**` | Frozen Engine catalog; coordinate Chat-owned models | `client`, `proxy`, `app-unit`, `web`, `native`, `lint`, both ABI APK builds |
| R2 typed bindings/config sync | `app/client/**`, configuration/connection platform adapters and tests | Module move; Rust schema and fixtures | Codec/schema goldens, concurrent revision conflict and reconnect tests; isolated connection/proxy device cases |
| R2 terminal repairs | `app/android/**/terminal/**`, `app/web/**` | Server path-resolution API | Offline tests and API 28 touch acceptance for all four defects, rotation and streaming |
| R2 Chat deletion (done 2026-10-04, before guest/device acceptance by user decision) | Client chat bindings, old JVM module includes and fixtures | Rust adapter/service parity; guest/device acceptance still pending | Chat/client suites |
| R2 docs reorganization | Entire maintained `docs/`, root/module READMEs and AGENTS | Integrated Engine/App layout | `documentation`, navigation audit, archive map, accurate status/report matrix |
| Integrated acceptance | Integration tests and reports, no new behavior | R2 complete | Source-bound host checks, both ABI builds, API 28 isolated AVD; ARM64/device gaps explicitly recorded |
