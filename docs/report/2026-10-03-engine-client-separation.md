# Engine-owned services and client separation

The Android application now contains presentation, Workspace connection management, disposable state projections and Android capability executors. Workspace business state and lifecycle decisions belong to Engine. This change keeps version 1.0.0 and the exact Workspace protocol; it adds no remote transport, host agent fallback or legacy data import.

## Delivered boundary

Rust is the only public Workspace RPC endpoint and persistence authority. It supervises the pure JVM chat service inside the guest environment. The existing Codex and Claude adapters run there, while `agent-model` supplies shared serializable values and projection reducers. Android's production dependency graph no longer contains the adapters, process launchers or conversation-index writer. Snapshots, bounded event replay and process epochs replace the Android-owned chat hub. Raw numeric request IDs and unknown JSON numbers remain lossless across both protocol layers.

Codex, the pinned Linux JRE and chat service are Engine tool payloads. They use ordinary guest executable paths, separate from Android JNI libraries. Rust validates the payload inventory and hashes, measures executable readiness, and manages optional Claude installation. Selecting Claude requests installation from Engine; a failed job requires an explicit retry. Partial network downloads can resume, but only exact size/hash verification and a successful version probe permit publication. Android displays progress and submits commands without release URLs, version constants or installation scripts.

Engine owns terminal identity, processes, title, working directory, retained output, generation and reference cleanup. Client attachment neither creates another shell nor stops one when the view disappears. A no-op environment restart preserves processes. Actual activation stops owned groups and restores terminal resources; chat rehydrates its persistent conversation metadata in a new service/process epoch. Unsupported terminal metadata disables that resource subsystem without preventing file access or overwriting the original record.

External imports use Engine's atomic name allocation and no-replace publication. Android only obtains URI permission, stages at most 512 MiB per item and streams bytes against the originating connection. Proxy configuration generation, secrets, CAS imports, desired state and operation tickets likewise belong to Engine. The Android executor retains root, foreground-service and device-specific safety checks, and can publish measurements only through its current lease. The legacy Android shell/PTY wrappers are instrumentation fixtures and are excluded from Release packaging.

A send is deduplicated by the Engine-owned conversation ID and composer revision, not a disposable client's request UUID. Accepted and ambiguous outcomes survive client recreation; only the exact submitted draft can be acknowledged. Unchanged client saves retain the same revision, and acknowledgements preserve newly typed local content.

## Verification records

Source-bound records are under `artifacts/workflow/runs/`. Each retains its source fingerprint, argv, output and selected test reports. Contributor branches and handoff records retain the independent deliveries; coordinator runs validate their integration.

| Boundary | Recorded result | Evidence |
| --- | --- | --- |
| Rust Server, protocol and lifecycle | 45 tests passed | `coordinator/20261003T100247Z-feb60466` |
| Core, including real host Engine and lossless JSON | 227 tests passed | `coordinator/20261003T090648Z-81930ff7` |
| Proxy configuration/control model | 85 tests passed | `coordinator/20261003T090755Z-e25e36c1` |
| App unit tests, including composer revision regressions | 77 tests passed | `coordinator/20261003T100312Z-20050768` |
| Shared chat model, adapter replay and guest service | 74 tests passed, including durable cross-client send replay | `coordinator/20261003T102141Z-bbe4934a` |
| Immutable x86_64 app/instrumentation APK pair | Built successfully | `coordinator/20261003T100536Z-d36f9928` |
| API 28 x86_64 isolated emulator | 13 tests passed, zero skips | `coordinator/20261003T101001Z-56b70ba1` |

The tested app APK SHA-256 is `6e90c6b475dd995160001c3ae9ccb1eaa4895e1b6312990d0477ec0385309c2a`; the instrumentation APK SHA-256 is `42707a46405573601e7a1b389cbe2ea3160fa706b900972b0fed380b5e7e1d75`. Its complete source fingerprint is `d865e8db33db1395fde204392b4f61b20668cfeece5225d293de217ac8d12cfd`.

The final device run exercised empty file-name cancellation, terminal attachment/restart behavior, connection-bound import, real guest JVM chat startup, Codex account/process state without credentials, client reattachment, chat generation restart, terminal CLI startup, Claude selection/install/initialization, transactional environment changes, Engine proxy configuration and owned TUN lifecycle/cleanup. The daily tablet and its other applications, routes, DNS and settings were not touched.

Both ABI package inspections established that `libcodex.so` is absent from JNI libraries, Codex/Claude adapter classes are absent from Android DEX, and the Engine tools archive matches its inventory digest and architecture. Host guest-JRE probes also exercised threads, file I/O, ProcessBuilder, stdout/stderr, wait and descendant cleanup. Supplemental package/build/probe evidence is retained under `artifacts/engine-client-separation/`. The final ARM64 Debug build and x86_64 lint completed successfully in `final-arm64-lint.log`; lint reported zero errors and 32 warnings.

## Failures investigated rather than hidden

Earlier runs exposed build-input overlap with the new JVM module, retired fixture references, redundant reconciliation when attaching to an already-ready environment, and concurrent home writes during post-script preparation. Each failed run remains in the registry. An APK installation exceeded the emulator driver's timeout before instrumentation began; the subsequent run used an idle build environment. A Claude transfer also failed; the installer now exposes its exit reason and supports bounded resumption, and the final on-demand installation/startup test passed.

The home conflict was traced in an isolated guest to Codex plugin staging and SQLite WAL/shared-memory writes. A copied file must remain stable during its own copy. Later changes to unrelated, already-copied entries no longer invalidate the whole snapshot, while activation still checks every script-modified path against its captured version. Tests separately establish post-script merge/rollback with an active terminal and chat restart across a generation change without a home-changing script. They do not weaken the same-file conflict guard or silently retry it.

## Acceptance limits

No real account authorization or model turn was performed. ARM64 packaging does not establish physical ARM64 or 16 KiB-device execution. The embedded server still owns a stdio transport-bound lifetime: closing that transport stops its processes; this is not a detached server or remote/SSH implementation. Active writes to the same file can legitimately reject a post-script snapshot and require an explicit retry. The final report adds verification context; immutable run records remain the authority for the precise source and APK identity tested.
