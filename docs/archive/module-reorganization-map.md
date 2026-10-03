# Module reorganization document map

The [JSON manifest](module-reorganization-map.json) uses the existing relocation schema: repository-relative source/destination paths, operation/category, verification and totals. Each preserved original has its byte count and SHA-256; `maintained_destinations` identifies the replacement references. The earlier [relocation manifest](relocations-2026-10-03.json) is unchanged.

| Former maintained reference | Current reference |
| --- | --- |
| `docs/implementation/README.md`, `architecture.md` | [Engine](../engine/README.md), [App](../app/README.md) |
| `docs/implementation/workspace-engine.md` | [Server](../engine/server.md) |
| `docs/implementation/workspace.md` | [Workspace](../engine/workspace.md), [FileWork](../engine/filework.md) |
| `docs/implementation/environment.md` | [Environment](../engine/environment.md) |
| `docs/implementation/container-runtime.md` | [Runtime](../engine/runtime.md), [Loader](../engine/loader.md) |
| `docs/implementation/agents.md` | [Chat](../engine/chat.md), [Workbench](../app/workbench.md) |
| `docs/implementation/android-client.md` | [Connection](../app/connection.md), [Configuration](../app/configuration.md), [Workbench](../app/workbench.md) |
| `docs/implementation/proxy.md` | [Proxy](../app/proxy.md) |
| `docs/implementation/protocol.md`, `image-format.md` | [Protocol](../engine/protocol.md), [Image format](../engine/image-format.md) |
| `docs/implementation/dependencies.md` | [Dependencies](../development/dependencies.md) |
| Top-level subject navigation stubs | The matching references above, [Product](../product/README.md), [UX](../ux/README.md), or [Testing](../development/testing.md), as recorded per file in the manifest |

[Original implementation references](module-reorganization-2026-10-03/implementation/) and [navigation stubs](module-reorganization-2026-10-03/navigation/) retain their original bytes and historical relative links. They are not maintained guidance. The [original workspace organization report](module-reorganization-2026-10-03/reports/2026-10-03-workspace-organization.md) is also preserved before its live copy's single retired navigation link was redirected to the archived original reference. No report commands, results or historical claims were rewritten.

The current documentation move does not establish implementation or device acceptance. [Status](../status.md) records the partial Rust Chat transition and separately owned runtime extraction; source-bound final evidence belongs in [reports](../report/README.md).
