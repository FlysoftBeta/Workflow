# Workspace Engine

The Engine is the authority for sessions, panel layout, files, drafts, configuration, the complete execution environment and published service state. Android is a [connection and presentation client](../app/README.md). Its workspace state is a disposable projection of an Engine snapshot, so reconnecting never creates a second writer. The [product](../product/README.md) and [UX](../ux/README.md) documents define supported behavior and presentation; the [protocol](protocol.md) defines the client boundary.

## Modules and dependencies

`engine/` is one Cargo workspace. `workflow-server` composes the domain crates and owns JSON-RPC, transport and cross-domain transactions. Domain crates expose typed APIs and depend on `workflow-environment`, not on each other or on Server wire methods. Server can export their serde data types without transferring protocol ownership to them.

| Source and package | Responsibility | Reference |
| --- | --- | --- |
| `engine/server`, `workflow-server` | Transport, typed protocol, subscriptions and domain composition; binary `workflow-engine` | [Server](server.md) |
| `engine/workspace`, `workflow-workspace` | Sessions, panel layouts, view state and paradigm selection | [Workspace](workspace.md) |
| `engine/filework`, `workflow-filework` | Workspace files, revisions, drafts, imports, diff and archive preflight | [FileWork](filework.md) |
| `engine/terminal`, `workflow-terminal` | Terminal resources, settings, PTY attachment, generation and cwd | [Terminal](terminal.md) |
| `engine/chat`, `workflow-chat` | Rust conversation domain, Codex and Claude adapters and chat service, linked into Server | [Chat](chat.md) |
| `engine/environment`, `workflow-environment` | Private typed storage, configuration, service intent/receipts, images, tools, lifecycle and guest process API | [Environment](environment.md) |
| `engine/environment/runtime`, `workflow-runtime` | Ptrace isolation executable and its platform implementation | [Runtime](runtime.md) |
| `engine/environment/loader`, `workflow-loader` | Freestanding guest ELF loader | [Loader](loader.md) |

Server runs the in-process Rust Chat service, the only chat implementation. The former Kotlin `:engine-chat` service, Engine-only `:agent` adapters, their JAR and the Linux JRE were deleted before isolated API 28 guest/device acceptance of the Rust chat, which is still pending; host parity tests do not establish it. Client presentation types belong to `:app:client`.

Environment owns `.workspace/` path construction, typed atomic publication, image and tool lifecycle, workspace-delivered appearance/client configuration, and service desired state and executor receipts. Agent/backend defaults belong to Chat, and terminal settings belong to Terminal. Server retains the combined transaction and maps domain data to the exported contract. Ordinary workspace-file IO belongs to FileWork; guest execution goes through Environment's runtime API. Known data uses typed models; explicitly opaque vendor content and unknown fields remain lossless.

## Workspace storage

The user chooses a root that appears as `/workspace` in the environment. User files live directly beneath it. Engine configuration and data live in its `.workspace/` directory, which holds only configuration and persistent data at its top level; everything that can be rebuilt sits in `cache/`:

```text
<workspace-root>/
  .workspace/
    config.json                 revisioned configuration, including the `environment` declaration
    proxy/                      canonical proxy YAML, providers and redacted logs
    services/<serviceId>/        other explicitly editable service files
    agents/{codex,claude}/       agent homes: configuration, credentials and sessions
    state/workspace.json         sessions, layouts, file and composer drafts
    state/terminals.json         terminal identities and metadata
    state/services/              measured service reports
    state/local-services/        desired state and executor receipts
    environment/                 lifecycle record and the persistent guest home store
    documents/                   typed or opaque documents and revision sidecars
    trash/                       reversible deletions
    corrupt/                     original data retained during recovery
    cache/                       disposable: tools, image archives, generations, toolchains,
                                 resolver and incomplete uploads
```

The explorer shows `.workspace` as a protected folder: Environment's typed allowlist (`access.rs`) marks editable configuration and hides private state and the cache, and FileWork and Workspace layout enforce it, so panels can open allowed files such as `.workspace/proxy/` while the file API rejects private state. Environment supplies the guest mask for private state and the cache. Deleting `cache/` while the Engine is stopped is safe: the tools payload is extracted again from the APK, and an environment whose generation disappeared becomes unavailable and is rebuilt from its declaration once it has been used before. Runtime metadata inside a generation uses `.workflow-engine/`, a separate implementation directory. Version 1.0.0 has no migration or scan for historical layouts: a former `env.json`, `agents/tools/` or generation below `environment/` is ignored, so an existing workspace rebuilds its environment from the `environment` section of `config.json`. The proxy directory is `.workspace/proxy/`; the earlier `.workspace/services/proxy/` location is not imported.

The Server commits session/layout state and Working Resources in one workspace transaction through the Environment store. Keeping their domain ownership separate must not split archive protection into independently acknowledged writes. Atomic publication, backup and quarantine happen before a committed revision is acknowledged.

## Execution and connection lifetime

After the exact-version `hello`, clients load the authoritative snapshot and begin connection-scoped watches. Commands return the committed revision and state; clients replace their projections. Watches, environment builds, process waits and output reads do not retain the workspace transaction lock while they wait. Remote and SSH transports remain abstractions only.

Terminals and coding agents require the verified environment. Environment runs product processes through the distinct runtime and loader executables; Android packages Server, runtime and loader as `libworkflow-engine.so`, `libworkflow-runtime.so` and `libworkflow-loader.so`. The verified tools archive carries guest Codex. Optional Claude installation is Engine-owned. There is no host, Android-shell or Android-JVM fallback.

The embedded stdio Server stops when its transport closes and cleans up its processes. Reconnection reloads committed drafts and metadata but does not imply a detached daemon or uninterrupted process/output survival. Closing a terminal panel only removes a view; Terminal applies reference-based cleanup. Local Android proxy execution is outside the guest, with Environment-owned intent and measured receipts described in the [App proxy reference](../app/proxy.md).

The runtime is one shared source tree; genuine Linux/Android and x86_64/aarch64 differences live in small `cfg` items, checked against the retained originals by `engine/environment/runtime/tools/check-platform-merge.py`. The merge passed `runtime-host`, Engine builds for both ABIs and isolated API 28 x86_64 guest exec, PTY and stop acceptance. ARM64 compilation does not establish ARM64 device acceptance, which remains open.

Build tooling lives in `image/`, `engine/tools/`, and `tools/`; App native adapters and offline terminal assets live under `app/`. Archived C code and retired WebView chat are outside production build inputs. [Status](../status.md) and [testing](../development/testing.md) distinguish recorded acceptance from pending work.
